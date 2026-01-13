//! Event types for Event Sourcing
//!
//! This module defines the core event types used for the append-only event log.
//! Events are immutable records of state changes that can be replayed to rebuild state.

use serde::{Deserialize, Serialize};

/// Event types that can occur in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A new entity was created
    EntityCreated,
    /// An entity's metadata was updated (e.g., entityType changed)
    EntityUpdated,
    /// An entity was deleted
    EntityDeleted,
    /// An observation was added to an entity
    ObservationAdded,
    /// An observation was removed from an entity
    ObservationRemoved,
    /// A new relation was created between entities
    RelationCreated,
    /// A relation was deleted
    RelationDeleted,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::EntityCreated => write!(f, "entity_created"),
            EventType::EntityUpdated => write!(f, "entity_updated"),
            EventType::EntityDeleted => write!(f, "entity_deleted"),
            EventType::ObservationAdded => write!(f, "observation_added"),
            EventType::ObservationRemoved => write!(f, "observation_removed"),
            EventType::RelationCreated => write!(f, "relation_created"),
            EventType::RelationDeleted => write!(f, "relation_deleted"),
        }
    }
}

/// Source of the event - how it was created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// User typed directly (manual input)
    #[default]
    Manual,
    /// Via MCP tool call from AI agent
    McpToolCall,
    /// Via REST/GraphQL API
    ApiRequest,
    /// System generated (e.g., snapshot metadata)
    SystemGenerated,
    /// Migration from legacy format
    Migration,
}

/// Data payload for EntityCreated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCreatedData {
    pub name: String,
    pub entity_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<String>,
}

/// Data payload for EntityUpdated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityUpdatedData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
}

/// Data payload for EntityDeleted event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDeletedData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Data payload for ObservationAdded event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationAddedData {
    pub entity: String,
    pub observation: String,
}

/// Data payload for ObservationRemoved event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRemovedData {
    pub entity: String,
    pub observation: String,
}

/// Data payload for RelationCreated event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationCreatedData {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<i64>,
}

/// Data payload for RelationDeleted event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDeletedData {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

/// Event data - typed payload for each event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventData {
    EntityCreated(EntityCreatedData),
    EntityUpdated(EntityUpdatedData),
    EntityDeleted(EntityDeletedData),
    ObservationAdded(ObservationAddedData),
    ObservationRemoved(ObservationRemovedData),
    RelationCreated(RelationCreatedData),
    RelationDeleted(RelationDeletedData),
}

/// An immutable event in the event log
///
/// Events are the source of truth in Event Sourcing.
/// The current state is derived by replaying all events in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Type of event
    #[serde(rename = "eventType")]
    pub event_type: EventType,

    /// Unique, auto-incrementing event ID
    #[serde(rename = "eventId")]
    pub event_id: u64,

    /// Unix timestamp when event occurred
    #[serde(rename = "ts")]
    pub timestamp: i64,

    /// User who triggered the event (from git config or API key)
    pub user: String,

    /// AI agent that generated the event (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// How the event was created
    #[serde(default, skip_serializing_if = "is_default_source")]
    pub source: EventSource,

    /// Event-specific payload
    pub data: serde_json::Value,
}

fn is_default_source(source: &EventSource) -> bool {
    matches!(source, EventSource::Manual)
}

impl Event {
    /// Create a new event with auto-generated timestamp
    pub fn new(
        event_type: EventType,
        event_id: u64,
        user: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type,
            event_id,
            timestamp: crate::utils::current_timestamp() as i64,
            user,
            agent: None,
            source: EventSource::McpToolCall,
            data,
        }
    }

    /// Create a new event with specific timestamp (for migration)
    pub fn with_timestamp(
        event_type: EventType,
        event_id: u64,
        timestamp: i64,
        user: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type,
            event_id,
            timestamp,
            user,
            agent: None,
            source: EventSource::Migration,
            data,
        }
    }

    /// Set the AI agent that generated this event
    pub fn with_agent(mut self, agent: String) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Set the event source
    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = source;
        self
    }

    /// Parse the event data as a specific type
    pub fn parse_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }

    /// Serialize event to JSON string (for JSONL)
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize event from JSON string
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

/// Snapshot metadata - first line in snapshot file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// Always "snapshot_meta" to identify this as metadata
    #[serde(rename = "type")]
    pub meta_type: String,

    /// Last event ID included in this snapshot
    pub last_event_id: u64,

    /// Timestamp when snapshot was created
    pub created_at: i64,

    /// Number of entities in snapshot
    pub entity_count: usize,

    /// Number of relations in snapshot
    pub relation_count: usize,

    /// Version of snapshot format (for future migrations)
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl SnapshotMeta {
    /// Create new snapshot metadata
    pub fn new(last_event_id: u64, entity_count: usize, relation_count: usize) -> Self {
        Self {
            meta_type: "snapshot_meta".to_string(),
            last_event_id,
            created_at: crate::utils::current_timestamp() as i64,
            entity_count,
            relation_count,
            version: 1,
        }
    }

    /// Parse from JSON string (first line of snapshot file)
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    /// Serialize to JSON string
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::EntityCreated;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"entity_created\"");

        let parsed: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, EventType::EntityCreated);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event {
            event_type: EventType::EntityCreated,
            event_id: 1,
            timestamp: 1704067200,
            user: "Duyan".to_string(),
            agent: None,
            source: EventSource::McpToolCall,
            data: json!({
                "name": "Bug:X",
                "entity_type": "Bug",
                "observations": []
            }),
        };

        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"eventType\":\"entity_created\""));
        assert!(json.contains("\"eventId\":1"));
        assert!(json.contains("\"user\":\"Duyan\""));

        let parsed = Event::from_json_line(&json).unwrap();
        assert_eq!(parsed.event_type, EventType::EntityCreated);
        assert_eq!(parsed.event_id, 1);
        assert_eq!(parsed.user, "Duyan");
    }

    #[test]
    fn test_event_with_agent() {
        let event = Event::new(
            EventType::EntityCreated,
            1,
            "Duyan".to_string(),
            json!({"name": "Test"}),
        )
        .with_agent("Claude-3.5".to_string());

        assert_eq!(event.agent, Some("Claude-3.5".to_string()));

        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"agent\":\"Claude-3.5\""));
    }

    #[test]
    fn test_snapshot_meta_serialization() {
        let meta = SnapshotMeta::new(1000, 50, 100);

        let json = meta.to_json_line().unwrap();
        assert!(json.contains("\"type\":\"snapshot_meta\""));
        assert!(json.contains("\"last_event_id\":1000"));
        assert!(json.contains("\"entity_count\":50"));

        let parsed = SnapshotMeta::from_json_line(&json).unwrap();
        assert_eq!(parsed.last_event_id, 1000);
        assert_eq!(parsed.entity_count, 50);
        assert_eq!(parsed.relation_count, 100);
    }

    #[test]
    fn test_parse_entity_created_data() {
        let event = Event::new(
            EventType::EntityCreated,
            1,
            "test".to_string(),
            json!({
                "name": "Bug:X",
                "entity_type": "Bug",
                "observations": ["obs1", "obs2"]
            }),
        );

        let data: EntityCreatedData = event.parse_data().unwrap();
        assert_eq!(data.name, "Bug:X");
        assert_eq!(data.entity_type, "Bug");
        assert_eq!(data.observations.len(), 2);
    }

    #[test]
    fn test_parse_relation_created_data() {
        let event = Event::new(
            EventType::RelationCreated,
            2,
            "test".to_string(),
            json!({
                "from": "Bug:X",
                "to": "Module:Auth",
                "relation_type": "affects"
            }),
        );

        let data: RelationCreatedData = event.parse_data().unwrap();
        assert_eq!(data.from, "Bug:X");
        assert_eq!(data.to, "Module:Auth");
        assert_eq!(data.relation_type, "affects");
        assert!(data.valid_from.is_none());
    }
}
