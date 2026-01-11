//! WebSocket connection handler

use std::sync::Arc;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::Response,
};
use serde::Deserialize;

use super::events::{ClientMessage, PongMessage, WelcomeMessage};
use super::state::AppState;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsParams {
    /// Optional authentication token
    pub token: Option<String>,
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(_params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // TODO: Validate token if provided
    // if let Some(token) = params.token {
    //     if !validate_token(&token) {
    //         return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    //     }
    // }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // Subscribe to broadcast events
    let mut rx = state.subscribe();

    // Send welcome message with current sequence ID
    let welcome = WelcomeMessage::new(state.current_sequence_id());
    if let Ok(json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(json)).await.is_err() {
            return; // Client disconnected immediately
        }
    }

    loop {
        tokio::select! {
            // Broadcast events to client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client is too slow, they missed events
                        // Send an error message so they know to refresh
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "code": "lagged",
                            "message": format!("Missed {} events, please refresh", n)
                        });
                        let _ = socket.send(Message::Text(error_msg.to_string())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Channel closed
                    }
                }
            }

            // Handle client messages
            result = socket.recv() => {
                match result {
                    Some(Ok(msg)) => {
                        if !handle_client_message(msg, &mut socket).await {
                            break; // Client requested close or error
                        }
                    }
                    Some(Err(_)) => break, // WebSocket error
                    None => break, // Client disconnected
                }
            }
        }
    }
}

/// Handle a message from the client
/// Returns false if the connection should be closed
async fn handle_client_message(msg: Message, socket: &mut WebSocket) -> bool {
    match msg {
        Message::Text(text) => {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Ping => {
                        let pong = PongMessage::default();
                        if let Ok(json) = serde_json::to_string(&pong) {
                            let _ = socket.send(Message::Text(json)).await;
                        }
                    }
                    ClientMessage::Subscribe { channel, filter } => {
                        // TODO: Implement channel filtering
                        // For now, all clients receive all events
                        let _ = (channel, filter);
                    }
                    ClientMessage::Unsubscribe { channel } => {
                        // TODO: Implement channel filtering
                        let _ = channel;
                    }
                }
            }
            true
        }
        Message::Binary(_) => true, // Ignore binary messages
        Message::Ping(data) => {
            let _ = socket.send(Message::Pong(data)).await;
            true
        }
        Message::Pong(_) => true, // Ignore pong responses
        Message::Close(_) => false, // Client requested close
    }
}

// Re-export for use by broadcast module
pub use tokio::sync::broadcast;
