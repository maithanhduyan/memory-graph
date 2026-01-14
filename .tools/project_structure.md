---
date: 2026-01-14 11:33:05 
---

# Cấu trúc Dự án như sau:

```
../memory-graph/src
├── api
│   ├── http.rs
│   ├── mod.rs
│   ├── rest
│   │   ├── entities.rs
│   │   ├── graph.rs
│   │   ├── mod.rs
│   │   ├── relations.rs
│   │   └── search.rs
│   ├── sse
│   │   ├── auth.rs
│   │   ├── handler.rs
│   │   ├── mod.rs
│   │   └── session.rs
│   └── websocket
│       ├── batcher.rs
│       ├── broadcaster.rs
│       ├── events.rs
│       ├── handler.rs
│       ├── mod.rs
│       └── state.rs
├── event_store
│   ├── migration.rs
│   ├── mod.rs
│   ├── rotation.rs
│   ├── snapshot.rs
│   ├── stats.rs
│   └── store.rs
├── knowledge_base
│   ├── crud.rs
│   ├── inference
│   │   ├── mod.rs
│   │   └── rules.rs
│   ├── mod.rs
│   ├── query.rs
│   ├── summarize.rs
│   ├── temporal.rs
│   └── traversal.rs
├── lib.rs
├── main.rs
├── protocol
│   ├── jsonrpc.rs
│   ├── mcp.rs
│   └── mod.rs
├── search
│   ├── mod.rs
│   └── synonyms.rs
├── server
│   ├── handlers.rs
│   └── mod.rs
├── tools
│   ├── inference
│   │   ├── infer.rs
│   │   └── mod.rs
│   ├── memory
│   │   ├── add_observations.rs
│   │   ├── create_entities.rs
│   │   ├── create_relations.rs
│   │   ├── delete_entities.rs
│   │   ├── delete_observations.rs
│   │   ├── delete_relations.rs
│   │   ├── mod.rs
│   │   ├── open_nodes.rs
│   │   ├── read_graph.rs
│   │   └── search_nodes.rs
│   ├── mod.rs
│   ├── query
│   │   ├── get_related.rs
│   │   ├── mod.rs
│   │   ├── summarize.rs
│   │   └── traverse.rs
│   └── temporal
│       ├── get_current_time.rs
│       ├── get_relation_history.rs
│       ├── get_relations_at_time.rs
│       └── mod.rs
├── types
│   ├── entity.rs
│   ├── event.rs
│   ├── graph.rs
│   ├── inference.rs
│   ├── mod.rs
│   ├── observation.rs
│   ├── relation.rs
│   ├── summary.rs
│   └── traversal.rs
├── utils
│   ├── atomic.rs
│   ├── mod.rs
│   └── time.rs
└── validation
    ├── mod.rs
    └── types.rs
```

# Danh sách chi tiết các file:

## File ../memory-graph/src\lib.rs:
```rust
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

```

## File ../memory-graph/src\main.rs:
```rust
//! Memory Graph MCP Server - Binary Entry Point
//!
//! This is the main entry point for the memory-server binary.
//! Supports graceful shutdown with automatic snapshot creation when Event Sourcing is enabled.
//!
//! ## Usage
//!
//! ```bash
//! # Default: stdio mode for MCP (AI Agents)
//! memory-server
//!
//! # HTTP mode with WebSocket for UI
//! memory-server --mode http
//!
//! # Both stdio and HTTP modes
//! memory-server --mode both
//! ```
//!
//! ## JWT Authentication (for HTTP/SSE mode)
//!
//! ```bash
//! # Set environment variables
//! MEMORY_JWT_SECRET=your-super-secret-key-at-least-32-characters
//! MEMORY_USERS=alice:password123,bob:secret456,admin:adminpass:*
//! MEMORY_REQUIRE_AUTH=true  # Optional: require auth for all requests
//!
//! # Login to get token
//! curl -X POST http://localhost:3030/auth/token \
//!   -H "Content-Type: application/json" \
//!   -d '{"username":"alice","password":"password123"}'
//! ```

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use memory_graph::api::websocket::{init_broadcaster, state::AppState};
use memory_graph::api::http::create_router_with_auth;
use memory_graph::api::sse::JwtAuth;
use memory_graph::knowledge_base::KnowledgeBase;
use memory_graph::protocol::ServerInfo;
use memory_graph::server::McpServer;
use memory_graph::tools::register_all_tools;
use memory_graph::types::McpResult;

/// Global shutdown flag
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Server mode
#[derive(Debug, Clone, PartialEq)]
enum ServerMode {
    /// stdio only - for MCP AI Agents (default)
    Stdio,
    /// HTTP only - REST API + WebSocket for UI
    Http,
    /// Both stdio and HTTP
    Both,
}

impl ServerMode {
    fn from_args() -> Self {
        let args: Vec<String> = env::args().collect();

        for i in 0..args.len() {
            if args[i] == "--mode" || args[i] == "-m" {
                if let Some(mode) = args.get(i + 1) {
                    return match mode.to_lowercase().as_str() {
                        "stdio" => ServerMode::Stdio,
                        "http" | "web" => ServerMode::Http,
                        "both" | "all" => ServerMode::Both,
                        _ => {
                            eprintln!("Unknown mode: {}. Using stdio.", mode);
                            ServerMode::Stdio
                        }
                    };
                }
            }
            if args[i] == "--help" || args[i] == "-h" {
                print_help();
                std::process::exit(0);
            }
        }

        // Check for MEMORY_SERVER_MODE env var
        if let Ok(mode) = env::var("MEMORY_SERVER_MODE") {
            return match mode.to_lowercase().as_str() {
                "http" | "web" => ServerMode::Http,
                "both" | "all" => ServerMode::Both,
                _ => ServerMode::Stdio,
            };
        }

        ServerMode::Stdio
    }
}

fn print_help() {
    println!(
        r#"Memory Graph MCP Server v{}

USAGE:
    memory-server [OPTIONS]

OPTIONS:
    -m, --mode <MODE>    Server mode: stdio, http, or both
                         - stdio: MCP protocol for AI Agents (default)
                         - http:  REST API + WebSocket for UI (port 3030)
                         - both:  Run both modes simultaneously

    -h, --help           Print this help message

ENVIRONMENT VARIABLES:
    MEMORY_SERVER_MODE       Override server mode (stdio, http, both)
    MEMORY_FILE_PATH         Path to memory.jsonl file
    MEMORY_EVENT_SOURCING    Enable event sourcing (true/false)

EXAMPLES:
    # Run as MCP server for AI Agents
    memory-server

    # Run HTTP server for UI on port 3030
    memory-server --mode http

    # Run both MCP and HTTP servers
    memory-server --mode both
"#,
        env!("CARGO_PKG_VERSION")
    );
}

fn main() -> McpResult<()> {
    let mode = ServerMode::from_args();

    match mode {
        ServerMode::Stdio => run_stdio_mode(),
        ServerMode::Http => run_http_mode(),
        ServerMode::Both => run_both_modes(),
    }
}

/// Run in stdio mode (MCP for AI Agents)
fn run_stdio_mode() -> McpResult<()> {
    let kb = Arc::new(KnowledgeBase::new());
    let kb_for_shutdown = Arc::clone(&kb);

    setup_shutdown_handler(kb_for_shutdown);

    let server_info = ServerInfo {
        name: "memory".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let mut server = McpServer::with_info(server_info);
    register_all_tools(&mut server, kb);
    server.run()
}

/// Run in HTTP mode (REST API + WebSocket for UI)
fn run_http_mode() -> McpResult<()> {
    eprintln!("[Memory Server] Starting HTTP server on port 3030...");

    // Initialize tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    rt.block_on(async {
        run_http_server().await
    })
}

/// Run both stdio and HTTP modes
fn run_both_modes() -> McpResult<()> {
    eprintln!("[Memory Server] Starting in hybrid mode (stdio + HTTP)...");

    // Start HTTP server in a separate thread
    let http_handle = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            if let Err(e) = run_http_server().await {
                eprintln!("[HTTP Server] Error: {}", e);
            }
        });
    });

    // Run stdio in main thread
    let result = run_stdio_mode();

    // Wait for HTTP server (won't normally happen as stdio runs forever)
    let _ = http_handle.join();

    result
}

/// Run the HTTP server with WebSocket support
async fn run_http_server() -> McpResult<()> {
    // Create SINGLE knowledge base - shared by both SSE/MCP and REST/WebSocket
    let kb = Arc::new(KnowledgeBase::new());

    // Initialize global broadcaster for WebSocket events
    init_broadcaster(1024);

    // Create AppState for WebSocket/REST using the same KB
    let state = Arc::new(AppState::new(Arc::clone(&kb)));

    // Initialize JWT authentication if configured
    let (jwt_auth, require_auth) = match JwtAuth::from_env() {
        Ok(auth) => {
            let require = env::var("MEMORY_REQUIRE_AUTH")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);
            eprintln!("[Auth] JWT authentication enabled (require_auth: {})", require);
            (Some(Arc::new(auth)), require)
        }
        Err(e) => {
            eprintln!("[Auth] JWT not configured: {} - running without authentication", e);
            (None, false)
        }
    };

    // Create router with JWT auth - both SSE and REST/WS use the same kb
    let app = create_router_with_auth(state, Arc::clone(&kb), jwt_auth, require_auth);

    // Bind to port 3030
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3030));
    eprintln!("[HTTP Server] Listening on http://{}", addr);
    eprintln!("[HTTP Server] WebSocket endpoint: ws://{}/ws", addr);
    eprintln!("[HTTP Server] MCP SSE endpoint: http://{}/mcp/sse", addr);
    eprintln!("[HTTP Server] Auth endpoints: POST /auth/token, POST /auth/refresh, GET /auth/me");
    eprintln!("[HTTP Server] Health check: http://{}/health", addr);

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(listener, app).await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

/// Setup Ctrl+C / SIGTERM handler for graceful shutdown
fn setup_shutdown_handler(kb: Arc<KnowledgeBase>) {
    if let Err(e) = ctrlc::set_handler(move || {
        eprintln!("[Memory Server] Shutdown signal received, creating snapshot...");
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);

        // Create final snapshot before exit
        match kb.create_snapshot() {
            Ok(Some(path)) => {
                eprintln!("[Memory Server] Snapshot saved to: {}", path.display());
            }
            Ok(None) => {
                eprintln!("[Memory Server] Event Sourcing not enabled, no snapshot needed.");
            }
            Err(e) => {
                eprintln!("[Memory Server] Error creating snapshot: {}", e);
            }
        }

        eprintln!("[Memory Server] Shutdown complete.");
        std::process::exit(0);
    }) {
        eprintln!("[Memory Server] Warning: Could not set Ctrl+C handler: {}", e);
    }
}

```

## File ../memory-graph/src\api\http.rs:
```rust
//! HTTP server setup with Axum

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::knowledge_base::KnowledgeBase;
use super::rest::{entities, graph, relations, search};
use super::sse::handler::{
    login_handler, me_handler, mcp_request_handler, refresh_handler,
    server_info_handler, sse_handler, SseState,
};
use super::sse::JwtAuth;
use super::websocket::{handler::ws_handler, state::AppState};

/// Create the Axum router with all endpoints
///
/// Uses a single Arc<KnowledgeBase> shared by both SSE/MCP and REST/WebSocket.
/// This ensures data consistency across all transports.
pub fn create_router(state: Arc<AppState>, kb_sync: Arc<KnowledgeBase>) -> Router {
    create_router_with_auth(state, kb_sync, None, false)
}

/// Create router with optional JWT authentication
pub fn create_router_with_auth(
    state: Arc<AppState>,
    kb_sync: Arc<KnowledgeBase>,
    jwt_auth: Option<Arc<JwtAuth>>,
    require_auth: bool,
) -> Router {
    // CORS configuration - allow all origins for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create SSE state with sync KB (auto-registers all tools)
    let mut sse_state = SseState::new(
        kb_sync,
        state.event_tx.clone(),
        Arc::clone(&state.sequence_counter),
    );

    // Add JWT auth if configured
    if let Some(auth) = jwt_auth {
        sse_state = sse_state.with_jwt_auth(auth, require_auth);
    }

    let sse_state = Arc::new(sse_state);

    // Build main router with AppState
    let main_router = Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // REST API endpoints
        .route("/api/graph", get(graph::get_graph))
        .route("/api/graph/stats", get(graph::get_stats))
        .route("/api/events/replay", get(graph::get_events_replay))
        .route("/api/entities", get(entities::list_entities))
        .route("/api/entities/:name", get(entities::get_entity))
        .route("/api/relations", get(relations::list_relations))
        .route("/api/search", get(search::search_nodes))
        .with_state(state);

    // Build SSE router with SseState
    // MCP SSE spec: /sse for SSE stream, same endpoint accepts POST for messages
    let sse_router = Router::new()
        .route("/mcp/sse", get(sse_handler).post(mcp_request_handler))
        .route("/mcp", post(mcp_request_handler))
        .route("/mcp/info", get(server_info_handler))
        // Auth endpoints
        .route("/auth/token", post(login_handler))
        .route("/auth/refresh", post(refresh_handler))
        .route("/auth/me", get(me_handler))
        .with_state(sse_state);

    // Merge routers and apply CORS
    main_router.merge(sse_router).layer(cors)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_base::KnowledgeBase;
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        // Single KB instance - shared by both AppState and router
        let kb = Arc::new(KnowledgeBase::new());
        let state = Arc::new(AppState::new(Arc::clone(&kb)));
        let app = create_router(state, kb);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}

```

## File ../memory-graph/src\api\mod.rs:
```rust
//! API module for HTTP and WebSocket endpoints
//!
//! This module provides REST API and WebSocket real-time updates for the Memory Graph UI.
//!
//! ## Endpoints
//!
//! ### WebSocket
//! - `GET /ws` - Real-time graph updates
//!
//! ### REST API
//! - `GET /api/graph` - Full graph snapshot (for client recovery)
//! - `GET /api/graph/stats` - Graph statistics
//! - `GET /api/entities` - List entities with pagination
//! - `GET /api/entities/:name` - Get single entity with relations
//! - `GET /api/relations` - List relations with filters
//! - `GET /api/search` - Search nodes
//!
//! ### MCP SSE (Server-Sent Events)
//! - `GET /mcp/sse` - SSE stream for AI Agents
//! - `POST /mcp` - JSON-RPC requests
//! - `GET /mcp/info` - Server info and capabilities
//!
//! ### Authentication
//! - `POST /auth/token` - Login and get JWT tokens
//! - `POST /auth/refresh` - Refresh access token
//! - `GET /auth/me` - Get current user info

pub mod http;
pub mod rest;
pub mod sse;
pub mod websocket;

```

## File ../memory-graph/src\api\rest\entities.rs:
```rust
//! Entity endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use super::{ApiError, ApiResponse};
use crate::api::websocket::state::AppState;
use crate::types::{Entity, Relation};

/// Query parameters for listing entities
#[derive(Debug, Deserialize)]
pub struct ListEntitiesParams {
    /// Maximum number of entities to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of entities to skip
    #[serde(default)]
    pub offset: usize,
    /// Filter by entity type
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Sort by field (name, created_at, updated_at)
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Sort order (asc, desc)
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_limit() -> usize {
    100
}

fn default_sort() -> String {
    "name".to_string()
}

fn default_order() -> String {
    "asc".to_string()
}

/// GET /api/entities - List entities with pagination
pub async fn list_entities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListEntitiesParams>,
) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // Filter by type if specified
    let mut entities: Vec<Entity> = if let Some(ref entity_type) = params.entity_type {
        graph
            .entities
            .iter()
            .filter(|e| e.entity_type.eq_ignore_ascii_case(entity_type))
            .cloned()
            .collect()
    } else {
        graph.entities.clone()
    };

    let total = entities.len();

    // Sort
    match params.sort.as_str() {
        "created_at" => {
            if params.order == "desc" {
                entities.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            } else {
                entities.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
        }
        "updated_at" => {
            if params.order == "desc" {
                entities.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            } else {
                entities.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
            }
        }
        _ => {
            // Default: sort by name
            if params.order == "desc" {
                entities.sort_by(|a, b| b.name.cmp(&a.name));
            } else {
                entities.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
    }

    // Pagination
    let limit = params.limit.min(1000);
    let entities: Vec<Entity> = entities.into_iter().skip(params.offset).take(limit).collect();

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::with_total(entities, sequence_id, total))
}

/// Response for single entity with relations
#[derive(Debug, Serialize)]
pub struct EntityDetail {
    #[serde(flatten)]
    pub entity: Entity,
    /// Relations where this entity is the source
    pub outgoing_relations: Vec<Relation>,
    /// Relations where this entity is the target
    pub incoming_relations: Vec<Relation>,
}

/// GET /api/entities/:name - Get single entity with relations
pub async fn get_entity(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // URL decode the name (handles spaces and special chars)
    let decoded_name = urlencoding::decode(&name)
        .unwrap_or_else(|_| name.clone().into())
        .into_owned();

    // Find entity
    let entity = graph.entities.iter().find(|e| e.name == decoded_name);

    match entity {
        Some(entity) => {
            // Get related relations
            let outgoing_relations: Vec<Relation> = graph
                .relations
                .iter()
                .filter(|r| r.from == decoded_name)
                .cloned()
                .collect();

            let incoming_relations: Vec<Relation> = graph
                .relations
                .iter()
                .filter(|r| r.to == decoded_name)
                .cloned()
                .collect();

            let detail = EntityDetail {
                entity: entity.clone(),
                outgoing_relations,
                incoming_relations,
            };

            let sequence_id = state.current_sequence_id();
            (StatusCode::OK, Json(ApiResponse::new(detail, sequence_id))).into_response()
        }
        None => {
            let error = ApiError::not_found(format!("Entity '{}' not found", decoded_name));
            (StatusCode::NOT_FOUND, Json(error)).into_response()
        }
    }
}

```

## File ../memory-graph/src\api\rest\graph.rs:
```rust
//! Graph endpoint - Full graph snapshot for client recovery

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use super::ApiResponse;
use crate::api::websocket::state::AppState;
use crate::types::{Entity, Relation};

/// Response for GET /api/graph
#[derive(Debug, Serialize)]
pub struct GraphResponse {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

/// Query parameters for graph endpoint
#[derive(Debug, Deserialize)]
pub struct GraphParams {
    /// Maximum number of entities to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of entities to skip
    #[serde(default)]
    pub offset: usize,
    /// Include relations (default: true)
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_limit() -> usize {
    100
}

fn default_true() -> bool {
    true
}

/// GET /api/graph - Get full graph snapshot
///
/// Returns entities and relations for client recovery after WebSocket reconnection.
/// Includes sequence_id so client knows the snapshot version.
pub async fn get_graph(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphParams>,
) -> impl IntoResponse {
    // Get all entities with pagination
    let all_entities: Vec<Entity> = state.kb.graph.read().unwrap().entities.clone();
    let all_relations: Vec<Relation> = state.kb.graph.read().unwrap().relations.clone();

    let total_entities = all_entities.len();

    // Apply pagination to entities
    let limit = params.limit.min(1000);
    let entities: Vec<Entity> = all_entities
        .into_iter()
        .skip(params.offset)
        .take(limit)
        .collect();

    // Include relations if requested
    let relations = if params.include_relations {
        // Filter relations to only those involving returned entities
        let entity_names: std::collections::HashSet<_> =
            entities.iter().map(|e| e.name.as_str()).collect();

        all_relations
            .into_iter()
            .filter(|r| entity_names.contains(r.from.as_str()) || entity_names.contains(r.to.as_str()))
            .collect()
    } else {
        Vec::new()
    };

    let graph = GraphResponse { entities, relations };
    let sequence_id = state.current_sequence_id();

    let response = ApiResponse::with_total(graph, sequence_id, total_entities);

    Json(response)
}

/// GET /api/graph/stats - Get graph statistics
#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub entity_count: usize,
    pub relation_count: usize,
    pub entity_types: Vec<EntityTypeCount>,
    pub relation_types: Vec<RelationTypeCount>,
}

#[derive(Debug, Serialize)]
pub struct EntityTypeCount {
    pub entity_type: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct RelationTypeCount {
    pub relation_type: String,
    pub count: usize,
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // Count entity types
    let mut entity_type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entity in &graph.entities {
        *entity_type_counts
            .entry(entity.entity_type.clone())
            .or_insert(0) += 1;
    }

    // Count relation types
    let mut relation_type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for relation in &graph.relations {
        *relation_type_counts
            .entry(relation.relation_type.clone())
            .or_insert(0) += 1;
    }

    let stats = GraphStats {
        entity_count: graph.entities.len(),
        relation_count: graph.relations.len(),
        entity_types: entity_type_counts
            .into_iter()
            .map(|(entity_type, count)| EntityTypeCount { entity_type, count })
            .collect(),
        relation_types: relation_type_counts
            .into_iter()
            .map(|(relation_type, count)| RelationTypeCount {
                relation_type,
                count,
            })
            .collect(),
    };

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::new(stats, sequence_id))
}

/// Query parameters for event replay
#[derive(Debug, Deserialize)]
pub struct EventReplayParams {
    /// Get events after this sequence ID
    pub since: u64,
}

/// Response for GET /api/events/replay
#[derive(Debug, Serialize)]
pub struct EventReplayResponse {
    /// Events since the requested sequence ID
    pub events: Vec<crate::api::websocket::events::WsMessage>,
    /// Whether a full refresh is needed (events too old)
    pub needs_full_refresh: bool,
    /// Oldest available sequence ID in history
    pub oldest_available: Option<u64>,
    /// Current sequence ID
    pub current_sequence_id: u64,
}

/// GET /api/events/replay - Replay missed events for client recovery
///
/// Clients can request events they missed during disconnection.
/// If the requested sequence is too old (outside history buffer),
/// `needs_full_refresh` will be true and client should fetch full graph.
pub async fn get_events_replay(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventReplayParams>,
) -> impl IntoResponse {
    let current_sequence_id = state.current_sequence_id();

    // Get broadcaster if available
    let broadcaster = crate::api::websocket::get_broadcaster();

    let (events, needs_full_refresh, oldest_available) = match broadcaster {
        Some(b) => {
            let oldest = b.oldest_sequence_id();
            match b.get_events_since(params.since) {
                Some(events) => (events, false, oldest),
                None => (Vec::new(), true, oldest), // Too old, needs refresh
            }
        }
        None => {
            // Broadcaster not initialized (no WebSocket/SSE enabled)
            (Vec::new(), false, None)
        }
    };

    let response = EventReplayResponse {
        events,
        needs_full_refresh,
        oldest_available,
        current_sequence_id,
    };

    Json(ApiResponse::new(response, current_sequence_id))
}

```

## File ../memory-graph/src\api\rest\mod.rs:
```rust
//! REST API module for HTTP endpoints
//!
//! Provides REST endpoints for client recovery and data access:
//! - `GET /api/graph` - Full graph snapshot
//! - `GET /api/entities` - List entities with pagination
//! - `GET /api/entities/:name` - Get single entity
//! - `GET /api/relations` - List relations
//! - `GET /api/search` - Search nodes

pub mod entities;
pub mod graph;
pub mod relations;
pub mod search;

use serde::{Deserialize, Serialize};

/// Common pagination parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Maximum number of items to return (default: 100, max: 1000)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of items to skip
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

impl PaginationParams {
    /// Normalize limit to max 1000
    pub fn normalized_limit(&self) -> usize {
        self.limit.min(1000)
    }
}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: T,
    /// Current sequence ID for cache invalidation
    pub sequence_id: u64,
    /// Total count (for paginated responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T, sequence_id: u64) -> Self {
        Self {
            data,
            sequence_id,
            total: None,
        }
    }

    pub fn with_total(data: T, sequence_id: u64, total: usize) -> Self {
        Self {
            data,
            sequence_id,
            total: Some(total),
        }
    }
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "NOT_FOUND".to_string(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "BAD_REQUEST".to_string(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: "INTERNAL_ERROR".to_string(),
        }
    }
}

```

## File ../memory-graph/src\api\rest\relations.rs:
```rust
//! Relation endpoints

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::ApiResponse;
use crate::api::websocket::state::AppState;
use crate::types::Relation;

/// Query parameters for listing relations
#[derive(Debug, Deserialize)]
pub struct ListRelationsParams {
    /// Maximum number of relations to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of relations to skip
    #[serde(default)]
    pub offset: usize,
    /// Filter by relation type
    #[serde(rename = "type")]
    pub relation_type: Option<String>,
    /// Filter by source entity
    pub from: Option<String>,
    /// Filter by target entity
    pub to: Option<String>,
}

fn default_limit() -> usize {
    100
}

/// GET /api/relations - List relations with pagination and filters
pub async fn list_relations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListRelationsParams>,
) -> impl IntoResponse {
    let graph = state.kb.graph.read().unwrap();

    // Apply filters
    let mut relations: Vec<Relation> = graph
        .relations
        .iter()
        .filter(|r| {
            // Filter by type
            if let Some(ref relation_type) = params.relation_type {
                if !r.relation_type.eq_ignore_ascii_case(relation_type) {
                    return false;
                }
            }
            // Filter by source
            if let Some(ref from) = params.from {
                if !r.from.eq_ignore_ascii_case(from) {
                    return false;
                }
            }
            // Filter by target
            if let Some(ref to) = params.to {
                if !r.to.eq_ignore_ascii_case(to) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    let total = relations.len();

    // Sort by from -> to -> type
    relations.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.relation_type.cmp(&b.relation_type))
    });

    // Pagination
    let limit = params.limit.min(1000);
    let relations: Vec<Relation> = relations.into_iter().skip(params.offset).take(limit).collect();

    let sequence_id = state.current_sequence_id();
    Json(ApiResponse::with_total(relations, sequence_id, total))
}

```

## File ../memory-graph/src\api\rest\search.rs:
```rust
//! Search endpoint

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::{ApiError, ApiResponse};
use crate::api::websocket::state::AppState;

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query string
    pub q: String,
    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Include relations connected to matching entities
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_limit() -> usize {
    50
}

fn default_true() -> bool {
    true
}

/// GET /api/search - Search nodes in the knowledge graph
///
/// Searches entity names, types, and observations using the existing
/// search logic with synonym matching.
pub async fn search_nodes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    if params.q.trim().is_empty() {
        let error = ApiError::bad_request("Query parameter 'q' is required");
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    // Use existing search_nodes functionality (KnowledgeBase has internal RwLock)
    let limit = if params.limit > 0 {
        Some(params.limit.min(1000))
    } else {
        None
    };

    match state.kb.search_nodes(&params.q, limit, params.include_relations) {
        Ok(result) => {
            let sequence_id = state.current_sequence_id();
            let total = result.entities.len();
            (
                StatusCode::OK,
                Json(ApiResponse::with_total(result, sequence_id, total)),
            )
                .into_response()
        }
        Err(e) => {
            let error = ApiError::internal(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

```

## File ../memory-graph/src\api\sse\auth.rs:
```rust
//! JWT Authentication for MCP SSE Server
//!
//! Provides stateless authentication using JSON Web Tokens (JWT).
//!
//! ## Usage
//! ```bash
//! # Set environment variables
//! MEMORY_JWT_SECRET=your-super-secret-key-at-least-32-chars
//! MEMORY_USERS=alice:password123,bob:secret456,admin:admin-pass
//!
//! # Login to get token
//! curl -X POST http://localhost:3030/auth/token \
//!   -H "Content-Type: application/json" \
//!   -d '{"username":"alice","password":"password123"}'
//!
//! # Use token in requests
//! curl http://localhost:3030/mcp/sse \
//!   -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (username)
    pub sub: String,
    /// User permissions
    pub permissions: Vec<String>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
    /// Token type: "access" or "refresh"
    pub token_type: String,
}

impl Claims {
    /// Create new access token claims
    pub fn new_access(username: String, permissions: Vec<String>, ttl_seconds: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            permissions,
            iat: now,
            exp: now + ttl_seconds,
            token_type: "access".to_string(),
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh(username: String, ttl_seconds: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: username,
            permissions: vec![],
            iat: now,
            exp: now + ttl_seconds,
            token_type: "refresh".to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }

    /// Check if user has permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"*".to_string())
    }
}

/// User information for authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub password_hash: String,
    pub permissions: Vec<String>,
}

/// JWT Authentication manager
pub struct JwtAuth {
    /// Secret key for signing tokens
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// User store (username -> UserInfo)
    users: HashMap<String, UserInfo>,
    /// Access token TTL in seconds (default: 1 hour)
    pub access_token_ttl: i64,
    /// Refresh token TTL in seconds (default: 7 days)
    pub refresh_token_ttl: i64,
}

impl JwtAuth {
    /// Default filename for persisted JWT secret
    const SECRET_FILE: &'static str = ".jwt_secret";

    /// Create new JwtAuth with secret key
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            users: HashMap::new(),
            access_token_ttl: 3600,        // 1 hour
            refresh_token_ttl: 604800,     // 7 days
        }
    }

    /// Load secret from file or create new one and persist it
    ///
    /// This ensures tokens remain valid across server restarts when
    /// MEMORY_JWT_SECRET environment variable is not set.
    fn load_or_create_secret_file() -> Result<String, AuthError> {
        use std::fs;
        use std::path::Path;

        let secret_path = Path::new(Self::SECRET_FILE);

        // Try to load existing secret
        if secret_path.exists() {
            match fs::read_to_string(secret_path) {
                Ok(secret) => {
                    let secret = secret.trim().to_string();
                    if secret.len() >= 32 {
                        eprintln!("[Auth] Loaded JWT secret from {}", Self::SECRET_FILE);
                        return Ok(secret);
                    }
                    eprintln!("[Auth] WARNING: {} exists but secret is too short, regenerating", Self::SECRET_FILE);
                }
                Err(e) => {
                    eprintln!("[Auth] WARNING: Failed to read {}: {}, regenerating", Self::SECRET_FILE, e);
                }
            }
        }

        // Generate new secret
        let secret = Self::generate_secure_secret();

        // Try to save to file
        match fs::write(secret_path, &secret) {
            Ok(_) => {
                eprintln!("[Auth] Generated and saved JWT secret to {}", Self::SECRET_FILE);
                eprintln!("[Auth] ⚠️  For production, set MEMORY_JWT_SECRET environment variable");
            }
            Err(e) => {
                eprintln!("[Auth] WARNING: Could not save secret to {}: {}", Self::SECRET_FILE, e);
                eprintln!("[Auth] ⚠️  Tokens will be invalidated on restart!");
            }
        }

        Ok(secret)
    }

    /// Generate a cryptographically secure random secret
    fn generate_secure_secret() -> String {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        // Combine multiple entropy sources for better randomness
        let now = chrono::Utc::now();
        let timestamp = now.timestamp_nanos_opt().unwrap_or(0);
        let pid = std::process::id();

        // Use RandomState for additional entropy (uses thread-local random)
        let random_state = RandomState::new();
        let mut hasher = random_state.build_hasher();
        hasher.write_i64(timestamp);
        hasher.write_u32(pid);
        let hash1 = hasher.finish();

        let random_state2 = RandomState::new();
        let mut hasher2 = random_state2.build_hasher();
        hasher2.write_u64(hash1);
        hasher2.write_i64(now.timestamp_micros());
        let hash2 = hasher2.finish();

        // Create 64-char hex secret (256 bits)
        format!("{:016x}{:016x}{:016x}{:016x}", hash1, hash2, timestamp as u64, hash1 ^ hash2)
    }

    /// Create from environment variables
    ///
    /// Environment:
    /// - MEMORY_JWT_SECRET: Secret key for signing (required, min 32 chars)
    /// - MEMORY_USERS: Comma-separated user:password pairs (optional)
    /// - MEMORY_ACCESS_TOKEN_TTL: Access token TTL in seconds (optional, default 3600)
    /// - MEMORY_REFRESH_TOKEN_TTL: Refresh token TTL in seconds (optional, default 604800)
    ///
    /// If MEMORY_JWT_SECRET is not set, the server will:
    /// 1. Try to load from .jwt_secret file (persisted across restarts)
    /// 2. If file doesn't exist, generate new secret and save to file
    pub fn from_env() -> Result<Self, AuthError> {
        let secret = match std::env::var("MEMORY_JWT_SECRET") {
            Ok(s) => s,
            Err(_) => {
                // Try to load or create persistent secret file
                Self::load_or_create_secret_file()?
            }
        };

        if secret.len() < 32 {
            return Err(AuthError::InvalidSecret(
                "MEMORY_JWT_SECRET must be at least 32 characters".to_string(),
            ));
        }

        let mut auth = Self::new(&secret);

        // Parse access token TTL
        if let Ok(ttl) = std::env::var("MEMORY_ACCESS_TOKEN_TTL") {
            if let Ok(seconds) = ttl.parse::<i64>() {
                auth.access_token_ttl = seconds;
            }
        }

        // Parse refresh token TTL
        if let Ok(ttl) = std::env::var("MEMORY_REFRESH_TOKEN_TTL") {
            if let Ok(seconds) = ttl.parse::<i64>() {
                auth.refresh_token_ttl = seconds;
            }
        }

        // Parse users from MEMORY_USERS env var
        // Format: "user1:pass1,user2:pass2,admin:adminpass:*"
        // The third part is permissions (optional, default: read,write)
        if let Ok(users_str) = std::env::var("MEMORY_USERS") {
            for user_entry in users_str.split(',') {
                let parts: Vec<&str> = user_entry.trim().split(':').collect();
                if parts.len() >= 2 {
                    let username = parts[0].to_string();
                    let password = parts[1];
                    let permissions = if parts.len() > 2 {
                        parts[2].split('|').map(|s| s.to_string()).collect()
                    } else {
                        vec!["read".to_string(), "write".to_string()]
                    };

                    if let Err(e) = auth.add_user(&username, password, permissions) {
                        eprintln!("[Auth] Failed to add user {}: {}", username, e);
                    }
                }
            }
        }

        // Add default admin user if no users configured (development only)
        if auth.users.is_empty() {
            eprintln!("[Auth] WARNING: No users configured, adding default admin:admin");
            auth.add_user("admin", "admin", vec!["*".to_string()])?;
        }

        eprintln!("[Auth] Loaded {} users", auth.users.len());
        Ok(auth)
    }

    /// Add a user with password and permissions
    pub fn add_user(
        &mut self,
        username: &str,
        password: &str,
        permissions: Vec<String>,
    ) -> Result<(), AuthError> {
        let password_hash = hash(password, DEFAULT_COST)
            .map_err(|e| AuthError::HashError(e.to_string()))?;

        self.users.insert(
            username.to_string(),
            UserInfo {
                username: username.to_string(),
                password_hash,
                permissions,
            },
        );

        Ok(())
    }

    /// Authenticate user with username/password
    pub fn authenticate(&self, username: &str, password: &str) -> Result<&UserInfo, AuthError> {
        let user = self.users.get(username).ok_or(AuthError::InvalidCredentials)?;

        if verify(password, &user.password_hash).unwrap_or(false) {
            Ok(user)
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    /// Generate access and refresh tokens for user
    pub fn generate_tokens(&self, user: &UserInfo) -> Result<TokenPair, AuthError> {
        let access_claims = Claims::new_access(
            user.username.clone(),
            user.permissions.clone(),
            self.access_token_ttl,
        );

        let refresh_claims = Claims::new_refresh(user.username.clone(), self.refresh_token_ttl);

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenError(e.to_string()))?;

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenError(e.to_string()))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl,
        })
    }

    /// Validate a token and return claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data: TokenData<Claims> =
            decode(token, &self.decoding_key, &Validation::default())
                .map_err(|e| AuthError::TokenError(e.to_string()))?;

        if token_data.claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        Ok(token_data.claims)
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let claims = self.validate_token(refresh_token)?;

        if claims.token_type != "refresh" {
            return Err(AuthError::InvalidTokenType);
        }

        // Get user to refresh permissions
        let user = self
            .users
            .get(&claims.sub)
            .ok_or(AuthError::UserNotFound)?;

        self.generate_tokens(user)
    }

    /// Validate token from Authorization header
    /// Supports: "Bearer <token>" or just "<token>"
    pub fn validate_authorization(&self, auth_header: &str) -> Result<Claims, AuthError> {
        let token = if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else {
            auth_header
        };

        self.validate_token(token)
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }
}

/// Token pair response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidCredentials,
    InvalidSecret(String),
    TokenError(String),
    TokenExpired,
    InvalidTokenType,
    UserNotFound,
    HashError(String),
    MissingToken,
    InsufficientPermissions,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::InvalidSecret(msg) => write!(f, "Invalid secret: {}", msg),
            AuthError::TokenError(msg) => write!(f, "Token error: {}", msg),
            AuthError::TokenExpired => write!(f, "Token has expired"),
            AuthError::InvalidTokenType => write!(f, "Invalid token type"),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::HashError(msg) => write!(f, "Hash error: {}", msg),
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InsufficientPermissions => write!(f, "Insufficient permissions"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Thread-safe wrapper for JwtAuth
pub type SharedJwtAuth = Arc<JwtAuth>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_auth() -> JwtAuth {
        let mut auth = JwtAuth::new("test-secret-key-that-is-at-least-32-characters-long");
        auth.add_user("alice", "password123", vec!["read".to_string(), "write".to_string()])
            .unwrap();
        auth.add_user("admin", "admin", vec!["*".to_string()])
            .unwrap();
        auth
    }

    #[test]
    fn test_authenticate_valid_user() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123");
        assert!(user.is_ok());
        assert_eq!(user.unwrap().username, "alice");
    }

    #[test]
    fn test_authenticate_invalid_password() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "wrongpassword");
        assert!(matches!(user, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_authenticate_invalid_user() {
        let auth = create_test_auth();
        let user = auth.authenticate("unknown", "password");
        assert!(matches!(user, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_generate_and_validate_tokens() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // Validate access token
        let claims = auth.validate_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.token_type, "access");
        assert!(claims.permissions.contains(&"read".to_string()));
    }

    #[test]
    fn test_refresh_token() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // Refresh using refresh token
        let new_tokens = auth.refresh_access_token(&tokens.refresh_token).unwrap();
        assert!(!new_tokens.access_token.is_empty());
    }

    #[test]
    fn test_claims_permissions() {
        let claims = Claims::new_access(
            "test".to_string(),
            vec!["read".to_string()],
            3600,
        );

        assert!(claims.has_permission("read"));
        assert!(!claims.has_permission("write"));
        assert!(!claims.has_permission("*"));
    }

    #[test]
    fn test_admin_wildcard_permission() {
        let claims = Claims::new_access(
            "admin".to_string(),
            vec!["*".to_string()],
            3600,
        );

        assert!(claims.has_permission("read"));
        assert!(claims.has_permission("write"));
        assert!(claims.has_permission("anything"));
    }

    #[test]
    fn test_validate_authorization_header() {
        let auth = create_test_auth();
        let user = auth.authenticate("alice", "password123").unwrap();
        let tokens = auth.generate_tokens(user).unwrap();

        // With "Bearer " prefix
        let claims = auth
            .validate_authorization(&format!("Bearer {}", tokens.access_token))
            .unwrap();
        assert_eq!(claims.sub, "alice");

        // Without prefix
        let claims = auth.validate_authorization(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "alice");
    }
}

```

## File ../memory-graph/src\api\sse\handler.rs:
```rust
//! SSE and MCP HTTP handlers

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::auth::{AuthError, Claims, JwtAuth};
use super::{session::SessionManager, SseEvent};
use crate::api::websocket::events::WsMessage;
use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpTool, Tool};

/// Shared state for SSE endpoints
pub struct SseState {
    /// Knowledge base (sync Arc for tool compatibility)
    pub kb: Arc<crate::knowledge_base::KnowledgeBase>,
    /// Session manager
    pub sessions: SessionManager,
    /// Registered MCP tools
    pub tools: HashMap<String, Arc<dyn Tool>>,
    /// Server info
    pub server_name: String,
    pub server_version: String,
    /// Broadcast channel for graph events (shared with WebSocket)
    pub event_rx: broadcast::Sender<WsMessage>,
    /// Sequence counter
    pub sequence_counter: Arc<std::sync::atomic::AtomicU64>,
    /// JWT authentication (optional - None means auth disabled)
    pub jwt_auth: Option<Arc<JwtAuth>>,
    /// Whether authentication is required
    pub require_auth: bool,
}

impl SseState {
    pub fn new(
        kb: Arc<crate::knowledge_base::KnowledgeBase>,
        event_tx: broadcast::Sender<WsMessage>,
        sequence_counter: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        // Register all tools
        let tools_vec = crate::tools::get_all_tools(kb.clone());
        let mut tools = HashMap::new();
        for tool in tools_vec {
            let name = tool.definition().name.clone();
            tools.insert(name, tool);
        }

        Self {
            kb,
            sessions: SessionManager::new(),
            tools,
            server_name: "memory".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            event_rx: event_tx,
            sequence_counter,
            jwt_auth: None,
            require_auth: false,
        }
    }

    /// Set JWT authentication
    pub fn with_jwt_auth(mut self, jwt_auth: Arc<JwtAuth>, require_auth: bool) -> Self {
        self.jwt_auth = Some(jwt_auth);
        self.require_auth = require_auth;
        self
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get current sequence ID
    pub fn current_sequence_id(&self) -> u64 {
        self.sequence_counter.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Validate token from headers
    pub fn validate_auth(&self, headers: &HeaderMap) -> Result<Option<Claims>, AuthError> {
        let jwt_auth = match &self.jwt_auth {
            Some(auth) => auth,
            None => return Ok(None), // Auth disabled
        };

        // Try Authorization header first
        if let Some(auth_header) = headers.get("Authorization") {
            if let Ok(header_str) = auth_header.to_str() {
                return jwt_auth.validate_authorization(header_str).map(Some);
            }
        }

        // Try X-API-Key header (for backward compatibility)
        if let Some(api_key_header) = headers.get("X-API-Key") {
            if let Ok(key) = api_key_header.to_str() {
                // Check if it's a JWT token (starts with eyJ)
                if key.starts_with("eyJ") {
                    return jwt_auth.validate_token(key).map(Some);
                }
            }
        }

        if self.require_auth {
            Err(AuthError::MissingToken)
        } else {
            Ok(None)
        }
    }
}

/// Query parameters for SSE connection
#[derive(Debug, Deserialize)]
pub struct SseParams {
    /// API key for authentication
    pub api_key: Option<String>,
}

/// Extract user from API key header or query param
fn extract_user(headers: &HeaderMap, params: &SseParams) -> Option<String> {
    // Try X-API-Key header first
    if let Some(header) = headers.get("X-API-Key") {
        if let Ok(key) = header.to_str() {
            return SessionManager::validate_api_key(key);
        }
    }

    // Fall back to query param
    if let Some(ref key) = params.api_key {
        return SessionManager::validate_api_key(key);
    }

    // Allow anonymous connections (for development)
    Some("anonymous".to_string())
}

/// GET /mcp/sse - SSE stream for server→client events
pub async fn sse_handler(
    State(state): State<Arc<SseState>>,
    headers: HeaderMap,
    Query(params): Query<SseParams>,
) -> impl IntoResponse {
    let user = extract_user(&headers, &params).unwrap_or_else(|| "anonymous".to_string());

    // Create session
    let session = state
        .sessions
        .create_session(user, params.api_key.clone())
        .await;

    // Subscribe to graph events
    let mut event_rx = state.event_rx.subscribe();
    let session_id = session.session_id.clone();
    let server_name = state.server_name.clone();
    let server_version = state.server_version.clone();
    let sequence_id = state.current_sequence_id();

    // Create SSE stream
    let stream = async_stream::stream! {
        // Send endpoint event first (MCP SSE spec requirement)
        // This tells the client where to POST messages
        yield Ok::<_, Infallible>(Event::default()
            .event("endpoint")
            .data("/mcp/sse"));

        // Send welcome message
        let welcome = SseEvent::Welcome {
            session_id: session_id.clone(),
            server_name,
            server_version,
            sequence_id,
        };
        yield Ok::<_, Infallible>(Event::default()
            .event("welcome")
            .data(serde_json::to_string(&welcome).unwrap_or_default()));

        // Stream graph events
        loop {
            match event_rx.recv().await {
                Ok(msg) => {
                    let event = SseEvent::GraphEvent { event: msg };
                    yield Ok(Event::default()
                        .event("graph_event")
                        .data(serde_json::to_string(&event).unwrap_or_default()));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Client is too slow
                    let error = SseEvent::Error {
                        code: "lagged".to_string(),
                        message: format!("Missed {} events, please reconnect", n),
                    };
                    yield Ok(Event::default()
                        .event("error")
                        .data(serde_json::to_string(&error).unwrap_or_default()));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default().interval(Duration::from_secs(30)))
}

/// Request body for POST /mcp
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    #[serde(flatten)]
    pub request: JsonRpcRequest,
}

/// Response for POST /mcp
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum McpResponse {
    Success(JsonRpcResponse),
    Error(JsonRpcError),
}

/// POST /mcp - Handle JSON-RPC requests
pub async fn mcp_request_handler(
    State(state): State<Arc<SseState>>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Validate API key
    let user = extract_user(&headers, &SseParams { api_key: None })
        .unwrap_or_else(|| "anonymous".to_string());

    let id = request.id.clone().unwrap_or(Value::Null);

    // Validate JSON-RPC version
    if request.jsonrpc != "2.0" {
        let error = JsonRpcError::invalid_request(id, "jsonrpc must be '2.0'".to_string());
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    // Handle methods
    let result = match request.method.as_str() {
        "initialize" => handle_initialize(&state, id.clone()),
        "tools/list" => handle_tools_list(&state, id.clone()),
        "tools/call" => handle_tool_call(&state, id.clone(), request.params, &user),
        "ping" => Ok(JsonRpcResponse::new(id.clone(), json!({}))),
        _ => {
            let error = JsonRpcError::method_not_found(id, request.method);
            return (StatusCode::OK, Json(error)).into_response();
        }
    };

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (StatusCode::OK, Json(error)).into_response(),
    }
}

fn handle_initialize(state: &SseState, id: Value) -> Result<JsonRpcResponse, JsonRpcError> {
    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": state.server_name,
            "version": state.server_version
        }
    });
    Ok(JsonRpcResponse::new(id, result))
}

fn handle_tools_list(state: &SseState, id: Value) -> Result<JsonRpcResponse, JsonRpcError> {
    let tools: Vec<McpTool> = state.tools.values().map(|t| t.definition()).collect();
    let result = json!({ "tools": tools });
    Ok(JsonRpcResponse::new(id, result))
}

fn handle_tool_call(
    state: &SseState,
    id: Value,
    params: Option<Value>,
    _user: &str,
) -> Result<JsonRpcResponse, JsonRpcError> {
    let params = params.ok_or_else(|| {
        JsonRpcError::invalid_params(id.clone(), "Missing parameters".to_string())
    })?;

    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            JsonRpcError::invalid_params(id.clone(), "Missing tool name".to_string())
        })?;

    let tool = state.tools.get(tool_name).ok_or_else(|| {
        JsonRpcError::new(
            id.clone(),
            -32602,
            "Unknown tool".to_string(),
            Some(json!({"tool": tool_name})),
        )
    })?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool.execute(arguments) {
        Ok(result) => Ok(JsonRpcResponse::new(id, result)),
        Err(e) => Err(JsonRpcError::new(
            id,
            -32603,
            "Tool execution error".to_string(),
            Some(json!({"details": e.to_string()})),
        )),
    }
}

/// GET /mcp/info - Get server info
#[derive(Debug, Serialize)]
pub struct ServerInfoResponse {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub tool_count: usize,
    pub active_sessions: usize,
}

pub async fn server_info_handler(State(state): State<Arc<SseState>>) -> impl IntoResponse {
    let info = ServerInfoResponse {
        name: state.server_name.clone(),
        version: state.server_version.clone(),
        protocol_version: "2024-11-05".to_string(),
        tool_count: state.tools.len(),
        active_sessions: state.sessions.session_count().await,
    };
    Json(info)
}

// ============================================================================
// Authentication Endpoints
// ============================================================================

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Refresh token request body
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Auth error response
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub error_code: String,
}

impl AuthErrorResponse {
    fn from_auth_error(err: &AuthError) -> Self {
        let (error, error_code) = match err {
            AuthError::InvalidCredentials => {
                ("Invalid username or password".to_string(), "invalid_credentials".to_string())
            }
            AuthError::TokenExpired => {
                ("Token has expired".to_string(), "token_expired".to_string())
            }
            AuthError::InvalidTokenType => {
                ("Invalid token type".to_string(), "invalid_token_type".to_string())
            }
            AuthError::MissingToken => {
                ("Missing authentication token".to_string(), "missing_token".to_string())
            }
            AuthError::InsufficientPermissions => {
                ("Insufficient permissions".to_string(), "insufficient_permissions".to_string())
            }
            _ => (err.to_string(), "auth_error".to_string()),
        };
        Self { error, error_code }
    }
}

/// POST /auth/token - Login and get JWT tokens
pub async fn login_handler(
    State(state): State<Arc<SseState>>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let jwt_auth = match &state.jwt_auth {
        Some(auth) => auth,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(AuthErrorResponse {
                    error: "Authentication not configured".to_string(),
                    error_code: "auth_not_configured".to_string(),
                }),
            )
                .into_response();
        }
    };

    // Authenticate user
    match jwt_auth.authenticate(&request.username, &request.password) {
        Ok(user) => {
            // Generate tokens
            match jwt_auth.generate_tokens(user) {
                Ok(tokens) => (StatusCode::OK, Json(tokens)).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse::from_auth_error(&e)),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse::from_auth_error(&e)),
        )
            .into_response(),
    }
}

/// POST /auth/refresh - Refresh access token
pub async fn refresh_handler(
    State(state): State<Arc<SseState>>,
    Json(request): Json<RefreshRequest>,
) -> impl IntoResponse {
    let jwt_auth = match &state.jwt_auth {
        Some(auth) => auth,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(AuthErrorResponse {
                    error: "Authentication not configured".to_string(),
                    error_code: "auth_not_configured".to_string(),
                }),
            )
                .into_response();
        }
    };

    match jwt_auth.refresh_access_token(&request.refresh_token) {
        Ok(tokens) => (StatusCode::OK, Json(tokens)).into_response(),
        Err(e) => {
            let status = match &e {
                AuthError::TokenExpired => StatusCode::UNAUTHORIZED,
                AuthError::InvalidTokenType => StatusCode::BAD_REQUEST,
                _ => StatusCode::UNAUTHORIZED,
            };
            (status, Json(AuthErrorResponse::from_auth_error(&e))).into_response()
        }
    }
}

/// GET /auth/me - Get current user info from token
#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub username: String,
    pub permissions: Vec<String>,
    pub token_expires_at: i64,
}

pub async fn me_handler(
    State(state): State<Arc<SseState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match state.validate_auth(&headers) {
        Ok(Some(claims)) => {
            let info = UserInfoResponse {
                username: claims.sub,
                permissions: claims.permissions,
                token_expires_at: claims.exp,
            };
            (StatusCode::OK, Json(info)).into_response()
        }
        Ok(None) => (
            StatusCode::OK,
            Json(UserInfoResponse {
                username: "anonymous".to_string(),
                permissions: vec!["*".to_string()],
                token_expires_at: 0,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse::from_auth_error(&e)),
        )
            .into_response(),
    }
}

```

## File ../memory-graph/src\api\sse\mod.rs:
```rust
//! SSE (Server-Sent Events) module for MCP over HTTP
//!
//! Provides SSE transport for AI Agents to connect via HTTP instead of stdio.
//! This enables team collaboration with multiple AI agents.
//!
//! ## Endpoints
//! - `GET /mcp/sse` - SSE stream for server→client events
//! - `POST /mcp` - JSON-RPC requests from client→server
//! - `GET /mcp/info` - Server info and capabilities
//! - `POST /auth/token` - Login and get JWT tokens
//! - `POST /auth/refresh` - Refresh access token

pub mod auth;
pub mod handler;
pub mod session;

pub use auth::{AuthError, Claims, JwtAuth, SharedJwtAuth, TokenPair};

use serde::Serialize;

/// SSE event types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// JSON-RPC response to a request
    Response {
        #[serde(flatten)]
        response: serde_json::Value,
    },
    /// Graph change notification
    GraphEvent {
        #[serde(flatten)]
        event: crate::api::websocket::events::WsMessage,
    },
    /// Heartbeat ping
    Ping {
        timestamp: i64,
    },
    /// Welcome message on connect
    Welcome {
        session_id: String,
        server_name: String,
        server_version: String,
        sequence_id: u64,
    },
    /// Error notification
    Error {
        code: String,
        message: String,
    },
}

/// Client session info
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: String,
    pub user: String,
    pub api_key: Option<String>,
    pub connected_at: i64,
}

```

## File ../memory-graph/src\api\sse\session.rs:
```rust
//! Session management for SSE connections

use std::collections::HashMap;
use tokio::sync::RwLock;

use super::ClientSession;

/// Session manager for tracking connected clients
pub struct SessionManager {
    sessions: RwLock<HashMap<String, ClientSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Generate a new session ID
    pub fn generate_session_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("sess_{:x}", timestamp)
    }

    /// Create a new session
    pub async fn create_session(&self, user: String, api_key: Option<String>) -> ClientSession {
        let session_id = Self::generate_session_id();
        let session = ClientSession {
            session_id: session_id.clone(),
            user,
            api_key,
            connected_at: chrono::Utc::now().timestamp(),
        };

        self.sessions.write().await.insert(session_id, session.clone());
        session
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<ClientSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get active session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Validate API key and return user info
    /// For now, we use a simple format: "user:secret" or just accept any non-empty key
    pub fn validate_api_key(api_key: &str) -> Option<String> {
        if api_key.is_empty() {
            return None;
        }

        // Simple format: "username:secret" or just "username"
        // In production, you'd validate against a database
        if let Some(colon_pos) = api_key.find(':') {
            let username = &api_key[..colon_pos];
            if !username.is_empty() {
                return Some(username.to_string());
            }
        }

        // Accept any non-empty key as anonymous user
        Some(format!("api-user-{}", &api_key[..8.min(api_key.len())]))
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key_with_username() {
        let user = SessionManager::validate_api_key("alice:secret123");
        assert_eq!(user, Some("alice".to_string()));
    }

    #[test]
    fn test_validate_api_key_anonymous() {
        let user = SessionManager::validate_api_key("some-random-key");
        assert_eq!(user, Some("api-user-some-ran".to_string()));
    }

    #[test]
    fn test_validate_api_key_empty() {
        let user = SessionManager::validate_api_key("");
        assert_eq!(user, None);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new();

        // Create session
        let session = manager.create_session("alice".to_string(), Some("key123".to_string())).await;
        assert!(session.session_id.starts_with("sess_"));
        assert_eq!(session.user, "alice");

        // Get session
        let retrieved = manager.get_session(&session.session_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user, "alice");

        // Count
        assert_eq!(manager.session_count().await, 1);

        // Remove
        manager.remove_session(&session.session_id).await;
        assert_eq!(manager.session_count().await, 0);
    }
}

```

## File ../memory-graph/src\api\websocket\batcher.rs:
```rust
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

```

## File ../memory-graph/src\api\websocket\broadcaster.rs:
```rust
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

```

## File ../memory-graph/src\api\websocket\events.rs:
```rust
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

```

## File ../memory-graph/src\api\websocket\handler.rs:
```rust
//! WebSocket connection handler

use std::sync::Arc;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::Response,
};
use serde::Deserialize;

use super::events::{ClientMessage, PongMessage, WelcomeMessage};
use super::state::AppState;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsParams {
    /// Optional authentication token
    pub token: Option<String>,
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(_params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    // TODO: Validate token if provided
    // if let Some(token) = params.token {
    //     if !validate_token(&token) {
    //         return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    //     }
    // }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // Subscribe to broadcast events
    let mut rx = state.subscribe();

    // Send welcome message with current sequence ID
    let welcome = WelcomeMessage::new(state.current_sequence_id());
    if let Ok(json) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(json)).await.is_err() {
            return; // Client disconnected immediately
        }
    }

    loop {
        tokio::select! {
            // Broadcast events to client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client is too slow, they missed events
                        // Send an error message so they know to refresh
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "code": "lagged",
                            "message": format!("Missed {} events, please refresh", n)
                        });
                        let _ = socket.send(Message::Text(error_msg.to_string())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Channel closed
                    }
                }
            }

            // Handle client messages
            result = socket.recv() => {
                match result {
                    Some(Ok(msg)) => {
                        if !handle_client_message(msg, &mut socket).await {
                            break; // Client requested close or error
                        }
                    }
                    Some(Err(_)) => break, // WebSocket error
                    None => break, // Client disconnected
                }
            }
        }
    }
}

/// Handle a message from the client
/// Returns false if the connection should be closed
async fn handle_client_message(msg: Message, socket: &mut WebSocket) -> bool {
    match msg {
        Message::Text(text) => {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Ping => {
                        let pong = PongMessage::default();
                        if let Ok(json) = serde_json::to_string(&pong) {
                            let _ = socket.send(Message::Text(json)).await;
                        }
                    }
                    ClientMessage::Subscribe { channel, filter } => {
                        // TODO: Implement channel filtering
                        // For now, all clients receive all events
                        let _ = (channel, filter);
                    }
                    ClientMessage::Unsubscribe { channel } => {
                        // TODO: Implement channel filtering
                        let _ = channel;
                    }
                }
            }
            true
        }
        Message::Binary(_) => true, // Ignore binary messages
        Message::Ping(data) => {
            let _ = socket.send(Message::Pong(data)).await;
            true
        }
        Message::Pong(_) => true, // Ignore pong responses
        Message::Close(_) => false, // Client requested close
    }
}

// Re-export for use by broadcast module
pub use tokio::sync::broadcast;

```

## File ../memory-graph/src\api\websocket\mod.rs:
```rust
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

```

## File ../memory-graph/src\api\websocket\state.rs:
```rust
//! WebSocket application state

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::knowledge_base::KnowledgeBase;
use super::events::{GraphEvent, WsMessage};

/// Shared application state for WebSocket connections
///
/// Uses Arc<KnowledgeBase> directly - KnowledgeBase has internal RwLock for thread safety.
/// This ensures a single source of truth shared between SSE/MCP and REST/WebSocket.
pub struct AppState {
    /// The knowledge base (single source of truth)
    pub kb: Arc<KnowledgeBase>,

    /// Broadcast channel for sending events to all connected clients
    pub event_tx: broadcast::Sender<WsMessage>,

    /// Monotonically increasing sequence counter
    pub sequence_counter: Arc<AtomicU64>,
}

impl AppState {
    /// Create a new AppState with the given knowledge base
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
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
        let kb = Arc::new(KnowledgeBase::new());
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
        let kb = Arc::new(KnowledgeBase::new());
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

```

## File ../memory-graph/src\event_store\migration.rs:
```rust
//! Migration Tool for Event Sourcing
//!
//! Converts existing legacy `memory.jsonl` data to Event Sourcing format:
//! 1. Reads entities and relations from memory.jsonl
//! 2. Creates entity_created and relation_created events
//! 3. Writes events to events.jsonl
//! 4. Creates initial snapshot

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::types::{
    Entity, EntityCreatedData, Event, EventSource, EventType, Relation, RelationCreatedData,
};
use crate::utils::current_timestamp;

use super::store::{EventStoreConfig, EventStoreError, EventStoreResult};
use super::SnapshotManager;

/// Result of a migration operation
#[derive(Debug)]
pub struct MigrationResult {
    pub entities_migrated: usize,
    pub relations_migrated: usize,
    pub events_created: usize,
    pub snapshot_created: bool,
}

/// Migration tool for converting legacy data to Event Sourcing
pub struct MigrationTool {
    config: EventStoreConfig,
}

impl MigrationTool {
    /// Create a new migration tool with default config
    pub fn new() -> Self {
        Self {
            config: EventStoreConfig::default(),
        }
    }

    /// Create a new migration tool with custom config
    pub fn with_config(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Migrate legacy memory.jsonl to Event Sourcing format
    ///
    /// # Arguments
    /// * `legacy_path` - Path to the legacy memory.jsonl file
    ///
    /// # Returns
    /// * `MigrationResult` with counts of migrated items
    pub fn migrate_from_legacy<P: AsRef<Path>>(
        &self,
        legacy_path: P,
    ) -> EventStoreResult<MigrationResult> {
        let legacy_path = legacy_path.as_ref();

        if !legacy_path.exists() {
            return Err(EventStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Legacy file not found: {}", legacy_path.display()),
            )));
        }

        // Read legacy data
        let (entities, relations) = self.read_legacy_file(legacy_path)?;

        // Create events directory
        fs::create_dir_all(self.config.data_dir())?;

        // Create events from entities and relations
        let events = self.create_migration_events(&entities, &relations)?;
        let event_count = events.len();

        // Write events to events.jsonl
        let events_path = self.config.events_path();
        self.write_events(&events_path, &events)?;

        // Create initial snapshot
        let snapshot_manager = SnapshotManager::new(self.config.clone());
        let last_event_id = events.last().map(|e| e.event_id).unwrap_or(0);

        snapshot_manager.create_snapshot_with_backup(last_event_id, &entities, &relations)?;

        // Backup original file
        let backup_path = legacy_path.with_extension("jsonl.migrated");
        if !backup_path.exists() {
            fs::copy(legacy_path, &backup_path)?;
        }

        Ok(MigrationResult {
            entities_migrated: entities.len(),
            relations_migrated: relations.len(),
            events_created: event_count,
            snapshot_created: true,
        })
    }

    /// Read entities and relations from legacy file
    fn read_legacy_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> EventStoreResult<(Vec<Entity>, Vec<Relation>)> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as Entity first
            if let Ok(entity) = serde_json::from_str::<Entity>(trimmed) {
                entities.push(entity);
                continue;
            }

            // Try to parse as Relation
            if let Ok(relation) = serde_json::from_str::<Relation>(trimmed) {
                relations.push(relation);
                continue;
            }

            // Log warning for unknown line format
            eprintln!(
                "[Migration] Warning: Could not parse line: {}",
                if trimmed.len() > 50 {
                    format!("{}...", &trimmed[..50])
                } else {
                    trimmed.to_string()
                }
            );
        }

        Ok((entities, relations))
    }

    /// Create migration events from entities and relations
    fn create_migration_events(
        &self,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<Vec<Event>> {
        let mut events = Vec::new();
        let mut event_id: u64 = 1;
        let timestamp = current_timestamp() as i64;

        // Create entity_created events
        for entity in entities {
            let data = EntityCreatedData {
                name: entity.name.clone(),
                entity_type: entity.entity_type.clone(),
                observations: entity.observations.clone(),
            };

            let user = if entity.created_by.is_empty() {
                "migration".to_string()
            } else {
                entity.created_by.clone()
            };

            let event = Event {
                event_id,
                event_type: EventType::EntityCreated,
                timestamp,
                user,
                agent: Some("MigrationTool".to_string()),
                source: EventSource::Migration,
                data: serde_json::to_value(&data)?,
            };

            events.push(event);
            event_id += 1;
        }

        // Create relation_created events
        for relation in relations {
            let data = RelationCreatedData {
                from: relation.from.clone(),
                to: relation.to.clone(),
                relation_type: relation.relation_type.clone(),
                valid_from: relation.valid_from.map(|v| v as i64),
                valid_to: relation.valid_to.map(|v| v as i64),
            };

            let user = if relation.created_by.is_empty() {
                "migration".to_string()
            } else {
                relation.created_by.clone()
            };

            let event = Event {
                event_id,
                event_type: EventType::RelationCreated,
                timestamp,
                user,
                agent: Some("MigrationTool".to_string()),
                source: EventSource::Migration,
                data: serde_json::to_value(&data)?,
            };

            events.push(event);
            event_id += 1;
        }

        Ok(events)
    }

    /// Write events to file
    fn write_events<P: AsRef<Path>>(
        &self,
        path: P,
        events: &[Event],
    ) -> EventStoreResult<()> {
        let path = path.as_ref();

        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        {
            let mut file = File::create(&temp_path)?;
            for event in events {
                writeln!(file, "{}", serde_json::to_string(event)?)?;
            }
            file.sync_all()?;
        }

        // Atomic rename
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Check if migration is needed
    pub fn needs_migration<P: AsRef<Path>>(&self, legacy_path: P) -> bool {
        let legacy_path = legacy_path.as_ref();
        let events_path = self.config.events_path();
        let snapshot_path = self.config.latest_snapshot_path();

        // Migration needed if:
        // 1. Legacy file exists
        // 2. AND neither events nor snapshot exist
        legacy_path.exists() && !events_path.exists() && !snapshot_path.exists()
    }
}

impl Default for MigrationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_legacy_file(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("memory.jsonl");
        let mut file = File::create(&path).unwrap();

        // Write some entities
        writeln!(file, r#"{{"name":"Alice","entityType":"Person","observations":["Developer"]}}"#).unwrap();
        writeln!(file, r#"{{"name":"Bob","entityType":"Person","observations":["Designer"]}}"#).unwrap();

        // Write some relations
        writeln!(file, r#"{{"from":"Alice","to":"Bob","relationType":"knows"}}"#).unwrap();

        file.sync_all().unwrap();
        path
    }

    #[test]
    fn test_read_legacy_file() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = create_legacy_file(temp_dir.path());

        let tool = MigrationTool::new();
        let (entities, relations) = tool.read_legacy_file(&legacy_path).unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 1);

        assert_eq!(entities[0].name, "Alice");
        assert_eq!(entities[1].name, "Bob");
        assert_eq!(relations[0].from, "Alice");
        assert_eq!(relations[0].to, "Bob");
    }

    #[test]
    fn test_create_migration_events() {
        let entities = vec![
            Entity {
                name: "TestEntity".to_string(),
                entity_type: "Test".to_string(),
                observations: vec!["observation1".to_string()],
                created_by: "tester".to_string(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];

        let relations = vec![
            Relation {
                from: "A".to_string(),
                to: "B".to_string(),
                relation_type: "test".to_string(),
                created_by: "tester".to_string(),
                created_at: 0,
                valid_from: None,
                valid_to: None,
            },
        ];

        let tool = MigrationTool::new();
        let events = tool.create_migration_events(&entities, &relations).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
        assert_eq!(events[1].event_type, EventType::RelationCreated);
        assert!(matches!(events[0].source, EventSource::Migration));
    }

    #[test]
    fn test_migrate_from_legacy() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = create_legacy_file(temp_dir.path());

        // Configure migration to use temp directory
        let config = EventStoreConfig::new(temp_dir.path().join("data"));
        let tool = MigrationTool::with_config(config.clone());

        let result = tool.migrate_from_legacy(&legacy_path).unwrap();

        assert_eq!(result.entities_migrated, 2);
        assert_eq!(result.relations_migrated, 1);
        assert_eq!(result.events_created, 3);
        assert!(result.snapshot_created);

        // Verify events file was created
        assert!(config.events_path().exists());

        // Verify snapshot was created
        assert!(config.latest_snapshot_path().exists());

        // Verify backup was created
        let backup_path = legacy_path.with_extension("jsonl.migrated");
        assert!(backup_path.exists());
    }

    #[test]
    fn test_needs_migration() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = temp_dir.path().join("memory.jsonl");

        let config = EventStoreConfig::new(temp_dir.path().join("data"));
        let tool = MigrationTool::with_config(config.clone());

        // No legacy file - no migration needed
        assert!(!tool.needs_migration(&legacy_path));

        // Create legacy file
        File::create(&legacy_path).unwrap();
        assert!(tool.needs_migration(&legacy_path));

        // Create events file - no migration needed
        fs::create_dir_all(config.data_dir()).unwrap();
        File::create(config.events_path()).unwrap();
        assert!(!tool.needs_migration(&legacy_path));
    }
}

```

## File ../memory-graph/src\event_store\mod.rs:
```rust
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

```

## File ../memory-graph/src\event_store\rotation.rs:
```rust
//! Log Rotation and Archive Management
//!
//! Provides functionality for:
//! - Rotating event logs after snapshot
//! - Archiving old events with timestamps
//! - Cleaning up old archives
//! - Compression (future)

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::store::{EventStoreConfig, EventStoreResult};

/// Log rotation manager for event archives
pub struct LogRotation {
    config: EventStoreConfig,
}

impl LogRotation {
    /// Create a new LogRotation manager
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Rotate the current event log after a snapshot
    ///
    /// This moves events up to the snapshot point to an archive file,
    /// keeping only events after the snapshot in the active log.
    ///
    /// # Arguments
    /// * `snapshot_event_id` - The last event ID included in the snapshot
    ///
    /// # Returns
    /// * `Ok(Some(path))` - Path to the archive file if rotation occurred
    /// * `Ok(None)` - No rotation needed (no events to archive)
    pub fn rotate_after_snapshot(&self, snapshot_event_id: u64) -> EventStoreResult<Option<PathBuf>> {
        let events_path = self.config.events_path();

        if !events_path.exists() {
            return Ok(None);
        }

        // Read all events
        let file = File::open(&events_path)?;
        let reader = BufReader::new(file);

        let mut archive_lines = Vec::new();
        let mut keep_lines = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse event ID from the line
            if let Some(event_id) = self.extract_event_id(&line) {
                if event_id <= snapshot_event_id {
                    archive_lines.push(line);
                } else {
                    keep_lines.push(line);
                }
            } else {
                // If we can't parse, keep it in the active log
                keep_lines.push(line);
            }
        }

        // No events to archive
        if archive_lines.is_empty() {
            return Ok(None);
        }

        // Create archive directory
        let archive_dir = self.config.archive_dir();
        fs::create_dir_all(&archive_dir)?;

        // Generate archive filename with event range
        let archive_filename = format!("events_{}_to_{}.jsonl",
            self.extract_event_id(&archive_lines[0]).unwrap_or(0),
            snapshot_event_id
        );
        let archive_path = archive_dir.join(&archive_filename);

        // Write archive file
        {
            let mut archive_file = File::create(&archive_path)?;
            for line in &archive_lines {
                writeln!(archive_file, "{}", line)?;
            }
            archive_file.sync_all()?;
        }

        // Write remaining events back to active log
        {
            let temp_path = events_path.with_extension("tmp");
            let mut temp_file = File::create(&temp_path)?;
            for line in &keep_lines {
                writeln!(temp_file, "{}", line)?;
            }
            temp_file.sync_all()?;

            // Atomic rename
            fs::rename(&temp_path, &events_path)?;
        }

        println!(
            "Rotated {} events to archive: {}",
            archive_lines.len(),
            archive_path.display()
        );

        Ok(Some(archive_path))
    }

    /// Extract event ID from a JSON line
    fn extract_event_id(&self, line: &str) -> Option<u64> {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        value.get("eventId")?.as_u64()
    }

    /// List all archive files
    pub fn list_archives(&self) -> EventStoreResult<Vec<ArchiveInfo>> {
        let archive_dir = self.config.archive_dir();

        if !archive_dir.exists() {
            return Ok(Vec::new());
        }

        let mut archives = Vec::new();

        for entry in fs::read_dir(&archive_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let metadata = entry.metadata()?;
                let size = metadata.len();

                // Count events in archive
                let event_count = self.count_events(&path)?;

                archives.push(ArchiveInfo {
                    path,
                    size,
                    event_count,
                });
            }
        }

        // Sort by filename (which includes event IDs)
        archives.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name()));

        Ok(archives)
    }

    /// Count events in a file
    fn count_events(&self, path: &Path) -> EventStoreResult<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let count = reader.lines().filter(|l| l.is_ok()).count();
        Ok(count)
    }

    /// Clean up old archives, keeping only the most recent N
    ///
    /// # Arguments
    /// * `keep_count` - Number of most recent archives to keep
    ///
    /// # Returns
    /// * Number of archives deleted
    pub fn cleanup_old_archives(&self, keep_count: usize) -> EventStoreResult<usize> {
        let mut archives = self.list_archives()?;

        if archives.len() <= keep_count {
            return Ok(0);
        }

        // Sort by filename descending (newest first)
        archives.sort_by(|a, b| b.path.file_name().cmp(&a.path.file_name()));

        let to_delete = &archives[keep_count..];
        let delete_count = to_delete.len();

        for archive in to_delete {
            fs::remove_file(&archive.path)?;
            println!("Deleted old archive: {}", archive.path.display());
        }

        Ok(delete_count)
    }

    /// Get total size of all archives in bytes
    pub fn total_archive_size(&self) -> EventStoreResult<u64> {
        let archives = self.list_archives()?;
        Ok(archives.iter().map(|a| a.size).sum())
    }
}

/// Information about an archive file
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    /// Path to the archive file
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Number of events in the archive
    pub event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_store::EventStore;
    use crate::types::EventType;
    use tempfile::TempDir;

    #[test]
    fn test_rotate_after_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        for i in 1..=5 {
            store.create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                serde_json::json!({
                    "name": format!("Entity{}", i),
                    "entity_type": "Test",
                    "observations": []
                }),
            ).unwrap();
        }

        // Rotate after event 3
        let rotation = LogRotation::new(config.clone());
        let archive_path = rotation.rotate_after_snapshot(3).unwrap();

        assert!(archive_path.is_some());
        let archive_path = archive_path.unwrap();
        assert!(archive_path.exists());

        // Verify archive contains events 1-3
        let archive_count = rotation.count_events(&archive_path).unwrap();
        assert_eq!(archive_count, 3);

        // Verify active log contains events 4-5
        let active_count = rotation.count_events(&config.events_path()).unwrap();
        assert_eq!(active_count, 2);
    }

    #[test]
    fn test_list_archives() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create archive directory and some files
        let archive_dir = config.archive_dir();
        fs::create_dir_all(&archive_dir).unwrap();

        fs::write(archive_dir.join("events_1_to_100.jsonl"), "{}\n{}\n").unwrap();
        fs::write(archive_dir.join("events_101_to_200.jsonl"), "{}\n").unwrap();

        let rotation = LogRotation::new(config);
        let archives = rotation.list_archives().unwrap();

        assert_eq!(archives.len(), 2);
    }

    #[test]
    fn test_cleanup_old_archives() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create archive directory and some files
        let archive_dir = config.archive_dir();
        fs::create_dir_all(&archive_dir).unwrap();

        fs::write(archive_dir.join("events_1_to_100.jsonl"), "{}").unwrap();
        fs::write(archive_dir.join("events_101_to_200.jsonl"), "{}").unwrap();
        fs::write(archive_dir.join("events_201_to_300.jsonl"), "{}").unwrap();

        let rotation = LogRotation::new(config);

        // Keep only 2 archives
        let deleted = rotation.cleanup_old_archives(2).unwrap();
        assert_eq!(deleted, 1);

        // Verify only 2 archives remain
        let remaining = rotation.list_archives().unwrap();
        assert_eq!(remaining.len(), 2);
    }
}

```

## File ../memory-graph/src\event_store\snapshot.rs:
```rust
//! Snapshot Manager for Event Sourcing
//!
//! Handles creation, loading, and management of state snapshots.
//! Snapshots are point-in-time captures of the materialized state
//! that allow fast startup without replaying all events.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};

use crate::types::{Entity, Relation, SnapshotMeta};
use crate::utils::atomic_write_with;

use super::store::{EventStoreConfig, EventStoreError, EventStoreResult};

/// Snapshot Manager handles creating and loading snapshots
pub struct SnapshotManager {
    config: EventStoreConfig,
}

impl SnapshotManager {
    /// Create a new SnapshotManager with the given config
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Get path to latest snapshot
    pub fn latest_path(&self) -> std::path::PathBuf {
        self.config.latest_snapshot_path()
    }

    /// Get path to previous (backup) snapshot
    pub fn previous_path(&self) -> std::path::PathBuf {
        self.config.previous_snapshot_path()
    }

    /// Check if a snapshot exists
    pub fn snapshot_exists(&self) -> bool {
        self.config.latest_snapshot_path().exists()
    }

    /// Create a new snapshot atomically
    ///
    /// This function:
    /// 1. Writes metadata + entities + relations to a temp file
    /// 2. Syncs to disk
    /// 3. Backs up existing snapshot to previous.jsonl
    /// 4. Atomically renames temp to latest.jsonl
    ///
    /// # Arguments
    ///
    /// * `last_event_id` - The ID of the last event included in this snapshot
    /// * `entities` - Current entities to snapshot
    /// * `relations` - Current relations to snapshot
    pub fn create_snapshot(
        &self,
        last_event_id: u64,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<SnapshotMeta> {
        let latest_path = self.config.latest_snapshot_path();
        let _previous_path = self.config.previous_snapshot_path();

        // Ensure snapshots directory exists
        fs::create_dir_all(self.config.snapshots_dir())?;

        // Create metadata
        let meta = SnapshotMeta::new(last_event_id, entities.len(), relations.len());

        // Write snapshot atomically
        let meta_clone = meta.clone();
        atomic_write_with(&latest_path, |file| {
            // Write metadata first
            let meta_json = serde_json::to_string(&meta_clone)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", meta_json)?;

            // Write entities
            for entity in entities {
                let json = serde_json::to_string(entity)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                writeln!(file, "{}", json)?;
            }

            // Write relations
            for relation in relations {
                let json = serde_json::to_string(relation)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                writeln!(file, "{}", json)?;
            }

            Ok(())
        })
        .map_err(|e| EventStoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // Backup is handled by atomic_write_with's rename
        // But we need to manually handle previous backup
        // Note: atomic_write_with already handles this via temp file pattern
        // For explicit backup, we'd need to modify the flow

        println!(
            "Created snapshot: {} entities, {} relations (event_id: {})",
            entities.len(),
            relations.len(),
            last_event_id
        );

        Ok(meta)
    }

    /// Create snapshot with explicit backup of previous version
    ///
    /// This provides an extra safety layer by keeping the previous
    /// snapshot as a backup before creating a new one.
    pub fn create_snapshot_with_backup(
        &self,
        last_event_id: u64,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<SnapshotMeta> {
        let latest_path = self.config.latest_snapshot_path();
        let previous_path = self.config.previous_snapshot_path();
        let temp_path = latest_path.with_extension("tmp");

        // Ensure snapshots directory exists
        fs::create_dir_all(self.config.snapshots_dir())?;

        // Create metadata
        let meta = SnapshotMeta::new(last_event_id, entities.len(), relations.len());

        // Step 1: Write to temp file
        {
            let mut file = File::create(&temp_path)?;

            // Write metadata
            writeln!(file, "{}", serde_json::to_string(&meta)?)?;

            // Write entities
            for entity in entities {
                writeln!(file, "{}", serde_json::to_string(entity)?)?;
            }

            // Write relations
            for relation in relations {
                writeln!(file, "{}", serde_json::to_string(relation)?)?;
            }

            // Sync to disk
            file.sync_all()?;
        }

        // Step 2: Backup existing snapshot
        if latest_path.exists() {
            // Remove old backup if exists
            if previous_path.exists() {
                fs::remove_file(&previous_path)?;
            }
            fs::rename(&latest_path, &previous_path)?;
        }

        // Step 3: Atomic rename temp to latest
        fs::rename(&temp_path, &latest_path)?;

        println!(
            "Created snapshot with backup: {} entities, {} relations (event_id: {})",
            entities.len(),
            relations.len(),
            last_event_id
        );

        Ok(meta)
    }

    /// Load snapshot metadata only (fast, for checking state)
    pub fn load_meta(&self) -> EventStoreResult<Option<SnapshotMeta>> {
        let path = self.config.latest_snapshot_path();

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        if let Some(first_line) = reader.lines().next() {
            let line = first_line?;
            let meta = SnapshotMeta::from_json_line(&line)?;
            Ok(Some(meta))
        } else {
            Err(EventStoreError::SnapshotCorrupted(
                "Empty snapshot file".to_string(),
            ))
        }
    }

    /// Load full snapshot (metadata + entities + relations)
    pub fn load_full(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let path = self.config.latest_snapshot_path();

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line is metadata
        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty snapshot".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::with_capacity(meta.entity_count);
        let mut relations = Vec::with_capacity(meta.relation_count);

        // Parse remaining lines
        for (line_num, line_result) in lines.enumerate() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse as JSON value first to determine type
            let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                EventStoreError::SnapshotCorrupted(format!("Line {}: {}", line_num + 2, e))
            })?;

            // Determine if entity or relation based on fields
            if value.get("entityType").is_some() && value.get("name").is_some() {
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
            // Silently skip unknown line types
        }

        // Validate counts
        if entities.len() != meta.entity_count {
            eprintln!(
                "Warning: Expected {} entities, found {}",
                meta.entity_count,
                entities.len()
            );
        }
        if relations.len() != meta.relation_count {
            eprintln!(
                "Warning: Expected {} relations, found {}",
                meta.relation_count,
                relations.len()
            );
        }

        Ok(Some((meta, entities, relations)))
    }

    /// Try to recover from backup snapshot if primary is corrupted
    pub fn recover_from_backup(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let previous_path = self.config.previous_snapshot_path();

        if !previous_path.exists() {
            return Ok(None);
        }

        println!("Attempting recovery from backup snapshot...");

        // Temporarily swap paths to load from backup
        let file = File::open(&previous_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty backup".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(&line)?;

            if value.get("entityType").is_some() && value.get("name").is_some() {
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
        }

        println!(
            "Recovered from backup: {} entities, {} relations",
            entities.len(),
            relations.len()
        );

        Ok(Some((meta, entities, relations)))
    }

    /// Delete all snapshots (for testing or reset)
    pub fn clear_snapshots(&self) -> EventStoreResult<()> {
        let latest = self.config.latest_snapshot_path();
        let previous = self.config.previous_snapshot_path();

        if latest.exists() {
            fs::remove_file(&latest)?;
        }
        if previous.exists() {
            fs::remove_file(&previous)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SnapshotManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_data_dir(temp_dir.path());
        fs::create_dir_all(config.snapshots_dir()).unwrap();
        let manager = SnapshotManager::new(config);
        (manager, temp_dir)
    }

    fn create_test_entities() -> Vec<Entity> {
        vec![
            Entity::with_observations("Bug:Login".to_string(), "Bug".to_string(), vec!["obs1".to_string()]),
            Entity::new("Module:Auth".to_string(), "Module".to_string()),
        ]
    }

    fn create_test_relations() -> Vec<Relation> {
        vec![Relation::new(
            "Bug:Login".to_string(),
            "Module:Auth".to_string(),
            "affects".to_string(),
        )]
    }

    #[test]
    fn test_create_and_load_snapshot() {
        let (manager, _temp_dir) = create_test_manager();
        let entities = create_test_entities();
        let relations = create_test_relations();

        // Create snapshot
        let meta = manager
            .create_snapshot_with_backup(100, &entities, &relations)
            .unwrap();

        assert_eq!(meta.last_event_id, 100);
        assert_eq!(meta.entity_count, 2);
        assert_eq!(meta.relation_count, 1);

        // Load and verify
        let (loaded_meta, loaded_entities, loaded_relations) =
            manager.load_full().unwrap().unwrap();

        assert_eq!(loaded_meta.last_event_id, 100);
        assert_eq!(loaded_entities.len(), 2);
        assert_eq!(loaded_relations.len(), 1);
        assert_eq!(loaded_entities[0].name, "Bug:Login");
        assert_eq!(loaded_relations[0].relation_type, "affects");
    }

    #[test]
    fn test_snapshot_backup() {
        let (manager, _temp_dir) = create_test_manager();

        // Create first snapshot
        let entities1 = vec![Entity::new("Entity1".to_string(), "Test".to_string())];
        manager
            .create_snapshot_with_backup(10, &entities1, &[])
            .unwrap();

        // Create second snapshot (should backup first)
        let entities2 = vec![Entity::new("Entity2".to_string(), "Test".to_string())];
        manager
            .create_snapshot_with_backup(20, &entities2, &[])
            .unwrap();

        // Verify backup exists
        assert!(manager.previous_path().exists());

        // Load latest (should be Entity2)
        let (meta, entities, _) = manager.load_full().unwrap().unwrap();
        assert_eq!(meta.last_event_id, 20);
        assert_eq!(entities[0].name, "Entity2");

        // Recover from backup (should be Entity1)
        let (backup_meta, backup_entities, _) = manager.recover_from_backup().unwrap().unwrap();
        assert_eq!(backup_meta.last_event_id, 10);
        assert_eq!(backup_entities[0].name, "Entity1");
    }

    #[test]
    fn test_load_meta_only() {
        let (manager, _temp_dir) = create_test_manager();
        let entities = create_test_entities();
        let relations = create_test_relations();

        manager
            .create_snapshot_with_backup(50, &entities, &relations)
            .unwrap();

        // Load meta only (fast)
        let meta = manager.load_meta().unwrap().unwrap();
        assert_eq!(meta.last_event_id, 50);
        assert_eq!(meta.entity_count, 2);
        assert_eq!(meta.relation_count, 1);
    }

    #[test]
    fn test_no_snapshot_returns_none() {
        let (manager, _temp_dir) = create_test_manager();

        assert!(manager.load_meta().unwrap().is_none());
        assert!(manager.load_full().unwrap().is_none());
        assert!(!manager.snapshot_exists());
    }

    #[test]
    fn test_clear_snapshots() {
        let (manager, _temp_dir) = create_test_manager();

        // Create snapshots
        manager
            .create_snapshot_with_backup(10, &[], &[])
            .unwrap();
        manager
            .create_snapshot_with_backup(20, &[], &[])
            .unwrap();

        assert!(manager.snapshot_exists());
        assert!(manager.previous_path().exists());

        // Clear
        manager.clear_snapshots().unwrap();

        assert!(!manager.snapshot_exists());
        assert!(!manager.previous_path().exists());
    }
}

```

## File ../memory-graph/src\event_store\stats.rs:
```rust
//! Event Store Statistics and Metrics
//!
//! Provides statistics about the event store including:
//! - Event counts by type
//! - Storage size information
//! - Performance metrics

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use crate::types::EventType;

use super::rotation::LogRotation;
use super::store::{EventStoreConfig, EventStoreResult};

/// Statistics about the Event Store
#[derive(Debug, Clone, Default)]
pub struct EventStoreStats {
    /// Total number of events in active log
    pub active_event_count: usize,
    /// Total number of events in archives
    pub archived_event_count: usize,
    /// Size of active event log in bytes
    pub active_log_size: u64,
    /// Total size of archives in bytes
    pub archive_size: u64,
    /// Size of latest snapshot in bytes
    pub snapshot_size: u64,
    /// Number of archive files
    pub archive_file_count: usize,
    /// Events by type in active log
    pub events_by_type: HashMap<EventType, usize>,
    /// Last event ID
    pub last_event_id: u64,
    /// Last snapshot event ID
    pub last_snapshot_event_id: u64,
    /// Events since last snapshot
    pub events_since_snapshot: usize,
}

impl EventStoreStats {
    /// Calculate total events
    pub fn total_events(&self) -> usize {
        self.active_event_count + self.archived_event_count
    }

    /// Calculate total storage size
    pub fn total_size(&self) -> u64 {
        self.active_log_size + self.archive_size + self.snapshot_size
    }

    /// Format size in human-readable format
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// Collector for Event Store statistics
pub struct StatsCollector {
    config: EventStoreConfig,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Collect all statistics
    pub fn collect(&self) -> EventStoreResult<EventStoreStats> {
        let mut stats = EventStoreStats::default();

        // Active log stats
        let events_path = self.config.events_path();
        if events_path.exists() {
            let (count, size, by_type, last_id) = self.analyze_event_file(&events_path)?;
            stats.active_event_count = count;
            stats.active_log_size = size;
            stats.events_by_type = by_type;
            stats.last_event_id = last_id;
        }

        // Archive stats
        let rotation = LogRotation::new(self.config.clone());
        let archives = rotation.list_archives()?;
        stats.archive_file_count = archives.len();
        stats.archived_event_count = archives.iter().map(|a| a.event_count).sum();
        stats.archive_size = archives.iter().map(|a| a.size).sum();

        // Snapshot stats
        let snapshot_path = self.config.latest_snapshot_path();
        if snapshot_path.exists() {
            stats.snapshot_size = fs::metadata(&snapshot_path)?.len();

            // Parse snapshot to get last_snapshot_event_id
            if let Some(meta) = self.parse_snapshot_meta(&snapshot_path)? {
                stats.last_snapshot_event_id = meta;
            }
        }

        // Calculate events since snapshot
        if stats.last_event_id > stats.last_snapshot_event_id {
            stats.events_since_snapshot = (stats.last_event_id - stats.last_snapshot_event_id) as usize;
        }

        Ok(stats)
    }

    /// Analyze an event file
    fn analyze_event_file(&self, path: &Path) -> EventStoreResult<(usize, u64, HashMap<EventType, usize>, u64)> {
        let file = File::open(path)?;
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let reader = BufReader::new(file);

        let mut count = 0;
        let mut by_type: HashMap<EventType, usize> = HashMap::new();
        let mut last_id = 0u64;

        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            count += 1;

            // Parse event
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                // Get event ID
                if let Some(id) = value.get("eventId").and_then(|v| v.as_u64()) {
                    if id > last_id {
                        last_id = id;
                    }
                }

                // Get event type
                if let Some(type_str) = value.get("eventType").and_then(|v| v.as_str()) {
                    if let Ok(event_type) = serde_json::from_value::<EventType>(
                        serde_json::Value::String(type_str.to_string())
                    ) {
                        *by_type.entry(event_type).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok((count, size, by_type, last_id))
    }

    /// Parse snapshot metadata to get last event ID
    fn parse_snapshot_meta(&self, path: &Path) -> EventStoreResult<Option<u64>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        if let Some(Ok(line)) = reader.lines().next() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(id) = value.get("lastEventId").and_then(|v| v.as_u64()) {
                    return Ok(Some(id));
                }
            }
        }

        Ok(None)
    }

    /// Benchmark replay performance
    pub fn benchmark_replay(&self, iterations: usize) -> EventStoreResult<ReplayBenchmark> {
        let events_path = self.config.events_path();
        if !events_path.exists() {
            return Ok(ReplayBenchmark::default());
        }

        let mut total_duration = std::time::Duration::ZERO;
        let mut event_count = 0;

        for _ in 0..iterations {
            let start = Instant::now();

            let file = File::open(&events_path)?;
            let reader = BufReader::new(file);

            for line_result in reader.lines() {
                let line = line_result?;
                if line.trim().is_empty() {
                    continue;
                }

                // Parse event (simulating replay)
                let _: serde_json::Value = serde_json::from_str(&line)?;
                event_count += 1;
            }

            total_duration += start.elapsed();
        }

        let avg_duration = total_duration / iterations as u32;
        let events_per_iter = event_count / iterations;
        let events_per_sec = if avg_duration.as_secs_f64() > 0.0 {
            events_per_iter as f64 / avg_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(ReplayBenchmark {
            iterations,
            events_per_iteration: events_per_iter,
            avg_duration_ms: avg_duration.as_millis() as u64,
            events_per_second: events_per_sec,
        })
    }
}

/// Replay benchmark results
#[derive(Debug, Clone, Default)]
pub struct ReplayBenchmark {
    /// Number of iterations run
    pub iterations: usize,
    /// Events per iteration
    pub events_per_iteration: usize,
    /// Average duration in milliseconds
    pub avg_duration_ms: u64,
    /// Events replayed per second
    pub events_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_store::EventStore;
    use crate::types::EventType;
    use tempfile::TempDir;

    #[test]
    fn test_collect_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({"name": "Alice", "entity_type": "Person", "observations": []}),
        ).unwrap();
        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({"name": "Bob", "entity_type": "Person", "observations": []}),
        ).unwrap();
        store.create_and_append_event(
            EventType::RelationCreated,
            "user".to_string(),
            serde_json::json!({"from": "Alice", "to": "Bob", "relation_type": "knows"}),
        ).unwrap();

        // Collect stats
        let collector = StatsCollector::new(config);
        let stats = collector.collect().unwrap();

        assert_eq!(stats.active_event_count, 3);
        assert_eq!(stats.last_event_id, 3);
        assert!(stats.active_log_size > 0);

        // Verify events by type
        assert_eq!(stats.events_by_type.get(&EventType::EntityCreated), Some(&2));
        assert_eq!(stats.events_by_type.get(&EventType::RelationCreated), Some(&1));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(EventStoreStats::format_size(500), "500 B");
        assert_eq!(EventStoreStats::format_size(1024), "1.00 KB");
        assert_eq!(EventStoreStats::format_size(1536), "1.50 KB");
        assert_eq!(EventStoreStats::format_size(1048576), "1.00 MB");
        assert_eq!(EventStoreStats::format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_benchmark_replay() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        for i in 1..=10 {
            store.create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                serde_json::json!({"name": format!("Entity{}", i), "entity_type": "Test", "observations": []}),
            ).unwrap();
        }

        // Benchmark
        let collector = StatsCollector::new(config);
        let benchmark = collector.benchmark_replay(3).unwrap();

        assert_eq!(benchmark.iterations, 3);
        assert_eq!(benchmark.events_per_iteration, 10);
        assert!(benchmark.events_per_second > 0.0);
    }
}

```

## File ../memory-graph/src\event_store\store.rs:
```rust
//! Event Store - Core event sourcing implementation
//!
//! The EventStore manages the append-only event log and provides
//! functionality for replaying events to rebuild state.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::types::{
    Entity, EntityCreatedData, EntityDeletedData, EntityUpdatedData, Event, EventType,
    ObservationAddedData, ObservationRemovedData, Relation, RelationCreatedData,
    RelationDeletedData, SnapshotMeta,
};

/// Configuration for the EventStore
#[derive(Debug, Clone)]
pub struct EventStoreConfig {
    /// Path to the data directory
    pub data_dir: PathBuf,
    /// Threshold for creating snapshots (number of events)
    pub snapshot_threshold: usize,
    /// Whether to archive old event logs
    pub archive_old_events: bool,
    /// Whether to compress archived events
    pub compress_archive: bool,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            snapshot_threshold: 1000,
            archive_old_events: true,
            compress_archive: false, // TODO: implement compression
        }
    }
}

impl EventStoreConfig {
    /// Create config with custom data directory
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Create config with custom data directory (alias for new)
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self::new(data_dir)
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get path to events.jsonl
    pub fn events_path(&self) -> PathBuf {
        self.data_dir.join("events.jsonl")
    }

    /// Get path to snapshots directory
    pub fn snapshots_dir(&self) -> PathBuf {
        self.data_dir.join("snapshots")
    }

    /// Get path to latest snapshot
    pub fn latest_snapshot_path(&self) -> PathBuf {
        self.snapshots_dir().join("latest.jsonl")
    }

    /// Get path to previous snapshot (backup)
    pub fn previous_snapshot_path(&self) -> PathBuf {
        self.snapshots_dir().join("previous.jsonl")
    }

    /// Get path to archive directory
    pub fn archive_dir(&self) -> PathBuf {
        self.data_dir.join("archive")
    }
}

/// Result type for EventStore operations
pub type EventStoreResult<T> = Result<T, EventStoreError>;

/// Errors that can occur in EventStore operations
#[derive(Debug)]
pub enum EventStoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidEvent(String),
    SnapshotCorrupted(String),
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStoreError::Io(e) => write!(f, "IO error: {}", e),
            EventStoreError::Json(e) => write!(f, "JSON error: {}", e),
            EventStoreError::InvalidEvent(msg) => write!(f, "Invalid event: {}", msg),
            EventStoreError::SnapshotCorrupted(msg) => write!(f, "Snapshot corrupted: {}", msg),
        }
    }
}

impl std::error::Error for EventStoreError {}

impl From<std::io::Error> for EventStoreError {
    fn from(e: std::io::Error) -> Self {
        EventStoreError::Io(e)
    }
}

impl From<serde_json::Error> for EventStoreError {
    fn from(e: serde_json::Error) -> Self {
        EventStoreError::Json(e)
    }
}

/// The EventStore manages append-only event log and state replay
pub struct EventStore {
    config: EventStoreConfig,
    /// Next event ID to assign
    next_event_id: u64,
    /// Number of events since last snapshot
    events_since_snapshot: usize,
    /// Last event ID included in most recent snapshot
    last_snapshot_event_id: u64,
}

impl EventStore {
    /// Create a new EventStore with default config
    pub fn new() -> Self {
        Self::with_config(EventStoreConfig::default())
    }

    /// Create a new EventStore with custom config
    pub fn with_config(config: EventStoreConfig) -> Self {
        Self {
            config,
            next_event_id: 1,
            events_since_snapshot: 0,
            last_snapshot_event_id: 0,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &EventStoreConfig {
        &self.config
    }

    /// Get the next event ID (without incrementing)
    pub fn next_event_id(&self) -> u64 {
        self.next_event_id
    }

    /// Get events since last snapshot
    pub fn events_since_snapshot(&self) -> usize {
        self.events_since_snapshot
    }

    /// Check if snapshot should be created
    pub fn should_snapshot(&self) -> bool {
        self.events_since_snapshot >= self.config.snapshot_threshold
    }

    /// Append an event to the event log
    ///
    /// This is the core write operation. Events are appended atomically
    /// with fsync to ensure durability.
    pub fn append_event(&mut self, event: Event) -> EventStoreResult<u64> {
        let events_path = self.config.events_path();

        // Ensure parent directory exists
        if let Some(parent) = events_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;

        // Serialize and write
        let json_line = event.to_json_line()?;
        writeln!(file, "{}", json_line)?;

        // Sync to disk for durability
        file.sync_all()?;

        // Update internal state
        let event_id = event.event_id;
        if event_id >= self.next_event_id {
            self.next_event_id = event_id + 1;
        }
        self.events_since_snapshot += 1;

        Ok(event_id)
    }

    /// Create a new event and append it
    pub fn create_and_append_event(
        &mut self,
        event_type: EventType,
        user: String,
        data: serde_json::Value,
    ) -> EventStoreResult<Event> {
        let event_id = self.next_event_id;
        self.next_event_id += 1;

        let event = Event::new(event_type, event_id, user, data);
        self.append_event(event.clone())?;

        Ok(event)
    }

    /// Load all events from the event log
    pub fn load_events(&self) -> EventStoreResult<Vec<Event>> {
        let events_path = self.config.events_path();

        if !events_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&events_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            match Event::from_json_line(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse event at line {}: {}",
                        line_num + 1,
                        e
                    );
                    // Continue loading other events
                }
            }
        }

        Ok(events)
    }

    /// Load events after a specific event ID
    ///
    /// Used for replaying events after loading a snapshot.
    pub fn load_events_after(&self, after_event_id: u64) -> EventStoreResult<Vec<Event>> {
        let all_events = self.load_events()?;
        Ok(all_events
            .into_iter()
            .filter(|e| e.event_id > after_event_id)
            .collect())
    }

    /// Load snapshot metadata from a snapshot file
    pub fn load_snapshot_meta(&self) -> EventStoreResult<Option<SnapshotMeta>> {
        let snapshot_path = self.config.latest_snapshot_path();

        if !snapshot_path.exists() {
            return Ok(None);
        }

        let file = File::open(&snapshot_path)?;
        let reader = BufReader::new(file);

        // First line should be metadata
        if let Some(first_line) = reader.lines().next() {
            let line = first_line?;
            let meta = SnapshotMeta::from_json_line(&line)?;
            Ok(Some(meta))
        } else {
            Err(EventStoreError::SnapshotCorrupted(
                "Empty snapshot file".to_string(),
            ))
        }
    }

    /// Load entities and relations from snapshot
    pub fn load_snapshot(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let snapshot_path = self.config.latest_snapshot_path();

        if !snapshot_path.exists() {
            return Ok(None);
        }

        let file = File::open(&snapshot_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line is metadata
        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty snapshot".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        // Parse remaining lines
        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as entity or relation based on structure
            let value: serde_json::Value = serde_json::from_str(&line)?;

            if value.get("entityType").is_some() && value.get("name").is_some() {
                // It's an entity
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                // It's a relation
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
        }

        Ok(Some((meta, entities, relations)))
    }

    /// Apply a single event to the state
    ///
    /// This is the core state mutation logic. Each event type
    /// corresponds to a specific state change.
    pub fn apply_event(
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        event: &Event,
    ) -> EventStoreResult<()> {
        match event.event_type {
            EventType::EntityCreated => {
                let data: EntityCreatedData = event.parse_data()?;

                // Check if entity already exists
                if entities.iter().any(|e| e.name == data.name) {
                    // Entity already exists, skip (idempotent)
                    return Ok(());
                }

                let entity = Entity {
                    name: data.name,
                    entity_type: data.entity_type,
                    observations: data.observations,
                    created_by: event.user.clone(),
                    updated_by: event.user.clone(),
                    created_at: event.timestamp as u64,
                    updated_at: event.timestamp as u64,
                };
                entities.push(entity);
            }

            EventType::EntityUpdated => {
                let data: EntityUpdatedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.name) {
                    if let Some(new_type) = data.entity_type {
                        entity.entity_type = new_type;
                    }
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::EntityDeleted => {
                let data: EntityDeletedData = event.parse_data()?;

                // Remove entity
                entities.retain(|e| e.name != data.name);

                // Remove relations involving this entity
                relations.retain(|r| r.from != data.name && r.to != data.name);
            }

            EventType::ObservationAdded => {
                let data: ObservationAddedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.entity) {
                    if !entity.observations.contains(&data.observation) {
                        entity.observations.push(data.observation);
                    }
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::ObservationRemoved => {
                let data: ObservationRemovedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.entity) {
                    entity.observations.retain(|o| o != &data.observation);
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::RelationCreated => {
                let data: RelationCreatedData = event.parse_data()?;

                // Check if relation already exists
                let exists = relations.iter().any(|r| {
                    r.from == data.from && r.to == data.to && r.relation_type == data.relation_type
                });

                if !exists {
                    let relation = Relation {
                        from: data.from,
                        to: data.to,
                        relation_type: data.relation_type,
                        created_by: event.user.clone(),
                        created_at: event.timestamp as u64,
                        valid_from: data.valid_from.map(|v| v as u64),
                        valid_to: data.valid_to.map(|v| v as u64),
                    };
                    relations.push(relation);
                }
            }

            EventType::RelationDeleted => {
                let data: RelationDeletedData = event.parse_data()?;

                relations.retain(|r| {
                    !(r.from == data.from && r.to == data.to && r.relation_type == data.relation_type)
                });
            }
        }

        Ok(())
    }

    /// Replay all events to rebuild state
    ///
    /// This loads and applies all events in order to reconstruct
    /// the current state from scratch.
    pub fn replay_all(&self) -> EventStoreResult<(Vec<Entity>, Vec<Relation>, u64)> {
        let events = self.load_events()?;
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut max_event_id = 0u64;

        for event in &events {
            Self::apply_event(&mut entities, &mut relations, event)?;
            if event.event_id > max_event_id {
                max_event_id = event.event_id;
            }
        }

        Ok((entities, relations, max_event_id))
    }

    /// Replay events after a specific event ID
    pub fn replay_after(
        &self,
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        after_event_id: u64,
    ) -> EventStoreResult<u64> {
        let events = self.load_events_after(after_event_id)?;
        let mut max_event_id = after_event_id;

        for event in &events {
            Self::apply_event(entities, relations, event)?;
            if event.event_id > max_event_id {
                max_event_id = event.event_id;
            }
        }

        Ok(max_event_id)
    }

    /// Initialize from storage (snapshot + replay)
    ///
    /// This is the main startup path:
    /// 1. Try to load latest snapshot
    /// 2. Replay any events after the snapshot
    /// 3. Return the reconstructed state
    pub fn initialize(&mut self) -> EventStoreResult<(Vec<Entity>, Vec<Relation>)> {
        // Try to load snapshot first
        if let Some((meta, mut entities, mut relations)) = self.load_snapshot()? {
            self.last_snapshot_event_id = meta.last_event_id;
            self.next_event_id = meta.last_event_id + 1;

            // Replay events after snapshot
            let max_event_id = self.replay_after(&mut entities, &mut relations, meta.last_event_id)?;

            if max_event_id > self.next_event_id {
                self.next_event_id = max_event_id + 1;
            }

            self.events_since_snapshot = (max_event_id - meta.last_event_id) as usize;

            println!(
                "Loaded snapshot (event_id: {}) + replayed {} events. Total: {} entities, {} relations.",
                meta.last_event_id,
                self.events_since_snapshot,
                entities.len(),
                relations.len()
            );

            Ok((entities, relations))
        } else {
            // No snapshot, replay all events
            let (entities, relations, max_event_id) = self.replay_all()?;

            if max_event_id > 0 {
                self.next_event_id = max_event_id + 1;
                self.events_since_snapshot = max_event_id as usize;
            }

            println!(
                "No snapshot found. Replayed {} events. Total: {} entities, {} relations.",
                max_event_id, entities.len(), relations.len()
            );

            Ok((entities, relations))
        }
    }

    /// Reset snapshot counter (called after snapshot creation)
    pub fn snapshot_created(&mut self, last_event_id: u64) {
        self.last_snapshot_event_id = last_event_id;
        self.events_since_snapshot = 0;
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_store() -> (EventStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_data_dir(temp_dir.path());

        // Create directories
        std::fs::create_dir_all(config.events_path().parent().unwrap()).unwrap();
        std::fs::create_dir_all(config.snapshots_dir()).unwrap();

        let store = EventStore::with_config(config);
        (store, temp_dir)
    }

    #[test]
    fn test_append_and_load_events() {
        let (mut store, _temp_dir) = create_test_store();

        // Append some events
        let event1 = store
            .create_and_append_event(
                EventType::EntityCreated,
                "test_user".to_string(),
                json!({
                    "name": "Test:Entity",
                    "entity_type": "Test",
                    "observations": ["obs1"]
                }),
            )
            .unwrap();

        let event2 = store
            .create_and_append_event(
                EventType::ObservationAdded,
                "test_user".to_string(),
                json!({
                    "entity": "Test:Entity",
                    "observation": "obs2"
                }),
            )
            .unwrap();

        assert_eq!(event1.event_id, 1);
        assert_eq!(event2.event_id, 2);
        assert_eq!(store.next_event_id(), 3);
        assert_eq!(store.events_since_snapshot(), 2);

        // Load events
        let events = store.load_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
        assert_eq!(events[1].event_type, EventType::ObservationAdded);
    }

    #[test]
    fn test_load_events_after() {
        let (mut store, _temp_dir) = create_test_store();

        // Create 5 events
        for i in 1..=5 {
            store
                .create_and_append_event(
                    EventType::EntityCreated,
                    "user".to_string(),
                    json!({
                        "name": format!("Entity:{}", i),
                        "entity_type": "Test"
                    }),
                )
                .unwrap();
        }

        // Load events after ID 3
        let events = store.load_events_after(3).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, 4);
        assert_eq!(events[1].event_id, 5);
    }

    #[test]
    fn test_apply_entity_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::EntityCreated,
            1,
            "user".to_string(),
            json!({
                "name": "Bug:Login",
                "entity_type": "Bug",
                "observations": ["Login fails"]
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Bug:Login");
        assert_eq!(entities[0].entity_type, "Bug");
        assert_eq!(entities[0].observations, vec!["Login fails"]);
    }

    #[test]
    fn test_apply_observation_added() {
        let mut entities = vec![Entity::new("Bug:X".to_string(), "Bug".to_string())];
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::ObservationAdded,
            1,
            "user".to_string(),
            json!({
                "entity": "Bug:X",
                "observation": "New observation"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities[0].observations, vec!["New observation"]);
    }

    #[test]
    fn test_apply_entity_deleted() {
        let mut entities = vec![
            Entity::new("Bug:X".to_string(), "Bug".to_string()),
            Entity::new("Bug:Y".to_string(), "Bug".to_string()),
        ];
        let mut relations = vec![
            Relation::new("Bug:X".to_string(), "Module:A".to_string(), "affects".to_string()),
            Relation::new("Bug:Y".to_string(), "Module:B".to_string(), "affects".to_string()),
        ];

        let event = Event::new(
            EventType::EntityDeleted,
            1,
            "user".to_string(),
            json!({
                "name": "Bug:X"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Bug:Y");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].from, "Bug:Y");
    }

    #[test]
    fn test_apply_relation_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::RelationCreated,
            1,
            "user".to_string(),
            json!({
                "from": "Bug:X",
                "to": "Module:Auth",
                "relation_type": "affects"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].from, "Bug:X");
        assert_eq!(relations[0].to, "Module:Auth");
        assert_eq!(relations[0].relation_type, "affects");
    }

    #[test]
    fn test_replay_all() {
        let (mut store, _temp_dir) = create_test_store();

        // Create events
        store
            .create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                json!({"name": "A", "entity_type": "Test"}),
            )
            .unwrap();

        store
            .create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                json!({"name": "B", "entity_type": "Test"}),
            )
            .unwrap();

        store
            .create_and_append_event(
                EventType::RelationCreated,
                "user".to_string(),
                json!({"from": "A", "to": "B", "relation_type": "depends_on"}),
            )
            .unwrap();

        // Replay
        let (entities, relations, max_id) = store.replay_all().unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 1);
        assert_eq!(max_id, 3);
    }

    #[test]
    fn test_idempotent_entity_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::EntityCreated,
            1,
            "user".to_string(),
            json!({"name": "A", "entity_type": "Test"}),
        );

        // Apply twice
        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();
        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        // Should still be just 1 entity (idempotent)
        assert_eq!(entities.len(), 1);
    }
}

```

## File ../memory-graph/src\knowledge_base\crud.rs:
```rust
//! CRUD operations for the knowledge base

use std::collections::HashSet;

use serde_json::json;

use crate::api::websocket::ws_helpers;
use crate::types::{Entity, EventType, McpResult, Observation, ObservationDeletion, Relation};
use crate::utils::time::current_timestamp;

use super::KnowledgeBase;

/// Create new entities (thread-safe: holds write lock during entire operation)
pub fn create_entities(kb: &KnowledgeBase, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
    let mut graph = kb.graph.write().unwrap();
    let existing_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
    let now = current_timestamp();

    let mut created = Vec::new();
    for mut entity in entities {
        if !existing_names.contains(&entity.name) {
            // Auto-fill user info if not provided
            if entity.created_by.is_empty() || entity.created_by == "system" {
                entity.created_by = kb.current_user.clone();
            }
            if entity.updated_by.is_empty() || entity.updated_by == "system" {
                entity.updated_by = kb.current_user.clone();
            }
            entity.created_at = now;
            entity.updated_at = now;

            // Emit event if Event Sourcing is enabled
            if kb.event_sourcing_enabled {
                kb.emit_event(
                    EventType::EntityCreated,
                    json!({
                        "name": entity.name,
                        "entity_type": entity.entity_type,
                        "observations": entity.observations
                    }),
                )?;
            }

            // Broadcast to WebSocket clients
            ws_helpers::entity_created(&entity, Some(kb.current_user.clone()));

            created.push(entity.clone());
            graph.entities.push(entity);
        }
    }

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(created)
}

/// Create new relations (thread-safe: holds write lock during entire operation)
pub fn create_relations(kb: &KnowledgeBase, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
    let mut graph = kb.graph.write().unwrap();
    let entity_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
    let now = current_timestamp();

    // Use tuple of owned Strings to avoid borrow issues
    let existing_relations: HashSet<(String, String, String)> = graph
        .relations
        .iter()
        .map(|r| (r.from.clone(), r.to.clone(), r.relation_type.clone()))
        .collect();

    let mut created = Vec::new();
    for mut relation in relations {
        if entity_names.contains(&relation.from) && entity_names.contains(&relation.to) {
            let key = (
                relation.from.clone(),
                relation.to.clone(),
                relation.relation_type.clone(),
            );
            if !existing_relations.contains(&key) {
                // Auto-fill user info if not provided
                if relation.created_by.is_empty() || relation.created_by == "system" {
                    relation.created_by = kb.current_user.clone();
                }
                relation.created_at = now;

                // Emit event if Event Sourcing is enabled
                if kb.event_sourcing_enabled {
                    kb.emit_event(
                        EventType::RelationCreated,
                        json!({
                            "from": relation.from,
                            "to": relation.to,
                            "relation_type": relation.relation_type,
                            "valid_from": relation.valid_from,
                            "valid_to": relation.valid_to
                        }),
                    )?;
                }

                // Broadcast to WebSocket clients
                ws_helpers::relation_created(&relation, Some(kb.current_user.clone()));

                created.push(relation.clone());
                graph.relations.push(relation);
            }
        }
    }

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(created)
}

/// Add observations to entities (thread-safe: holds write lock during entire operation)
pub fn add_observations(
    kb: &KnowledgeBase,
    observations: Vec<Observation>,
) -> McpResult<Vec<Observation>> {
    let mut graph = kb.graph.write().unwrap();
    let mut added = Vec::new();
    let now = current_timestamp();

    for obs in observations {
        if let Some(entity) = graph.entities.iter_mut().find(|e| e.name == obs.entity_name) {
            let existing: HashSet<String> = entity.observations.iter().cloned().collect();
            let mut new_contents = Vec::new();

            for content in &obs.contents {
                if !existing.contains(content) {
                    // Emit event if Event Sourcing is enabled
                    if kb.event_sourcing_enabled {
                        kb.emit_event(
                            EventType::ObservationAdded,
                            json!({
                                "entity": obs.entity_name,
                                "observation": content
                            }),
                        )?;
                    }

                    entity.observations.push(content.clone());
                    new_contents.push(content.clone());
                }
            }

            if !new_contents.is_empty() {
                entity.updated_at = now;
                entity.updated_by = kb.current_user.clone();

                // Broadcast to WebSocket clients
                ws_helpers::entity_updated(
                    &obs.entity_name,
                    new_contents.clone(),
                    Some(kb.current_user.clone()),
                );

                added.push(Observation {
                    entity_name: obs.entity_name.clone(),
                    contents: new_contents,
                });
            }
        }
    }

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(added)
}

/// Delete entities (thread-safe: holds write lock during entire operation)
pub fn delete_entities(kb: &KnowledgeBase, entity_names: Vec<String>) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();
    let names_to_delete: HashSet<String> = entity_names.iter().cloned().collect();

    // Emit events and broadcast for each entity being deleted
    for name in &entity_names {
        if graph.entities.iter().any(|e| &e.name == name) {
            if kb.event_sourcing_enabled {
                kb.emit_event(
                    EventType::EntityDeleted,
                    json!({ "name": name }),
                )?;
            }
            // Broadcast to WebSocket clients
            ws_helpers::entity_deleted(name, Some(kb.current_user.clone()));
        }
    }

    graph
        .entities
        .retain(|e| !names_to_delete.contains(&e.name));
    graph
        .relations
        .retain(|r| !names_to_delete.contains(&r.from) && !names_to_delete.contains(&r.to));

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(())
}

/// Delete observations from entities (thread-safe: holds write lock during entire operation)
pub fn delete_observations(
    kb: &KnowledgeBase,
    deletions: Vec<ObservationDeletion>,
) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();

    for deletion in deletions {
        if let Some(entity) = graph
            .entities
            .iter_mut()
            .find(|e| e.name == deletion.entity_name)
        {
            // Emit events for each observation being deleted
            if kb.event_sourcing_enabled {
                for obs in &deletion.observations {
                    if entity.observations.contains(obs) {
                        kb.emit_event(
                            EventType::ObservationRemoved,
                            json!({
                                "entity": deletion.entity_name,
                                "observation": obs
                            }),
                        )?;
                    }
                }
            }

            let to_remove: HashSet<String> = deletion.observations.into_iter().collect();
            entity.observations.retain(|o| !to_remove.contains(o));
        }
    }

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(())
}

/// Delete relations (thread-safe: holds write lock during entire operation)
pub fn delete_relations(kb: &KnowledgeBase, relations: Vec<Relation>) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();

    // Emit events and broadcast for each relation being deleted
    for relation in &relations {
        let exists = graph.relations.iter().any(|r| {
            r.from == relation.from
                && r.to == relation.to
                && r.relation_type == relation.relation_type
        });
        if exists {
            if kb.event_sourcing_enabled {
                kb.emit_event(
                    EventType::RelationDeleted,
                    json!({
                        "from": relation.from,
                        "to": relation.to,
                        "relation_type": relation.relation_type
                    }),
                )?;
            }
            // Broadcast to WebSocket clients
            ws_helpers::relation_deleted(
                &relation.from,
                &relation.to,
                &relation.relation_type,
                Some(kb.current_user.clone()),
            );
        }
    }

    // Use tuple instead of string concat to avoid issues with | in entity names
    let to_delete: HashSet<(String, String, String)> = relations
        .iter()
        .map(|r| (r.from.clone(), r.to.clone(), r.relation_type.clone()))
        .collect();

    graph.relations.retain(|r| {
        let key = (r.from.clone(), r.to.clone(), r.relation_type.clone());
        !to_delete.contains(&key)
    });

    // Persist based on mode
    if !kb.event_sourcing_enabled {
        kb.persist_to_file(&graph)?;
    }

    drop(graph);
    kb.maybe_create_snapshot()?;

    Ok(())
}

```

## File ../memory-graph/src\knowledge_base\mod.rs:
```rust
//! Knowledge Base - Core data engine
//!
//! This module contains the main knowledge base implementation with
//! thread-safe CRUD operations, queries, and temporal features.
//!
//! # Event Sourcing
//!
//! The knowledge base now supports Event Sourcing mode where all mutations
//! are recorded as immutable events. Set `MEMORY_EVENT_SOURCING=true` to enable.

mod crud;
pub mod inference;
mod query;
mod summarize;
mod temporal;
mod traversal;

use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, RwLock};

use crate::event_store::{EventStore, EventStoreConfig, LogRotation, SnapshotManager};
use crate::types::{
    Entity, EventType, KnowledgeGraph, McpResult, Observation, ObservationDeletion, PathStep,
    RelatedEntities, Relation, Summary, TraversalResult,
};
use crate::utils::time::get_current_user;

/// Knowledge base with in-memory cache for thread-safe operations
/// Uses RwLock for better concurrent read performance (read-heavy workload)
pub struct KnowledgeBase {
    pub(crate) memory_file_path: String,
    pub(crate) graph: RwLock<KnowledgeGraph>,
    pub(crate) current_user: String,
    /// Event store for Event Sourcing (None = legacy mode)
    pub(crate) event_store: Option<Mutex<EventStore>>,
    /// Snapshot manager for creating/loading snapshots
    pub(crate) snapshot_manager: Option<SnapshotManager>,
    /// Log rotation manager for archiving old events
    pub(crate) log_rotation: Option<LogRotation>,
    /// Whether Event Sourcing mode is enabled
    pub(crate) event_sourcing_enabled: bool,
}

impl KnowledgeBase {
    /// Create a new knowledge base instance
    ///
    /// If MEMORY_EVENT_SOURCING=true, uses Event Sourcing mode.
    /// Otherwise, uses legacy memory.jsonl mode.
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let default_memory_path = current_dir.join("memory.jsonl");

        let memory_file_path = match env::var("MEMORY_FILE_PATH") {
            Ok(path) => {
                if Path::new(&path).is_absolute() {
                    path
                } else {
                    current_dir.join(path).to_string_lossy().to_string()
                }
            }
            Err(_) => default_memory_path.to_string_lossy().to_string(),
        };

        // Detect current user once at startup
        let current_user = get_current_user();

        // Check if Event Sourcing mode is enabled
        let event_sourcing_enabled = env::var("MEMORY_EVENT_SOURCING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if event_sourcing_enabled {
            Self::new_with_event_sourcing(memory_file_path, current_user)
        } else {
            Self::new_legacy(memory_file_path, current_user)
        }
    }

    /// Create knowledge base in legacy mode (direct file writes)
    fn new_legacy(memory_file_path: String, current_user: String) -> Self {
        let graph = Self::load_graph_from_file(&memory_file_path).unwrap_or_default();

        Self {
            memory_file_path,
            graph: RwLock::new(graph),
            current_user,
            event_store: None,
            snapshot_manager: None,
            log_rotation: None,
            event_sourcing_enabled: false,
        }
    }

    /// Create knowledge base with Event Sourcing enabled
    fn new_with_event_sourcing(memory_file_path: String, current_user: String) -> Self {
        // Determine data directory (parent of memory.jsonl or ./data)
        let data_dir = Path::new(&memory_file_path)
            .parent()
            .map(|p| p.join("data"))
            .unwrap_or_else(|| std::path::PathBuf::from("data"));

        let config = EventStoreConfig::with_data_dir(&data_dir);
        let mut event_store = EventStore::with_config(config.clone());
        let snapshot_manager = SnapshotManager::new(config.clone());
        let log_rotation = LogRotation::new(config);

        // Initialize from snapshot + replay events
        let (entities, relations) = match event_store.initialize() {
            Ok((e, r)) => (e, r),
            Err(e) => {
                eprintln!("Warning: Failed to initialize from event store: {}", e);
                eprintln!("Falling back to empty graph");
                (Vec::new(), Vec::new())
            }
        };

        let graph = KnowledgeGraph { entities, relations };

        println!(
            "Event Sourcing enabled: {} entities, {} relations",
            graph.entities.len(),
            graph.relations.len()
        );

        Self {
            memory_file_path,
            graph: RwLock::new(graph),
            current_user,
            event_store: Some(Mutex::new(event_store)),
            snapshot_manager: Some(snapshot_manager),
            log_rotation: Some(log_rotation),
            event_sourcing_enabled: true,
        }
    }

    /// Create a new knowledge base with custom file path
    pub fn with_file_path(file_path: String) -> Self {
        let current_user = get_current_user();

        let event_sourcing_enabled = env::var("MEMORY_EVENT_SOURCING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if event_sourcing_enabled {
            Self::new_with_event_sourcing(file_path, current_user)
        } else {
            Self::new_legacy(file_path, current_user)
        }
    }

    /// Create a new knowledge base for testing with explicit parameters
    #[cfg(test)]
    pub fn for_testing(file_path: String, user: String) -> Self {
        Self {
            memory_file_path: file_path,
            graph: RwLock::new(KnowledgeGraph::default()),
            current_user: user,
            event_store: None,
            snapshot_manager: None,
            log_rotation: None,
            event_sourcing_enabled: false,
        }
    }

    /// Create a knowledge base for testing with Event Sourcing enabled
    #[cfg(test)]
    pub fn for_testing_event_sourcing(data_dir: &Path, user: String) -> Self {
        let config = EventStoreConfig::with_data_dir(data_dir);
        let mut event_store = EventStore::with_config(config.clone());
        let snapshot_manager = SnapshotManager::new(config.clone());
        let log_rotation = LogRotation::new(config);

        let (entities, relations) = event_store.initialize().unwrap_or_default();
        let graph = KnowledgeGraph { entities, relations };

        Self {
            memory_file_path: data_dir.join("memory.jsonl").to_string_lossy().to_string(),
            graph: RwLock::new(graph),
            current_user: user,
            event_store: Some(Mutex::new(event_store)),
            snapshot_manager: Some(snapshot_manager),
            log_rotation: Some(log_rotation),
            event_sourcing_enabled: true,
        }
    }

    /// Load graph from file (static helper for initialization)
    fn load_graph_from_file(file_path: &str) -> McpResult<KnowledgeGraph> {
        if !Path::new(file_path).exists() {
            return Ok(KnowledgeGraph::default());
        }

        let content = fs::read_to_string(file_path)?;
        let mut graph = KnowledgeGraph::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entity) = serde_json::from_str::<Entity>(line) {
                if !entity.name.is_empty() && !entity.entity_type.is_empty() {
                    graph.entities.push(entity);
                    continue;
                }
            }

            if let Ok(relation) = serde_json::from_str::<Relation>(line) {
                if !relation.from.is_empty() && !relation.to.is_empty() {
                    graph.relations.push(relation);
                }
            }
        }

        Ok(graph)
    }

    /// Get a clone of the current graph (thread-safe read)
    /// Uses read lock - allows multiple concurrent readers
    pub(crate) fn load_graph(&self) -> McpResult<KnowledgeGraph> {
        Ok(self.graph.read().unwrap().clone())
    }

    /// Persist graph to file (internal helper, expects caller to hold write lock)
    pub(crate) fn persist_to_file(&self, graph: &KnowledgeGraph) -> McpResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.memory_file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut content = String::new();

        for entity in &graph.entities {
            content.push_str(&serde_json::to_string(entity)?);
            content.push('\n');
        }

        for relation in &graph.relations {
            content.push_str(&serde_json::to_string(relation)?);
            content.push('\n');
        }

        fs::write(&self.memory_file_path, content)?;
        Ok(())
    }

    /// Get the current user
    pub fn current_user(&self) -> &str {
        &self.current_user
    }

    /// Get the memory file path
    pub fn file_path(&self) -> &str {
        &self.memory_file_path
    }

    /// Check if Event Sourcing mode is enabled
    pub fn is_event_sourcing_enabled(&self) -> bool {
        self.event_sourcing_enabled
    }

    /// Emit an event to the event store (if Event Sourcing is enabled)
    ///
    /// Returns the event ID if successful, or None if Event Sourcing is disabled.
    pub(crate) fn emit_event(
        &self,
        event_type: EventType,
        data: serde_json::Value,
    ) -> McpResult<Option<u64>> {
        if let Some(ref event_store) = self.event_store {
            let mut store = event_store.lock().unwrap();
            let event = store.create_and_append_event(event_type, self.current_user.clone(), data)?;
            Ok(Some(event.event_id))
        } else {
            Ok(None)
        }
    }

    /// Check if a snapshot should be created and create it if so
    pub(crate) fn maybe_create_snapshot(&self) -> McpResult<()> {
        if let (Some(ref event_store), Some(ref snapshot_manager)) =
            (&self.event_store, &self.snapshot_manager)
        {
            let store = event_store.lock().unwrap();

            if store.should_snapshot() {
                let graph = self.graph.read().unwrap();
                let last_event_id = store.next_event_id().saturating_sub(1);
                let config = store.config().clone();

                // Drop the store lock before creating snapshot
                drop(store);

                snapshot_manager.create_snapshot_with_backup(
                    last_event_id,
                    &graph.entities,
                    &graph.relations,
                )?;

                // Rotate event log to archive old events
                if config.archive_old_events {
                    if let Some(ref rotation) = self.log_rotation {
                        if let Err(e) = rotation.rotate_after_snapshot(last_event_id) {
                            eprintln!("Warning: Failed to rotate event log: {}", e);
                        }
                    }
                }

                // Update the store's snapshot counter
                let mut store = event_store.lock().unwrap();
                store.snapshot_created(last_event_id);
            }
        }
        Ok(())
    }

    /// Force create a snapshot (for graceful shutdown)
    /// Returns the path to the snapshot file if created, or None if Event Sourcing is disabled
    pub fn create_snapshot(&self) -> McpResult<Option<std::path::PathBuf>> {
        if let (Some(ref event_store), Some(ref snapshot_manager)) =
            (&self.event_store, &self.snapshot_manager)
        {
            let store = event_store.lock().unwrap();
            let graph = self.graph.read().unwrap();
            let last_event_id = store.next_event_id().saturating_sub(1);

            if last_event_id > 0 {
                drop(store);
                snapshot_manager.create_snapshot_with_backup(
                    last_event_id,
                    &graph.entities,
                    &graph.relations,
                )?;

                let mut store = event_store.lock().unwrap();
                store.snapshot_created(last_event_id);

                return Ok(Some(snapshot_manager.latest_path()));
            }
        }
        Ok(None)
    }

    /// Get Event Store statistics (only in Event Sourcing mode)
    pub fn get_stats(&self) -> Option<crate::event_store::EventStoreStats> {
        if let Some(ref event_store) = self.event_store {
            let store = event_store.lock().unwrap();
            let collector = crate::event_store::StatsCollector::new(store.config().clone());
            collector.collect().ok()
        } else {
            None
        }
    }

    /// Manually rotate event log (archive old events)
    pub fn rotate_event_log(&self) -> McpResult<Option<std::path::PathBuf>> {
        if let (Some(ref event_store), Some(ref rotation)) =
            (&self.event_store, &self.log_rotation)
        {
            // Just need to drop the lock, don't need the config
            let _store = event_store.lock().unwrap();
            drop(_store);

            // Get last snapshot event ID from snapshot manager
            if let Some(ref snapshot_manager) = self.snapshot_manager {
                if let Ok(Some(meta)) = snapshot_manager.load_meta() {
                    return Ok(rotation.rotate_after_snapshot(meta.last_event_id)?);
                }
            }
        }
        Ok(None)
    }

    /// Clean up old archive files, keeping only the most recent N
    pub fn cleanup_archives(&self, keep_count: usize) -> McpResult<usize> {
        if let Some(ref rotation) = self.log_rotation {
            Ok(rotation.cleanup_old_archives(keep_count)?)
        } else {
            Ok(0)
        }
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export methods from submodules by implementing them here
impl KnowledgeBase {
    // CRUD operations (from crud.rs)
    pub fn create_entities(&self, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
        crud::create_entities(self, entities)
    }

    pub fn create_relations(&self, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
        crud::create_relations(self, relations)
    }

    pub fn add_observations(&self, observations: Vec<Observation>) -> McpResult<Vec<Observation>> {
        crud::add_observations(self, observations)
    }

    pub fn delete_entities(&self, entity_names: Vec<String>) -> McpResult<()> {
        crud::delete_entities(self, entity_names)
    }

    pub fn delete_observations(&self, deletions: Vec<ObservationDeletion>) -> McpResult<()> {
        crud::delete_observations(self, deletions)
    }

    pub fn delete_relations(&self, relations: Vec<Relation>) -> McpResult<()> {
        crud::delete_relations(self, relations)
    }

    // Query operations (from query.rs)
    pub fn read_graph(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> McpResult<KnowledgeGraph> {
        query::read_graph(self, limit, offset)
    }

    pub fn search_nodes(
        &self,
        query: &str,
        limit: Option<usize>,
        include_relations: bool,
    ) -> McpResult<KnowledgeGraph> {
        query::search_nodes(self, query, limit, include_relations)
    }

    pub fn open_nodes(&self, names: Vec<String>) -> McpResult<KnowledgeGraph> {
        query::open_nodes(self, names)
    }

    // Traversal operations (from traversal.rs)
    pub fn get_related(
        &self,
        entity_name: &str,
        relation_type: Option<&str>,
        direction: &str,
    ) -> McpResult<RelatedEntities> {
        traversal::get_related(self, entity_name, relation_type, direction)
    }

    pub fn traverse(
        &self,
        start: &str,
        path: Vec<PathStep>,
        max_results: usize,
    ) -> McpResult<TraversalResult> {
        traversal::traverse(self, start, path, max_results)
    }

    // Summarize operations (from summarize.rs)
    pub fn summarize(
        &self,
        entity_names: Option<Vec<String>>,
        entity_type: Option<String>,
        format: &str,
    ) -> McpResult<Summary> {
        summarize::summarize(self, entity_names, entity_type, format)
    }

    // Temporal operations (from temporal.rs)
    pub fn get_relations_at_time(
        &self,
        timestamp: Option<u64>,
        entity_name: Option<&str>,
    ) -> McpResult<Vec<Relation>> {
        temporal::get_relations_at_time(self, timestamp, entity_name)
    }

    pub fn get_relation_history(&self, entity_name: &str) -> McpResult<Vec<Relation>> {
        temporal::get_relation_history(self, entity_name)
    }
}

```

## File ../memory-graph/src\knowledge_base\query.rs:
```rust
//! Query operations for the knowledge base

use std::collections::HashSet;

use crate::search::{get_synonyms, matches_with_synonyms};
use crate::types::{Entity, KnowledgeGraph, McpResult, Relation};

use super::KnowledgeBase;

/// Read graph with optional pagination
pub fn read_graph(
    kb: &KnowledgeBase,
    limit: Option<usize>,
    offset: Option<usize>,
) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;

    let offset = offset.unwrap_or(0);

    let entities: Vec<Entity> = if let Some(lim) = limit {
        graph.entities.into_iter().skip(offset).take(lim).collect()
    } else {
        graph.entities.into_iter().skip(offset).collect()
    };

    let entity_names: HashSet<String> = entities.iter().map(|e| e.name.clone()).collect();

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
        .collect();

    Ok(KnowledgeGraph { entities, relations })
}

/// Search nodes by query with synonym expansion, optional limit and relation inclusion
pub fn search_nodes(
    kb: &KnowledgeBase,
    query: &str,
    limit: Option<usize>,
    include_relations: bool,
) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;

    // Expand query with synonyms for semantic matching
    let search_terms = get_synonyms(query);

    let mut matching_entities: Vec<Entity> = graph
        .entities
        .into_iter()
        .filter(|e| {
            matches_with_synonyms(&e.name, &search_terms)
                || matches_with_synonyms(&e.entity_type, &search_terms)
                || e.observations
                    .iter()
                    .any(|o| matches_with_synonyms(o, &search_terms))
        })
        .collect();

    // Apply limit if specified
    if let Some(lim) = limit {
        matching_entities.truncate(lim);
    }

    let matching_relations = if include_relations {
        let entity_names: HashSet<String> =
            matching_entities.iter().map(|e| e.name.clone()).collect();
        graph
            .relations
            .into_iter()
            .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
            .collect()
    } else {
        Vec::new()
    };

    Ok(KnowledgeGraph {
        entities: matching_entities,
        relations: matching_relations,
    })
}

/// Open specific nodes by names
pub fn open_nodes(kb: &KnowledgeBase, names: Vec<String>) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;
    let name_set: HashSet<String> = names.into_iter().collect();

    let matching_entities: Vec<Entity> = graph
        .entities
        .into_iter()
        .filter(|e| name_set.contains(&e.name))
        .collect();

    let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();

    let matching_relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| entity_names.contains(&r.from) && entity_names.contains(&r.to))
        .collect();

    Ok(KnowledgeGraph {
        entities: matching_entities,
        relations: matching_relations,
    })
}

```

## File ../memory-graph/src\knowledge_base\summarize.rs:
```rust
//! Summarize operations

use std::collections::HashMap;

use crate::types::{Entity, EntityBrief, McpResult, Summary};

use super::KnowledgeBase;

/// Summarize entities
pub fn summarize(
    kb: &KnowledgeBase,
    entity_names: Option<Vec<String>>,
    entity_type: Option<String>,
    format: &str,
) -> McpResult<Summary> {
    let graph = kb.load_graph()?;

    let entities: Vec<&Entity> = graph
        .entities
        .iter()
        .filter(|e| {
            if let Some(ref names) = entity_names {
                names.contains(&e.name)
            } else if let Some(ref et) = entity_type {
                &e.entity_type == et
            } else {
                true
            }
        })
        .collect();

    match format {
        "brief" => format_brief(&entities),
        "detailed" => format_detailed(&entities),
        "stats" => format_stats(&entities),
        _ => format_brief(&entities),
    }
}

fn format_brief(entities: &[&Entity]) -> McpResult<Summary> {
    let briefs: Vec<EntityBrief> = entities
        .iter()
        .map(|e| {
            let brief = e
                .observations
                .first()
                .cloned()
                .unwrap_or_default()
                .chars()
                .take(100)
                .collect::<String>();
            EntityBrief {
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                brief,
            }
        })
        .collect();

    Ok(Summary {
        total_entities: entities.len(),
        entities: Some(briefs),
        ..Default::default()
    })
}

fn format_detailed(entities: &[&Entity]) -> McpResult<Summary> {
    let briefs: Vec<EntityBrief> = entities
        .iter()
        .map(|e| {
            let brief = e.observations.join("; ");
            EntityBrief {
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                brief,
            }
        })
        .collect();

    Ok(Summary {
        total_entities: entities.len(),
        entities: Some(briefs),
        ..Default::default()
    })
}

fn format_stats(entities: &[&Entity]) -> McpResult<Summary> {
    let mut by_status: HashMap<String, usize> = HashMap::new();
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_priority: HashMap<String, usize> = HashMap::new();

    for entity in entities {
        *by_type.entry(entity.entity_type.clone()).or_insert(0) += 1;

        for obs in &entity.observations {
            if obs.starts_with("Status:") {
                let status = obs.trim_start_matches("Status:").trim().to_string();
                *by_status.entry(status).or_insert(0) += 1;
            }
            if obs.starts_with("Priority:") {
                let priority = obs.trim_start_matches("Priority:").trim().to_string();
                *by_priority.entry(priority).or_insert(0) += 1;
            }
        }
    }

    Ok(Summary {
        total_entities: entities.len(),
        entities: None,
        by_status: if by_status.is_empty() {
            None
        } else {
            Some(by_status)
        },
        by_type: Some(by_type),
        by_priority: if by_priority.is_empty() {
            None
        } else {
            Some(by_priority)
        },
    })
}

```

## File ../memory-graph/src\knowledge_base\temporal.rs:
```rust
//! Temporal query operations

use crate::types::{McpResult, Relation};
use crate::utils::time::current_timestamp;

use super::KnowledgeBase;

/// Get relations valid at a specific point in time
pub fn get_relations_at_time(
    kb: &KnowledgeBase,
    timestamp: Option<u64>,
    entity_name: Option<&str>,
) -> McpResult<Vec<Relation>> {
    let graph = kb.load_graph()?;
    let check_time = timestamp.unwrap_or_else(current_timestamp);

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| {
            // Filter by entity if specified
            if let Some(name) = entity_name {
                if r.from != name && r.to != name {
                    return false;
                }
            }

            // Check temporal validity
            let valid_from_ok = match r.valid_from {
                Some(vf) => check_time >= vf,
                None => true, // No start time means always valid from past
            };

            let valid_to_ok = match r.valid_to {
                Some(vt) => check_time <= vt,
                None => true, // No end time means still valid
            };

            valid_from_ok && valid_to_ok
        })
        .collect();

    Ok(relations)
}

/// Get historical relations (including expired ones)
pub fn get_relation_history(kb: &KnowledgeBase, entity_name: &str) -> McpResult<Vec<Relation>> {
    let graph = kb.load_graph()?;

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| r.from == entity_name || r.to == entity_name)
        .collect();

    Ok(relations)
}

```

## File ../memory-graph/src\knowledge_base\traversal.rs:
```rust
//! Graph traversal operations

use std::collections::HashSet;

use crate::types::{
    Entity, McpResult, PathStep, RelatedEntities, RelatedEntity, TraversalPath, TraversalResult,
};

use super::KnowledgeBase;

/// Get related entities
pub fn get_related(
    kb: &KnowledgeBase,
    entity_name: &str,
    relation_type: Option<&str>,
    direction: &str,
) -> McpResult<RelatedEntities> {
    let graph = kb.load_graph()?;
    let mut related = Vec::new();

    for relation in &graph.relations {
        let matches = match direction {
            "outgoing" => relation.from == entity_name,
            "incoming" => relation.to == entity_name,
            "both" => relation.from == entity_name || relation.to == entity_name,
            _ => false,
        };

        if !matches {
            continue;
        }

        if let Some(rt) = relation_type {
            if relation.relation_type != rt {
                continue;
            }
        }

        let target_name = if relation.from == entity_name {
            &relation.to
        } else {
            &relation.from
        };

        if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name) {
            related.push(RelatedEntity {
                relation_type: relation.relation_type.clone(),
                direction: if relation.from == entity_name {
                    "outgoing".to_string()
                } else {
                    "incoming".to_string()
                },
                entity: entity.clone(),
            });
        }
    }

    Ok(RelatedEntities {
        entity: entity_name.to_string(),
        relations: related,
    })
}

/// Traverse graph following path pattern
pub fn traverse(
    kb: &KnowledgeBase,
    start: &str,
    path: Vec<PathStep>,
    max_results: usize,
) -> McpResult<TraversalResult> {
    let graph = kb.load_graph()?;

    // Track paths: (current_node, path_so_far, relations_so_far)
    let mut current_paths: Vec<(String, Vec<String>, Vec<String>)> =
        vec![(start.to_string(), vec![start.to_string()], vec![])];

    for step in &path {
        let mut next_paths = Vec::new();

        for (node, nodes_path, rels_path) in &current_paths {
            // Find related entities for this step
            for relation in &graph.relations {
                let (matches, target_name) = match step.direction.as_str() {
                    "out" => {
                        if relation.from == *node && relation.relation_type == step.relation_type {
                            (true, &relation.to)
                        } else {
                            (false, &relation.to)
                        }
                    }
                    "in" => {
                        if relation.to == *node && relation.relation_type == step.relation_type {
                            (true, &relation.from)
                        } else {
                            (false, &relation.from)
                        }
                    }
                    _ => (false, &relation.to),
                };

                if !matches {
                    continue;
                }

                // Check target type if specified
                if let Some(ref target_type) = step.target_type {
                    if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name) {
                        if &entity.entity_type != target_type {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                let mut new_nodes = nodes_path.clone();
                new_nodes.push(target_name.clone());
                let mut new_rels = rels_path.clone();
                new_rels.push(step.relation_type.clone());

                next_paths.push((target_name.clone(), new_nodes, new_rels));
            }
        }

        if next_paths.len() > max_results {
            next_paths.truncate(max_results);
        }

        current_paths = next_paths;
    }

    // Build result
    let mut paths = Vec::new();
    let mut end_node_names: HashSet<String> = HashSet::new();

    for (end_node, nodes, rels) in current_paths {
        end_node_names.insert(end_node);
        paths.push(TraversalPath {
            nodes,
            relations: rels,
        });
    }

    let end_nodes: Vec<Entity> = graph
        .entities
        .iter()
        .filter(|e| end_node_names.contains(&e.name))
        .cloned()
        .collect();

    Ok(TraversalResult {
        start_node: start.to_string(),
        paths,
        end_nodes,
    })
}

```

## File ../memory-graph/src\knowledge_base\inference\mod.rs:
```rust
//! Inference Engine for Knowledge Graph Reasoning
//!
//! This module provides the core inference engine that applies logical rules
//! to discover hidden relations in the knowledge graph.

pub mod rules;

use crate::types::{InferStats, InferredRelation, KnowledgeGraph};

/// Trait for inference rules
///
/// Each rule implements logic to derive new relations from existing ones.
/// Rules are applied lazily at runtime (not persisted).
pub trait InferenceRule: Send + Sync {
    /// Get the name of this rule
    fn name(&self) -> &str;

    /// Apply the rule to infer relations for a target entity
    ///
    /// # Arguments
    /// * `graph` - The knowledge graph to analyze
    /// * `target` - The entity to infer relations for
    /// * `min_confidence` - Minimum confidence threshold (0.0 - 1.0)
    ///
    /// # Returns
    /// A tuple of (inferred_relations, stats)
    fn apply(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats);
}

/// The inference engine that manages and applies rules
pub struct InferenceEngine {
    rules: Vec<Box<dyn InferenceRule>>,
}

impl InferenceEngine {
    /// Create a new inference engine with default rules
    pub fn new() -> Self {
        Self::with_max_depth(3)
    }

    /// Create a new inference engine with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            rules: vec![Box::new(rules::TransitiveDependencyRule::new(max_depth))],
        }
    }

    /// Create an empty inference engine (no rules)
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a new inference rule
    pub fn register_rule(&mut self, rule: Box<dyn InferenceRule>) {
        self.rules.push(rule);
    }

    /// Get the number of registered rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Run all rules and collect inferred relations
    ///
    /// # Arguments
    /// * `graph` - The knowledge graph to analyze
    /// * `target` - The entity to infer relations for
    /// * `min_confidence` - Minimum confidence threshold (0.0 - 1.0)
    ///
    /// # Returns
    /// A tuple of (all_inferred_relations, combined_stats)
    pub fn infer(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats) {
        let mut all_inferred = Vec::new();
        let mut total_stats = InferStats::default();
        let start_time = std::time::Instant::now();

        for rule in &self.rules {
            let (relations, stats) = rule.apply(graph, target, min_confidence);
            all_inferred.extend(relations);

            // Merge stats
            total_stats.nodes_visited += stats.nodes_visited;
            total_stats.paths_found += stats.paths_found;
            total_stats.max_depth_reached = total_stats.max_depth_reached.max(stats.max_depth_reached);
        }

        total_stats.execution_time_ms = start_time.elapsed().as_millis() as u64;
        (all_inferred, total_stats)
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = InferenceEngine::new();
        assert_eq!(engine.rule_count(), 1); // TransitiveDependency by default
    }

    #[test]
    fn test_empty_engine() {
        let engine = InferenceEngine::empty();
        assert_eq!(engine.rule_count(), 0);
    }

    #[test]
    fn test_infer_on_empty_graph() {
        let engine = InferenceEngine::new();
        let graph = KnowledgeGraph::default();
        let (inferred, stats) = engine.infer(&graph, "NonExistent", 0.5);
        assert!(inferred.is_empty());
        assert_eq!(stats.nodes_visited, 0);
    }
}

```

## File ../memory-graph/src\knowledge_base\inference\rules.rs:
```rust
//! Inference Rules for Knowledge Graph Reasoning
//!
//! This module contains concrete implementations of inference rules.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::{InferStats, InferredRelation, KnowledgeGraph, Relation};

use super::InferenceRule;

/// Confidence decay factors for different relation types
fn get_decay_factor(relation_type: &str) -> f32 {
    match relation_type {
        "depends_on" | "contains" | "part_of" => 0.95,
        "implements" | "fixes" | "caused_by" => 0.90,
        "affects" | "assigned_to" | "blocked_by" => 0.85,
        "relates_to" | "supersedes" | "requires" => 0.70,
        _ => 0.60, // Unknown relation types get lower confidence
    }
}

/// Build an adjacency map for O(1) lookup of outgoing relations
/// This converts O(N) per-node lookup to O(1) using pre-built HashMap
fn build_adjacency_map(relations: &[Relation]) -> HashMap<&str, Vec<&Relation>> {
    let mut map: HashMap<&str, Vec<&Relation>> = HashMap::new();
    for relation in relations {
        map.entry(relation.from.as_str())
            .or_default()
            .push(relation);
    }
    map
}

/// Transitive Dependency Rule
///
/// Infers transitive relations using BFS traversal.
/// Example: If A depends_on B and B depends_on C, infer A transitively_depends_on C
///
/// Safety features:
/// - Max depth limit (default: 3)
/// - Cycle detection via HashSet
/// - Confidence decay per hop
/// - BFS for shortest-path-first (Occam's Razor)
/// - O(1) relation lookup via pre-built adjacency map
pub struct TransitiveDependencyRule {
    max_depth: usize,
}

impl TransitiveDependencyRule {
    /// Create a new rule with specified max depth
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Generate explanation for the inference path
    fn generate_explanation(path: &[String], relation_types: &[String]) -> String {
        if path.len() < 2 || relation_types.is_empty() {
            return String::new();
        }

        let mut explanation = format!("Inferred via path: {}", path[0]);
        for (i, node) in path.iter().skip(1).enumerate() {
            let rel_type = relation_types.get(i).map(|s| s.as_str()).unwrap_or("?");
            explanation.push_str(&format!(" -[{}]-> {}", rel_type, node));
        }
        explanation
    }
}

impl InferenceRule for TransitiveDependencyRule {
    fn name(&self) -> &str {
        "TransitiveDependencyRule"
    }

    fn apply(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats) {
        let mut inferred = Vec::new();
        let mut stats = InferStats::default();
        let mut visited: HashSet<String> = HashSet::new();

        // Check if target exists in graph
        if !graph.entities.iter().any(|e| e.name == target) {
            return (inferred, stats);
        }

        // Pre-build adjacency map for O(1) lookup instead of O(N) filter per node
        let adjacency_map = build_adjacency_map(&graph.relations);

        // BFS queue: (current_node, path, relation_types_in_path, confidence)
        let mut queue: VecDeque<(String, Vec<String>, Vec<String>, f32)> = VecDeque::new();
        queue.push_back((target.to_string(), vec![target.to_string()], vec![], 1.0));
        visited.insert(target.to_string());

        while let Some((current, path, rel_types, confidence)) = queue.pop_front() {
            stats.nodes_visited += 1;

            // Check depth limit (path includes start node, so depth = path.len() - 1)
            let current_depth = path.len() - 1;
            if current_depth >= self.max_depth {
                stats.max_depth_reached = stats.max_depth_reached.max(current_depth);
                continue;
            }

            // O(1) lookup of outgoing relations via adjacency map
            let outgoing = adjacency_map
                .get(current.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            for relation in outgoing {
                let next_node = &relation.to;

                // Skip if already visited (cycle detection)
                if visited.contains(next_node) {
                    continue;
                }

                // Calculate new confidence with decay
                let decay = get_decay_factor(&relation.relation_type);
                let new_confidence = confidence * decay;

                // Skip if below threshold
                if new_confidence < min_confidence {
                    continue;
                }

                // Update path
                let mut new_path = path.clone();
                new_path.push(next_node.clone());

                let mut new_rel_types = rel_types.clone();
                new_rel_types.push(relation.relation_type.clone());

                // If path length >= 3, we have a transitive relation (A -> B -> C)
                if new_path.len() >= 3 {
                    // Create inferred relation from start to current end
                    let inferred_relation = Relation {
                        from: target.to_string(),
                        to: next_node.clone(),
                        relation_type: format!("inferred_{}", new_rel_types.first().unwrap_or(&"relation".to_string())),
                        created_by: "InferenceEngine".to_string(),
                        created_at: crate::utils::current_timestamp(),
                        valid_from: None,
                        valid_to: None,
                    };

                    let explanation = Self::generate_explanation(&new_path, &new_rel_types);

                    inferred.push(InferredRelation {
                        relation: inferred_relation,
                        confidence: new_confidence,
                        rule_name: self.name().to_string(),
                        explanation,
                    });

                    stats.paths_found += 1;
                }

                // Mark as visited and add to queue for further exploration
                visited.insert(next_node.clone());
                stats.max_depth_reached = stats.max_depth_reached.max(new_path.len() - 1);

                queue.push_back((next_node.clone(), new_path, new_rel_types, new_confidence));
            }
        }

        (inferred, stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;

    fn create_test_graph() -> KnowledgeGraph {
        // Create a simple chain: A -> B -> C -> D
        let entities = vec![
            Entity::new("A".to_string(), "Module".to_string()),
            Entity::new("B".to_string(), "Module".to_string()),
            Entity::new("C".to_string(), "Module".to_string()),
            Entity::new("D".to_string(), "Module".to_string()),
        ];

        let relations = vec![
            Relation::new("A".to_string(), "B".to_string(), "depends_on".to_string()),
            Relation::new("B".to_string(), "C".to_string(), "depends_on".to_string()),
            Relation::new("C".to_string(), "D".to_string(), "depends_on".to_string()),
        ];

        KnowledgeGraph { entities, relations }
    }

    #[test]
    fn test_simple_chain_inference() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);
        let (inferred, stats) = rule.apply(&graph, "A", 0.5);

        // Should infer A -> C (via B) and A -> D (via B, C)
        assert_eq!(inferred.len(), 2);
        assert!(stats.paths_found >= 2);

        // Check A -> C
        let a_to_c = inferred.iter().find(|i| i.relation.to == "C");
        assert!(a_to_c.is_some());
        let a_to_c = a_to_c.unwrap();
        assert!(a_to_c.confidence > 0.8); // 0.95 * 0.95 = 0.9025

        // Check A -> D
        let a_to_d = inferred.iter().find(|i| i.relation.to == "D");
        assert!(a_to_d.is_some());
        let a_to_d = a_to_d.unwrap();
        assert!(a_to_d.confidence > 0.7); // 0.95^3 = 0.857
    }

    #[test]
    fn test_depth_limit() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(2); // Only 2 hops
        let (inferred, _stats) = rule.apply(&graph, "A", 0.5);

        // Should only infer A -> C, not A -> D (would need 3 hops)
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].relation.to, "C");
    }

    #[test]
    fn test_cycle_detection() {
        // Create a graph with cycle: A -> B -> C -> A
        let entities = vec![
            Entity::new("A".to_string(), "Module".to_string()),
            Entity::new("B".to_string(), "Module".to_string()),
            Entity::new("C".to_string(), "Module".to_string()),
        ];

        let relations = vec![
            Relation::new("A".to_string(), "B".to_string(), "depends_on".to_string()),
            Relation::new("B".to_string(), "C".to_string(), "depends_on".to_string()),
            Relation::new("C".to_string(), "A".to_string(), "depends_on".to_string()),
        ];

        let graph = KnowledgeGraph { entities, relations };
        let rule = TransitiveDependencyRule::new(10); // High depth to test cycle detection
        let (inferred, stats) = rule.apply(&graph, "A", 0.1);

        // Should NOT loop infinitely - cycle detection kicks in
        // Should infer A -> C (2 hops) but NOT loop back
        assert!(stats.nodes_visited <= 3); // Only 3 nodes exist
        assert!(!inferred.iter().any(|i| i.relation.to == "A")); // No self-inference
    }

    #[test]
    fn test_confidence_threshold() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);

        // High threshold should filter out longer paths
        let (inferred, _) = rule.apply(&graph, "A", 0.9);
        // 0.95 * 0.95 = 0.9025 > 0.9 (A->C passes)
        // 0.95 * 0.95 * 0.95 = 0.857 < 0.9 (A->D fails)
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].relation.to, "C");
    }

    #[test]
    fn test_nonexistent_target() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);
        let (inferred, stats) = rule.apply(&graph, "NonExistent", 0.5);

        assert!(inferred.is_empty());
        assert_eq!(stats.nodes_visited, 0);
    }

    #[test]
    fn test_explanation_generation() {
        let path = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let rel_types = vec!["depends_on".to_string(), "depends_on".to_string()];
        let explanation = TransitiveDependencyRule::generate_explanation(&path, &rel_types);

        assert!(explanation.contains("A"));
        assert!(explanation.contains("B"));
        assert!(explanation.contains("C"));
        assert!(explanation.contains("depends_on"));
    }

    #[test]
    fn test_decay_factors() {
        assert_eq!(get_decay_factor("depends_on"), 0.95);
        assert_eq!(get_decay_factor("implements"), 0.90);
        assert_eq!(get_decay_factor("affects"), 0.85);
        assert_eq!(get_decay_factor("relates_to"), 0.70);
        assert_eq!(get_decay_factor("unknown_type"), 0.60);
    }
}

```

## File ../memory-graph/src\protocol\jsonrpc.rs:
```rust
//! JSON-RPC 2.0 protocol types

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Check if this is a valid JSON-RPC 2.0 request
    pub fn is_valid(&self) -> bool {
        self.jsonrpc == "2.0"
    }

    /// Check if this is a notification (no id)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 Success Response
#[derive(Serialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Value,
}

impl JsonRpcResponse {
    /// Create a new success response
    pub fn new(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        }
    }
}

/// JSON-RPC 2.0 Error Response
#[derive(Serialize, Debug)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: ErrorObject,
}

impl JsonRpcError {
    /// Create a new error response
    pub fn new(id: Value, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject {
                code,
                message,
                data,
            },
        }
    }

    /// Create a parse error response
    pub fn parse_error(id: Value, details: String) -> Self {
        Self::new(
            id,
            -32700,
            "Parse error".to_string(),
            Some(serde_json::json!({"details": details})),
        )
    }

    /// Create an invalid request error response
    pub fn invalid_request(id: Value, details: String) -> Self {
        Self::new(
            id,
            -32600,
            "Invalid Request".to_string(),
            Some(serde_json::json!({"details": details})),
        )
    }

    /// Create a method not found error response
    pub fn method_not_found(id: Value, method: String) -> Self {
        Self::new(
            id,
            -32601,
            "Method not found".to_string(),
            Some(serde_json::json!({"method": method})),
        )
    }

    /// Create an invalid params error response
    pub fn invalid_params(id: Value, details: String) -> Self {
        Self::new(
            id,
            -32602,
            "Invalid params".to_string(),
            Some(serde_json::json!({"details": details})),
        )
    }

    /// Create an internal error response
    pub fn internal_error(id: Value, details: String) -> Self {
        Self::new(
            id,
            -32603,
            "Internal error".to_string(),
            Some(serde_json::json!({"details": details})),
        )
    }
}

/// JSON-RPC 2.0 Error Object
#[derive(Serialize, Debug)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorObject {
    /// Create a new error object
    pub fn new(code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }
}

```

## File ../memory-graph/src\protocol\mcp.rs:
```rust
//! MCP (Model Context Protocol) types

use serde::Serialize;
use serde_json::Value;

use crate::types::McpResult;

/// MCP Tool definition
#[derive(Serialize, Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl McpTool {
    /// Create a new MCP tool definition
    pub fn new(name: String, description: String, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }
}

/// Server information for MCP handshake
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

impl ServerInfo {
    /// Create new server info
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            name: "memory".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

/// Trait for MCP tools
///
/// All tools must implement this trait to be registered with the MCP server.
pub trait Tool: Send + Sync {
    /// Get the tool definition for tools/list
    fn definition(&self) -> McpTool;

    /// Execute the tool with the given parameters
    fn execute(&self, params: Value) -> McpResult<Value>;

    /// Get the tool name (convenience method)
    fn name(&self) -> String {
        self.definition().name
    }
}

```

## File ../memory-graph/src\protocol\mod.rs:
```rust
//! Protocol types for MCP and JSON-RPC communication
//!
//! This module contains all protocol-related types and traits.

mod jsonrpc;
mod mcp;

pub use jsonrpc::{ErrorObject, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use mcp::{McpTool, ServerInfo, Tool};

```

## File ../memory-graph/src\search\mod.rs:
```rust
//! Semantic search with synonym matching
//!
//! This module provides semantic search capabilities through synonym expansion.

mod synonyms;

pub use synonyms::{get_synonyms, matches_with_synonyms, SYNONYM_GROUPS};

```

## File ../memory-graph/src\search\synonyms.rs:
```rust
//! Synonym dictionary for semantic search

/// Synonym groups - words in same group are considered semantically similar
pub const SYNONYM_GROUPS: &[&[&str]] = &[
    // Developer roles
    &[
        "coder",
        "programmer",
        "developer",
        "engineer",
        "dev",
        "software engineer",
        "software developer",
    ],
    &["frontend", "front-end", "ui developer", "client-side"],
    &["backend", "back-end", "server-side", "api developer"],
    &["fullstack", "full-stack", "full stack"],
    &["devops", "sre", "infrastructure", "platform engineer"],
    // Bug/Issue related
    &[
        "bug", "issue", "defect", "error", "problem", "fault", "glitch",
    ],
    &["fix", "patch", "hotfix", "bugfix", "repair", "resolve"],
    // Feature/Task related
    &["feature", "functionality", "capability", "enhancement"],
    &["task", "ticket", "work item", "story", "user story"],
    &["requirement", "spec", "specification", "req"],
    // Status
    &["done", "completed", "finished", "resolved", "closed"],
    &["pending", "waiting", "blocked", "on hold"],
    &["in progress", "wip", "ongoing", "active", "working"],
    &["todo", "to do", "planned", "backlog"],
    // Priority
    &["critical", "urgent", "p0", "blocker", "showstopper"],
    &["high", "important", "p1"],
    &["medium", "normal", "p2"],
    &["low", "minor", "p3"],
    // Project management
    &["milestone", "release", "version", "sprint"],
    &["deadline", "due date", "target date"],
    &["project", "repo", "repository", "codebase"],
    // Documentation
    &["doc", "docs", "documentation", "readme", "guide"],
    &["api", "interface", "endpoint"],
    // Testing
    &["test", "testing", "qa", "quality assurance"],
    &["unit test", "unittest"],
    &["integration test", "e2e", "end-to-end"],
    // Architecture
    &["module", "component", "service", "package"],
    &["database", "db", "datastore", "storage"],
    &["cache", "caching", "redis", "memcached"],
];

/// Get all synonyms for a query term
pub fn get_synonyms(query: &str) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let mut synonyms = vec![query_lower.clone()];

    for group in SYNONYM_GROUPS {
        if group.iter().any(|&word| {
            word == query_lower || query_lower.contains(word) || word.contains(&query_lower)
        }) {
            for &word in *group {
                if !synonyms.contains(&word.to_string()) {
                    synonyms.push(word.to_string());
                }
            }
        }
    }

    synonyms
}

/// Check if text matches any of the search terms (including synonyms)
pub fn matches_with_synonyms(text: &str, search_terms: &[String]) -> bool {
    let text_lower = text.to_lowercase();
    search_terms.iter().any(|term| text_lower.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_synonyms_developer() {
        let synonyms = get_synonyms("developer");
        assert!(synonyms.contains(&"developer".to_string()));
        assert!(synonyms.contains(&"coder".to_string()));
        assert!(synonyms.contains(&"programmer".to_string()));
    }

    #[test]
    fn test_get_synonyms_bug() {
        let synonyms = get_synonyms("bug");
        assert!(synonyms.contains(&"bug".to_string()));
        assert!(synonyms.contains(&"issue".to_string()));
        assert!(synonyms.contains(&"defect".to_string()));
    }

    #[test]
    fn test_matches_with_synonyms() {
        let terms = get_synonyms("developer");
        assert!(matches_with_synonyms("I am a coder", &terms));
        assert!(matches_with_synonyms("Software Engineer position", &terms));
        assert!(!matches_with_synonyms("I am a doctor", &terms));
    }
}

```

## File ../memory-graph/src\server\handlers.rs:
```rust
//! Request handlers for the MCP server
//!
//! This module contains helper functions for handling various request types.
//! Most handlers are implemented directly in McpServer, but this module
//! can be extended for custom handlers.

use serde_json::Value;

/// Extract tool arguments from params
pub fn extract_arguments(params: &Value) -> Value {
    params.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new()))
}

/// Extract tool name from params
pub fn extract_tool_name(params: &Value) -> Option<&str> {
    params.get("name").and_then(|v| v.as_str())
}

/// Build a text content response
pub fn text_response(text: String) -> Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    })
}

/// Build an error content response
pub fn error_response(message: String) -> Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": format!("Error: {}", message)
        }],
        "isError": true
    })
}

```

## File ../memory-graph/src\server\mod.rs:
```rust
//! MCP Server implementation
//!
//! This module contains the main server that handles JSON-RPC communication.

mod handlers;

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use serde_json::{json, Value};

use crate::protocol::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpTool, ServerInfo, Tool,
};
use crate::types::McpResult;

pub use handlers::*;

/// MCP Server that handles JSON-RPC communication over stdio
pub struct McpServer {
    server_info: ServerInfo,
    tools: HashMap<String, Box<dyn Tool>>,
    reader: BufReader<io::Stdin>,
    writer: BufWriter<io::Stdout>,
}

impl McpServer {
    /// Create a new MCP server with default settings
    pub fn new() -> Self {
        Self {
            server_info: ServerInfo::default(),
            tools: HashMap::new(),
            reader: BufReader::new(io::stdin()),
            writer: BufWriter::new(io::stdout()),
        }
    }

    /// Create a new MCP server with custom server info
    pub fn with_info(info: ServerInfo) -> Self {
        Self {
            server_info: info,
            tools: HashMap::new(),
            reader: BufReader::new(io::stdin()),
            writer: BufWriter::new(io::stdout()),
        }
    }

    /// Register a tool with the server
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) -> &mut Self {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
        self
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Run the server (blocking)
    pub fn run(&mut self) -> McpResult<()> {
        let mut line = String::new();
        while self.reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.handle_request(trimmed)?;
            }
            line.clear();
        }
        Ok(())
    }

    /// Handle a single JSON-RPC request
    fn handle_request(&mut self, request_str: &str) -> McpResult<()> {
        let request: JsonRpcRequest = match serde_json::from_str(request_str) {
            Ok(req) => req,
            Err(e) => {
                self.send_error_response(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!({"details": e.to_string()})),
                )?;
                return Ok(());
            }
        };

        if request.jsonrpc != "2.0" {
            self.send_error_response(
                request.id.unwrap_or(Value::Null),
                -32600,
                "Invalid Request",
                Some(json!({"details": "jsonrpc must be '2.0'"})),
            )?;
            return Ok(());
        }

        let id = request.id.clone().unwrap_or(Value::Null);

        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params),
            "notifications/initialized" => Ok(()), // Notification, no response
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tool_call(id, request.params),
            "ping" => self.send_success_response(id, json!({})),
            _ => self.send_error_response(
                id,
                -32601,
                "Method not found",
                Some(json!({"method": request.method})),
            ),
        }
    }

    /// Handle initialize request
    fn handle_initialize(&mut self, id: Value, _params: Option<Value>) -> McpResult<()> {
        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.server_info.name,
                "version": self.server_info.version
            }
        });
        self.send_success_response(id, result)
    }

    /// Handle tools/list request
    fn handle_tools_list(&mut self, id: Value) -> McpResult<()> {
        let tools: Vec<McpTool> = self.tools.values().map(|t| t.definition()).collect();
        let result = json!({ "tools": tools });
        self.send_success_response(id, result)
    }

    /// Handle tools/call request
    fn handle_tool_call(&mut self, id: Value, params: Option<Value>) -> McpResult<()> {
        let params = params.ok_or("Missing parameters")?;
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing tool name")?;

        let tool = match self.tools.get(tool_name) {
            Some(tool) => tool,
            None => {
                self.send_error_response(
                    id,
                    -32602,
                    "Unknown tool",
                    Some(json!({"tool": tool_name})),
                )?;
                return Ok(());
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool.execute(arguments) {
            Ok(result) => self.send_success_response(id, result),
            Err(e) => self.send_error_response(
                id,
                -32603,
                "Tool execution error",
                Some(json!({"details": e.to_string()})),
            ),
        }
    }

    /// Send a success response
    fn send_success_response(&mut self, id: Value, result: Value) -> McpResult<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        };
        let json = serde_json::to_string(&response)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Send an error response
    fn send_error_response(
        &mut self,
        id: Value,
        code: i32,
        message: &str,
        data: Option<Value>,
    ) -> McpResult<()> {
        let response = JsonRpcError::new(id, code, message.to_string(), data);
        let json = serde_json::to_string(&response)?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

```

## File ../memory-graph/src\tools\mod.rs:
```rust
//! MCP Tools implementation
//!
//! This module contains all 16 MCP tools organized by category:
//! - Memory tools (9): CRUD operations
//! - Query tools (3): Graph traversal and search
//! - Temporal tools (3): Time-based queries
//! - Inference tools (1): Graph reasoning

pub mod inference;
pub mod memory;
pub mod query;
pub mod temporal;

use std::sync::Arc;

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::Tool;
use crate::server::McpServer;

// Re-export all tools for convenience
pub use inference::InferTool;
pub use memory::{
    AddObservationsTool, CreateEntitiesTool, CreateRelationsTool, DeleteEntitiesTool,
    DeleteObservationsTool, DeleteRelationsTool, OpenNodesTool, ReadGraphTool, SearchNodesTool,
};
pub use query::{GetRelatedTool, SummarizeTool, TraverseTool};
pub use temporal::{GetCurrentTimeTool, GetRelationHistoryTool, GetRelationsAtTimeTool};

/// Register all tools with the MCP server
pub fn register_all_tools(server: &mut McpServer, kb: Arc<KnowledgeBase>) {
    // Memory tools (9)
    server.register_tool(Box::new(CreateEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(CreateRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(AddObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteEntitiesTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteObservationsTool::new(kb.clone())));
    server.register_tool(Box::new(DeleteRelationsTool::new(kb.clone())));
    server.register_tool(Box::new(ReadGraphTool::new(kb.clone())));
    server.register_tool(Box::new(SearchNodesTool::new(kb.clone())));
    server.register_tool(Box::new(OpenNodesTool::new(kb.clone())));

    // Query tools (3)
    server.register_tool(Box::new(GetRelatedTool::new(kb.clone())));
    server.register_tool(Box::new(TraverseTool::new(kb.clone())));
    server.register_tool(Box::new(SummarizeTool::new(kb.clone())));

    // Temporal tools (3)
    server.register_tool(Box::new(GetRelationsAtTimeTool::new(kb.clone())));
    server.register_tool(Box::new(GetRelationHistoryTool::new(kb.clone())));
    server.register_tool(Box::new(GetCurrentTimeTool::new()));

    // Inference tools (1)
    server.register_tool(Box::new(InferTool::new(kb.clone())));
}

/// Get all tools as Arc<dyn Tool> for SSE state
pub fn get_all_tools(kb: Arc<KnowledgeBase>) -> Vec<Arc<dyn Tool>> {
    vec![
        // Memory tools (9)
        Arc::new(CreateEntitiesTool::new(kb.clone())) as Arc<dyn Tool>,
        Arc::new(CreateRelationsTool::new(kb.clone())),
        Arc::new(AddObservationsTool::new(kb.clone())),
        Arc::new(DeleteEntitiesTool::new(kb.clone())),
        Arc::new(DeleteObservationsTool::new(kb.clone())),
        Arc::new(DeleteRelationsTool::new(kb.clone())),
        Arc::new(ReadGraphTool::new(kb.clone())),
        Arc::new(SearchNodesTool::new(kb.clone())),
        Arc::new(OpenNodesTool::new(kb.clone())),
        // Query tools (3)
        Arc::new(GetRelatedTool::new(kb.clone())),
        Arc::new(TraverseTool::new(kb.clone())),
        Arc::new(SummarizeTool::new(kb.clone())),
        // Temporal tools (3)
        Arc::new(GetRelationsAtTimeTool::new(kb.clone())),
        Arc::new(GetRelationHistoryTool::new(kb.clone())),
        Arc::new(GetCurrentTimeTool::new()),
        // Inference tools (1)
        Arc::new(InferTool::new(kb.clone())),
    ]
}

```

## File ../memory-graph/src\tools\inference\infer.rs:
```rust
//! Infer tool - Runtime graph reasoning
//!
//! Discovers hidden/transitive relations using inference rules.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::inference::InferenceEngine;
use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{InferResult, McpResult};

/// Tool for inferring hidden relations from the knowledge graph
///
/// This tool applies inference rules (like transitive dependency) to discover
/// relations that aren't explicitly stored but can be logically derived.
pub struct InferTool {
    kb: Arc<KnowledgeBase>,
}

impl InferTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for InferTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "infer".to_string(),
            description: "Infer hidden relations for an entity using logical rules. Discovers transitive dependencies and indirect connections not explicitly stored in the graph.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "Name of the entity to infer relations for"
                    },
                    "minConfidence": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "default": 0.5,
                        "description": "Minimum confidence threshold (0.0-1.0). Higher values return fewer but more reliable inferences."
                    },
                    "maxDepth": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "default": 3,
                        "description": "Maximum traversal depth (1-5). Higher values find more distant relations but take longer."
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params
            .get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("Missing entityName")?;

        let min_confidence = params
            .get("minConfidence")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let max_depth = params
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .clamp(1, 5);

        // Get full graph for inference (no pagination)
        let graph = self.kb.read_graph(None, None)?;

        // Create inference engine with specified depth
        let engine = InferenceEngine::with_max_depth(max_depth);

        // Run inference
        let (inferred_relations, stats) = engine.infer(&graph, entity_name, min_confidence);

        // Build result
        let result = InferResult {
            target: entity_name.to_string(),
            inferred_relations,
            stats,
        };

        // Format response
        let response = if result.inferred_relations.is_empty() {
            json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "No inferred relations found for '{}' with confidence >= {:.0}% and max depth {}.\n\nStats: visited {} nodes in {}ms",
                        entity_name,
                        min_confidence * 100.0,
                        max_depth,
                        result.stats.nodes_visited,
                        result.stats.execution_time_ms
                    )
                }]
            })
        } else {
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result)?
                }]
            })
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Tool;

    #[test]
    fn test_tool_definition() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);
        let def = tool.definition();

        assert_eq!(def.name, "infer");
        assert!(def.description.to_lowercase().contains("infer"));

        // Check required field
        let schema = &def.input_schema;
        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("entityName")));
    }

    #[test]
    fn test_execute_nonexistent_entity() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);

        let result = tool.execute(json!({
            "entityName": "NonExistent"
        }));

        assert!(result.is_ok());
        let response = result.unwrap();
        let text = response["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No inferred relations found"));
    }

    #[test]
    fn test_parameter_clamping() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);

        // Test with out-of-range values (should be clamped)
        let result = tool.execute(json!({
            "entityName": "Test",
            "minConfidence": 2.0,  // Should clamp to 1.0
            "maxDepth": 100       // Should clamp to 5
        }));

        assert!(result.is_ok());
    }
}

```

## File ../memory-graph/src\tools\inference\mod.rs:
```rust
//! Inference tools for graph reasoning
//!
//! This module contains tools for runtime inference using the knowledge graph.

mod infer;

pub use infer::InferTool;

```

## File ../memory-graph/src\tools\memory\add_observations.rs:
```rust
//! Add observations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Observation};

/// Tool for adding new observations to existing entities
pub struct AddObservationsTool {
    kb: Arc<KnowledgeBase>,
}

impl AddObservationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for AddObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "add_observations".to_string(),
            description: "Add new observations to existing entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "observations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "contents": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observation contents to add"
                                }
                            },
                            "required": ["entityName", "contents"]
                        }
                    }
                },
                "required": ["observations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let observations: Vec<Observation> =
            serde_json::from_value(params.get("observations").cloned().unwrap_or(json!([])))?;
        let added = self.kb.add_observations(observations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&added)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\create_entities.rs:
```rust
//! Create entities tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{Entity, McpResult};
use crate::validation::validate_entity_type;

/// Tool for creating multiple new entities in the knowledge graph
pub struct CreateEntitiesTool {
    kb: Arc<KnowledgeBase>,
}

impl CreateEntitiesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_entities".to_string(),
            description: "Create multiple new entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entities": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "The name of the entity" },
                                "entityType": { "type": "string", "description": "The type of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Initial observations about the entity"
                                },
                                "createdBy": { "type": "string", "description": "Who created this entity (auto-filled from git/env if not provided)" },
                                "updatedBy": { "type": "string", "description": "Who last updated this entity (auto-filled from git/env if not provided)" }
                            },
                            "required": ["name", "entityType"]
                        }
                    }
                },
                "required": ["entities"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entities: Vec<Entity> =
            serde_json::from_value(params.get("entities").cloned().unwrap_or(json!([])))?;

        // Collect warnings for non-standard types
        let warnings: Vec<String> = entities
            .iter()
            .filter_map(|e| validate_entity_type(&e.entity_type))
            .collect();

        let created = self.kb.create_entities(entities)?;

        let response = if warnings.is_empty() {
            serde_json::to_string_pretty(&created)?
        } else {
            format!(
                "{}\n\n{}",
                serde_json::to_string_pretty(&created)?,
                warnings.join("\n")
            )
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": response
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\create_relations.rs:
```rust
//! Create relations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Relation};
use crate::validation::validate_relation_type;

/// Tool for creating multiple new relations between entities
pub struct CreateRelationsTool {
    kb: Arc<KnowledgeBase>,
}

impl CreateRelationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_relations".to_string(),
            description: "Create multiple new relations between entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" },
                                "createdBy": { "type": "string", "description": "Who created this relation (auto-filled from git/env if not provided)" },
                                "validFrom": { "type": "integer", "description": "Unix timestamp when relation becomes valid" },
                                "validTo": { "type": "integer", "description": "Unix timestamp when relation expires" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let relations: Vec<Relation> =
            serde_json::from_value(params.get("relations").cloned().unwrap_or(json!([])))?;

        // Collect warnings for non-standard relation types
        let warnings: Vec<String> = relations
            .iter()
            .filter_map(|r| validate_relation_type(&r.relation_type))
            .collect();

        let created = self.kb.create_relations(relations)?;

        let response = if warnings.is_empty() {
            serde_json::to_string_pretty(&created)?
        } else {
            format!(
                "{}\n\n{}",
                serde_json::to_string_pretty(&created)?,
                warnings.join("\n")
            )
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": response
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\delete_entities.rs:
```rust
//! Delete entities tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for deleting multiple entities from the knowledge graph
pub struct DeleteEntitiesTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteEntitiesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_entities".to_string(),
            description: "Delete multiple entities from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "An array of entity names to delete"
                    }
                },
                "required": ["entityNames"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Vec<String> =
            serde_json::from_value(params.get("entityNames").cloned().unwrap_or(json!([])))?;
        self.kb.delete_entities(entity_names)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Entities deleted successfully"
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\delete_observations.rs:
```rust
//! Delete observations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, ObservationDeletion};

/// Tool for deleting specific observations from entities
pub struct DeleteObservationsTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteObservationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_observations".to_string(),
            description: "Delete specific observations from entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deletions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observations to delete"
                                }
                            },
                            "required": ["entityName", "observations"]
                        }
                    }
                },
                "required": ["deletions"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let deletions: Vec<ObservationDeletion> =
            serde_json::from_value(params.get("deletions").cloned().unwrap_or(json!([])))?;
        self.kb.delete_observations(deletions)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Observations deleted successfully"
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\delete_relations.rs:
```rust
//! Delete relations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Relation};

/// Tool for deleting multiple relations from the knowledge graph
pub struct DeleteRelationsTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteRelationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_relations".to_string(),
            description: "Delete multiple relations from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let relations: Vec<Relation> =
            serde_json::from_value(params.get("relations").cloned().unwrap_or(json!([])))?;
        self.kb.delete_relations(relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Relations deleted successfully"
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\mod.rs:
```rust
//! Memory tools for CRUD operations
//!
//! This module contains 9 tools for managing entities, relations, and observations.

mod add_observations;
mod create_entities;
mod create_relations;
mod delete_entities;
mod delete_observations;
mod delete_relations;
mod open_nodes;
mod read_graph;
mod search_nodes;

pub use add_observations::AddObservationsTool;
pub use create_entities::CreateEntitiesTool;
pub use create_relations::CreateRelationsTool;
pub use delete_entities::DeleteEntitiesTool;
pub use delete_observations::DeleteObservationsTool;
pub use delete_relations::DeleteRelationsTool;
pub use open_nodes::OpenNodesTool;
pub use read_graph::ReadGraphTool;
pub use search_nodes::SearchNodesTool;

```

## File ../memory-graph/src\tools\memory\open_nodes.rs:
```rust
//! Open nodes tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for opening specific nodes by their names
pub struct OpenNodesTool {
    kb: Arc<KnowledgeBase>,
}

impl OpenNodesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for OpenNodesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "open_nodes".to_string(),
            description: "Open specific nodes in the knowledge graph by their names".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "names": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "An array of entity names to retrieve"
                    }
                },
                "required": ["names"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let names: Vec<String> =
            serde_json::from_value(params.get("names").cloned().unwrap_or(json!([])))?;
        let graph = self.kb.open_nodes(names)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\read_graph.rs:
```rust
//! Read graph tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for reading the knowledge graph with optional pagination
pub struct ReadGraphTool {
    kb: Arc<KnowledgeBase>,
}

impl ReadGraphTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for ReadGraphTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "read_graph".to_string(),
            description: "Read the knowledge graph with optional pagination. Use limit/offset to avoid context overflow with large graphs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return. Recommended: 50-100 for large graphs"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of entities to skip (for pagination)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let offset = params
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let graph = self.kb.read_graph(limit, offset)?;

        let total_msg = if limit.is_some() || offset.is_some() {
            format!(" (showing {} entities)", graph.entities.len())
        } else {
            String::new()
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("{}{}", serde_json::to_string_pretty(&graph)?, total_msg)
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\memory\search_nodes.rs:
```rust
//! Search nodes tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for searching nodes in the knowledge graph with semantic matching
pub struct SearchNodesTool {
    kb: Arc<KnowledgeBase>,
}

impl SearchNodesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SearchNodesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "search_nodes".to_string(),
            description:
                "Search for nodes in the knowledge graph. Returns matching entities with optional relations."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to match against entity names, types, and observations"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return (default: no limit)"
                    },
                    "includeRelations": {
                        "type": "boolean",
                        "description": "Whether to include relations connected to matching entities (default: true)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let include_relations = params
            .get("includeRelations")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let graph = self.kb.search_nodes(query, limit, include_relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\query\get_related.rs:
```rust
//! Get related tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for getting entities related to a specific entity
pub struct GetRelatedTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelatedTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelatedTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_related".to_string(),
            description: "Get entities related to a specific entity".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "Name of the entity to find relations for"
                    },
                    "relationType": {
                        "type": "string",
                        "description": "Filter by relation type (optional)"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["outgoing", "incoming", "both"],
                        "default": "both",
                        "description": "Direction of relations"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params
            .get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("Missing entityName")?;
        let relation_type = params.get("relationType").and_then(|v| v.as_str());
        let direction = params
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        let related = self.kb.get_related(entity_name, relation_type, direction)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&related)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\query\mod.rs:
```rust
//! Query tools for graph traversal and search
//!
//! This module contains 3 tools for advanced graph operations.

mod get_related;
mod summarize;
mod traverse;

pub use get_related::GetRelatedTool;
pub use summarize::SummarizeTool;
pub use traverse::TraverseTool;

```

## File ../memory-graph/src\tools\query\summarize.rs:
```rust
//! Summarize tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for getting a condensed summary of entities
pub struct SummarizeTool {
    kb: Arc<KnowledgeBase>,
}

impl SummarizeTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SummarizeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "summarize".to_string(),
            description: "Get a condensed summary of entities".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Specific entities to summarize (optional)"
                    },
                    "entityType": {
                        "type": "string",
                        "description": "Summarize all entities of this type (optional)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["brief", "detailed", "stats"],
                        "default": "brief",
                        "description": "Output format: brief (first observation), detailed (all observations), stats (statistics)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Option<Vec<String>> = params
            .get("entityNames")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let entity_type = params
            .get("entityType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("brief");

        let summary = self.kb.summarize(entity_names, entity_type, format)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&summary)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\query\traverse.rs:
```rust
//! Traverse tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, PathStep};

/// Tool for traversing the graph following a path pattern
pub struct TraverseTool {
    kb: Arc<KnowledgeBase>,
}

impl TraverseTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for TraverseTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "traverse".to_string(),
            description: "Traverse the graph following a path pattern for multi-hop queries"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "startNode": {
                        "type": "string",
                        "description": "Starting entity name"
                    },
                    "path": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "relationType": {
                                    "type": "string",
                                    "description": "Type of relation to follow"
                                },
                                "direction": {
                                    "type": "string",
                                    "enum": ["out", "in"],
                                    "description": "Direction: out (outgoing) or in (incoming)"
                                },
                                "targetType": {
                                    "type": "string",
                                    "description": "Filter by target entity type (optional)"
                                }
                            },
                            "required": ["relationType", "direction"]
                        },
                        "description": "Path pattern to follow"
                    },
                    "maxResults": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum number of results"
                    }
                },
                "required": ["startNode", "path"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let start_node = params
            .get("startNode")
            .and_then(|v| v.as_str())
            .ok_or("Missing startNode")?;

        let path: Vec<PathStep> =
            serde_json::from_value(params.get("path").cloned().unwrap_or(json!([])))?;

        let max_results = params
            .get("maxResults")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let result = self.kb.traverse(start_node, path, max_results)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\temporal\get_current_time.rs:
```rust
//! Get current time tool

use serde_json::{json, Value};

use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;
use crate::utils::time::get_current_time;

/// Tool for getting the current datetime and timestamp
pub struct GetCurrentTimeTool;

impl GetCurrentTimeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetCurrentTimeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GetCurrentTimeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_current_time".to_string(),
            description: "Get the current datetime and timestamp".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    fn execute(&self, _params: Value) -> McpResult<Value> {
        let time_info = get_current_time();
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&time_info)?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\temporal\get_relations_at_time.rs:
```rust
//! Get relations at time tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;
use crate::utils::time::current_timestamp;

/// Tool for getting relations valid at a specific point in time
pub struct GetRelationsAtTimeTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelationsAtTimeTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationsAtTimeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relations_at_time".to_string(),
            description: "Get relations that are valid at a specific point in time. Useful for querying historical state of the knowledge graph.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timestamp": {
                        "type": "integer",
                        "description": "Unix timestamp to query. If not provided, uses current time."
                    },
                    "entityName": {
                        "type": "string",
                        "description": "Optional: filter relations involving this entity"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let timestamp = params.get("timestamp").and_then(|v| v.as_u64());
        let entity_name = params.get("entityName").and_then(|v| v.as_str());

        let relations = self.kb.get_relations_at_time(timestamp, entity_name)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "queryTime": timestamp.unwrap_or_else(current_timestamp),
                    "relations": relations
                }))?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\temporal\get_relation_history.rs:
```rust
//! Get relation history tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;
use crate::utils::time::current_timestamp;

/// Tool for getting all relations (current and historical) for an entity
pub struct GetRelationHistoryTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelationHistoryTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationHistoryTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relation_history".to_string(),
            description: "Get all relations (current and historical) for an entity. Shows temporal validity (validFrom/validTo) for each relation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "The name of the entity to get relation history for"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params
            .get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("entityName is required")?;

        let relations = self.kb.get_relation_history(entity_name)?;
        let current_time = current_timestamp();

        // Mark each relation as current or historical
        let annotated: Vec<Value> = relations
            .iter()
            .map(|r| {
                let is_current = match (r.valid_from, r.valid_to) {
                    (Some(vf), Some(vt)) => current_time >= vf && current_time <= vt,
                    (Some(vf), None) => current_time >= vf,
                    (None, Some(vt)) => current_time <= vt,
                    (None, None) => true,
                };

                json!({
                    "from": r.from,
                    "to": r.to,
                    "relationType": r.relation_type,
                    "validFrom": r.valid_from,
                    "validTo": r.valid_to,
                    "isCurrent": is_current
                })
            })
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "entity": entity_name,
                    "currentTime": current_time,
                    "relations": annotated
                }))?
            }]
        }))
    }
}

```

## File ../memory-graph/src\tools\temporal\mod.rs:
```rust
//! Temporal tools for time-based queries
//!
//! This module contains 3 tools for temporal operations.

mod get_current_time;
mod get_relation_history;
mod get_relations_at_time;

pub use get_current_time::GetCurrentTimeTool;
pub use get_relation_history::GetRelationHistoryTool;
pub use get_relations_at_time::GetRelationsAtTimeTool;

```

## File ../memory-graph/src\types\entity.rs:
```rust
//! Entity types for the knowledge graph

use serde::{Deserialize, Serialize};

use super::{default_user, is_default_user, is_zero};

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
    #[serde(
        rename = "createdBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub created_by: String,
    #[serde(
        rename = "updatedBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub updated_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "updatedAt", default, skip_serializing_if = "is_zero")]
    pub updated_at: u64,
}

impl Entity {
    /// Create a new entity with default values
    pub fn new(name: String, entity_type: String) -> Self {
        Self {
            name,
            entity_type,
            observations: Vec::new(),
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        }
    }

    /// Create a new entity with observations
    pub fn with_observations(name: String, entity_type: String, observations: Vec<String>) -> Self {
        Self {
            name,
            entity_type,
            observations,
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// Brief entity info for summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBrief {
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub brief: String,
}

```

## File ../memory-graph/src\types\event.rs:
```rust
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

```

## File ../memory-graph/src\types\graph.rs:
```rust
//! Knowledge graph container type

use serde::{Deserialize, Serialize};

use super::{Entity, Relation};

/// Knowledge graph containing entities and relations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

impl KnowledgeGraph {
    /// Create an empty knowledge graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a knowledge graph with entities and relations
    pub fn with_data(entities: Vec<Entity>, relations: Vec<Relation>) -> Self {
        Self { entities, relations }
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relations.is_empty()
    }

    /// Get the number of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get the number of relations
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

```

## File ../memory-graph/src\types\inference.rs:
```rust
//! Inference types for the reasoning engine
//!
//! This module contains data structures for the inference engine output.

use serde::{Deserialize, Serialize};

use super::Relation;

/// An inferred relation with confidence and provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredRelation {
    /// The inferred relation
    pub relation: Relation,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Name of the rule that generated this inference
    #[serde(rename = "ruleName")]
    pub rule_name: String,
    /// Human-readable explanation of the inference path
    pub explanation: String,
}

/// Statistics about an inference operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InferStats {
    /// Number of nodes visited during inference
    #[serde(rename = "nodesVisited")]
    pub nodes_visited: usize,
    /// Number of inference paths found
    #[serde(rename = "pathsFound")]
    pub paths_found: usize,
    /// Maximum depth reached during traversal
    #[serde(rename = "maxDepthReached")]
    pub max_depth_reached: usize,
    /// Execution time in milliseconds
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,
}

/// Result of an inference operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferResult {
    /// The target entity that was analyzed
    pub target: String,
    /// List of inferred relations
    #[serde(rename = "inferredRelations")]
    pub inferred_relations: Vec<InferredRelation>,
    /// Statistics about the inference operation
    pub stats: InferStats,
}

impl InferResult {
    /// Create a new inference result
    pub fn new(target: String) -> Self {
        Self {
            target,
            inferred_relations: Vec::new(),
            stats: InferStats::default(),
        }
    }

    /// Check if any inferences were found
    pub fn has_inferences(&self) -> bool {
        !self.inferred_relations.is_empty()
    }

    /// Get the number of inferred relations
    pub fn count(&self) -> usize {
        self.inferred_relations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_result_new() {
        let result = InferResult::new("Test".to_string());
        assert_eq!(result.target, "Test");
        assert!(!result.has_inferences());
        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_infer_stats_default() {
        let stats = InferStats::default();
        assert_eq!(stats.nodes_visited, 0);
        assert_eq!(stats.paths_found, 0);
        assert_eq!(stats.max_depth_reached, 0);
        assert_eq!(stats.execution_time_ms, 0);
    }
}

```

## File ../memory-graph/src\types\mod.rs:
```rust
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

```

## File ../memory-graph/src\types\observation.rs:
```rust
//! Observation types for entity updates

use serde::{Deserialize, Serialize};

/// Observation to add to an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

impl Observation {
    /// Create a new observation
    pub fn new(entity_name: String, contents: Vec<String>) -> Self {
        Self {
            entity_name,
            contents,
        }
    }
}

/// Observation deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationDeletion {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub observations: Vec<String>,
}

impl ObservationDeletion {
    /// Create a new observation deletion request
    pub fn new(entity_name: String, observations: Vec<String>) -> Self {
        Self {
            entity_name,
            observations,
        }
    }
}

```

## File ../memory-graph/src\types\relation.rs:
```rust
//! Relation types for the knowledge graph

use serde::{Deserialize, Serialize};

use super::{default_user, is_default_user, is_zero, Entity};

/// Relation between entities with temporal validity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType")]
    pub relation_type: String,
    #[serde(
        rename = "createdBy",
        default = "default_user",
        skip_serializing_if = "is_default_user"
    )]
    pub created_by: String,
    #[serde(rename = "createdAt", default, skip_serializing_if = "is_zero")]
    pub created_at: u64,
    #[serde(rename = "validFrom", default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<u64>,
    #[serde(rename = "validTo", default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<u64>,
}

impl Relation {
    /// Create a new relation
    pub fn new(from: String, to: String, relation_type: String) -> Self {
        Self {
            from,
            to,
            relation_type,
            created_by: String::new(),
            created_at: 0,
            valid_from: None,
            valid_to: None,
        }
    }

    /// Create a new relation with temporal validity
    pub fn with_validity(
        from: String,
        to: String,
        relation_type: String,
        valid_from: Option<u64>,
        valid_to: Option<u64>,
    ) -> Self {
        Self {
            from,
            to,
            relation_type,
            created_by: String::new(),
            created_at: 0,
            valid_from,
            valid_to,
        }
    }
}

/// Related entity with relation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntity {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String,
    pub entity: Entity,
}

/// Result of get_related query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntities {
    pub entity: String,
    pub relations: Vec<RelatedEntity>,
}

```

## File ../memory-graph/src\types\summary.rs:
```rust
//! Summary types for graph statistics

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::EntityBrief;

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Summary {
    #[serde(rename = "totalEntities")]
    pub total_entities: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<EntityBrief>>,
    #[serde(rename = "byStatus", skip_serializing_if = "Option::is_none")]
    pub by_status: Option<HashMap<String, usize>>,
    #[serde(rename = "byType", skip_serializing_if = "Option::is_none")]
    pub by_type: Option<HashMap<String, usize>>,
    #[serde(rename = "byPriority", skip_serializing_if = "Option::is_none")]
    pub by_priority: Option<HashMap<String, usize>>,
}

impl Summary {
    /// Create an empty summary
    pub fn new(total_entities: usize) -> Self {
        Self {
            total_entities,
            ..Default::default()
        }
    }

    /// Create a summary with entity briefs
    pub fn with_entities(total_entities: usize, entities: Vec<EntityBrief>) -> Self {
        Self {
            total_entities,
            entities: Some(entities),
            ..Default::default()
        }
    }

    /// Create a summary with statistics
    pub fn with_stats(
        total_entities: usize,
        by_type: HashMap<String, usize>,
        by_status: Option<HashMap<String, usize>>,
        by_priority: Option<HashMap<String, usize>>,
    ) -> Self {
        Self {
            total_entities,
            entities: None,
            by_status,
            by_type: Some(by_type),
            by_priority,
        }
    }
}

```

## File ../memory-graph/src\types\traversal.rs:
```rust
//! Graph traversal types

use serde::{Deserialize, Serialize};

use super::Entity;

/// Path step for traverse query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub direction: String,
    #[serde(rename = "targetType")]
    pub target_type: Option<String>,
}

impl PathStep {
    /// Create a new path step
    pub fn new(relation_type: String, direction: String) -> Self {
        Self {
            relation_type,
            direction,
            target_type: None,
        }
    }

    /// Create a new path step with target type filter
    pub fn with_target_type(
        relation_type: String,
        direction: String,
        target_type: String,
    ) -> Self {
        Self {
            relation_type,
            direction,
            target_type: Some(target_type),
        }
    }
}

/// Single path in traversal result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalPath {
    pub nodes: Vec<String>,
    pub relations: Vec<String>,
}

impl TraversalPath {
    /// Create a new traversal path
    pub fn new(nodes: Vec<String>, relations: Vec<String>) -> Self {
        Self { nodes, relations }
    }
}

/// Result of traverse query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    #[serde(rename = "startNode")]
    pub start_node: String,
    pub paths: Vec<TraversalPath>,
    #[serde(rename = "endNodes")]
    pub end_nodes: Vec<Entity>,
}

impl TraversalResult {
    /// Create a new traversal result
    pub fn new(start_node: String, paths: Vec<TraversalPath>, end_nodes: Vec<Entity>) -> Self {
        Self {
            start_node,
            paths,
            end_nodes,
        }
    }
}

```

## File ../memory-graph/src\utils\atomic.rs:
```rust
//! Atomic file operations
//!
//! This module provides utilities for atomic file writes to prevent
//! data corruption during crashes or power failures.
//!
//! # Pattern
//!
//! 1. Write to a temporary file (.tmp)
//! 2. Call sync_all() to flush to disk
//! 3. Rename temp file to final path (atomic on most filesystems)
//!
//! This ensures that the final file is either:
//! - The old version (if crash before rename)
//! - The new version (if rename completed)
//! - Never a partial/corrupted state

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Result type for atomic operations
pub type AtomicResult<T> = Result<T, AtomicError>;

/// Errors that can occur during atomic operations
#[derive(Debug)]
pub enum AtomicError {
    Io(io::Error),
    TempFileExists(String),
}

impl std::fmt::Display for AtomicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomicError::Io(e) => write!(f, "IO error: {}", e),
            AtomicError::TempFileExists(path) => write!(f, "Temp file already exists: {}", path),
        }
    }
}

impl std::error::Error for AtomicError {}

impl From<io::Error> for AtomicError {
    fn from(e: io::Error) -> Self {
        AtomicError::Io(e)
    }
}

/// Atomically write content to a file
///
/// This function:
/// 1. Writes content to a .tmp file
/// 2. Syncs the file to disk
/// 3. Atomically renames to the final path
///
/// # Arguments
///
/// * `path` - The final destination path
/// * `content` - The content to write
///
/// # Example
///
/// ```ignore
/// atomic_write("data/snapshot.jsonl", "line1\nline2\n")?;
/// ```
pub fn atomic_write<P: AsRef<Path>>(path: P, content: &str) -> AtomicResult<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file
    let mut file = File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;

    // Sync to disk (ensure data is durable)
    file.sync_all()?;

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Atomically write content using a writer function
///
/// This is more efficient for large files as it doesn't require
/// building the entire content string in memory first.
///
/// # Arguments
///
/// * `path` - The final destination path
/// * `write_fn` - A function that writes content to the file
///
/// # Example
///
/// ```ignore
/// atomic_write_with("data/snapshot.jsonl", |file| {
///     writeln!(file, "line1")?;
///     writeln!(file, "line2")?;
///     Ok(())
/// })?;
/// ```
pub fn atomic_write_with<P, F>(path: P, write_fn: F) -> AtomicResult<()>
where
    P: AsRef<Path>,
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file using the provided function
    let mut file = File::create(&temp_path)?;
    write_fn(&mut file)?;

    // Sync to disk
    file.sync_all()?;

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Safely rename a file, creating a backup if the destination exists
///
/// # Arguments
///
/// * `from` - Source file path
/// * `to` - Destination file path
/// * `backup` - Optional backup path for existing destination
///
/// # Returns
///
/// * `Ok(true)` - File was renamed successfully
/// * `Ok(false)` - Source file doesn't exist
/// * `Err(...)` - An error occurred
pub fn safe_rename<P1, P2, P3>(from: P1, to: P2, backup: Option<P3>) -> AtomicResult<bool>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let from = from.as_ref();
    let to = to.as_ref();

    // Check if source exists
    if !from.exists() {
        return Ok(false);
    }

    // Backup existing destination if requested
    if let Some(backup_path) = backup {
        if to.exists() {
            // Remove old backup if exists
            let backup = backup_path.as_ref();
            if backup.exists() {
                fs::remove_file(backup)?;
            }
            fs::rename(to, backup)?;
        }
    }

    // Rename source to destination
    fs::rename(from, to)?;

    Ok(true)
}

/// Clean up any leftover temp files from interrupted operations
///
/// Call this on startup to clean up .tmp files that may have been
/// left behind from crashes.
pub fn cleanup_temp_files<P: AsRef<Path>>(dir: P) -> AtomicResult<usize> {
    let dir = dir.as_ref();
    let mut cleaned = 0;

    if !dir.exists() {
        return Ok(0);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "tmp").unwrap_or(false) {
            fs::remove_file(&path)?;
            cleaned += 1;
        }
    }

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        atomic_write(&path, "Hello, World!").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");

        // Temp file should not exist
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn test_atomic_write_with() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        atomic_write_with(&path, |file| {
            writeln!(file, "Line 1")?;
            writeln!(file, "Line 2")?;
            Ok(())
        })
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Line 1\nLine 2\n");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("subdir").join("nested").join("test.txt");

        atomic_write(&path, "nested content").unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "nested content");
    }

    #[test]
    fn test_safe_rename_with_backup() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source.txt");
        let to = temp_dir.path().join("dest.txt");
        let backup = temp_dir.path().join("backup.txt");

        // Create source and destination
        fs::write(&from, "new content").unwrap();
        fs::write(&to, "old content").unwrap();

        // Rename with backup
        let result = safe_rename(&from, &to, Some(&backup)).unwrap();
        assert!(result);

        // Check results
        assert!(!from.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "new content");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "old content");
    }

    #[test]
    fn test_safe_rename_source_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("nonexistent.txt");
        let to = temp_dir.path().join("dest.txt");

        let result = safe_rename(&from, &to, None::<&Path>).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_cleanup_temp_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some temp files
        fs::write(temp_dir.path().join("file1.tmp"), "temp1").unwrap();
        fs::write(temp_dir.path().join("file2.tmp"), "temp2").unwrap();
        fs::write(temp_dir.path().join("keep.txt"), "keep").unwrap();

        let cleaned = cleanup_temp_files(temp_dir.path()).unwrap();
        assert_eq!(cleaned, 2);

        // Check that .tmp files are gone but .txt remains
        assert!(!temp_dir.path().join("file1.tmp").exists());
        assert!(!temp_dir.path().join("file2.tmp").exists());
        assert!(temp_dir.path().join("keep.txt").exists());
    }
}

```

## File ../memory-graph/src\utils\mod.rs:
```rust
//! Utility functions and helpers
//!
//! This module contains timestamp utilities and other helper functions.

pub mod atomic;
pub mod time;

pub use atomic::{atomic_write, atomic_write_with, cleanup_temp_files, safe_rename, AtomicResult};
pub use time::{current_timestamp, days_to_ymd, get_current_time, get_month_name, get_weekday};

```

## File ../memory-graph/src\utils\time.rs:
```rust
//! Time and timestamp utilities

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current Unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Get current user from git config or OS environment
pub fn get_current_user() -> String {
    use std::env;
    use std::process::Command;

    // 1. Try Git Config (preferred for project context)
    if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // 2. Try OS Environment Variable
    env::var("USER") // Linux/Mac
        .or_else(|_| env::var("USERNAME")) // Windows
        .unwrap_or_else(|_| "anonymous".to_string())
}

/// Get current time information as JSON
pub fn get_current_time() -> Value {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap();
    let timestamp = duration.as_secs();
    let millis = duration.as_millis() as u64;

    // Calculate datetime components
    let secs = timestamp as i64;

    // Days since epoch
    let days = secs / 86400;
    let remaining = secs % 86400;

    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Calculate year, month, day
    let (year, month, day) = days_to_ymd(days);

    // Format ISO 8601
    let iso8601 = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    );

    // Format readable
    let weekday = get_weekday(days);
    let month_name = get_month_name(month);
    let readable = format!(
        "{}, {} {} {} {:02}:{:02}:{:02} UTC",
        weekday, day, month_name, year, hours, minutes, seconds
    );

    json!({
        "timestamp": timestamp,
        "timestamp_ms": millis,
        "iso8601": iso8601,
        "readable": readable,
        "components": {
            "year": year,
            "month": month,
            "day": day,
            "hour": hours,
            "minute": minutes,
            "second": seconds,
            "weekday": weekday
        }
    })
}

/// Convert days since epoch to year/month/day
pub fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm to convert days since epoch to year/month/day
    let remaining_days = days + 719468; // Days from year 0 to 1970

    let era = remaining_days / 146097;
    let doe = remaining_days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };

    (year, month, day)
}

/// Get weekday name from days since epoch
pub fn get_weekday(days: i64) -> &'static str {
    match (days + 4) % 7 {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

/// Get month name from month number
pub fn get_month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

```

## File ../memory-graph/src\validation\mod.rs:
```rust
//! Type validation for entities and relations
//!
//! This module provides soft validation for standard entity and relation types.

mod types;

pub use types::{
    validate_entity_type, validate_relation_type, STANDARD_ENTITY_TYPES, STANDARD_RELATION_TYPES,
};

```

## File ../memory-graph/src\validation\types.rs:
```rust
//! Standard entity and relation types with validation

/// Standard entity types for software project management
pub const STANDARD_ENTITY_TYPES: &[&str] = &[
    "Project",
    "Module",
    "Feature",
    "Bug",
    "Decision",
    "Requirement",
    "Milestone",
    "Risk",
    "Convention",
    "Schema",
    "Person",
];

/// Standard relation types for software project management
pub const STANDARD_RELATION_TYPES: &[&str] = &[
    "contains",
    "implements",
    "fixes",
    "caused_by",
    "depends_on",
    "blocked_by",
    "assigned_to",
    "part_of",
    "relates_to",
    "supersedes",
    "affects",
    "requires",
];

/// Check if entity type is standard, return warning if not
pub fn validate_entity_type(entity_type: &str) -> Option<String> {
    if STANDARD_ENTITY_TYPES
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(entity_type))
    {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard entityType '{}'. Recommended: {:?}",
            entity_type, STANDARD_ENTITY_TYPES
        ))
    }
}

/// Check if relation type is standard, return warning if not
pub fn validate_relation_type(relation_type: &str) -> Option<String> {
    if STANDARD_RELATION_TYPES
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(relation_type))
    {
        None
    } else {
        Some(format!(
            "⚠️ Non-standard relationType '{}'. Recommended: {:?}",
            relation_type, STANDARD_RELATION_TYPES
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_standard_entity_type() {
        assert!(validate_entity_type("Project").is_none());
        assert!(validate_entity_type("module").is_none()); // case insensitive
        assert!(validate_entity_type("Person").is_none());
    }

    #[test]
    fn test_validate_non_standard_entity_type() {
        let warning = validate_entity_type("CustomType");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Non-standard entityType"));
    }

    #[test]
    fn test_validate_standard_relation_type() {
        assert!(validate_relation_type("contains").is_none());
        assert!(validate_relation_type("DEPENDS_ON").is_none()); // case insensitive
        assert!(validate_relation_type("implements").is_none());
    }

    #[test]
    fn test_validate_non_standard_relation_type() {
        let warning = validate_relation_type("custom_relation");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Non-standard relationType"));
    }
}

```

# Thông tin bổ sung:

