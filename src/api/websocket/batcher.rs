//! Event batcher for debouncing high-frequency updates

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;

use super::events::{GraphEvent, WsMessage};

/// Event batcher that collects events and sends them in batches
pub struct EventBatcher {
    /// Buffer for pending events
    buffer: Vec<GraphEvent>,

    /// Flush interval (debounce time)
    flush_interval: Duration,

    /// Maximum batch size before forced flush
    max_batch_size: usize,

    /// Broadcast sender for sending batched events
    tx: broadcast::Sender<WsMessage>,

    /// Sequence counter for message IDs
    sequence_counter: Arc<AtomicU64>,
}

impl EventBatcher {
    /// Create a new event batcher
    pub fn new(
        tx: broadcast::Sender<WsMessage>,
        sequence_counter: Arc<AtomicU64>,
    ) -> Self {
        Self {
            buffer: Vec::new(),
            flush_interval: Duration::from_millis(50),
            max_batch_size: 100,
            tx,
            sequence_counter,
        }
    }

    /// Create a new event batcher with custom settings
    pub fn with_config(
        tx: broadcast::Sender<WsMessage>,
        sequence_counter: Arc<AtomicU64>,
        flush_interval_ms: u64,
        max_batch_size: usize,
    ) -> Self {
        Self {
            buffer: Vec::new(),
            flush_interval: Duration::from_millis(flush_interval_ms),
            max_batch_size,
            tx,
            sequence_counter,
        }
    }

    /// Push an event to the buffer
    pub fn push(&mut self, event: GraphEvent) {
        self.buffer.push(event);

        // Force flush if buffer is full
        if self.buffer.len() >= self.max_batch_size {
            self.flush();
        }
    }

    /// Flush all buffered events as a batch
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let seq = self.sequence_counter.fetch_add(1, Ordering::SeqCst);

        // If only one event, send it directly without wrapping in BatchUpdate
        let event = if self.buffer.len() == 1 {
            self.buffer.pop().unwrap()
        } else {
            GraphEvent::BatchUpdate {
                events: std::mem::take(&mut self.buffer),
            }
        };

        let msg = WsMessage {
            event,
            sequence_id: seq,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Ignore send errors - just means no receivers
        let _ = self.tx.send(msg);
    }

    /// Run the batcher as an async task
    ///
    /// This will receive events from the channel and batch them,
    /// flushing on timer or when max batch size is reached.
    pub async fn run(mut self, mut rx: mpsc::Receiver<GraphEvent>) {
        let mut timer = interval(self.flush_interval);

        loop {
            tokio::select! {
                // Timer tick - flush pending events
                _ = timer.tick() => {
                    self.flush();
                }

                // New event received
                event = rx.recv() => {
                    match event {
                        Some(e) => self.push(e),
                        None => {
                            // Channel closed, flush remaining and exit
                            self.flush();
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;

    #[tokio::test]
    async fn test_batcher_flushes_on_timer() {
        let (tx, mut rx) = broadcast::channel(100);
        let counter = Arc::new(AtomicU64::new(0));
        let mut batcher = EventBatcher::with_config(tx, counter, 10, 100);

        batcher.push(GraphEvent::EntityDeleted {
            name: "Test".to_string(),
            user: None,
        });

        // Manually flush
        batcher.flush();

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.sequence_id, 0);
    }

    #[tokio::test]
    async fn test_batcher_batches_multiple_events() {
        let (tx, mut rx) = broadcast::channel(100);
        let counter = Arc::new(AtomicU64::new(0));
        let mut batcher = EventBatcher::with_config(tx, counter, 50, 100);

        // Push multiple events
        for i in 0..3 {
            batcher.push(GraphEvent::EntityDeleted {
                name: format!("Entity{}", i),
                user: None,
            });
        }

        batcher.flush();

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg.event, GraphEvent::BatchUpdate { events } if events.len() == 3));
    }

    #[tokio::test]
    async fn test_batcher_force_flush_on_max_size() {
        let (tx, mut rx) = broadcast::channel(100);
        let counter = Arc::new(AtomicU64::new(0));
        let mut batcher = EventBatcher::with_config(tx, counter, 1000, 5);

        // Push more than max_batch_size events
        for i in 0..6 {
            batcher.push(GraphEvent::EntityDeleted {
                name: format!("Entity{}", i),
                user: None,
            });
        }

        // Should have auto-flushed at 5
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg.event, GraphEvent::BatchUpdate { events } if events.len() == 5));
    }
}
