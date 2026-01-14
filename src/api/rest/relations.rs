//! Relation endpoints

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::ApiResponse;
use crate::api::websocket::state::AppState;
use crate::types::Relation;

/// Query parameters for listing relations
#[derive(Debug, Deserialize)]
pub struct ListRelationsParams {
    /// Maximum number of relations to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of relations to skip
    #[serde(default)]
    pub offset: usize,
    /// Filter by relation type
    #[serde(rename = "type")]
    pub relation_type: Option<String>,
    /// Filter by source entity
    pub from: Option<String>,
    /// Filter by target entity
    pub to: Option<String>,
}

fn default_limit() -> usize {
    100
}

/// GET /api/relations - List relations with pagination and filters
pub async fn list_relations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListRelationsParams>,
) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // Apply filters
    let mut relations: Vec<Relation> = graph
        .relations
        .iter()
        .filter(|r| {
            // Filter by type
            if let Some(ref relation_type) = params.relation_type {
                if !r.relation_type.eq_ignore_ascii_case(relation_type) {
                    return false;
                }
            }
            // Filter by source
            if let Some(ref from) = params.from {
                if !r.from.eq_ignore_ascii_case(from) {
                    return false;
                }
            }
            // Filter by target
            if let Some(ref to) = params.to {
                if !r.to.eq_ignore_ascii_case(to) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    let total = relations.len();

    // Sort by from -> to -> type
    relations.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.relation_type.cmp(&b.relation_type))
    });

    // Pagination
    let limit = params.limit.min(1000);
    let relations: Vec<Relation> = relations.into_iter().skip(params.offset).take(limit).collect();

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::with_total(relations, sequence_id, total))
}
