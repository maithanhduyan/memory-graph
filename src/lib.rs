//! Memory Graph MCP Server
//!
//! A knowledge graph server implementing the Model Context Protocol (MCP)
//! using pure Rust with minimal dependencies.
//!
//! # Features
//!
//! - **16 MCP Tools**: Full CRUD, query, temporal, and inference operations
//! - **Thread-Safe**: Production-ready with RwLock-based concurrency
//! - **Semantic Search**: Built-in synonym matching
//! - **Time Travel**: Query historical state with validFrom/validTo
//! - **Pagination**: Handle massive graphs with limit/offset
//! - **Inference Engine**: Discover hidden relations via logical rules
//!
//! # Modules
//!
//! - `types`: Core data structures (Entity, Relation, KnowledgeGraph)
//! - `protocol`: MCP and JSON-RPC protocol types
//! - `knowledge_base`: Core data engine with CRUD, queries, and inference
//! - `tools`: 16 MCP tool implementations
//! - `search`: Semantic search with synonym expansion
//! - `validation`: Entity and relation type validation
//! - `utils`: Utility functions (timestamps, etc.)
//! - `server`: MCP server implementation
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use memory_graph::{KnowledgeBase, McpServer, ServerInfo};
//! use memory_graph::tools::register_all_tools;
//!
//! fn main() {
//!     let kb = Arc::new(KnowledgeBase::new());
//!     let server_info = ServerInfo::new("memory".to_string(), "1.0.0".to_string());
//!     let mut server = McpServer::with_info(server_info);
//!     register_all_tools(&mut server, kb);
//!     server.run().unwrap();
//! }
//! ```

pub mod api;
pub mod event_store;
pub mod knowledge_base;
pub mod protocol;
pub mod search;
pub mod server;
pub mod tools;
pub mod types;
pub mod utils;
pub mod validation;

// Re-export commonly used items at crate root
pub use api::websocket::{state::AppState, events::GraphEvent};
pub use api::http::create_router;
pub use event_store::{
    ArchiveInfo, EventStore, EventStoreConfig, EventStoreError, EventStoreResult,
    EventStoreStats, LogRotation, MigrationResult, MigrationTool, ReplayBenchmark,
    SnapshotManager, StatsCollector,
};
pub use knowledge_base::inference::{InferenceEngine, InferenceRule};
pub use knowledge_base::KnowledgeBase;
pub use protocol::{McpTool, ServerInfo, Tool};
pub use server::McpServer;
pub use types::{
    Entity, EntityBrief, Event, EventData, EventSource, EventType, InferResult, InferStats,
    InferredRelation, KnowledgeGraph, McpResult, Observation, ObservationDeletion, PathStep,
    RelatedEntities, RelatedEntity, Relation, SnapshotMeta, Summary, TraversalPath, TraversalResult,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
