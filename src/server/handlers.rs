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
