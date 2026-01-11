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
