use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::AppState;
use crate::handlers;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        // Project sync
        .route("/sync-project", post(handlers::sync::sync_project))
        // Build
        .route("/build", post(handlers::build::start_build))
        .route("/build/{build_id}", get(handlers::build::get_build_status))
        // Simulator
        .route("/simulator/list", get(handlers::simulator::list_simulators))
        .route("/simulator/boot", post(handlers::simulator::boot_simulator))
        .route("/simulator/run", post(handlers::simulator::run_app))
        .route(
            "/simulator/shutdown",
            post(handlers::simulator::shutdown_simulator),
        )
        // Logs (WebSocket)
        .route("/logs/{build_id}", get(handlers::logs::logs_websocket))
        // State
        .with_state(state)
}
