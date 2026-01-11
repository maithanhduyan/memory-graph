//! Search nodes tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for searching nodes in the knowledge graph with semantic matching
pub struct SearchNodesTool {
    kb: Arc<KnowledgeBase>,
}

impl SearchNodesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SearchNodesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "search_nodes".to_string(),
            description:
                "Search for nodes in the knowledge graph. Returns matching entities with optional relations."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to match against entity names, types, and observations"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of entities to return (default: no limit)"
                    },
                    "includeRelations": {
                        "type": "boolean",
                        "description": "Whether to include relations connected to matching entities (default: true)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let include_relations = params
            .get("includeRelations")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let graph = self.kb.search_nodes(query, limit, include_relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&graph)?
            }]
        }))
    }
}
