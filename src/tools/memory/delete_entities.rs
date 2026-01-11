//! Delete entities tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for deleting multiple entities from the knowledge graph
pub struct DeleteEntitiesTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteEntitiesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_entities".to_string(),
            description: "Delete multiple entities from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "An array of entity names to delete"
                    }
                },
                "required": ["entityNames"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Vec<String> =
            serde_json::from_value(params.get("entityNames").cloned().unwrap_or(json!([])))?;
        self.kb.delete_entities(entity_names)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Entities deleted successfully"
            }]
        }))
    }
}
