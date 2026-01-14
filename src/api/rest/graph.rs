//! Graph endpoint - Full graph snapshot for client recovery

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use super::ApiResponse;
use crate::api::websocket::state::AppState;
use crate::types::{Entity, Relation};

/// Response for GET /api/graph
#[derive(Debug, Serialize)]
pub struct GraphResponse {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

/// Query parameters for graph endpoint
#[derive(Debug, Deserialize)]
pub struct GraphParams {
    /// Maximum number of entities to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of entities to skip
    #[serde(default)]
    pub offset: usize,
    /// Include relations (default: true)
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_limit() -> usize {
    100
}

fn default_true() -> bool {
    true
}

/// GET /api/graph - Get full graph snapshot
///
/// Returns entities and relations for client recovery after WebSocket reconnection.
/// Includes sequence_id so client knows the snapshot version.
pub async fn get_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphParams>,
) -> impl IntoResponse {
    // Get all entities with pagination
    let all_entities: Vec<Entity> = state.kb.graph.read().unwrap().entities.clone();
    let all_relations: Vec<Relation> = state.kb.graph.read().unwrap().relations.clone();

    let total_entities = all_entities.len();

    // Apply pagination to entities
    let limit = params.limit.min(1000);
    let entities: Vec<Entity> = all_entities
        .into_iter()
        .skip(params.offset)
        .take(limit)
        .collect();

    // Include relations if requested
    let relations = if params.include_relations {
        // Filter relations to only those involving returned entities
        let entity_names: std::collections::HashSet<_> =
            entities.iter().map(|e| e.name.as_str()).collect();

        all_relations
            .into_iter()
            .filter(|r| entity_names.contains(r.from.as_str()) || entity_names.contains(r.to.as_str()))
            .collect()
    } else {
        Vec::new()
    };

    let graph = GraphResponse { entities, relations };
    let sequence_id = state.current_sequence_id();

    let response = ApiResponse::with_total(graph, sequence_id, total_entities);

    Json(response)
}

/// GET /api/graph/stats - Get graph statistics
#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub entity_count: usize,
    pub relation_count: usize,
    pub entity_types: Vec<EntityTypeCount>,
    pub relation_types: Vec<RelationTypeCount>,
}

#[derive(Debug, Serialize)]
pub struct EntityTypeCount {
    pub entity_type: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct RelationTypeCount {
    pub relation_type: String,
    pub count: usize,
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // Count entity types
    let mut entity_type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entity in &graph.entities {
        *entity_type_counts
            .entry(entity.entity_type.clone())
            .or_insert(0) += 1;
    }

    // Count relation types
    let mut relation_type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for relation in &graph.relations {
        *relation_type_counts
            .entry(relation.relation_type.clone())
            .or_insert(0) += 1;
    }

    let stats = GraphStats {
        entity_count: graph.entities.len(),
        relation_count: graph.relations.len(),
        entity_types: entity_type_counts
            .into_iter()
            .map(|(entity_type, count)| EntityTypeCount { entity_type, count })
            .collect(),
        relation_types: relation_type_counts
            .into_iter()
            .map(|(relation_type, count)| RelationTypeCount {
                relation_type,
                count,
            })
            .collect(),
    };

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::new(stats, sequence_id))
}

/// Query parameters for event replay
#[derive(Debug, Deserialize)]
pub struct EventReplayParams {
    /// Get events after this sequence ID
    pub since: u64,
}

/// Response for GET /api/events/replay
#[derive(Debug, Serialize)]
pub struct EventReplayResponse {
    /// Events since the requested sequence ID
    pub events: Vec<crate::api::websocket::events::WsMessage>,
    /// Whether a full refresh is needed (events too old)
    pub needs_full_refresh: bool,
    /// Oldest available sequence ID in history
    pub oldest_available: Option<u64>,
    /// Current sequence ID
    pub current_sequence_id: u64,
}

/// GET /api/events/replay - Replay missed events for client recovery
///
/// Clients can request events they missed during disconnection.
/// If the requested sequence is too old (outside history buffer),
/// `needs_full_refresh` will be true and client should fetch full graph.
pub async fn get_events_replay(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventReplayParams>,
) -> impl IntoResponse {
    let current_sequence_id = state.current_sequence_id();

    // Get broadcaster if available
    let broadcaster = crate::api::websocket::get_broadcaster();

    let (events, needs_full_refresh, oldest_available) = match broadcaster {
        Some(b) => {
            let oldest = b.oldest_sequence_id();
            match b.get_events_since(params.since) {
                Some(events) => (events, false, oldest),
                None => (Vec::new(), true, oldest), // Too old, needs refresh
            }
        }
        None => {
            // Broadcaster not initialized (no WebSocket/SSE enabled)
            (Vec::new(), false, None)
        }
    };

    let response = EventReplayResponse {
        events,
        needs_full_refresh,
        oldest_available,
        current_sequence_id,
    };

    Json(ApiResponse::new(response, current_sequence_id))
}
