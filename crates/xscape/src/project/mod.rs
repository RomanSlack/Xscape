use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;
use tar::Builder;
use tracing::{debug, info};

/// Create a tarball of a project directory
/// Returns (tarball_bytes, sha256_checksum)
pub fn create_tarball(
    project_path: &Path,
    exclude_patterns: &[String],
) -> Result<(Vec<u8>, String)> {
    info!("Creating tarball of {:?}", project_path);

    let mut tarball_data = Vec::new();
    let encoder = GzEncoder::new(&mut tarball_data, Compression::default());
    let mut tar = Builder::new(encoder);

    let mut hasher = Sha256::new();
    let mut file_count = 0u32;

    // Walk directory respecting .gitignore
    let walker = WalkBuilder::new(project_path)
        .hidden(false)         // Include hidden files
        .git_ignore(true)      // Respect .gitignore
        .git_global(true)      // Respect global gitignore
        .git_exclude(true)     // Respect .git/info/exclude
        .build();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip directories (tar handles them implicitly)
        if path.is_dir() {
            continue;
        }

        // Get relative path
        let relative = match path.strip_prefix(project_path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check custom exclude patterns
        let relative_str = relative.to_string_lossy();
        if should_exclude(&relative_str, exclude_patterns) {
            debug!("Excluding: {}", relative_str);
            continue;
        }

        // Read file and add to tar
        match std::fs::read(path) {
            Ok(contents) => {
                // Update hash
                hasher.update(&contents);

                // Create tar header
                let mut header = tar::Header::new_gnu();
                header.set_path(relative)?;
                header.set_size(contents.len() as u64);
                header.set_mode(0o644);
                header.set_mtime(
                    std::fs::metadata(path)
                        .and_then(|m| m.modified())
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
                        .unwrap_or(0),
                );
                header.set_cksum();

                tar.append(&header, contents.as_slice())?;
                file_count += 1;
            }
            Err(e) => {
                debug!("Skipping file {:?}: {}", path, e);
            }
        }
    }

    // Finish tar
    let encoder = tar.into_inner()?;
    encoder.finish()?;

    let checksum = format!("{:x}", hasher.finalize());

    info!(
        "Created tarball: {} files, {} bytes, checksum: {}",
        file_count,
        tarball_data.len(),
        &checksum[..8]
    );

    Ok((tarball_data, checksum))
}

/// Check if a path should be excluded
fn should_exclude(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Simple glob matching
        if pattern.starts_with('*') {
            // Suffix match (e.g., "*.xcuserstate")
            let suffix = &pattern[1..];
            if path.ends_with(suffix) {
                return true;
            }
        } else if pattern.ends_with('*') {
            // Prefix match
            let prefix = &pattern[..pattern.len() - 1];
            if path.starts_with(prefix) {
                return true;
            }
        } else {
            // Exact or contains match
            if path.contains(pattern) {
                return true;
            }
        }
    }
    false
}

/// Find the project name from directory
pub fn get_project_name(project_path: &Path) -> String {
    project_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

/// Find Xcode project/workspace in directory
pub fn find_xcode_project(project_path: &Path) -> Option<String> {
    // Look for workspace first
    for entry in std::fs::read_dir(project_path).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "xcworkspace") {
            return Some(path.file_name()?.to_string_lossy().to_string());
        }
    }

    // Fall back to project
    for entry in std::fs::read_dir(project_path).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "xcodeproj") {
            return Some(path.file_name()?.to_string_lossy().to_string());
        }
    }

    None
}
