//! Graph traversal operations

use std::collections::HashSet;

use crate::types::{
    Entity, McpResult, PathStep, RelatedEntities, RelatedEntity, TraversalPath, TraversalResult,
};

use super::KnowledgeBase;

/// Get related entities
pub fn get_related(
    kb: &KnowledgeBase,
    entity_name: &str,
    relation_type: Option<&str>,
    direction: &str,
) -> McpResult<RelatedEntities> {
    let graph = kb.load_graph()?;
    let mut related = Vec::new();

    for relation in &graph.relations {
        let matches = match direction {
            "outgoing" => relation.from == entity_name,
            "incoming" => relation.to == entity_name,
            "both" => relation.from == entity_name || relation.to == entity_name,
            _ => false,
        };

        if !matches {
            continue;
        }

        if let Some(rt) = relation_type {
            if relation.relation_type != rt {
                continue;
            }
        }

        let target_name = if relation.from == entity_name {
            &relation.to
        } else {
            &relation.from
        };

        if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name) {
            related.push(RelatedEntity {
                relation_type: relation.relation_type.clone(),
                direction: if relation.from == entity_name {
                    "outgoing".to_string()
                } else {
                    "incoming".to_string()
                },
                entity: entity.clone(),
            });
        }
    }

    Ok(RelatedEntities {
        entity: entity_name.to_string(),
        relations: related,
    })
}

/// Traverse graph following path pattern
pub fn traverse(
    kb: &KnowledgeBase,
    start: &str,
    path: Vec<PathStep>,
    max_results: usize,
) -> McpResult<TraversalResult> {
    let graph = kb.load_graph()?;

    // Track paths: (current_node, path_so_far, relations_so_far)
    let mut current_paths: Vec<(String, Vec<String>, Vec<String>)> =
        vec![(start.to_string(), vec![start.to_string()], vec![])];

    for step in &path {
        let mut next_paths = Vec::new();

        for (node, nodes_path, rels_path) in &current_paths {
            // Find related entities for this step
            for relation in &graph.relations {
                let (matches, target_name) = match step.direction.as_str() {
                    "out" => {
                        if relation.from == *node && relation.relation_type == step.relation_type {
                            (true, &relation.to)
                        } else {
                            (false, &relation.to)
                        }
                    }
                    "in" => {
                        if relation.to == *node && relation.relation_type == step.relation_type {
                            (true, &relation.from)
                        } else {
                            (false, &relation.from)
                        }
                    }
                    _ => (false, &relation.to),
                };

                if !matches {
                    continue;
                }

                // Check target type if specified
                if let Some(ref target_type) = step.target_type {
                    if let Some(entity) = graph.entities.iter().find(|e| e.name == *target_name) {
                        if &entity.entity_type != target_type {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                let mut new_nodes = nodes_path.clone();
                new_nodes.push(target_name.clone());
                let mut new_rels = rels_path.clone();
                new_rels.push(step.relation_type.clone());

                next_paths.push((target_name.clone(), new_nodes, new_rels));
            }
        }

        if next_paths.len() > max_results {
            next_paths.truncate(max_results);
        }

        current_paths = next_paths;
    }

    // Build result
    let mut paths = Vec::new();
    let mut end_node_names: HashSet<String> = HashSet::new();

    for (end_node, nodes, rels) in current_paths {
        end_node_names.insert(end_node);
        paths.push(TraversalPath {
            nodes,
            relations: rels,
        });
    }

    let end_nodes: Vec<Entity> = graph
        .entities
        .iter()
        .filter(|e| end_node_names.contains(&e.name))
        .cloned()
        .collect();

    Ok(TraversalResult {
        start_node: start.to_string(),
        paths,
        end_nodes,
    })
}
