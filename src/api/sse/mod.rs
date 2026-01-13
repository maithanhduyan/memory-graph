//! SSE (Server-Sent Events) module for MCP over HTTP
//!
//! Provides SSE transport for AI Agents to connect via HTTP instead of stdio.
//! This enables team collaboration with multiple AI agents.
//!
//! ## Endpoints
//! - `GET /mcp/sse` - SSE stream for server→client events
//! - `POST /mcp` - JSON-RPC requests from client→server
//! - `GET /mcp/info` - Server info and capabilities

pub mod handler;
pub mod session;

use serde::Serialize;

/// SSE event types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// JSON-RPC response to a request
    Response {
        #[serde(flatten)]
        response: serde_json::Value,
    },
    /// Graph change notification
    GraphEvent {
        #[serde(flatten)]
        event: crate::api::websocket::events::WsMessage,
    },
    /// Heartbeat ping
    Ping {
        timestamp: i64,
    },
    /// Welcome message on connect
    Welcome {
        session_id: String,
        server_name: String,
        server_version: String,
        sequence_id: u64,
    },
    /// Error notification
    Error {
        code: String,
        message: String,
    },
}

/// Client session info
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: String,
    pub user: String,
    pub api_key: Option<String>,
    pub connected_at: i64,
}
