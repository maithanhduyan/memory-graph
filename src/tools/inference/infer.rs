//! Infer tool - Runtime graph reasoning
//!
//! Discovers hidden/transitive relations using inference rules.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::knowledge_base::inference::InferenceEngine;
use crate::knowledge_base::KnowledgeBase;
use crate::protocol::{McpTool, Tool};
use crate::types::{InferResult, McpResult};

/// Tool for inferring hidden relations from the knowledge graph
///
/// This tool applies inference rules (like transitive dependency) to discover
/// relations that aren't explicitly stored but can be logically derived.
pub struct InferTool {
    kb: Arc<KnowledgeBase>,
}

impl InferTool {
    pub fn new(kb: Arc<KnowledgeBase>) -> Self {
        Self { kb }
    }
}

impl Tool for InferTool {
    fn definition(&self) -> McpTool {
        McpTool {
            name: "infer".to_string(),
            description: "Infer hidden relations for an entity using logical rules. Discovers transitive dependencies and indirect connections not explicitly stored in the graph.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityName": {
                        "type": "string",
                        "description": "Name of the entity to infer relations for"
                    },
                    "minConfidence": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "default": 0.5,
                        "description": "Minimum confidence threshold (0.0-1.0). Higher values return fewer but more reliable inferences."
                    },
                    "maxDepth": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "default": 3,
                        "description": "Maximum traversal depth (1-5). Higher values find more distant relations but take longer."
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

        let min_confidence = params
            .get("minConfidence")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);

        let max_depth = params
            .get("maxDepth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .clamp(1, 5);

        // Get full graph for inference (no pagination)
        let graph = self.kb.read_graph(None, None)?;

        // Create inference engine with specified depth
        let engine = InferenceEngine::with_max_depth(max_depth);

        // Run inference
        let (inferred_relations, stats) = engine.infer(&graph, entity_name, min_confidence);

        // Build result
        let result = InferResult {
            target: entity_name.to_string(),
            inferred_relations,
            stats,
        };

        // Format response
        let response = if result.inferred_relations.is_empty() {
            json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "No inferred relations found for '{}' with confidence >= {:.0}% and max depth {}.\n\nStats: visited {} nodes in {}ms",
                        entity_name,
                        min_confidence * 100.0,
                        max_depth,
                        result.stats.nodes_visited,
                        result.stats.execution_time_ms
                    )
                }]
            })
        } else {
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result)?
                }]
            })
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Tool;

    #[test]
    fn test_tool_definition() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);
        let def = tool.definition();

        assert_eq!(def.name, "infer");
        assert!(def.description.to_lowercase().contains("infer"));

        // Check required field
        let schema = &def.input_schema;
        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("entityName")));
    }

    #[test]
    fn test_execute_nonexistent_entity() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);

        let result = tool.execute(json!({
            "entityName": "NonExistent"
        }));

        assert!(result.is_ok());
        let response = result.unwrap();
        let text = response["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No inferred relations found"));
    }

    #[test]
    fn test_parameter_clamping() {
        let kb = Arc::new(KnowledgeBase::new());
        let tool = InferTool::new(kb);

        // Test with out-of-range values (should be clamped)
        let result = tool.execute(json!({
            "entityName": "Test",
            "minConfidence": 2.0,  // Should clamp to 1.0
            "maxDepth": 100       // Should clamp to 5
        }));

        assert!(result.is_ok());
    }
}
