mod routes;
mod state;

pub use routes::create_router;
pub use state::AppState;

use anyhow::Result;
use axum::Router;
use xscape_common::AgentServerConfig;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

pub async fn run(config: AgentServerConfig) -> Result<()> {
    let state = AppState::new(config.clone()).await?;
    let state = Arc::new(state);

    let app = create_router(state).layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
