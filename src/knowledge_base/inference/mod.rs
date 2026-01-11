//! Inference Engine for Knowledge Graph Reasoning
//!
//! This module provides the core inference engine that applies logical rules
//! to discover hidden relations in the knowledge graph.

pub mod rules;

use crate::types::{InferStats, InferredRelation, KnowledgeGraph};

/// Trait for inference rules
///
/// Each rule implements logic to derive new relations from existing ones.
/// Rules are applied lazily at runtime (not persisted).
pub trait InferenceRule: Send + Sync {
    /// Get the name of this rule
    fn name(&self) -> &str;

    /// Apply the rule to infer relations for a target entity
    ///
    /// # Arguments
    /// * `graph` - The knowledge graph to analyze
    /// * `target` - The entity to infer relations for
    /// * `min_confidence` - Minimum confidence threshold (0.0 - 1.0)
    ///
    /// # Returns
    /// A tuple of (inferred_relations, stats)
    fn apply(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats);
}

/// The inference engine that manages and applies rules
pub struct InferenceEngine {
    rules: Vec<Box<dyn InferenceRule>>,
}

impl InferenceEngine {
    /// Create a new inference engine with default rules
    pub fn new() -> Self {
        Self::with_max_depth(3)
    }

    /// Create a new inference engine with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            rules: vec![Box::new(rules::TransitiveDependencyRule::new(max_depth))],
        }
    }

    /// Create an empty inference engine (no rules)
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a new inference rule
    pub fn register_rule(&mut self, rule: Box<dyn InferenceRule>) {
        self.rules.push(rule);
    }

    /// Get the number of registered rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Run all rules and collect inferred relations
    ///
    /// # Arguments
    /// * `graph` - The knowledge graph to analyze
    /// * `target` - The entity to infer relations for
    /// * `min_confidence` - Minimum confidence threshold (0.0 - 1.0)
    ///
    /// # Returns
    /// A tuple of (all_inferred_relations, combined_stats)
    pub fn infer(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats) {
        let mut all_inferred = Vec::new();
        let mut total_stats = InferStats::default();
        let start_time = std::time::Instant::now();

        for rule in &self.rules {
            let (relations, stats) = rule.apply(graph, target, min_confidence);
            all_inferred.extend(relations);

            // Merge stats
            total_stats.nodes_visited += stats.nodes_visited;
            total_stats.paths_found += stats.paths_found;
            total_stats.max_depth_reached = total_stats.max_depth_reached.max(stats.max_depth_reached);
        }

        total_stats.execution_time_ms = start_time.elapsed().as_millis() as u64;
        (all_inferred, total_stats)
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = InferenceEngine::new();
        assert_eq!(engine.rule_count(), 1); // TransitiveDependency by default
    }

    #[test]
    fn test_empty_engine() {
        let engine = InferenceEngine::empty();
        assert_eq!(engine.rule_count(), 0);
    }

    #[test]
    fn test_infer_on_empty_graph() {
        let engine = InferenceEngine::new();
        let graph = KnowledgeGraph::default();
        let (inferred, stats) = engine.infer(&graph, "NonExistent", 0.5);
        assert!(inferred.is_empty());
        assert_eq!(stats.nodes_visited, 0);
    }
}
