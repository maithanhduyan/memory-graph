//! Knowledge Base - Core data engine
//!
//! This module contains the main knowledge base implementation with
//! thread-safe CRUD operations, queries, and temporal features.

mod crud;
mod query;
mod summarize;
mod temporal;
mod traversal;

use std::env;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::types::{
    Entity, KnowledgeGraph, McpResult, Observation, ObservationDeletion,
    PathStep, RelatedEntities, Relation, Summary, TraversalResult,
};
use crate::utils::time::get_current_user;

/// Knowledge base with in-memory cache for thread-safe operations
pub struct KnowledgeBase {
    pub(crate) memory_file_path: String,
    pub(crate) graph: Mutex<KnowledgeGraph>,
    pub(crate) current_user: String,
}

impl KnowledgeBase {
    /// Create a new knowledge base instance
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let default_memory_path = current_dir.join("memory.jsonl");

        let memory_file_path = match env::var("MEMORY_FILE_PATH") {
            Ok(path) => {
                if Path::new(&path).is_absolute() {
                    path
                } else {
                    current_dir.join(path).to_string_lossy().to_string()
                }
            }
            Err(_) => default_memory_path.to_string_lossy().to_string(),
        };

        // Detect current user once at startup
        let current_user = get_current_user();

        // Load graph from file at startup (or create empty if not exists)
        let graph = Self::load_graph_from_file(&memory_file_path).unwrap_or_default();

        Self {
            memory_file_path,
            graph: Mutex::new(graph),
            current_user,
        }
    }

    /// Create a new knowledge base with custom file path
    pub fn with_file_path(file_path: String) -> Self {
        let current_user = get_current_user();
        let graph = Self::load_graph_from_file(&file_path).unwrap_or_default();

        Self {
            memory_file_path: file_path,
            graph: Mutex::new(graph),
            current_user,
        }
    }

    /// Create a new knowledge base for testing with explicit parameters
    #[cfg(test)]
    pub fn for_testing(file_path: String, user: String) -> Self {
        Self {
            memory_file_path: file_path,
            graph: Mutex::new(KnowledgeGraph::default()),
            current_user: user,
        }
    }

    /// Load graph from file (static helper for initialization)
    fn load_graph_from_file(file_path: &str) -> McpResult<KnowledgeGraph> {
        if !Path::new(file_path).exists() {
            return Ok(KnowledgeGraph::default());
        }

        let content = fs::read_to_string(file_path)?;
        let mut graph = KnowledgeGraph::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entity) = serde_json::from_str::<Entity>(line) {
                if !entity.name.is_empty() && !entity.entity_type.is_empty() {
                    graph.entities.push(entity);
                    continue;
                }
            }

            if let Ok(relation) = serde_json::from_str::<Relation>(line) {
                if !relation.from.is_empty() && !relation.to.is_empty() {
                    graph.relations.push(relation);
                }
            }
        }

        Ok(graph)
    }

    /// Get a clone of the current graph (thread-safe read)
    pub(crate) fn load_graph(&self) -> McpResult<KnowledgeGraph> {
        Ok(self.graph.lock().unwrap().clone())
    }

    /// Persist graph to file (internal helper, expects caller to hold lock)
    pub(crate) fn persist_to_file(&self, graph: &KnowledgeGraph) -> McpResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.memory_file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let mut content = String::new();

        for entity in &graph.entities {
            content.push_str(&serde_json::to_string(entity)?);
            content.push('\n');
        }

        for relation in &graph.relations {
            content.push_str(&serde_json::to_string(relation)?);
            content.push('\n');
        }

        fs::write(&self.memory_file_path, content)?;
        Ok(())
    }

    /// Get the current user
    pub fn current_user(&self) -> &str {
        &self.current_user
    }

    /// Get the memory file path
    pub fn file_path(&self) -> &str {
        &self.memory_file_path
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export methods from submodules by implementing them here
impl KnowledgeBase {
    // CRUD operations (from crud.rs)
    pub fn create_entities(&self, entities: Vec<Entity>) -> McpResult<Vec<Entity>> {
        crud::create_entities(self, entities)
    }

    pub fn create_relations(&self, relations: Vec<Relation>) -> McpResult<Vec<Relation>> {
        crud::create_relations(self, relations)
    }

    pub fn add_observations(&self, observations: Vec<Observation>) -> McpResult<Vec<Observation>> {
        crud::add_observations(self, observations)
    }

    pub fn delete_entities(&self, entity_names: Vec<String>) -> McpResult<()> {
        crud::delete_entities(self, entity_names)
    }

    pub fn delete_observations(&self, deletions: Vec<ObservationDeletion>) -> McpResult<()> {
        crud::delete_observations(self, deletions)
    }

    pub fn delete_relations(&self, relations: Vec<Relation>) -> McpResult<()> {
        crud::delete_relations(self, relations)
    }

    // Query operations (from query.rs)
    pub fn read_graph(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> McpResult<KnowledgeGraph> {
        query::read_graph(self, limit, offset)
    }

    pub fn search_nodes(
        &self,
        query: &str,
        limit: Option<usize>,
        include_relations: bool,
    ) -> McpResult<KnowledgeGraph> {
        query::search_nodes(self, query, limit, include_relations)
    }

    pub fn open_nodes(&self, names: Vec<String>) -> McpResult<KnowledgeGraph> {
        query::open_nodes(self, names)
    }

    // Traversal operations (from traversal.rs)
    pub fn get_related(
        &self,
        entity_name: &str,
        relation_type: Option<&str>,
        direction: &str,
    ) -> McpResult<RelatedEntities> {
        traversal::get_related(self, entity_name, relation_type, direction)
    }

    pub fn traverse(
        &self,
        start: &str,
        path: Vec<PathStep>,
        max_results: usize,
    ) -> McpResult<TraversalResult> {
        traversal::traverse(self, start, path, max_results)
    }

    // Summarize operations (from summarize.rs)
    pub fn summarize(
        &self,
        entity_names: Option<Vec<String>>,
        entity_type: Option<String>,
        format: &str,
    ) -> McpResult<Summary> {
        summarize::summarize(self, entity_names, entity_type, format)
    }

    // Temporal operations (from temporal.rs)
    pub fn get_relations_at_time(
        &self,
        timestamp: Option<u64>,
        entity_name: Option<&str>,
    ) -> McpResult<Vec<Relation>> {
        temporal::get_relations_at_time(self, timestamp, entity_name)
    }

    pub fn get_relation_history(&self, entity_name: &str) -> McpResult<Vec<Relation>> {
        temporal::get_relation_history(self, entity_name)
    }
}
