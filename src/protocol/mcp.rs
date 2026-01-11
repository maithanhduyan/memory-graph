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
