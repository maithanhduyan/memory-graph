//! Summarize operations

use std::collections::HashMap;

use crate::types::{Entity, EntityBrief, McpResult, Summary};

use super::KnowledgeBase;

/// Summarize entities
pub fn summarize(
    kb: &KnowledgeBase,
    entity_names: Option<Vec<String>>,
    entity_type: Option<String>,
    format: &str,
) -> McpResult<Summary> {
    let graph = kb.load_graph()?;

    let entities: Vec<&Entity> = graph
        .entities
        .iter()
        .filter(|e| {
            if let Some(ref names) = entity_names {
                names.contains(&e.name)
            } else if let Some(ref et) = entity_type {
                &e.entity_type == et
            } else {
                true
            }
        })
        .collect();

    match format {
        "brief" => format_brief(&entities),
        "detailed" => format_detailed(&entities),
        "stats" => format_stats(&entities),
        _ => format_brief(&entities),
    }
}

fn format_brief(entities: &[&Entity]) -> McpResult<Summary> {
    let briefs: Vec<EntityBrief> = entities
        .iter()
        .map(|e| {
            let brief = e
                .observations
                .first()
                .cloned()
                .unwrap_or_default()
                .chars()
                .take(100)
                .collect::<String>();
            EntityBrief {
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                brief,
            }
        })
        .collect();

    Ok(Summary {
        total_entities: entities.len(),
        entities: Some(briefs),
        ..Default::default()
    })
}

fn format_detailed(entities: &[&Entity]) -> McpResult<Summary> {
    let briefs: Vec<EntityBrief> = entities
        .iter()
        .map(|e| {
            let brief = e.observations.join("; ");
            EntityBrief {
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                brief,
            }
        })
        .collect();

    Ok(Summary {
        total_entities: entities.len(),
        entities: Some(briefs),
        ..Default::default()
    })
}

fn format_stats(entities: &[&Entity]) -> McpResult<Summary> {
    let mut by_status: HashMap<String, usize> = HashMap::new();
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_priority: HashMap<String, usize> = HashMap::new();

    for entity in entities {
        *by_type.entry(entity.entity_type.clone()).or_insert(0) += 1;

        for obs in &entity.observations {
            if obs.starts_with("Status:") {
                let status = obs.trim_start_matches("Status:").trim().to_string();
                *by_status.entry(status).or_insert(0) += 1;
            }
            if obs.starts_with("Priority:") {
                let priority = obs.trim_start_matches("Priority:").trim().to_string();
                *by_priority.entry(priority).or_insert(0) += 1;
            }
        }
    }

    Ok(Summary {
        total_entities: entities.len(),
        entities: None,
        by_status: if by_status.is_empty() {
            None
        } else {
            Some(by_status)
        },
        by_type: Some(by_type),
        by_priority: if by_priority.is_empty() {
            None
        } else {
            Some(by_priority)
        },
    })
}
