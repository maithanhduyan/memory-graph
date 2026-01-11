//! Create relations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Relation};
use crate::validation::validate_relation_type;

/// Tool for creating multiple new relations between entities
pub struct CreateRelationsTool {
    kb: Arc<KnowledgeBase>,
}

impl CreateRelationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for CreateRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "create_relations".to_string(),
            description: "Create multiple new relations between entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" },
                                "createdBy": { "type": "string", "description": "Who created this relation (auto-filled from git/env if not provided)" },
                                "validFrom": { "type": "integer", "description": "Unix timestamp when relation becomes valid" },
                                "validTo": { "type": "integer", "description": "Unix timestamp when relation expires" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let relations: Vec<Relation> =
            serde_json::from_value(params.get("relations").cloned().unwrap_or(json!([])))?;

        // Collect warnings for non-standard relation types
        let warnings: Vec<String> = relations
            .iter()
            .filter_map(|r| validate_relation_type(&r.relation_type))
            .collect();

        let created = self.kb.create_relations(relations)?;

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
