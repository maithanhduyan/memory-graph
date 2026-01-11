//! Get relations at time tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;
use crate::utils::time::current_timestamp;

/// Tool for getting relations valid at a specific point in time
pub struct GetRelationsAtTimeTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelationsAtTimeTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationsAtTimeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relations_at_time".to_string(),
            description: "Get relations that are valid at a specific point in time. Useful for querying historical state of the knowledge graph.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timestamp": {
                        "type": "integer",
                        "description": "Unix timestamp to query. If not provided, uses current time."
                    },
                    "entityName": {
                        "type": "string",
                        "description": "Optional: filter relations involving this entity"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let timestamp = params.get("timestamp").and_then(|v| v.as_u64());
        let entity_name = params.get("entityName").and_then(|v| v.as_str());

        let relations = self.kb.get_relations_at_time(timestamp, entity_name)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "queryTime": timestamp.unwrap_or_else(current_timestamp),
                    "relations": relations
                }))?
            }]
        }))
    }
}
