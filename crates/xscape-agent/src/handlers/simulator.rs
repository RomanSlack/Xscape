use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use xscape_common::{
    ApiError, BootSimulatorRequest, BootSimulatorResponse, ListSimulatorsResponse,
    RunAppRequest, RunAppResponse, ShutdownSimulatorRequest, SimulatorState,
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::server::AppState;
use crate::simctl;

/// GET /simulator/list - List available simulators
pub async fn list_simulators(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListSimulatorsResponse>, (StatusCode, Json<ApiError>)> {
    let devices = simctl::list_devices().await.map_err(|e| {
        error!("Failed to list devices: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to list devices: {}", e))),
        )
    })?;

    let runtimes = simctl::list_runtimes().await.map_err(|e| {
        error!("Failed to list runtimes: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to list runtimes: {}", e))),
        )
    })?;

    Ok(Json(ListSimulatorsResponse { devices, runtimes }))
}

/// POST /simulator/boot - Boot a simulator
pub async fn boot_simulator(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<BootSimulatorRequest>,
) -> Result<Json<BootSimulatorResponse>, (StatusCode, Json<ApiError>)> {
    info!("Booting simulator: {}", request.device_udid);

    simctl::boot_device(&request.device_udid).await.map_err(|e| {
        error!("Failed to boot simulator: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to boot simulator: {}", e))),
        )
    })?;

    Ok(Json(BootSimulatorResponse {
        device_udid: request.device_udid,
        state: SimulatorState::Booted,
    }))
}

/// POST /simulator/run - Install and launch app in simulator
pub async fn run_app(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RunAppRequest>,
) -> Result<Json<RunAppResponse>, (StatusCode, Json<ApiError>)> {
    // Get build artifacts
    let artifacts = state.get_artifacts(&request.build_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found("Build", &request.build_id.to_string())),
        )
    })?;

    let bundle_id = artifacts.bundle_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::bad_request("Build has no bundle ID")),
        )
    })?;

    info!(
        "Running app {} on simulator {}",
        bundle_id, request.device_udid
    );

    // Ensure simulator is booted
    let devices = simctl::list_devices().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to list devices: {}", e))),
        )
    })?;

    let device = devices
        .iter()
        .find(|d| d.udid == request.device_udid)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found("Simulator", &request.device_udid)),
            )
        })?;

    if device.state != SimulatorState::Booted {
        simctl::boot_device(&request.device_udid).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to boot simulator: {}", e))),
            )
        })?;
    }

    // Install app
    simctl::install_app(&request.device_udid, &artifacts.app_path)
        .await
        .map_err(|e| {
            error!("Failed to install app: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!("Failed to install app: {}", e))),
            )
        })?;

    // Launch app
    let pid = simctl::launch_app(
        &request.device_udid,
        &bundle_id,
        &request.launch_args,
        &request.environment,
    )
    .await
    .map_err(|e| {
        error!("Failed to launch app: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to launch app: {}", e))),
        )
    })?;

    let session_id = Uuid::new_v4();

    Ok(Json(RunAppResponse {
        session_id,
        bundle_id,
        pid,
        device_udid: request.device_udid,
    }))
}

/// POST /simulator/shutdown - Shutdown a simulator
pub async fn shutdown_simulator(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ShutdownSimulatorRequest>,
) -> Result<Json<BootSimulatorResponse>, (StatusCode, Json<ApiError>)> {
    info!("Shutting down simulator: {}", request.device_udid);

    simctl::shutdown_device(&request.device_udid)
        .await
        .map_err(|e| {
            error!("Failed to shutdown simulator: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(format!(
                    "Failed to shutdown simulator: {}",
                    e
                ))),
            )
        })?;

    Ok(Json(BootSimulatorResponse {
        device_udid: request.device_udid,
        state: SimulatorState::Shutdown,
    }))
}
