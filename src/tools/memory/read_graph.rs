//! Read graph tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for reading the knowledge graph with optional pagination
pub struct ReadGraphTool {
    kb: Arc<KnowledgeBase>,
}

impl ReadGraphTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for ReadGraphTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "read_graph".to_string(),
            description: "Read the knowledge graph with optional pagination. Use limit/offset to avoid context overflow with large graphs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return. Recommended: 50-100 for large graphs"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of entities to skip (for pagination)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let offset = params
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let graph = self.kb.read_graph(limit, offset)?;

        let total_msg = if limit.is_some() || offset.is_some() {
            format!(" (showing {} entities)", graph.entities.len())
        } else {
            String::new()
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("{}{}", serde_json::to_string_pretty(&graph)?, total_msg)
            }]
        }))
    }
}
