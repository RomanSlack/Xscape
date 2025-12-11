use anyhow::{anyhow, Context, Result};
use xscape_common::{BuildConfiguration, BuildRequest, LogLevel, LogMessage};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use walkdir::WalkDir;

use crate::storage::BuildArtifacts;

/// Information about Xcode installation
pub struct XcodeInfo {
    pub version: String,
    pub path: String,
}

/// Get Xcode installation info
pub async fn get_xcode_info() -> Result<XcodeInfo> {
    // Get Xcode path
    let output = Command::new("xcode-select")
        .arg("-p")
        .output()
        .await
        .context("Failed to run xcode-select")?;

    if !output.status.success() {
        return Err(anyhow!("xcode-select failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Get Xcode version
    let output = Command::new("xcodebuild")
        .arg("-version")
        .output()
        .await
        .context("Failed to run xcodebuild -version")?;

    if !output.status.success() {
        return Err(anyhow!("xcodebuild -version failed"));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = version_output
        .lines()
        .next()
        .unwrap_or("Unknown")
        .replace("Xcode ", "")
        .to_string();

    Ok(XcodeInfo { version, path })
}

/// Run xcodebuild for a project
pub async fn run_build(
    project_path: &str,
    request: &BuildRequest,
    log_sender: broadcast::Sender<String>,
) -> Result<BuildArtifacts> {
    let project_dir = Path::new(project_path);

    // Find .xcodeproj or .xcworkspace
    let (project_file, is_workspace) = find_xcode_project(project_dir, &request.project_file)?;

    info!("Building {} (workspace: {})", project_file, is_workspace);

    // Build xcodebuild command
    let mut cmd = Command::new("xcodebuild");

    if is_workspace {
        cmd.arg("-workspace").arg(&project_file);
    } else {
        cmd.arg("-project").arg(&project_file);
    }

    cmd.arg("-scheme")
        .arg(&request.scheme)
        .arg("-configuration")
        .arg(request.configuration.to_string())
        .arg("-sdk")
        .arg("iphonesimulator")
        .arg("-destination")
        .arg(request.destination.to_xcodebuild_arg());

    // Add clean if requested
    if request.clean {
        cmd.arg("clean");
    }
    cmd.arg("build");

    // Add extra args
    for arg in &request.extra_args {
        cmd.arg(arg);
    }

    // Set working directory
    cmd.current_dir(project_dir);

    // Capture output
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    debug!("Running xcodebuild: {:?}", cmd);

    // Send build started event
    let _ = log_sender.send(serde_json::to_string(&LogMessage::system_event(
        xscape_common::SystemEventType::BuildStarted,
        format!("Building scheme '{}' for {}", request.scheme, request.destination.device_name),
    ))?);

    let mut child = cmd.spawn().context("Failed to spawn xcodebuild")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Stream stdout
    let log_sender_clone = log_sender.clone();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut warnings = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            // Parse xcodebuild output
            let (level, message) = parse_xcodebuild_line(&line);

            if level == LogLevel::Warning {
                warnings.push(line.clone());
            }

            let log_msg = LogMessage::build_output(level, &line);
            let _ = log_sender_clone.send(serde_json::to_string(&log_msg).unwrap_or_default());
        }

        warnings
    });

    // Stream stderr
    let log_sender_clone = log_sender.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let log_msg = LogMessage::build_output(LogLevel::Error, &line);
            let _ = log_sender_clone.send(serde_json::to_string(&log_msg).unwrap_or_default());
        }
    });

    // Wait for build to complete
    let status = child.wait().await.context("Failed to wait for xcodebuild")?;
    let warnings = stdout_task.await.unwrap_or_default();
    let _ = stderr_task.await;

    if !status.success() {
        let _ = log_sender.send(serde_json::to_string(&LogMessage::system_event(
            xscape_common::SystemEventType::BuildFailed,
            format!("Build failed with exit code: {:?}", status.code()),
        ))?);
        return Err(anyhow!("Build failed with exit code: {:?}", status.code()));
    }

    // Find built app
    let app_path = find_built_app(project_dir, &request.scheme, &request.configuration)?;
    let bundle_id = get_bundle_id(&app_path)?;

    let _ = log_sender.send(serde_json::to_string(&LogMessage::system_event(
        xscape_common::SystemEventType::BuildSucceeded,
        format!("Build succeeded: {}", app_path),
    ))?);

    Ok(BuildArtifacts {
        app_path,
        bundle_id: Some(bundle_id),
        warnings,
    })
}

/// Find .xcodeproj or .xcworkspace in project directory
fn find_xcode_project(project_dir: &Path, specified: &Option<String>) -> Result<(String, bool)> {
    if let Some(file) = specified {
        let path = project_dir.join(file);
        let is_workspace = file.ends_with(".xcworkspace");
        if path.exists() {
            return Ok((path.to_string_lossy().to_string(), is_workspace));
        }
        return Err(anyhow!("Specified project file not found: {}", file));
    }

    // Look for workspace first (preferred)
    for entry in std::fs::read_dir(project_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "xcworkspace") {
            return Ok((path.to_string_lossy().to_string(), true));
        }
    }

    // Fall back to project
    for entry in std::fs::read_dir(project_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "xcodeproj") {
            return Ok((path.to_string_lossy().to_string(), false));
        }
    }

    Err(anyhow!("No .xcodeproj or .xcworkspace found in project directory"))
}

/// Parse xcodebuild output line to determine log level
fn parse_xcodebuild_line(line: &str) -> (LogLevel, String) {
    let line_lower = line.to_lowercase();

    if line_lower.contains("error:") || line_lower.contains("fatal error") {
        (LogLevel::Error, line.to_string())
    } else if line_lower.contains("warning:") {
        (LogLevel::Warning, line.to_string())
    } else if line_lower.contains("note:") {
        (LogLevel::Debug, line.to_string())
    } else {
        (LogLevel::Info, line.to_string())
    }
}

/// Find the built .app in DerivedData
fn find_built_app(project_dir: &Path, scheme: &str, config: &BuildConfiguration) -> Result<String> {
    // Common DerivedData locations
    let derived_data_paths = [
        project_dir.join("DerivedData"),
        project_dir.join("build"),
        dirs::home_dir()
            .unwrap_or_default()
            .join("Library/Developer/Xcode/DerivedData"),
    ];

    let config_str = config.to_string();

    for dd_path in &derived_data_paths {
        if !dd_path.exists() {
            continue;
        }

        // Walk DerivedData looking for the app
        for entry in WalkDir::new(dd_path)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "app") {
                let path_str = path.to_string_lossy();
                // Check if it's in the right build products directory
                if path_str.contains(&config_str) && path_str.contains("iphonesimulator") {
                    return Ok(path.to_string_lossy().to_string());
                }
            }
        }
    }

    Err(anyhow!(
        "Could not find built app for scheme '{}' in DerivedData",
        scheme
    ))
}

/// Get bundle identifier from app's Info.plist
fn get_bundle_id(app_path: &str) -> Result<String> {
    let plist_path = Path::new(app_path).join("Info.plist");

    // Use PlistBuddy to read bundle ID
    let output = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIdentifier", plist_path.to_str().unwrap()])
        .output()
        .context("Failed to read bundle ID from Info.plist")?;

    if !output.status.success() {
        return Err(anyhow!("Failed to read bundle ID: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// Helper for home directory
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}
