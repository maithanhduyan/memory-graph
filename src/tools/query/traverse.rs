//! Traverse tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, PathStep};

/// Tool for traversing the graph following a path pattern
pub struct TraverseTool {
    kb: Arc<KnowledgeBase>,
}

impl TraverseTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for TraverseTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "traverse".to_string(),
            description: "Traverse the graph following a path pattern for multi-hop queries"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "startNode": {
                        "type": "string",
                        "description": "Starting entity name"
                    },
                    "path": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "relationType": {
                                    "type": "string",
                                    "description": "Type of relation to follow"
                                },
                                "direction": {
                                    "type": "string",
                                    "enum": ["out", "in"],
                                    "description": "Direction: out (outgoing) or in (incoming)"
                                },
                                "targetType": {
                                    "type": "string",
                                    "description": "Filter by target entity type (optional)"
                                }
                            },
                            "required": ["relationType", "direction"]
                        },
                        "description": "Path pattern to follow"
                    },
                    "maxResults": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum number of results"
                    }
                },
                "required": ["startNode", "path"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let start_node = params
            .get("startNode")
            .and_then(|v| v.as_str())
            .ok_or("Missing startNode")?;

        let path: Vec<PathStep> =
            serde_json::from_value(params.get("path").cloned().unwrap_or(json!([])))?;

        let max_results = params
            .get("maxResults")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let result = self.kb.traverse(start_node, path, max_results)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        }))
    }
}
