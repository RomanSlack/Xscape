use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use xscape_common::{ApiError, ProjectInfo, SyncProjectResponse};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::server::AppState;
use crate::storage;

/// POST /sync-project - Upload and extract project tarball
pub async fn sync_project(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<SyncProjectResponse>, (StatusCode, Json<ApiError>)> {
    let mut project_name: Option<String> = None;
    let mut checksum: Option<String> = None;
    let mut tarball_data: Option<Vec<u8>> = None;

    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request(format!("Invalid multipart data: {}", e))),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "project_name" => {
                project_name = Some(field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ApiError::bad_request(format!("Invalid project_name: {}", e))),
                    )
                })?);
            }
            "checksum" => {
                checksum = Some(field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ApiError::bad_request(format!("Invalid checksum: {}", e))),
                    )
                })?);
            }
            "tarball" => {
                tarball_data = Some(field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ApiError::bad_request(format!("Invalid tarball: {}", e))),
                    )
                })?.to_vec());
            }
            _ => {
                debug!("Ignoring unknown field: {}", name);
            }
        }
    }

    // Validate required fields
    let project_name = project_name.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Missing project_name field")),
        )
    })?;
    let checksum = checksum.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Missing checksum field")),
        )
    })?;
    let tarball_data = tarball_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Missing tarball field")),
        )
    })?;

    info!(
        "Syncing project '{}' ({} bytes, checksum: {})",
        project_name,
        tarball_data.len(),
        &checksum[..8]
    );

    // Check if we already have this exact project (same checksum)
    {
        let projects = state.projects.read().await;
        for project in projects.values() {
            if project.checksum == checksum {
                info!("Project already cached with ID {}", project.project_id);
                return Ok(Json(SyncProjectResponse {
                    project_id: project.project_id,
                    path: project.path.clone(),
                    files_extracted: 0,
                    was_cached: true,
                }));
            }
        }
    }

    // Extract tarball
    let project_id = Uuid::new_v4();
    let (extract_path, files_extracted) = storage::extract_project(
        &state.config.storage,
        project_id,
        &tarball_data,
    )
    .await
    .map_err(|e| {
        error!("Failed to extract project: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to extract project: {}", e))),
        )
    })?;

    // Store project info
    let project_info = ProjectInfo {
        project_id,
        project_name: project_name.clone(),
        checksum: checksum.clone(),
        path: extract_path.clone(),
        synced_at: Utc::now(),
    };
    state.store_project(project_info).await;

    info!(
        "Project '{}' synced successfully: {} files extracted to {}",
        project_name, files_extracted, extract_path
    );

    Ok(Json(SyncProjectResponse {
        project_id,
        path: extract_path,
        files_extracted,
        was_cached: false,
    }))
}
