//! SSE and MCP HTTP handlers

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::{session::SessionManager, SseEvent};
use crate::api::websocket::events::WsMessage;
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpTool, Tool};

/// Shared state for SSE endpoints
pub struct SseState {
    /// Knowledge base (sync Arc for tool compatibility)
    pub kb: Arc<crate::knowledge_base::KnowledgeBase>,
    /// Session manager
    pub sessions: SessionManager,
    /// Registered MCP tools
    pub tools: HashMap<String, Arc<dyn Tool>>,
    /// Server info
    pub server_name: String,
    pub server_version: String,
    /// Broadcast channel for graph events (shared with WebSocket)
    pub event_rx: broadcast::Sender<WsMessage>,
    /// Sequence counter
    pub sequence_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl SseState {
    pub fn new(
        kb: Arc<crate::knowledge_base::KnowledgeBase>,
        event_tx: broadcast::Sender<WsMessage>,
        sequence_counter: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        // Register all tools
        let tools_vec = crate::tools::get_all_tools(kb.clone());
        let mut tools = HashMap::new();
        for tool in tools_vec {
            let name = tool.definition().name.clone();
            tools.insert(name, tool);
        }

        Self {
            kb,
            sessions: SessionManager::new(),
            tools,
            server_name: "memory".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            event_rx: event_tx,
            sequence_counter,
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get current sequence ID
    pub fn current_sequence_id(&self) -> u64 {
        self.sequence_counter.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Query parameters for SSE connection
#[derive(Debug, Deserialize)]
pub struct SseParams {
    /// API key for authentication
    pub api_key: Option<String>,
}

/// Extract user from API key header or query param
fn extract_user(headers: &HeaderMap, params: &SseParams) -> Option<String> {
    // Try X-API-Key header first
    if let Some(header) = headers.get("X-API-Key") {
        if let Ok(key) = header.to_str() {
            return SessionManager::validate_api_key(key);
        }
    }

    // Fall back to query param
    if let Some(ref key) = params.api_key {
        return SessionManager::validate_api_key(key);
    }

    // Allow anonymous connections (for development)
    Some("anonymous".to_string())
}

/// GET /mcp/sse - SSE stream for server→client events
pub async fn sse_handler(
    State(state): State<Arc<SseState>>,
    headers: HeaderMap,
    Query(params): Query<SseParams>,
) -> impl IntoResponse {
    let user = extract_user(&headers, &params).unwrap_or_else(|| "anonymous".to_string());

    // Create session
    let session = state
        .sessions
        .create_session(user, params.api_key.clone())
        .await;

    // Subscribe to graph events
    let mut event_rx = state.event_rx.subscribe();
    let session_id = session.session_id.clone();
    let server_name = state.server_name.clone();
    let server_version = state.server_version.clone();
    let sequence_id = state.current_sequence_id();

    // Create SSE stream
    let stream = async_stream::stream! {
        // Send endpoint event first (MCP SSE spec requirement)
        // This tells the client where to POST messages
        yield Ok::<_, Infallible>(Event::default()
            .event("endpoint")
            .data("/mcp/sse"));

        // Send welcome message
        let welcome = SseEvent::Welcome {
            session_id: session_id.clone(),
            server_name,
            server_version,
            sequence_id,
        };
        yield Ok::<_, Infallible>(Event::default()
            .event("welcome")
            .data(serde_json::to_string(&welcome).unwrap_or_default()));

        // Stream graph events
        loop {
            match event_rx.recv().await {
                Ok(msg) => {
                    let event = SseEvent::GraphEvent { event: msg };
                    yield Ok(Event::default()
                        .event("graph_event")
                        .data(serde_json::to_string(&event).unwrap_or_default()));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Client is too slow
                    let error = SseEvent::Error {
                        code: "lagged".to_string(),
                        message: format!("Missed {} events, please reconnect", n),
                    };
                    yield Ok(Event::default()
                        .event("error")
                        .data(serde_json::to_string(&error).unwrap_or_default()));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(30)))
}

/// Request body for POST /mcp
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    #[serde(flatten)]
    pub request: JsonRpcRequest,
}

/// Response for POST /mcp
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum McpResponse {
    Success(JsonRpcResponse),
    Error(JsonRpcError),
}

/// POST /mcp - Handle JSON-RPC requests
pub async fn mcp_request_handler(
    State(state): State<Arc<SseState>>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Validate API key
    let user = extract_user(&headers, &SseParams { api_key: None })
        .unwrap_or_else(|| "anonymous".to_string());

    let id = request.id.clone().unwrap_or(Value::Null);

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        let error = JsonRpcError::invalid_request(id, "jsonrpc must be '2.0'".to_string());
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    // Handle methods
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(&state, id.clone()),
        "tools/list" => handle_tools_list(&state, id.clone()),
        "tools/call" => handle_tool_call(&state, id.clone(), request.params, &user),
        "ping" => Ok(JsonRpcResponse::new(id.clone(), json!({}))),
        _ => {
            let error = JsonRpcError::method_not_found(id, request.method);
            return (StatusCode::OK, Json(error)).into_response();
        }
    };

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (StatusCode::OK, Json(error)).into_response(),
    }
}

fn handle_initialize(state: &SseState, id: Value) -> Result<JsonRpcResponse, JsonRpcError> {
    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": state.server_name,
            "version": state.server_version
        }
    });
    Ok(JsonRpcResponse::new(id, result))
}

fn handle_tools_list(state: &SseState, id: Value) -> Result<JsonRpcResponse, JsonRpcError> {
    let tools: Vec<McpTool> = state.tools.values().map(|t| t.definition()).collect();
    let result = json!({ "tools": tools });
    Ok(JsonRpcResponse::new(id, result))
}

fn handle_tool_call(
    state: &SseState,
    id: Value,
    params: Option<Value>,
    _user: &str,
) -> Result<JsonRpcResponse, JsonRpcError> {
    let params = params.ok_or_else(|| {
        JsonRpcError::invalid_params(id.clone(), "Missing parameters".to_string())
    })?;

    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcError::invalid_params(id.clone(), "Missing tool name".to_string())
        })?;

    let tool = state.tools.get(tool_name).ok_or_else(|| {
        JsonRpcError::new(
            id.clone(),
            -32602,
            "Unknown tool".to_string(),
            Some(json!({"tool": tool_name})),
        )
    })?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool.execute(arguments) {
        Ok(result) => Ok(JsonRpcResponse::new(id, result)),
        Err(e) => Err(JsonRpcError::new(
            id,
            -32603,
            "Tool execution error".to_string(),
            Some(json!({"details": e.to_string()})),
        )),
    }
}

/// GET /mcp/info - Get server info
#[derive(Debug, Serialize)]
pub struct ServerInfoResponse {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub tool_count: usize,
    pub active_sessions: usize,
}

pub async fn server_info_handler(State(state): State<Arc<SseState>>) -> impl IntoResponse {
    let info = ServerInfoResponse {
        name: state.server_name.clone(),
        version: state.server_version.clone(),
        protocol_version: "2024-11-05".to_string(),
        tool_count: state.tools.len(),
        active_sessions: state.sessions.session_count().await,
    };
    Json(info)
}
