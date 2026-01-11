//! WebSocket event broadcaster
//!
//! This module provides a global event broadcaster that can be used by
//! mutation operations to notify WebSocket clients of changes.
//!
//! # Design
//!
//! Instead of modifying KnowledgeBase directly (which would add complexity
//! and break the single-responsibility principle), we use a separate broadcaster
//! that can be optionally initialized when running in HTTP mode.

use std::sync::OnceLock;
use tokio::sync::broadcast;

use super::events::{GraphEvent, WsMessage};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global broadcaster instance (initialized once when HTTP server starts)
static BROADCASTER: OnceLock<EventBroadcaster> = OnceLock::new();

/// Event broadcaster for WebSocket notifications
pub struct EventBroadcaster {
    tx: broadcast::Sender<WsMessage>,
    sequence_counter: AtomicU64,
}

impl EventBroadcaster {
    /// Create a new broadcaster with the given capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            sequence_counter: AtomicU64::new(0),
        }
    }

    /// Broadcast an event to all connected WebSocket clients
    pub fn broadcast(&self, event: GraphEvent) {
        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let msg = WsMessage {
            event,
            sequence_id: seq,
            timestamp: chrono::Utc::now().timestamp(),
        };
        // Ignore errors - just means no receivers are connected
        let _ = self.tx.send(msg);
    }

    /// Get the current sequence ID
    pub fn current_sequence_id(&self) -> u64 {
        self.sequence_counter.load(Ordering::SeqCst)
    }

    /// Subscribe to receive broadcast events
    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.tx.subscribe()
    }

    /// Get the sender for cloning into state
    pub fn sender(&self) -> broadcast::Sender<WsMessage> {
        self.tx.clone()
    }
}

/// Initialize the global broadcaster (call once when HTTP server starts)
pub fn init_broadcaster(capacity: usize) -> &'static EventBroadcaster {
    BROADCASTER.get_or_init(|| EventBroadcaster::new(capacity))
}

/// Get the global broadcaster (returns None if not initialized)
pub fn get_broadcaster() -> Option<&'static EventBroadcaster> {
    BROADCASTER.get()
}

/// Broadcast an event if the broadcaster is initialized
/// This is the main entry point for mutation operations
pub fn broadcast_event(event: GraphEvent) {
    if let Some(broadcaster) = BROADCASTER.get() {
        broadcaster.broadcast(event);
    }
}

/// Helper functions for common events
pub mod helpers {
    use super::*;
    use crate::types::{Entity, Relation};

    /// Broadcast entity created event
    pub fn entity_created(entity: &Entity, user: Option<String>) {
        broadcast_event(GraphEvent::EntityCreated {
            payload: entity.clone(),
            user,
        });
    }

    /// Broadcast entity updated event (observations added)
    pub fn entity_updated(name: &str, new_observations: Vec<String>, user: Option<String>) {
        broadcast_event(GraphEvent::EntityUpdated {
            name: name.to_string(),
            new_observations,
            user,
        });
    }

    /// Broadcast entity deleted event
    pub fn entity_deleted(name: &str, user: Option<String>) {
        broadcast_event(GraphEvent::EntityDeleted {
            name: name.to_string(),
            user,
        });
    }

    /// Broadcast relation created event
    pub fn relation_created(relation: &Relation, user: Option<String>) {
        broadcast_event(GraphEvent::RelationCreated {
            payload: relation.clone(),
            user,
        });
    }

    /// Broadcast relation deleted event
    pub fn relation_deleted(from: &str, to: &str, relation_type: &str, user: Option<String>) {
        broadcast_event(GraphEvent::RelationDeleted {
            from: from.to_string(),
            to: to.to_string(),
            relation_type: relation_type.to_string(),
            user,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcaster_sequence_increment() {
        let broadcaster = EventBroadcaster::new(100);
        assert_eq!(broadcaster.current_sequence_id(), 0);

        broadcaster.broadcast(GraphEvent::EntityDeleted {
            name: "Test".to_string(),
            user: None,
        });

        assert_eq!(broadcaster.current_sequence_id(), 1);
    }
}
