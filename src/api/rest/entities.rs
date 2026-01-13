//! Entity endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse};
use crate::api::websocket::state::AppState;
use crate::types::{Entity, Relation};

/// Query parameters for listing entities
#[derive(Debug, Deserialize)]
pub struct ListEntitiesParams {
    /// Maximum number of entities to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of entities to skip
    #[serde(default)]
    pub offset: usize,
    /// Filter by entity type
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Sort by field (name, created_at, updated_at)
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Sort order (asc, desc)
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_limit() -> usize {
    100
}

fn default_sort() -> String {
    "name".to_string()
}

fn default_order() -> String {
    "asc".to_string()
}

/// GET /api/entities - List entities with pagination
pub async fn list_entities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListEntitiesParams>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;
    let graph = kb.graph.read().unwrap();

    // Filter by type if specified
    let mut entities: Vec<Entity> = if let Some(ref entity_type) = params.entity_type {
        graph
            .entities
            .iter()
            .filter(|e| e.entity_type.eq_ignore_ascii_case(entity_type))
            .cloned()
            .collect()
    } else {
        graph.entities.clone()
    };

    let total = entities.len();

    // Sort
    match params.sort.as_str() {
        "created_at" => {
            if params.order == "desc" {
                entities.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            } else {
                entities.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
        }
        "updated_at" => {
            if params.order == "desc" {
                entities.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            } else {
                entities.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
            }
        }
        _ => {
            // Default: sort by name
            if params.order == "desc" {
                entities.sort_by(|a, b| b.name.cmp(&a.name));
            } else {
                entities.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    // Pagination
    let limit = params.limit.min(1000);
    let entities: Vec<Entity> = entities.into_iter().skip(params.offset).take(limit).collect();

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::with_total(entities, sequence_id, total))
}

/// Response for single entity with relations
#[derive(Debug, Serialize)]
pub struct EntityDetail {
    #[serde(flatten)]
    pub entity: Entity,
    /// Relations where this entity is the source
    pub outgoing_relations: Vec<Relation>,
    /// Relations where this entity is the target
    pub incoming_relations: Vec<Relation>,
}

/// GET /api/entities/:name - Get single entity with relations
pub async fn get_entity(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let kb = state.kb.read().await;
    let graph = kb.graph.read().unwrap();

    // URL decode the name (handles spaces and special chars)
    let decoded_name = urlencoding::decode(&name)
        .unwrap_or_else(|_| name.clone().into())
        .into_owned();

    // Find entity
    let entity = graph.entities.iter().find(|e| e.name == decoded_name);

    match entity {
        Some(entity) => {
            // Get related relations
            let outgoing_relations: Vec<Relation> = graph
                .relations
                .iter()
                .filter(|r| r.from == decoded_name)
                .cloned()
                .collect();

            let incoming_relations: Vec<Relation> = graph
                .relations
                .iter()
                .filter(|r| r.to == decoded_name)
                .cloned()
                .collect();

            let detail = EntityDetail {
                entity: entity.clone(),
                outgoing_relations,
                incoming_relations,
            };

            let sequence_id = state.current_sequence_id();
            (StatusCode::OK, Json(ApiResponse::new(detail, sequence_id))).into_response()
        }
        None => {
            let error = ApiError::not_found(format!("Entity '{}' not found", decoded_name));
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}
