//! Get relation history tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;
use crate::utils::time::current_timestamp;

/// Tool for getting all relations (current and historical) for an entity
pub struct GetRelationHistoryTool {
    kb: Arc<KnowledgeBase>,
}

impl GetRelationHistoryTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for GetRelationHistoryTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "get_relation_history".to_string(),
            description: "Get all relations (current and historical) for an entity. Shows temporal validity (validFrom/validTo) for each relation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "The name of the entity to get relation history for"
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
            .ok_or("entityName is required")?;

        let relations = self.kb.get_relation_history(entity_name)?;
        let current_time = current_timestamp();

        // Mark each relation as current or historical
        let annotated: Vec<Value> = relations
            .iter()
            .map(|r| {
                let is_current = match (r.valid_from, r.valid_to) {
                    (Some(vf), Some(vt)) => current_time >= vf && current_time <= vt,
                    (Some(vf), None) => current_time >= vf,
                    (None, Some(vt)) => current_time <= vt,
                    (None, None) => true,
                };

                json!({
                    "from": r.from,
                    "to": r.to,
                    "relationType": r.relation_type,
                    "validFrom": r.valid_from,
                    "validTo": r.valid_to,
                    "isCurrent": is_current
                })
            })
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&json!({
                    "entity": entity_name,
                    "currentTime": current_time,
                    "relations": annotated
                }))?
            }]
        }))
    }
}
