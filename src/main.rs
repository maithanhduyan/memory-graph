//! Memory Graph MCP Server - Binary Entry Point
//!
//! This is the main entry point for the memory-server binary.

use std::sync::Arc;

use memory_graph::knowledge_base::KnowledgeBase;
use memory_graph::protocol::ServerInfo;
use memory_graph::server::McpServer;
use memory_graph::tools::register_all_tools;
use memory_graph::types::McpResult;

fn main() -> McpResult<()> {
    let kb = Arc::new(KnowledgeBase::new());

    let server_info = ServerInfo {
        name: "memory".to_string(),
        version: "1.0.0".to_string(),
    };

    let mut server = McpServer::with_info(server_info);

    // Register all 15 tools
    register_all_tools(&mut server, kb);

    server.run()
}
