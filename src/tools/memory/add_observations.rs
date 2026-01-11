//! Add observations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Observation};

/// Tool for adding new observations to existing entities
pub struct AddObservationsTool {
    kb: Arc<KnowledgeBase>,
}

impl AddObservationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for AddObservationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "add_observations".to_string(),
            description: "Add new observations to existing entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "observations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The name of the entity" },
                                "contents": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Observation contents to add"
                                }
                            },
                            "required": ["entityName", "contents"]
                        }
                    }
                },
                "required": ["observations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let observations: Vec<Observation> =
            serde_json::from_value(params.get("observations").cloned().unwrap_or(json!([])))?;
        let added = self.kb.add_observations(observations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&added)?
            }]
        }))
    }
}
