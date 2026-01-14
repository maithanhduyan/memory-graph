//! Inference Rules for Knowledge Graph Reasoning
//!
//! This module contains concrete implementations of inference rules.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::{InferStats, InferredRelation, KnowledgeGraph, Relation};

use super::InferenceRule;

/// Confidence decay factors for different relation types
fn get_decay_factor(relation_type: &str) -> f32 {
    match relation_type {
        "depends_on" | "contains" | "part_of" => 0.95,
        "implements" | "fixes" | "caused_by" => 0.90,
        "affects" | "assigned_to" | "blocked_by" => 0.85,
        "relates_to" | "supersedes" | "requires" => 0.70,
        _ => 0.60, // Unknown relation types get lower confidence
    }
}

/// Build an adjacency map for O(1) lookup of outgoing relations
/// This converts O(N) per-node lookup to O(1) using pre-built HashMap
fn build_adjacency_map(relations: &[Relation]) -> HashMap<&str, Vec<&Relation>> {
    let mut map: HashMap<&str, Vec<&Relation>> = HashMap::new();
    for relation in relations {
        map.entry(relation.from.as_str())
            .or_default()
            .push(relation);
    }
    map
}

/// Transitive Dependency Rule
///
/// Infers transitive relations using BFS traversal.
/// Example: If A depends_on B and B depends_on C, infer A transitively_depends_on C
///
/// Safety features:
/// - Max depth limit (default: 3)
/// - Cycle detection via HashSet
/// - Confidence decay per hop
/// - BFS for shortest-path-first (Occam's Razor)
/// - O(1) relation lookup via pre-built adjacency map
pub struct TransitiveDependencyRule {
    max_depth: usize,
}

impl TransitiveDependencyRule {
    /// Create a new rule with specified max depth
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Generate explanation for the inference path
    fn generate_explanation(path: &[String], relation_types: &[String]) -> String {
        if path.len() < 2 || relation_types.is_empty() {
            return String::new();
        }

        let mut explanation = format!("Inferred via path: {}", path[0]);
        for (i, node) in path.iter().skip(1).enumerate() {
            let rel_type = relation_types.get(i).map(|s| s.as_str()).unwrap_or("?");
            explanation.push_str(&format!(" -[{}]-> {}", rel_type, node));
        }
        explanation
    }
}

impl InferenceRule for TransitiveDependencyRule {
    fn name(&self) -> &str {
        "TransitiveDependencyRule"
    }

    fn apply(
        &self,
        graph: &KnowledgeGraph,
        target: &str,
        min_confidence: f32,
    ) -> (Vec<InferredRelation>, InferStats) {
        let mut inferred = Vec::new();
        let mut stats = InferStats::default();
        let mut visited: HashSet<String> = HashSet::new();

        // Check if target exists in graph
        if !graph.entities.iter().any(|e| e.name == target) {
            return (inferred, stats);
        }

        // Pre-build adjacency map for O(1) lookup instead of O(N) filter per node
        let adjacency_map = build_adjacency_map(&graph.relations);

        // BFS queue: (current_node, path, relation_types_in_path, confidence)
        let mut queue: VecDeque<(String, Vec<String>, Vec<String>, f32)> = VecDeque::new();
        queue.push_back((target.to_string(), vec![target.to_string()], vec![], 1.0));
        visited.insert(target.to_string());

        while let Some((current, path, rel_types, confidence)) = queue.pop_front() {
            stats.nodes_visited += 1;

            // Check depth limit (path includes start node, so depth = path.len() - 1)
            let current_depth = path.len() - 1;
            if current_depth >= self.max_depth {
                stats.max_depth_reached = stats.max_depth_reached.max(current_depth);
                continue;
            }

            // O(1) lookup of outgoing relations via adjacency map
            let outgoing = adjacency_map
                .get(current.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            for relation in outgoing {
                let next_node = &relation.to;

                // Skip if already visited (cycle detection)
                if visited.contains(next_node) {
                    continue;
                }

                // Calculate new confidence with decay
                let decay = get_decay_factor(&relation.relation_type);
                let new_confidence = confidence * decay;

                // Skip if below threshold
                if new_confidence < min_confidence {
                    continue;
                }

                // Update path
                let mut new_path = path.clone();
                new_path.push(next_node.clone());

                let mut new_rel_types = rel_types.clone();
                new_rel_types.push(relation.relation_type.clone());

                // If path length >= 3, we have a transitive relation (A -> B -> C)
                if new_path.len() >= 3 {
                    // Create inferred relation from start to current end
                    let inferred_relation = Relation {
                        from: target.to_string(),
                        to: next_node.clone(),
                        relation_type: format!("inferred_{}", new_rel_types.first().unwrap_or(&"relation".to_string())),
                        created_by: "InferenceEngine".to_string(),
                        created_at: crate::utils::current_timestamp(),
                        valid_from: None,
                        valid_to: None,
                    };

                    let explanation = Self::generate_explanation(&new_path, &new_rel_types);

                    inferred.push(InferredRelation {
                        relation: inferred_relation,
                        confidence: new_confidence,
                        rule_name: self.name().to_string(),
                        explanation,
                    });

                    stats.paths_found += 1;
                }

                // Mark as visited and add to queue for further exploration
                visited.insert(next_node.clone());
                stats.max_depth_reached = stats.max_depth_reached.max(new_path.len() - 1);

                queue.push_back((next_node.clone(), new_path, new_rel_types, new_confidence));
            }
        }

        (inferred, stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Entity;

    fn create_test_graph() -> KnowledgeGraph {
        // Create a simple chain: A -> B -> C -> D
        let entities = vec![
            Entity::new("A".to_string(), "Module".to_string()),
            Entity::new("B".to_string(), "Module".to_string()),
            Entity::new("C".to_string(), "Module".to_string()),
            Entity::new("D".to_string(), "Module".to_string()),
        ];

        let relations = vec![
            Relation::new("A".to_string(), "B".to_string(), "depends_on".to_string()),
            Relation::new("B".to_string(), "C".to_string(), "depends_on".to_string()),
            Relation::new("C".to_string(), "D".to_string(), "depends_on".to_string()),
        ];

        KnowledgeGraph { entities, relations }
    }

    #[test]
    fn test_simple_chain_inference() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);
        let (inferred, stats) = rule.apply(&graph, "A", 0.5);

        // Should infer A -> C (via B) and A -> D (via B, C)
        assert_eq!(inferred.len(), 2);
        assert!(stats.paths_found >= 2);

        // Check A -> C
        let a_to_c = inferred.iter().find(|i| i.relation.to == "C");
        assert!(a_to_c.is_some());
        let a_to_c = a_to_c.unwrap();
        assert!(a_to_c.confidence > 0.8); // 0.95 * 0.95 = 0.9025

        // Check A -> D
        let a_to_d = inferred.iter().find(|i| i.relation.to == "D");
        assert!(a_to_d.is_some());
        let a_to_d = a_to_d.unwrap();
        assert!(a_to_d.confidence > 0.7); // 0.95^3 = 0.857
    }

    #[test]
    fn test_depth_limit() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(2); // Only 2 hops
        let (inferred, _stats) = rule.apply(&graph, "A", 0.5);

        // Should only infer A -> C, not A -> D (would need 3 hops)
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].relation.to, "C");
    }

    #[test]
    fn test_cycle_detection() {
        // Create a graph with cycle: A -> B -> C -> A
        let entities = vec![
            Entity::new("A".to_string(), "Module".to_string()),
            Entity::new("B".to_string(), "Module".to_string()),
            Entity::new("C".to_string(), "Module".to_string()),
        ];

        let relations = vec![
            Relation::new("A".to_string(), "B".to_string(), "depends_on".to_string()),
            Relation::new("B".to_string(), "C".to_string(), "depends_on".to_string()),
            Relation::new("C".to_string(), "A".to_string(), "depends_on".to_string()),
        ];

        let graph = KnowledgeGraph { entities, relations };
        let rule = TransitiveDependencyRule::new(10); // High depth to test cycle detection
        let (inferred, stats) = rule.apply(&graph, "A", 0.1);

        // Should NOT loop infinitely - cycle detection kicks in
        // Should infer A -> C (2 hops) but NOT loop back
        assert!(stats.nodes_visited <= 3); // Only 3 nodes exist
        assert!(!inferred.iter().any(|i| i.relation.to == "A")); // No self-inference
    }

    #[test]
    fn test_confidence_threshold() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);

        // High threshold should filter out longer paths
        let (inferred, _) = rule.apply(&graph, "A", 0.9);
        // 0.95 * 0.95 = 0.9025 > 0.9 (A->C passes)
        // 0.95 * 0.95 * 0.95 = 0.857 < 0.9 (A->D fails)
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].relation.to, "C");
    }

    #[test]
    fn test_nonexistent_target() {
        let graph = create_test_graph();
        let rule = TransitiveDependencyRule::new(3);
        let (inferred, stats) = rule.apply(&graph, "NonExistent", 0.5);

        assert!(inferred.is_empty());
        assert_eq!(stats.nodes_visited, 0);
    }

    #[test]
    fn test_explanation_generation() {
        let path = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let rel_types = vec!["depends_on".to_string(), "depends_on".to_string()];
        let explanation = TransitiveDependencyRule::generate_explanation(&path, &rel_types);

        assert!(explanation.contains("A"));
        assert!(explanation.contains("B"));
        assert!(explanation.contains("C"));
        assert!(explanation.contains("depends_on"));
    }

    #[test]
    fn test_decay_factors() {
        assert_eq!(get_decay_factor("depends_on"), 0.95);
        assert_eq!(get_decay_factor("implements"), 0.90);
        assert_eq!(get_decay_factor("affects"), 0.85);
        assert_eq!(get_decay_factor("relates_to"), 0.70);
        assert_eq!(get_decay_factor("unknown_type"), 0.60);
    }
}
