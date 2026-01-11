//! Get related tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for getting entities related to a specific entity
pub struct GetRelatedTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelatedTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelatedTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_related".to_string(),
            description: "Get entities related to a specific entity".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "Name of the entity to find relations for"
                    },
                    "relationType": {
                        "type": "string",
                        "description": "Filter by relation type (optional)"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["outgoing", "incoming", "both"],
                        "default": "both",
                        "description": "Direction of relations"
                    }
                },
                "required": ["entityName"]
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_name = params
            .get("entityName")
            .and_then(|v| v.as_str())
            .ok_or("Missing entityName")?;
        let relation_type = params.get("relationType").and_then(|v| v.as_str());
        let direction = params
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("both");

        let related = self.kb.get_related(entity_name, relation_type, direction)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&related)?
            }]
        }))
    }
}
