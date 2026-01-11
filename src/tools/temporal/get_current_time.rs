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
