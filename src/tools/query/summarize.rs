//! Summarize tool

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::McpResult;

/// Tool for getting a condensed summary of entities
pub struct SummarizeTool {
    kb: Arc<KnowledgeBase>,
}

impl SummarizeTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for SummarizeTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "summarize".to_string(),
            description: "Get a condensed summary of entities".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Specific entities to summarize (optional)"
                    },
                    "entityType": {
                        "type": "string",
                        "description": "Summarize all entities of this type (optional)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["brief", "detailed", "stats"],
                        "default": "brief",
                        "description": "Output format: brief (first observation), detailed (all observations), stats (statistics)"
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, params: Value) -> McpResult<Value> {
        let entity_names: Option<Vec<String>> = params
            .get("entityNames")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let entity_type = params
            .get("entityType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("brief");

        let summary = self.kb.summarize(entity_names, entity_type, format)?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&summary)?
            }]
        }))
    }
}
