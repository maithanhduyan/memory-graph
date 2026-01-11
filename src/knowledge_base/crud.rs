//! CRUD operations for the knowledge base

use std::collections::HashSet;

use crate::types::{Entity, McpResult, Observation, ObservationDeletion, Relation};
use crate::utils::time::current_timestamp;

use super::KnowledgeBase;

/// Create new entities (thread-safe: holds write lock during entire operation)
pub fn create_entities(kb: &KnowledgeBase, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
    let mut graph = kb.graph.write().unwrap();
    let existing_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
    let now = current_timestamp();

    let mut created = Vec::new();
    for mut entity in entities {
        if !existing_names.contains(&entity.name) {
            // Auto-fill user info if not provided
            if entity.created_by.is_empty() || entity.created_by == "system" {
                entity.created_by = kb.current_user.clone();
            }
            if entity.updated_by.is_empty() || entity.updated_by == "system" {
                entity.updated_by = kb.current_user.clone();
            }
            entity.created_at = now;
            entity.updated_at = now;
            created.push(entity.clone());
            graph.entities.push(entity);
        }
    }

    kb.persist_to_file(&graph)?;
    Ok(created)
}

/// Create new relations (thread-safe: holds write lock during entire operation)
pub fn create_relations(kb: &KnowledgeBase, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
    let mut graph = kb.graph.write().unwrap();
    let entity_names: HashSet<String> = graph.entities.iter().map(|e| e.name.clone()).collect();
    let now = current_timestamp();

    let existing_relations: HashSet<String> = graph
        .relations
        .iter()
        .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
        .collect();

    let mut created = Vec::new();
    for mut relation in relations {
        if entity_names.contains(&relation.from) && entity_names.contains(&relation.to) {
            let key = format!(
                "{}|{}|{}",
                relation.from, relation.to, relation.relation_type
            );
            if !existing_relations.contains(&key) {
                // Auto-fill user info if not provided
                if relation.created_by.is_empty() || relation.created_by == "system" {
                    relation.created_by = kb.current_user.clone();
                }
                relation.created_at = now;
                created.push(relation.clone());
                graph.relations.push(relation);
            }
        }
    }

    kb.persist_to_file(&graph)?;
    Ok(created)
}

/// Add observations to entities (thread-safe: holds write lock during entire operation)
pub fn add_observations(
    kb: &KnowledgeBase,
    observations: Vec<Observation>,
) -> McpResult<Vec<Observation>> {
    let mut graph = kb.graph.write().unwrap();
    let mut added = Vec::new();
    let now = current_timestamp();

    for obs in observations {
        if let Some(entity) = graph.entities.iter_mut().find(|e| e.name == obs.entity_name) {
            let existing: HashSet<String> = entity.observations.iter().cloned().collect();
            let mut new_contents = Vec::new();

            for content in &obs.contents {
                if !existing.contains(content) {
                    entity.observations.push(content.clone());
                    new_contents.push(content.clone());
                }
            }

            if !new_contents.is_empty() {
                entity.updated_at = now;
                entity.updated_by = kb.current_user.clone();
                added.push(Observation {
                    entity_name: obs.entity_name.clone(),
                    contents: new_contents,
                });
            }
        }
    }

    kb.persist_to_file(&graph)?;
    Ok(added)
}

/// Delete entities (thread-safe: holds write lock during entire operation)
pub fn delete_entities(kb: &KnowledgeBase, entity_names: Vec<String>) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();
    let names_to_delete: HashSet<String> = entity_names.into_iter().collect();

    graph
        .entities
        .retain(|e| !names_to_delete.contains(&e.name));
    graph
        .relations
        .retain(|r| !names_to_delete.contains(&r.from) && !names_to_delete.contains(&r.to));

    kb.persist_to_file(&graph)?;
    Ok(())
}

/// Delete observations from entities (thread-safe: holds write lock during entire operation)
pub fn delete_observations(
    kb: &KnowledgeBase,
    deletions: Vec<ObservationDeletion>,
) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();

    for deletion in deletions {
        if let Some(entity) = graph
            .entities
            .iter_mut()
            .find(|e| e.name == deletion.entity_name)
        {
            let to_remove: HashSet<String> = deletion.observations.into_iter().collect();
            entity.observations.retain(|o| !to_remove.contains(o));
        }
    }

    kb.persist_to_file(&graph)?;
    Ok(())
}

/// Delete relations (thread-safe: holds write lock during entire operation)
pub fn delete_relations(kb: &KnowledgeBase, relations: Vec<Relation>) -> McpResult<()> {
    let mut graph = kb.graph.write().unwrap();

    let to_delete: HashSet<String> = relations
        .iter()
        .map(|r| format!("{}|{}|{}", r.from, r.to, r.relation_type))
        .collect();

    graph.relations.retain(|r| {
        let key = format!("{}|{}|{}", r.from, r.to, r.relation_type);
        !to_delete.contains(&key)
    });

    kb.persist_to_file(&graph)?;
    Ok(())
}
