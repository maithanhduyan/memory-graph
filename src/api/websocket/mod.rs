//! WebSocket module for real-time UI updates
//!
//! Provides WebSocket endpoint at `/ws` for broadcasting graph changes to connected clients.
//!
//! ## Features
//! - Real-time entity/relation updates
//! - Event batching (debounce 50ms, max 100 events)
//! - Sequence ID tracking for gap detection
//! - Reconnection support with "Snapshot then Subscribe" strategy

pub mod events;
pub mod handler;
pub mod state;
pub mod batcher;
pub mod broadcaster;

// Re-export commonly used items
pub use broadcaster::{broadcast_event, get_broadcaster, init_broadcaster, helpers as ws_helpers};
