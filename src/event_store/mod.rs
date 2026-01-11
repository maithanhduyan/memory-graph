//! Event Store Module for Event Sourcing
//!
//! This module provides the core event sourcing infrastructure:
//! - `EventStore`: Manages append-only event log and state replay
//! - `SnapshotManager`: Handles snapshot creation and loading
//! - `LogRotation`: Archives old events after snapshots
//! - `StatsCollector`: Collects metrics and statistics
//!
//! # Architecture
//!
//! ```text
//! Write Path:
//! ┌─────────┐    ┌─────────────┐    ┌──────────────────┐    ┌─────────────┐
//! │ MCP/UI  │───►│ append to   │───►│ maybe_snapshot() │───►│ rotate_log()|
//! │ Request │    │ events.jsonl│    │ every 1000 events│    │ archive old │
//! └─────────┘    └─────────────┘    └──────────────────┘    └─────────────┘
//!
//! Read Path (Startup):
//! ┌───────────────┐    ┌─────────────────┐
//! │ Load snapshot │───►│ Replay events   │───► Ready!
//! │ (latest.jsonl)│    │ after snapshot  │
//! └───────────────┘    └─────────────────┘
//! ```

mod migration;
mod rotation;
mod snapshot;
mod stats;
mod store;

pub use migration::{MigrationResult, MigrationTool};
pub use rotation::{ArchiveInfo, LogRotation};
pub use snapshot::SnapshotManager;
pub use stats::{EventStoreStats, ReplayBenchmark, StatsCollector};
pub use store::{EventStore, EventStoreConfig, EventStoreError, EventStoreResult};
