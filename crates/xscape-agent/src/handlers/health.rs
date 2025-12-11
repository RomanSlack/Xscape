use axum::{extract::State, Json};
use xscape_common::{HealthResponse, HealthStatus};
use std::sync::Arc;
use tracing::debug;

use crate::server::AppState;
use crate::simctl;
use crate::xcode;

/// GET /health - Health check endpoint
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    debug!("Health check requested");

    let mut response = HealthResponse {
        status: HealthStatus::Healthy,
        xcode_version: None,
        xcode_path: None,
        available_simulators: 0,
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // Check Xcode
    match xcode::get_xcode_info().await {
        Ok(info) => {
            response.xcode_version = Some(info.version);
            response.xcode_path = Some(info.path);
        }
        Err(e) => {
            debug!("Xcode check failed: {}", e);
            response.status = HealthStatus::Degraded;
        }
    }

    // Check simulators
    match simctl::list_devices().await {
        Ok(devices) => {
            response.available_simulators = devices.len() as u32;
        }
        Err(e) => {
            debug!("Simulator check failed: {}", e);
            if response.status == HealthStatus::Healthy {
                response.status = HealthStatus::Degraded;
            }
        }
    }

    // If Xcode is completely missing, mark as unhealthy
    if response.xcode_version.is_none() {
        response.status = HealthStatus::Unhealthy;
    }

    Json(response)
}
