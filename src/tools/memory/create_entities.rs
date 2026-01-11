//! Create entities tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{Entity, McpResult};
use crate::validation::validate_entity_type;

/// Tool for creating multiple new entities in the knowledge graph
pub struct CreateEntitiesTool {
    kb: Arc<KnowledgeBase>,
}

impl CreateEntitiesTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateEntitiesTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_entities".to_string(),
            description: "Create multiple new entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entities": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "The name of the entity" },
                                "entityType": { "type": "string", "description": "The type of the entity" },
                                "observations": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Initial observations about the entity"
                                },
                                "createdBy": { "type": "string", "description": "Who created this entity (auto-filled from git/env if not provided)" },
                                "updatedBy": { "type": "string", "description": "Who last updated this entity (auto-filled from git/env if not provided)" }
                            },
                            "required": ["name", "entityType"]
                        }
                    }
                },
                "required": ["entities"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entities: Vec<Entity> =
            serde_json::from_value(params.get("entities").cloned().unwrap_or(json!([])))?;

        // Collect warnings for non-standard types
        let warnings: Vec<String> = entities
            .iter()
            .filter_map(|e| validate_entity_type(&e.entity_type))
            .collect();

        let created = self.kb.create_entities(entities)?;

        let response = if warnings.is_empty() {
            serde_json::to_string_pretty(&created)?
        } else {
            format!(
                "{}\n\n{}",
                serde_json::to_string_pretty(&created)?,
                warnings.join("\n")
            )
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": response
            }]
        }))
    }
}
