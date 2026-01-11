//! Data types for the Memory Graph MCP Server
//!
//! This module contains all the core data structures used throughout the application.

mod entity;
mod event;
mod graph;
mod inference;
mod observation;
mod relation;
mod summary;
mod traversal;

pub use entity::{Entity, EntityBrief};
pub use event::{
    EntityCreatedData, EntityDeletedData, EntityUpdatedData, Event, EventData, EventSource,
    EventType, ObservationAddedData, ObservationRemovedData, RelationCreatedData,
    RelationDeletedData, SnapshotMeta,
};
pub use graph::KnowledgeGraph;
pub use inference::{InferResult, InferStats, InferredRelation};
pub use observation::{Observation, ObservationDeletion};
pub use relation::{RelatedEntities, RelatedEntity, Relation};
pub use summary::Summary;
pub use traversal::{PathStep, TraversalPath, TraversalResult};

/// Result type for MCP operations
pub type McpResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Default user for serde deserialization
pub fn default_user() -> String {
    "system".to_string()
}

/// Check if string is empty or "system" (for skip_serializing_if)
pub fn is_default_user(val: &str) -> bool {
    val.is_empty() || val == "system"
}

/// Check if value is zero (for skip_serializing_if)
pub fn is_zero(val: &u64) -> bool {
    *val == 0
}
