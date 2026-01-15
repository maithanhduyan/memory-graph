//! Query operations for the knowledge base
//!
//! # Phase 1 Search Optimization
//!
//! Uses inverted index for O(1) lookups instead of O(n) linear scans.
//! Falls back to linear scan for complex queries or if index is unavailable.

use std::collections::HashSet;

use rayon::prelude::*;

use crate::search::{get_synonyms, matches_with_synonyms};
use crate::types::{Entity, KnowledgeGraph, McpResult, Relation};

use super::KnowledgeBase;

/// Threshold for using parallel search (entities count)
const PARALLEL_SEARCH_THRESHOLD: usize = 1000;

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
///
/// # Optimization Strategy (Phase 1)
/// 1. Use inverted index for fast initial candidate lookup
/// 2. Expand synonyms and search across all synonym terms
/// 3. Use parallel iteration for large result sets (>1000 entities)
pub fn search_nodes(
    kb: &KnowledgeBase,
    query: &str,
    limit: Option<usize>,
    include_relations: bool,
) -> McpResult<KnowledgeGraph> {
    // Expand query with synonyms for semantic matching
    let search_terms = get_synonyms(query);

    // Try indexed search first (Phase 1 optimization)
    let matching_entities = if let Ok(index) = kb.search_index.read() {
        // Use inverted index for O(1) lookup
        let candidate_names = index.search_terms(&search_terms);

        if !candidate_names.is_empty() {
            // Get entities from index
            let mut entities = index.get_entities(&candidate_names);

            // Apply limit if specified
            if let Some(lim) = limit {
                entities.truncate(lim);
            }

            entities
        } else {
            // Fallback to linear scan if no index matches
            fallback_linear_search(kb, &search_terms, limit)?
        }
    } else {
        // Index unavailable, use linear scan
        fallback_linear_search(kb, &search_terms, limit)?
    };

    // Get matching relations if requested
    let matching_relations = if include_relations {
        let graph = kb.load_graph()?;
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

/// Fallback linear search when index is not available or empty
fn fallback_linear_search(
    kb: &KnowledgeBase,
    search_terms: &[String],
    limit: Option<usize>,
) -> McpResult<Vec<Entity>> {
    let graph = kb.load_graph()?;

    // Use parallel search for large graphs
    let mut matching_entities: Vec<Entity> = if graph.entities.len() > PARALLEL_SEARCH_THRESHOLD {
        // Parallel search using Rayon
        graph
            .entities
            .par_iter()
            .filter(|e| {
                matches_with_synonyms(&e.name, search_terms)
                    || matches_with_synonyms(&e.entity_type, search_terms)
                    || e.observations
                        .iter()
                        .any(|o| matches_with_synonyms(o, search_terms))
            })
            .cloned()
            .collect()
    } else {
        // Sequential search for small graphs
        graph
            .entities
            .into_iter()
            .filter(|e| {
                matches_with_synonyms(&e.name, search_terms)
                    || matches_with_synonyms(&e.entity_type, search_terms)
                    || e.observations
                        .iter()
                        .any(|o| matches_with_synonyms(o, search_terms))
            })
            .collect()
    };

    // Apply limit if specified
    if let Some(lim) = limit {
        matching_entities.truncate(lim);
    }

    Ok(matching_entities)
}

/// Open specific nodes by names
/// Uses index for O(1) lookup when available
pub fn open_nodes(kb: &KnowledgeBase, names: Vec<String>) -> McpResult<KnowledgeGraph> {
    let name_set: HashSet<String> = names.into_iter().collect();

    // Try indexed lookup first (Phase 1 optimization)
    let matching_entities: Vec<Entity> = if let Ok(index) = kb.search_index.read() {
        // O(1) lookup per entity from index
        index.get_entities(&name_set)
    } else {
        // Fallback to linear scan
        let graph = kb.load_graph()?;
        graph
            .entities
            .into_iter()
            .filter(|e| name_set.contains(&e.name))
            .collect()
    };

    let entity_names: HashSet<String> = matching_entities.iter().map(|e| e.name.clone()).collect();

    // Load relations (still need graph access for relations)
    let graph = kb.load_graph()?;
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
