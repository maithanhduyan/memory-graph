//! Delete relations tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{McpResult, Relation};

/// Tool for deleting multiple relations from the knowledge graph
pub struct DeleteRelationsTool {
    kb: Arc<KnowledgeBase>,
}

impl DeleteRelationsTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for DeleteRelationsTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "delete_relations".to_string(),
            description: "Delete multiple relations from the knowledge graph".to_string(),
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
                                "relationType": { "type": "string", "description": "The type of relation" }
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
        self.kb.delete_relations(relations)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": "Relations deleted successfully"
            }]
        }))
    }
}
