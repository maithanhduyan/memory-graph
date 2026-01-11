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
