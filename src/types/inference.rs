//! Inference types for the reasoning engine
//!
//! This module contains data structures for the inference engine output.

use serde::{Deserialize, Serialize};

use super::Relation;

/// An inferred relation with confidence and provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredRelation {
    /// The inferred relation
    pub relation: Relation,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Name of the rule that generated this inference
    #[serde(rename = "ruleName")]
    pub rule_name: String,
    /// Human-readable explanation of the inference path
    pub explanation: String,
}

/// Statistics about an inference operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InferStats {
    /// Number of nodes visited during inference
    #[serde(rename = "nodesVisited")]
    pub nodes_visited: usize,
    /// Number of inference paths found
    #[serde(rename = "pathsFound")]
    pub paths_found: usize,
    /// Maximum depth reached during traversal
    #[serde(rename = "maxDepthReached")]
    pub max_depth_reached: usize,
    /// Execution time in milliseconds
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,
}

/// Result of an inference operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferResult {
    /// The target entity that was analyzed
    pub target: String,
    /// List of inferred relations
    #[serde(rename = "inferredRelations")]
    pub inferred_relations: Vec<InferredRelation>,
    /// Statistics about the inference operation
    pub stats: InferStats,
}

impl InferResult {
    /// Create a new inference result
    pub fn new(target: String) -> Self {
        Self {
            target,
            inferred_relations: Vec::new(),
            stats: InferStats::default(),
        }
    }

    /// Check if any inferences were found
    pub fn has_inferences(&self) -> bool {
        !self.inferred_relations.is_empty()
    }

    /// Get the number of inferred relations
    pub fn count(&self) -> usize {
        self.inferred_relations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_result_new() {
        let result = InferResult::new("Test".to_string());
        assert_eq!(result.target, "Test");
        assert!(!result.has_inferences());
        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_infer_stats_default() {
        let stats = InferStats::default();
        assert_eq!(stats.nodes_visited, 0);
        assert_eq!(stats.paths_found, 0);
        assert_eq!(stats.max_depth_reached, 0);
        assert_eq!(stats.execution_time_ms, 0);
    }
}
