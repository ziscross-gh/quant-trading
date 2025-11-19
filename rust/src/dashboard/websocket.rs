//! WebSocket handler for real-time dashboard updates

use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tracing::{error, info, warn};

use super::{state::DashboardState, DashboardUpdate};

/// Handle WebSocket connection
pub async fn handle_socket(socket: WebSocket, state: Arc<DashboardState>) {
    info!("New WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Spawn task to send updates to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            // Serialize update to JSON
            match serde_json::to_string(&update) {
                Ok(json) => {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize update: {}", e);
                }
            }
        }
    });

    // Spawn task to receive messages from client (for ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Handle client messages if needed
                    if text == "ping" {
                        // Could respond with pong
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed");
}

/// Send initial state to newly connected client
async fn send_initial_state(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    state: &Arc<DashboardState>,
) -> Result<(), axum::Error> {
    let data = state.data.read().unwrap();

    let initial_update = DashboardUpdate::HealthUpdate {
        status: "connected".to_string(),
        uptime_seconds: state.uptime_seconds(),
        errors_count: data.errors_count,
    };

    let json = serde_json::to_string(&initial_update).unwrap();
    sender.send(Message::Text(json)).await
}
