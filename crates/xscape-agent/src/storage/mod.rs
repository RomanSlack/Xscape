use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use xscape_common::StorageConfig;
use std::io::Cursor;
use std::path::Path;
use tar::Archive;
use tracing::{debug, info};
use uuid::Uuid;

/// Build artifacts stored after successful build
#[derive(Debug, Clone)]
pub struct BuildArtifacts {
    /// Path to built .app bundle
    pub app_path: String,
    /// Bundle identifier
    pub bundle_id: Option<String>,
    /// Build warnings
    pub warnings: Vec<String>,
}

/// Initialize storage directories
pub async fn init(config: &StorageConfig) -> Result<()> {
    info!("Initializing storage at {:?}", config.projects_dir);

    tokio::fs::create_dir_all(&config.projects_dir)
        .await
        .context("Failed to create projects directory")?;

    tokio::fs::create_dir_all(&config.logs_dir)
        .await
        .context("Failed to create logs directory")?;

    Ok(())
}

/// Extract a project tarball to storage
/// Returns (extract_path, files_extracted)
pub async fn extract_project(
    config: &StorageConfig,
    project_id: Uuid,
    tarball_data: &[u8],
) -> Result<(String, u32)> {
    let extract_path = config.projects_dir.join(project_id.to_string());

    // Remove existing if present
    if extract_path.exists() {
        tokio::fs::remove_dir_all(&extract_path)
            .await
            .context("Failed to remove existing project directory")?;
    }

    tokio::fs::create_dir_all(&extract_path)
        .await
        .context("Failed to create project directory")?;

    debug!(
        "Extracting {} bytes to {:?}",
        tarball_data.len(),
        extract_path
    );

    // Extract tarball (blocking operation, run in spawn_blocking)
    let extract_path_clone = extract_path.clone();
    let tarball_data = tarball_data.to_vec();

    let files_extracted = tokio::task::spawn_blocking(move || -> Result<u32> {
        let cursor = Cursor::new(tarball_data);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        let mut count = 0u32;
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            // Security: prevent path traversal
            if path.components().any(|c| c == std::path::Component::ParentDir) {
                continue;
            }

            let dest = extract_path_clone.join(&path);

            // Create parent directories if needed
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }

            entry.unpack(&dest)?;
            count += 1;
        }

        Ok(count)
    })
    .await
    .context("Extract task panicked")??;

    info!(
        "Extracted {} files to {}",
        files_extracted,
        extract_path.display()
    );

    Ok((extract_path.to_string_lossy().to_string(), files_extracted))
}

/// Clean up old projects
pub async fn cleanup_old_projects(config: &StorageConfig) -> Result<u32> {
    let cutoff = chrono::Utc::now()
        - chrono::Duration::hours(config.cleanup_after_hours as i64);

    let mut removed = 0u32;

    let mut entries = tokio::fs::read_dir(&config.projects_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        if let Ok(modified) = metadata.modified() {
            let modified: chrono::DateTime<chrono::Utc> = modified.into();
            if modified < cutoff {
                if let Err(e) = tokio::fs::remove_dir_all(entry.path()).await {
                    debug!("Failed to remove old project {:?}: {}", entry.path(), e);
                } else {
                    removed += 1;
                }
            }
        }
    }

    if removed > 0 {
        info!("Cleaned up {} old projects", removed);
    }

    Ok(removed)
}

/// Get total size of projects directory
pub async fn get_storage_size(config: &StorageConfig) -> Result<u64> {
    let path = config.projects_dir.clone();

    tokio::task::spawn_blocking(move || {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        Ok(total)
    })
    .await
    .context("Storage size task panicked")?
}
