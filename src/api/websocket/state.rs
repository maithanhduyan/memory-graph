//! WebSocket application state

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::knowledge_base::KnowledgeBase;
use super::events::{GraphEvent, WsMessage};

/// Shared application state for WebSocket connections
pub struct AppState {
    /// The knowledge base
    pub kb: Arc<RwLock<KnowledgeBase>>,

    /// Broadcast channel for sending events to all connected clients
    pub event_tx: broadcast::Sender<WsMessage>,

    /// Monotonically increasing sequence counter
    pub sequence_counter: Arc<AtomicU64>,
}

impl AppState {
    /// Create a new AppState with the given knowledge base
    pub fn new(kb: Arc<RwLock<KnowledgeBase>>) -> Self {
        // Buffer 1024 events - if clients are too slow, they'll miss events
        // and need to do a full refresh
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            kb,
            event_tx,
            sequence_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Broadcast a graph event to all connected WebSocket clients
    pub fn broadcast(&self, event: GraphEvent) {
        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);
        let msg = WsMessage {
            event,
            sequence_id: seq,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Ignore send errors - they just mean no receivers are listening
        let _ = self.event_tx.send(msg);
    }

    /// Get the current sequence ID
    pub fn current_sequence_id(&self) -> u64 {
        self.sequence_counter.load(Ordering::SeqCst)
    }

    /// Subscribe to receive broadcast events
    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.event_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;

    #[tokio::test]
    async fn test_broadcast_increments_sequence() {
        let kb = Arc::new(RwLock::new(KnowledgeBase::new()));
        let state = AppState::new(kb);

        assert_eq!(state.current_sequence_id(), 0);

        state.broadcast(GraphEvent::EntityDeleted {
            name: "Test".to_string(),
            user: None,
        });

        assert_eq!(state.current_sequence_id(), 1);
    }

    #[tokio::test]
    async fn test_subscribe_receives_events() {
        let kb = Arc::new(RwLock::new(KnowledgeBase::new()));
        let state = AppState::new(kb);
        let mut rx = state.subscribe();

        state.broadcast(GraphEvent::EntityCreated {
            payload: Entity {
                name: "Test".to_string(),
                entity_type: "Feature".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            user: Some("tester".to_string()),
        });

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.sequence_id, 0);
        assert!(matches!(msg.event, GraphEvent::EntityCreated { .. }));
    }
}
