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
    use tokio::sync::RwLock;

    // Create sync KB for MCP tools (SSE transport)
    let kb_sync = Arc::new(KnowledgeBase::new());

    // Create async KB wrapper for WebSocket/REST
    let kb_async = Arc::new(RwLock::new(KnowledgeBase::new()));

    // Initialize global broadcaster for WebSocket events
    init_broadcaster(1024);

    // Create AppState for WebSocket
    let state = Arc::new(AppState::new(kb_async));

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

    // Create router with JWT auth
    let app = create_router_with_auth(state, kb_sync, jwt_auth, require_auth);

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
