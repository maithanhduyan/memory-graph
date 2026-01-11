//! Temporal query operations

use crate::types::{McpResult, Relation};
use crate::utils::time::current_timestamp;

use super::KnowledgeBase;

/// Get relations valid at a specific point in time
pub fn get_relations_at_time(
    kb: &KnowledgeBase,
    timestamp: Option<u64>,
    entity_name: Option<&str>,
) -> McpResult<Vec<Relation>> {
    let graph = kb.load_graph()?;
    let check_time = timestamp.unwrap_or_else(current_timestamp);

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| {
            // Filter by entity if specified
            if let Some(name) = entity_name {
                if r.from != name && r.to != name {
                    return false;
                }
            }

            // Check temporal validity
            let valid_from_ok = match r.valid_from {
                Some(vf) => check_time >= vf,
                None => true, // No start time means always valid from past
            };

            let valid_to_ok = match r.valid_to {
                Some(vt) => check_time <= vt,
                None => true, // No end time means still valid
            };

            valid_from_ok && valid_to_ok
        })
        .collect();

    Ok(relations)
}

/// Get historical relations (including expired ones)
pub fn get_relation_history(kb: &KnowledgeBase, entity_name: &str) -> McpResult<Vec<Relation>> {
    let graph = kb.load_graph()?;

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| r.from == entity_name || r.to == entity_name)
        .collect();

    Ok(relations)
}
