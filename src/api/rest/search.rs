//! Search endpoint

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::{ApiError, ApiResponse};
use crate::api::websocket::state::AppState;

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query string
    pub q: String,
    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Include relations connected to matching entities
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_limit() -> usize {
    50
}

fn default_true() -> bool {
    true
}

/// GET /api/search - Search nodes in the knowledge graph
///
/// Searches entity names, types, and observations using the existing
/// search logic with synonym matching.
pub async fn search_nodes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    if params.q.trim().is_empty() {
        let error = ApiError::bad_request("Query parameter 'q' is required");
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    let kb = state.kb.read().await;

    // Use existing search_nodes functionality
    let limit = if params.limit > 0 {
        Some(params.limit.min(1000))
    } else {
        None
    };

    match kb.search_nodes(&params.q, limit, params.include_relations) {
        Ok(result) => {
            let sequence_id = state.current_sequence_id();
            let total = result.entities.len();
            (
                StatusCode::OK,
                Json(ApiResponse::with_total(result, sequence_id, total)),
            )
                .into_response()
        }
        Err(e) => {
            let error = ApiError::internal(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}
