//! HTTP server setup with Axum

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::knowledge_base::KnowledgeBase;
use super::rest::{entities, graph, relations, search};
use super::sse::handler::{mcp_request_handler, server_info_handler, sse_handler, SseState};
use super::websocket::{handler::ws_handler, state::AppState};

/// Create the Axum router with all endpoints
///
/// Takes both AppState (for WebSocket/REST with async RwLock<KB>) and
/// sync Arc<KnowledgeBase> (for SSE with MCP tools)
pub fn create_router(state: Arc<AppState>, kb_sync: Arc<KnowledgeBase>) -> Router {
    // CORS configuration - allow all origins for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create SSE state with sync KB (auto-registers all tools)
    let sse_state = Arc::new(SseState::new(
        kb_sync,
        state.event_tx.clone(),
        Arc::clone(&state.sequence_counter),
    ));

    // Build main router with AppState
    let main_router = Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // REST API endpoints
        .route("/api/graph", get(graph::get_graph))
        .route("/api/graph/stats", get(graph::get_stats))
        .route("/api/entities", get(entities::list_entities))
        .route("/api/entities/:name", get(entities::get_entity))
        .route("/api/relations", get(relations::list_relations))
        .route("/api/search", get(search::search_nodes))
        .with_state(state);

    // Build SSE router with SseState
    // MCP SSE spec: /sse for SSE stream, same endpoint accepts POST for messages
    let sse_router = Router::new()
        .route("/mcp/sse", get(sse_handler).post(mcp_request_handler))
        .route("/mcp", post(mcp_request_handler))
        .route("/mcp/info", get(server_info_handler))
        .with_state(sse_state);

    // Merge routers and apply CORS
    main_router.merge(sse_router).layer(cors)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_base::KnowledgeBase;
    use tokio::sync::RwLock;
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let kb = Arc::new(KnowledgeBase::new());
        let kb_async = Arc::new(RwLock::new(KnowledgeBase::new()));
        let state = Arc::new(AppState::new(kb_async));
        let app = create_router(state, kb);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
