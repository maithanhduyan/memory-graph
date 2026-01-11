//! Protocol types for MCP and JSON-RPC communication
//!
//! This module contains all protocol-related types and traits.

mod jsonrpc;
mod mcp;

pub use jsonrpc::{ErrorObject, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use mcp::{McpTool, ServerInfo, Tool};
