//! HTTP server setup with Axum

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::knowledge_base::KnowledgeBase;
use super::rest::{entities, graph, relations, search};
use super::sse::handler::{
    login_handler, me_handler, mcp_request_handler, refresh_handler,
    server_info_handler, sse_handler, SseState,
};
use super::sse::JwtAuth;
use super::websocket::{handler::ws_handler, state::AppState};

/// Create the Axum router with all endpoints
///
/// Uses a single Arc<KnowledgeBase> shared by both SSE/MCP and REST/WebSocket.
/// This ensures data consistency across all transports.
pub fn create_router(state: Arc<AppState>, kb_sync: Arc<KnowledgeBase>) -> Router {
    create_router_with_auth(state, kb_sync, None, false)
}

/// Create router with optional JWT authentication
pub fn create_router_with_auth(
    state: Arc<AppState>,
    kb_sync: Arc<KnowledgeBase>,
    jwt_auth: Option<Arc<JwtAuth>>,
    require_auth: bool,
) -> Router {
    // CORS configuration - allow all origins for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create SSE state with sync KB (auto-registers all tools)
    let mut sse_state = SseState::new(
        kb_sync,
        state.event_tx.clone(),
        Arc::clone(&state.sequence_counter),
    );

    // Add JWT auth if configured
    if let Some(auth) = jwt_auth {
        sse_state = sse_state.with_jwt_auth(auth, require_auth);
    }

    let sse_state = Arc::new(sse_state);

    // Build main router with AppState
    let main_router = Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // REST API endpoints
        .route("/api/graph", get(graph::get_graph))
        .route("/api/graph/stats", get(graph::get_stats))
        .route("/api/events/replay", get(graph::get_events_replay))
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
        // Auth endpoints
        .route("/auth/token", post(login_handler))
        .route("/auth/refresh", post(refresh_handler))
        .route("/auth/me", get(me_handler))
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
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        // Single KB instance - shared by both AppState and router
        let kb = Arc::new(KnowledgeBase::new());
        let state = Arc::new(AppState::new(Arc::clone(&kb)));
        let app = create_router(state, kb);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
