use anyhow::{anyhow, Context, Result};
use xscape_common::{SimulatorDevice, SimulatorRuntime, SimulatorState};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

/// Raw simctl JSON output structures for devices
#[derive(Debug, Deserialize)]
struct SimctlDeviceList {
    devices: HashMap<String, Vec<SimctlDevice>>,
}

/// Raw simctl JSON output structures for runtimes
#[derive(Debug, Deserialize)]
struct SimctlRuntimeList {
    runtimes: Vec<SimctlRuntime>,
}

#[derive(Debug, Deserialize)]
struct SimctlDevice {
    #[serde(default)]
    udid: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "deviceTypeIdentifier", default)]
    device_type_identifier: Option<String>,
    #[serde(default)]
    state: String,
    #[serde(rename = "isAvailable", default)]
    is_available: Option<bool>,
    #[serde(rename = "availabilityError", default)]
    availability_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SimctlRuntime {
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "buildversion", default)]
    build_version: Option<String>,
    #[serde(rename = "isAvailable", default)]
    is_available: bool,
}

/// List all simulator devices
pub async fn list_devices() -> Result<Vec<SimulatorDevice>> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "--json"])
        .output()
        .await
        .context("Failed to run simctl list devices")?;

    if !output.status.success() {
        return Err(anyhow!(
            "simctl list devices failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let list: SimctlDeviceList = serde_json::from_slice(&output.stdout)
        .context("Failed to parse simctl JSON output")?;

    let mut devices = Vec::new();
    for (runtime_id, runtime_devices) in list.devices {
        // Extract human-readable runtime name
        let runtime_name = runtime_id
            .replace("com.apple.CoreSimulator.SimRuntime.", "")
            .replace('-', " ")
            .replace('.', " ");

        for device in runtime_devices {
            devices.push(SimulatorDevice {
                udid: device.udid,
                name: device.name,
                device_type_identifier: device.device_type_identifier.unwrap_or_default(),
                runtime_identifier: runtime_id.clone(),
                runtime: runtime_name.clone(),
                state: parse_state(&device.state),
                is_available: device.is_available.unwrap_or(true),
            });
        }
    }

    Ok(devices)
}

/// List available runtimes
pub async fn list_runtimes() -> Result<Vec<SimulatorRuntime>> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "runtimes", "--json"])
        .output()
        .await
        .context("Failed to run simctl list runtimes")?;

    if !output.status.success() {
        return Err(anyhow!(
            "simctl list runtimes failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let list: SimctlRuntimeList = serde_json::from_slice(&output.stdout)
        .context("Failed to parse simctl JSON output")?;

    let runtimes = list
        .runtimes
        .into_iter()
        .map(|r| SimulatorRuntime {
            identifier: r.identifier,
            name: r.name,
            version: r.version,
            build_version: r.build_version.unwrap_or_default(),
            is_available: r.is_available,
        })
        .collect();

    Ok(runtimes)
}

/// Boot a simulator device
pub async fn boot_device(udid: &str) -> Result<()> {
    info!("Booting simulator: {}", udid);

    let output = Command::new("xcrun")
        .args(["simctl", "boot", udid])
        .output()
        .await
        .context("Failed to run simctl boot")?;

    // "Unable to boot device in current state: Booted" is not an error
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("Booted") {
            return Err(anyhow!("simctl boot failed: {}", stderr));
        }
    }

    // Wait a moment for boot to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(())
}

/// Shutdown a simulator device
pub async fn shutdown_device(udid: &str) -> Result<()> {
    info!("Shutting down simulator: {}", udid);

    let output = Command::new("xcrun")
        .args(["simctl", "shutdown", udid])
        .output()
        .await
        .context("Failed to run simctl shutdown")?;

    // Ignore "Unable to shutdown device in current state: Shutdown"
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("Shutdown") {
            return Err(anyhow!("simctl shutdown failed: {}", stderr));
        }
    }

    Ok(())
}

/// Install an app on a simulator
pub async fn install_app(udid: &str, app_path: &str) -> Result<()> {
    info!("Installing app {} on simulator {}", app_path, udid);

    let output = Command::new("xcrun")
        .args(["simctl", "install", udid, app_path])
        .output()
        .await
        .context("Failed to run simctl install")?;

    if !output.status.success() {
        return Err(anyhow!(
            "simctl install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Launch an app on a simulator
pub async fn launch_app(
    udid: &str,
    bundle_id: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Option<u32>> {
    info!("Launching app {} on simulator {}", bundle_id, udid);

    let mut cmd = Command::new("xcrun");
    // Use --terminate-running-process to restart if already running
    // Don't use --console-pty as it blocks waiting for the app
    cmd.args(["simctl", "launch", "--terminate-running-process", udid, bundle_id]);

    // Add app arguments
    for arg in args {
        cmd.arg(arg);
    }

    // Add environment variables
    for (key, value) in env {
        cmd.env(key, value);
    }

    let output = cmd.output().await.context("Failed to run simctl launch")?;

    if !output.status.success() {
        return Err(anyhow!(
            "simctl launch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Try to parse PID from output (format: "com.example.App: 12345")
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pid = stdout
        .lines()
        .find_map(|line| {
            if line.contains(bundle_id) {
                line.split_whitespace()
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
            } else {
                None
            }
        });

    Ok(pid)
}

/// Terminate a running app
pub async fn terminate_app(udid: &str, bundle_id: &str) -> Result<()> {
    info!("Terminating app {} on simulator {}", bundle_id, udid);

    let output = Command::new("xcrun")
        .args(["simctl", "terminate", udid, bundle_id])
        .output()
        .await
        .context("Failed to run simctl terminate")?;

    // Don't fail if app wasn't running
    if !output.status.success() {
        debug!(
            "simctl terminate returned non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Uninstall an app from a simulator
pub async fn uninstall_app(udid: &str, bundle_id: &str) -> Result<()> {
    info!("Uninstalling app {} from simulator {}", bundle_id, udid);

    let output = Command::new("xcrun")
        .args(["simctl", "uninstall", udid, bundle_id])
        .output()
        .await
        .context("Failed to run simctl uninstall")?;

    if !output.status.success() {
        return Err(anyhow!(
            "simctl uninstall failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Get device by name (finds first matching available device)
pub async fn find_device_by_name(name: &str) -> Result<SimulatorDevice> {
    let devices = list_devices().await?;

    devices
        .into_iter()
        .find(|d| d.name.to_lowercase().contains(&name.to_lowercase()) && d.is_available)
        .ok_or_else(|| anyhow!("No available simulator found matching '{}'", name))
}

/// Parse state string to enum
fn parse_state(state: &str) -> SimulatorState {
    match state.to_lowercase().as_str() {
        "booted" => SimulatorState::Booted,
        "booting" => SimulatorState::Booting,
        "shuttingdown" | "shutting down" => SimulatorState::ShuttingDown,
        _ => SimulatorState::Shutdown,
    }
}
