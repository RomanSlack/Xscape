use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use xscape_common::{ApiError, BuildRequest, BuildResponse, BuildStatus, BuildStatusResponse};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::server::AppState;
use crate::xcode;

/// POST /build - Start a new build
pub async fn start_build(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuildRequest>,
) -> Result<Json<BuildResponse>, (StatusCode, Json<ApiError>)> {
    // Verify project exists
    let project = state.get_project(&request.project_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found("Project", &request.project_id.to_string())),
        )
    })?;

    info!(
        "Starting build for project '{}' (scheme: {}, config: {:?})",
        project.project_name, request.scheme, request.configuration
    );

    let build_id = Uuid::new_v4();
    let started_at = Utc::now();

    // Create initial build status
    let build_status = BuildStatusResponse {
        build_id,
        project_id: request.project_id,
        scheme: request.scheme.clone(),
        status: BuildStatus::Queued,
        started_at,
        finished_at: None,
        app_path: None,
        bundle_id: None,
        error_message: None,
        warnings: Vec::new(),
        duration_secs: None,
    };
    state.store_build(build_status).await;

    // Create log channel for this build
    let log_sender = state.create_log_channel(build_id).await;

    // Spawn build task
    let state_clone = state.clone();
    let project_path = project.path.clone();
    tokio::spawn(async move {
        let result = xcode::run_build(
            &project_path,
            &request,
            log_sender,
        )
        .await;

        // Update build status based on result
        let mut build_status = state_clone.get_build(&build_id).await.unwrap();
        let finished_at = Utc::now();
        build_status.finished_at = Some(finished_at);
        build_status.duration_secs = Some(
            (finished_at - build_status.started_at).num_milliseconds() as f64 / 1000.0,
        );

        match result {
            Ok(artifacts) => {
                info!("Build {} succeeded: {:?}", build_id, artifacts.app_path);
                build_status.status = BuildStatus::Succeeded;
                build_status.app_path = Some(artifacts.app_path.clone());
                build_status.bundle_id = artifacts.bundle_id.clone();
                build_status.warnings = artifacts.warnings.clone();
                state_clone.store_artifacts(build_id, artifacts).await;
            }
            Err(e) => {
                error!("Build {} failed: {}", build_id, e);
                build_status.status = BuildStatus::Failed;
                build_status.error_message = Some(e.to_string());
            }
        }

        state_clone.store_build(build_status).await;
    });

    Ok(Json(BuildResponse {
        build_id,
        status: BuildStatus::Queued,
        started_at,
    }))
}

/// GET /build/{build_id} - Get build status
pub async fn get_build_status(
    State(state): State<Arc<AppState>>,
    Path(build_id): Path<Uuid>,
) -> Result<Json<BuildStatusResponse>, (StatusCode, Json<ApiError>)> {
    let build = state.get_build(&build_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found("Build", &build_id.to_string())),
        )
    })?;

    Ok(Json(build))
}
