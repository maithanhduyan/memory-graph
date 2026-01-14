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
//!
//! # Event Replay
//!
//! The broadcaster maintains a circular buffer of recent events for replay.
//! Clients can reconnect with their last_sequence_id to receive missed events.

use std::collections::VecDeque;
use std::sync::{OnceLock, RwLock};
use tokio::sync::broadcast;

use super::events::{GraphEvent, WsMessage};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global broadcaster instance (initialized once when HTTP server starts)
static BROADCASTER: OnceLock<EventBroadcaster> = OnceLock::new();

/// Maximum number of events to keep in history for replay
const EVENT_HISTORY_SIZE: usize = 1000;

/// Event broadcaster for WebSocket notifications
pub struct EventBroadcaster {
    tx: broadcast::Sender<WsMessage>,
    sequence_counter: AtomicU64,
    /// Circular buffer of recent events for replay on reconnect
    event_history: RwLock<VecDeque<WsMessage>>,
}

impl EventBroadcaster {
    /// Create a new broadcaster with the given capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            sequence_counter: AtomicU64::new(0),
            event_history: RwLock::new(VecDeque::with_capacity(EVENT_HISTORY_SIZE)),
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

        // Store in history for replay
        if let Ok(mut history) = self.event_history.write() {
            if history.len() >= EVENT_HISTORY_SIZE {
                history.pop_front();
            }
            history.push_back(msg.clone());
        }

        // Broadcast to live subscribers
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

    /// Get events since a given sequence ID for replay
    ///
    /// Returns None if the requested sequence is too old (no longer in history).
    /// Returns empty Vec if already up to date.
    pub fn get_events_since(&self, since_sequence_id: u64) -> Option<Vec<WsMessage>> {
        let history = self.event_history.read().ok()?;

        // Check if we have events in history
        if history.is_empty() {
            return Some(Vec::new());
        }

        // Get the oldest sequence ID in history
        let oldest_seq = history.front().map(|m| m.sequence_id)?;

        // If requested sequence is older than our history, return None
        // Client needs to do a full refresh
        if since_sequence_id < oldest_seq {
            return None;
        }

        // Collect events newer than since_sequence_id
        let events: Vec<WsMessage> = history
            .iter()
            .filter(|m| m.sequence_id > since_sequence_id)
            .cloned()
            .collect();

        Some(events)
    }

    /// Get the oldest sequence ID still in history
    pub fn oldest_sequence_id(&self) -> Option<u64> {
        self.event_history
            .read()
            .ok()
            .and_then(|h| h.front().map(|m| m.sequence_id))
    }

    /// Get the number of events in history
    pub fn history_len(&self) -> usize {
        self.event_history.read().map(|h| h.len()).unwrap_or(0)
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

    #[test]
    fn test_event_history_storage() {
        let broadcaster = EventBroadcaster::new(100);

        // Broadcast several events
        for i in 0..5 {
            broadcaster.broadcast(GraphEvent::EntityDeleted {
                name: format!("Entity{}", i),
                user: None,
            });
        }

        assert_eq!(broadcaster.history_len(), 5);
        assert_eq!(broadcaster.oldest_sequence_id(), Some(0));
        assert_eq!(broadcaster.current_sequence_id(), 5);
    }

    #[test]
    fn test_get_events_since() {
        let broadcaster = EventBroadcaster::new(100);

        // Broadcast 5 events (seq 0, 1, 2, 3, 4)
        for i in 0..5 {
            broadcaster.broadcast(GraphEvent::EntityDeleted {
                name: format!("Entity{}", i),
                user: None,
            });
        }

        // Get events since seq 2 -> should return seq 3, 4
        let events = broadcaster.get_events_since(2).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence_id, 3);
        assert_eq!(events[1].sequence_id, 4);

        // Get events since seq 4 -> should return empty
        let events = broadcaster.get_events_since(4).unwrap();
        assert_eq!(events.len(), 0);

        // Get events since seq 0 -> should return seq 1, 2, 3, 4
        let events = broadcaster.get_events_since(0).unwrap();
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_history_circular_buffer() {
        // Create broadcaster with small history for testing
        let broadcaster = EventBroadcaster::new(100);

        // Fill beyond EVENT_HISTORY_SIZE (1000)
        for i in 0..1005 {
            broadcaster.broadcast(GraphEvent::EntityDeleted {
                name: format!("Entity{}", i),
                user: None,
            });
        }

        // Should have exactly 1000 events
        assert_eq!(broadcaster.history_len(), 1000);
        // Oldest should be seq 5 (first 5 were evicted)
        assert_eq!(broadcaster.oldest_sequence_id(), Some(5));

        // Request seq 0 should return None (too old)
        assert!(broadcaster.get_events_since(0).is_none());

        // Request seq 5 should work
        assert!(broadcaster.get_events_since(5).is_some());
    }
}
