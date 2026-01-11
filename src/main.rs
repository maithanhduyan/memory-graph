//! Memory Graph MCP Server - Binary Entry Point
//!
//! This is the main entry point for the memory-server binary.
//! Supports graceful shutdown with automatic snapshot creation when Event Sourcing is enabled.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use memory_graph::knowledge_base::KnowledgeBase;
use memory_graph::protocol::ServerInfo;
use memory_graph::server::McpServer;
use memory_graph::tools::register_all_tools;
use memory_graph::types::McpResult;

/// Global shutdown flag
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main() -> McpResult<()> {
    let kb = Arc::new(KnowledgeBase::new());
    let kb_for_shutdown = Arc::clone(&kb);

    // Set up Ctrl+C / SIGTERM handler for graceful shutdown
    if let Err(e) = ctrlc::set_handler(move || {
        eprintln!("[Memory Server] Shutdown signal received, creating snapshot...");
        SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);

        // Create final snapshot before exit
        match kb_for_shutdown.create_snapshot() {
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

    let server_info = ServerInfo {
        name: "memory".to_string(),
        version: "1.2.0".to_string(),
    };

    let mut server = McpServer::with_info(server_info);

    // Register all 15 tools
    register_all_tools(&mut server, kb);

    server.run()
}
