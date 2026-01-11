//! Knowledge graph container type

use serde::{Deserialize, Serialize};

use super::{Entity, Relation};

/// Knowledge graph containing entities and relations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

impl KnowledgeGraph {
    /// Create an empty knowledge graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a knowledge graph with entities and relations
    pub fn with_data(entities: Vec<Entity>, relations: Vec<Relation>) -> Self {
        Self { entities, relations }
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relations.is_empty()
    }

    /// Get the number of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get the number of relations
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}
