//! HTTP server setup with Axum

use std::sync::Arc;
use axum::{
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use super::websocket::{handler::ws_handler, state::AppState};

/// Create the Axum router with all endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    // CORS configuration - allow all origins for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // TODO: REST API endpoints
        // .route("/api/entities", get(list_entities).post(create_entities))
        // .route("/api/entities/:name", get(get_entity).delete(delete_entity))
        // .route("/api/relations", get(list_relations).post(create_relations))
        // .route("/api/graph", get(get_graph))
        // .route("/api/search", get(search_nodes))
        .layer(cors)
        .with_state(state)
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
        let kb = Arc::new(RwLock::new(KnowledgeBase::new()));
        let state = Arc::new(AppState::new(kb));
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
