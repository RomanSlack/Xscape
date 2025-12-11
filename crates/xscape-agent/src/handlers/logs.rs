use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use crate::server::AppState;

/// GET /logs/{build_id} - WebSocket endpoint for streaming logs
pub async fn logs_websocket(
    State(state): State<Arc<AppState>>,
    Path(build_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, build_id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, build_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    // Get or create log channel for this build
    let log_sender = match state.get_log_sender(&build_id).await {
        Some(s) => s,
        None => {
            // Build doesn't exist or already finished
            let _ = sender
                .send(Message::Text(
                    format!("{{\"error\": \"Build {} not found or already finished\"}}", build_id).into(),
                ))
                .await;
            return;
        }
    };

    let mut log_receiver = log_sender.subscribe();

    debug!("WebSocket connected for build {}", build_id);

    // Spawn task to forward logs to WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(log_line) = log_receiver.recv().await {
            if sender.send(Message::Text(log_line.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (mainly for keepalive pings)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Ping(data)) => {
                    debug!("Received ping, sending pong");
                    // Pong is automatically sent by axum
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket closed by client");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    debug!("WebSocket disconnected for build {}", build_id);
}
