//! Query operations for the knowledge base

use std::collections::HashSet;

use crate::search::{get_synonyms, matches_with_synonyms};
use crate::types::{Entity, KnowledgeGraph, McpResult, Relation};

use super::KnowledgeBase;

/// Read graph with optional pagination
pub fn read_graph(
    kb: &KnowledgeBase,
    limit: Option<usize>,
    offset: Option<usize>,
) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;

    let offset = offset.unwrap_or(0);

    let entities: Vec<Entity> = if let Some(lim) = limit {
        graph.entities.into_iter().skip(offset).take(lim).collect()
    } else {
        graph.entities.into_iter().skip(offset).collect()
    };

    let entity_names: HashSet<String> = entities.iter().map(|e| e.name.clone()).collect();

    let relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
        .collect();

    Ok(KnowledgeGraph { entities, relations })
}

/// Search nodes by query with synonym expansion, optional limit and relation inclusion
pub fn search_nodes(
    kb: &KnowledgeBase,
    query: &str,
    limit: Option<usize>,
    include_relations: bool,
) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;

    // Expand query with synonyms for semantic matching
    let search_terms = get_synonyms(query);

    let mut matching_entities: Vec<Entity> = graph
        .entities
        .into_iter()
        .filter(|e| {
            matches_with_synonyms(&e.name, &search_terms)
                || matches_with_synonyms(&e.entity_type, &search_terms)
                || e.observations
                    .iter()
                    .any(|o| matches_with_synonyms(o, &search_terms))
        })
        .collect();

    // Apply limit if specified
    if let Some(lim) = limit {
        matching_entities.truncate(lim);
    }

    let matching_relations = if include_relations {
        let entity_names: HashSet<String> =
            matching_entities.iter().map(|e| e.name.clone()).collect();
        graph
            .relations
            .into_iter()
            .filter(|r| entity_names.contains(&r.from) || entity_names.contains(&r.to))
            .collect()
    } else {
        Vec::new()
    };

    Ok(KnowledgeGraph {
        entities: matching_entities,
        relations: matching_relations,
    })
}

/// Open specific nodes by names
pub fn open_nodes(kb: &KnowledgeBase, names: Vec<String>) -> McpResult<KnowledgeGraph> {
    let graph = kb.load_graph()?;
    let name_set: HashSet<String> = names.into_iter().collect();

    let matching_entities: Vec<Entity> = graph
        .entities
        .into_iter()
        .filter(|e| name_set.contains(&e.name))
        .collect();

    let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();

    let matching_relations: Vec<Relation> = graph
        .relations
        .into_iter()
        .filter(|r| entity_names.contains(&r.from) && entity_names.contains(&r.to))
        .collect();

    Ok(KnowledgeGraph {
        entities: matching_entities,
        relations: matching_relations,
    })
}
