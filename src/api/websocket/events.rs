//! WebSocket event types for real-time graph updates

use serde::{Deserialize, Serialize};
use crate::types::{Entity, Relation};

/// Graph events that can be broadcast to WebSocket clients
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphEvent {
    /// A new entity was created
    EntityCreated {
        payload: Entity,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },

    /// An entity was updated (new observations added)
    EntityUpdated {
        name: String,
        new_observations: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },

    /// An entity was deleted
    EntityDeleted {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },

    /// A new relation was created
    RelationCreated {
        payload: Relation,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },

    /// A relation was deleted
    RelationDeleted {
        from: String,
        to: String,
        relation_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
    },

    /// Batch update containing multiple events
    BatchUpdate {
        events: Vec<GraphEvent>,
    },
}

/// WebSocket message wrapper with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsMessage {
    /// The graph event
    #[serde(flatten)]
    pub event: GraphEvent,

    /// Monotonically increasing sequence ID for gap detection
    pub sequence_id: u64,

    /// Unix timestamp when event was created
    pub timestamp: i64,
}

/// Client message types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to a channel
    Subscribe {
        #[serde(default)]
        channel: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<SubscribeFilter>,
    },

    /// Unsubscribe from a channel
    Unsubscribe {
        #[serde(default)]
        channel: String,
    },

    /// Ping for heartbeat
    Ping,
}

/// Filter for subscription
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscribeFilter {
    /// Filter by entity types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Filter by entity names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_names: Option<Vec<String>>,
}

/// Welcome message sent on connection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WelcomeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub current_sequence_id: u64,
}

impl WelcomeMessage {
    pub fn new(current_sequence_id: u64) -> Self {
        Self {
            msg_type: "connected".to_string(),
            current_sequence_id,
        }
    }
}

/// Pong response message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PongMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
}

impl Default for PongMessage {
    fn default() -> Self {
        Self {
            msg_type: "pong".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_event_serialization() {
        let event = GraphEvent::EntityCreated {
            payload: Entity {
                name: "Test".to_string(),
                entity_type: "Feature".to_string(),
                observations: vec!["obs1".to_string()],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            user: Some("test_user".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("entity_created"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage {
            event: GraphEvent::EntityDeleted {
                name: "OldEntity".to_string(),
                user: None,
            },
            sequence_id: 42,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("sequence_id"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_client_message_parsing() {
        let json = r#"{"type":"ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }
}
