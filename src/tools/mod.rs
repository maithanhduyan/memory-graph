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
