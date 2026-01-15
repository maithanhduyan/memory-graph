//! Search Index for fast entity lookups
//!
//! This module provides an inverted index for O(1) search operations
//! instead of O(n) linear scans.

use std::collections::{HashMap, HashSet};

use crate::types::{Entity, KnowledgeGraph};

/// Inverted index for fast full-text search
#[derive(Debug, Default, Clone)]
pub struct SearchIndex {
    /// word (lowercased) → Set<entity_name>
    inverted_index: HashMap<String, HashSet<String>>,

    /// entity_type → Set<entity_name>
    type_index: HashMap<String, HashSet<String>>,

    /// entity_name → entity (for O(1) lookup)
    entity_map: HashMap<String, Entity>,

    /// Total indexed entities count
    entity_count: usize,
}

impl SearchIndex {
    /// Create a new empty search index
    pub fn new() -> Self {
        Self::default()
    }

    /// Build index from a knowledge graph
    pub fn from_graph(graph: &KnowledgeGraph) -> Self {
        let mut index = Self::new();
        index.rebuild(graph);
        index
    }

    /// Rebuild the entire index from scratch
    pub fn rebuild(&mut self, graph: &KnowledgeGraph) {
        self.inverted_index.clear();
        self.type_index.clear();
        self.entity_map.clear();

        for entity in &graph.entities {
            self.index_entity(entity);
        }

        self.entity_count = graph.entities.len();
    }

    /// Index a single entity
    pub fn index_entity(&mut self, entity: &Entity) {
        let name = entity.name.clone();

        // Store entity in map for O(1) lookup
        self.entity_map.insert(name.clone(), entity.clone());

        // Index entity name tokens
        for token in tokenize(&entity.name) {
            self.inverted_index
                .entry(token)
                .or_default()
                .insert(name.clone());
        }

        // Index entity type
        let type_lower = entity.entity_type.to_lowercase();
        self.type_index
            .entry(type_lower.clone())
            .or_default()
            .insert(name.clone());

        // Also index type as searchable token
        for token in tokenize(&entity.entity_type) {
            self.inverted_index
                .entry(token)
                .or_default()
                .insert(name.clone());
        }

        // Index observations
        for obs in &entity.observations {
            for token in tokenize(obs) {
                self.inverted_index
                    .entry(token)
                    .or_default()
                    .insert(name.clone());
            }
        }
    }

    /// Remove an entity from the index
    pub fn remove_entity(&mut self, entity_name: &str) {
        if let Some(entity) = self.entity_map.remove(entity_name) {
            // Remove from inverted index
            for token in tokenize(&entity.name) {
                if let Some(set) = self.inverted_index.get_mut(&token) {
                    set.remove(entity_name);
                    if set.is_empty() {
                        self.inverted_index.remove(&token);
                    }
                }
            }

            // Remove from type index
            let type_lower = entity.entity_type.to_lowercase();
            if let Some(set) = self.type_index.get_mut(&type_lower) {
                set.remove(entity_name);
                if set.is_empty() {
                    self.type_index.remove(&type_lower);
                }
            }

            // Remove observation tokens
            for obs in &entity.observations {
                for token in tokenize(obs) {
                    if let Some(set) = self.inverted_index.get_mut(&token) {
                        set.remove(entity_name);
                    }
                }
            }

            self.entity_count = self.entity_count.saturating_sub(1);
        }
    }

    /// Update an entity in the index (remove old, add new)
    pub fn update_entity(&mut self, entity: &Entity) {
        self.remove_entity(&entity.name);
        self.index_entity(entity);
    }

    /// Search for entities matching a query term
    /// Returns entity names that contain the term
    pub fn search(&self, query: &str) -> HashSet<String> {
        let query_lower = query.to_lowercase();

        // Direct token match
        if let Some(matches) = self.inverted_index.get(&query_lower) {
            return matches.clone();
        }

        // Partial match - search tokens that contain query
        let mut results = HashSet::new();
        for (token, names) in &self.inverted_index {
            if token.contains(&query_lower) {
                results.extend(names.clone());
            }
        }

        results
    }

    /// Search with multiple terms (returns union of all matches)
    pub fn search_terms(&self, terms: &[String]) -> HashSet<String> {
        let mut results = HashSet::new();
        for term in terms {
            results.extend(self.search(term));
        }
        results
    }

    /// Get entities by type
    pub fn get_by_type(&self, entity_type: &str) -> HashSet<String> {
        self.type_index
            .get(&entity_type.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Get entity by name (O(1) lookup)
    pub fn get_entity(&self, name: &str) -> Option<&Entity> {
        self.entity_map.get(name)
    }

    /// Get multiple entities by names
    pub fn get_entities(&self, names: &HashSet<String>) -> Vec<Entity> {
        names
            .iter()
            .filter_map(|name| self.entity_map.get(name).cloned())
            .collect()
    }

    /// Check if an entity exists
    pub fn contains(&self, name: &str) -> bool {
        self.entity_map.contains_key(name)
    }

    /// Get total indexed entities
    pub fn len(&self) -> usize {
        self.entity_count
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }

    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            entity_count: self.entity_count,
            unique_tokens: self.inverted_index.len(),
            unique_types: self.type_index.len(),
        }
    }
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub entity_count: usize,
    pub unique_tokens: usize,
    pub unique_types: usize,
}

/// Tokenize text into searchable tokens
/// Splits on whitespace and punctuation, lowercases
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty() && s.len() >= 2) // Skip very short tokens
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(name: &str, entity_type: &str, observations: Vec<&str>) -> Entity {
        Entity {
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            observations: observations.into_iter().map(|s| s.to_string()).collect(),
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn test_index_and_search() {
        let mut index = SearchIndex::new();

        let entity = make_entity("Alice", "Person", vec!["Software developer", "Loves Rust"]);
        index.index_entity(&entity);

        // Search by name
        let results = index.search("alice");
        assert!(results.contains("Alice"));

        // Search by observation
        let results = index.search("developer");
        assert!(results.contains("Alice"));

        // Search by type
        let results = index.search("person");
        assert!(results.contains("Alice"));
    }

    #[test]
    fn test_type_index() {
        let mut index = SearchIndex::new();

        index.index_entity(&make_entity("Alice", "Person", vec![]));
        index.index_entity(&make_entity("Bob", "Person", vec![]));
        index.index_entity(&make_entity("Project X", "Project", vec![]));

        let persons = index.get_by_type("Person");
        assert_eq!(persons.len(), 2);
        assert!(persons.contains("Alice"));
        assert!(persons.contains("Bob"));

        let projects = index.get_by_type("Project");
        assert_eq!(projects.len(), 1);
        assert!(projects.contains("Project X"));
    }

    #[test]
    fn test_remove_entity() {
        let mut index = SearchIndex::new();

        index.index_entity(&make_entity("Alice", "Person", vec!["developer"]));
        assert!(index.contains("Alice"));
        assert!(!index.search("developer").is_empty());

        index.remove_entity("Alice");
        assert!(!index.contains("Alice"));
        // After removal, search should not find Alice
        let results = index.search("alice");
        assert!(!results.contains("Alice"));
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a TEST.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"this".to_string()));
        // Short tokens filtered (len < 2)
        assert!(!tokens.contains(&"a".to_string()));
        // "is" is 2 chars so it passes the filter
        assert!(tokens.contains(&"is".to_string()));
    }
}
