//! Delete observations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, ObservationDeletion};

/// Tool for deleting specific observations from entities
pub struct DeleteObservationsTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteObservationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_observations".to_string(),
            description: "Delete specific observations from entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deletions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observations to delete"
                                }
                            },
                            "required": ["entityName", "observations"]
                        }
                    }
                },
                "required": ["deletions"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let deletions: Vec<ObservationDeletion> =
            serde_json::from_value(params.get("deletions").cloned().unwrap_or(json!([])))?;
        self.kb.delete_observations(deletions)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Observations deleted successfully"
            }]
        }))
    }
}
