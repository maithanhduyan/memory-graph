//! Knowledge Base - Core data engine
//!
//! This module contains the main knowledge base implementation with
//! thread-safe CRUD operations, queries, and temporal features.
//!
//! # Event Sourcing
//!
//! The knowledge base now supports Event Sourcing mode where all mutations
//! are recorded as immutable events. Set `MEMORY_EVENT_SOURCING=true` to enable.

mod crud;
pub mod inference;
mod query;
mod summarize;
mod temporal;
mod traversal;

use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, RwLock};

use crate::event_store::{EventStore, EventStoreConfig, LogRotation, SnapshotManager};
use crate::types::{
    Entity, EventType, KnowledgeGraph, McpResult, Observation, ObservationDeletion, PathStep,
    RelatedEntities, Relation, Summary, TraversalResult,
};
use crate::utils::time::get_current_user;

/// Knowledge base with in-memory cache for thread-safe operations
/// Uses RwLock for better concurrent read performance (read-heavy workload)
pub struct KnowledgeBase {
    pub(crate) memory_file_path: String,
    pub(crate) graph: RwLock<KnowledgeGraph>,
    pub(crate) current_user: String,
    /// Event store for Event Sourcing (None = legacy mode)
    pub(crate) event_store: Option<Mutex<EventStore>>,
    /// Snapshot manager for creating/loading snapshots
    pub(crate) snapshot_manager: Option<SnapshotManager>,
    /// Log rotation manager for archiving old events
    pub(crate) log_rotation: Option<LogRotation>,
    /// Whether Event Sourcing mode is enabled
    pub(crate) event_sourcing_enabled: bool,
}

impl KnowledgeBase {
    /// Create a new knowledge base instance
    ///
    /// If MEMORY_EVENT_SOURCING=true, uses Event Sourcing mode.
    /// Otherwise, uses legacy memory.jsonl mode.
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

        // Check if Event Sourcing mode is enabled
        let event_sourcing_enabled = env::var("MEMORY_EVENT_SOURCING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if event_sourcing_enabled {
            Self::new_with_event_sourcing(memory_file_path, current_user)
        } else {
            Self::new_legacy(memory_file_path, current_user)
        }
    }

    /// Create knowledge base in legacy mode (direct file writes)
    fn new_legacy(memory_file_path: String, current_user: String) -> Self {
        let graph = Self::load_graph_from_file(&memory_file_path).unwrap_or_default();

        Self {
            memory_file_path,
            graph: RwLock::new(graph),
            current_user,
            event_store: None,
            snapshot_manager: None,
            log_rotation: None,
            event_sourcing_enabled: false,
        }
    }

    /// Create knowledge base with Event Sourcing enabled
    fn new_with_event_sourcing(memory_file_path: String, current_user: String) -> Self {
        // Determine data directory (parent of memory.jsonl or ./data)
        let data_dir = Path::new(&memory_file_path)
            .parent()
            .map(|p| p.join("data"))
            .unwrap_or_else(|| std::path::PathBuf::from("data"));

        let config = EventStoreConfig::with_data_dir(&data_dir);
        let mut event_store = EventStore::with_config(config.clone());
        let snapshot_manager = SnapshotManager::new(config.clone());
        let log_rotation = LogRotation::new(config);

        // Initialize from snapshot + replay events
        let (entities, relations) = match event_store.initialize() {
            Ok((e, r)) => (e, r),
            Err(e) => {
                eprintln!("Warning: Failed to initialize from event store: {}", e);
                eprintln!("Falling back to empty graph");
                (Vec::new(), Vec::new())
            }
        };

        let graph = KnowledgeGraph { entities, relations };

        println!(
            "Event Sourcing enabled: {} entities, {} relations",
            graph.entities.len(),
            graph.relations.len()
        );

        Self {
            memory_file_path,
            graph: RwLock::new(graph),
            current_user,
            event_store: Some(Mutex::new(event_store)),
            snapshot_manager: Some(snapshot_manager),
            log_rotation: Some(log_rotation),
            event_sourcing_enabled: true,
        }
    }

    /// Create a new knowledge base with custom file path
    pub fn with_file_path(file_path: String) -> Self {
        let current_user = get_current_user();

        let event_sourcing_enabled = env::var("MEMORY_EVENT_SOURCING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if event_sourcing_enabled {
            Self::new_with_event_sourcing(file_path, current_user)
        } else {
            Self::new_legacy(file_path, current_user)
        }
    }

    /// Create a new knowledge base for testing with explicit parameters
    #[cfg(test)]
    pub fn for_testing(file_path: String, user: String) -> Self {
        Self {
            memory_file_path: file_path,
            graph: RwLock::new(KnowledgeGraph::default()),
            current_user: user,
            event_store: None,
            snapshot_manager: None,
            log_rotation: None,
            event_sourcing_enabled: false,
        }
    }

    /// Create a knowledge base for testing with Event Sourcing enabled
    #[cfg(test)]
    pub fn for_testing_event_sourcing(data_dir: &Path, user: String) -> Self {
        let config = EventStoreConfig::with_data_dir(data_dir);
        let mut event_store = EventStore::with_config(config.clone());
        let snapshot_manager = SnapshotManager::new(config.clone());
        let log_rotation = LogRotation::new(config);

        let (entities, relations) = event_store.initialize().unwrap_or_default();
        let graph = KnowledgeGraph { entities, relations };

        Self {
            memory_file_path: data_dir.join("memory.jsonl").to_string_lossy().to_string(),
            graph: RwLock::new(graph),
            current_user: user,
            event_store: Some(Mutex::new(event_store)),
            snapshot_manager: Some(snapshot_manager),
            log_rotation: Some(log_rotation),
            event_sourcing_enabled: true,
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
    /// Uses read lock - allows multiple concurrent readers
    pub(crate) fn load_graph(&self) -> McpResult<KnowledgeGraph> {
        Ok(self.graph.read().unwrap().clone())
    }

    /// Persist graph to file (internal helper, expects caller to hold write lock)
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

    /// Check if Event Sourcing mode is enabled
    pub fn is_event_sourcing_enabled(&self) -> bool {
        self.event_sourcing_enabled
    }

    /// Emit an event to the event store (if Event Sourcing is enabled)
    ///
    /// Returns the event ID if successful, or None if Event Sourcing is disabled.
    pub(crate) fn emit_event(
        &self,
        event_type: EventType,
        data: serde_json::Value,
    ) -> McpResult<Option<u64>> {
        if let Some(ref event_store) = self.event_store {
            let mut store = event_store.lock().unwrap();
            let event = store.create_and_append_event(event_type, self.current_user.clone(), data)?;
            Ok(Some(event.event_id))
        } else {
            Ok(None)
        }
    }

    /// Check if a snapshot should be created and create it if so
    pub(crate) fn maybe_create_snapshot(&self) -> McpResult<()> {
        if let (Some(ref event_store), Some(ref snapshot_manager)) =
            (&self.event_store, &self.snapshot_manager)
        {
            let store = event_store.lock().unwrap();

            if store.should_snapshot() {
                let graph = self.graph.read().unwrap();
                let last_event_id = store.next_event_id().saturating_sub(1);
                let config = store.config().clone();

                // Drop the store lock before creating snapshot
                drop(store);

                snapshot_manager.create_snapshot_with_backup(
                    last_event_id,
                    &graph.entities,
                    &graph.relations,
                )?;

                // Rotate event log to archive old events
                if config.archive_old_events {
                    if let Some(ref rotation) = self.log_rotation {
                        if let Err(e) = rotation.rotate_after_snapshot(last_event_id) {
                            eprintln!("Warning: Failed to rotate event log: {}", e);
                        }
                    }
                }

                // Update the store's snapshot counter
                let mut store = event_store.lock().unwrap();
                store.snapshot_created(last_event_id);
            }
        }
        Ok(())
    }

    /// Force create a snapshot (for graceful shutdown)
    /// Returns the path to the snapshot file if created, or None if Event Sourcing is disabled
    pub fn create_snapshot(&self) -> McpResult<Option<std::path::PathBuf>> {
        if let (Some(ref event_store), Some(ref snapshot_manager)) =
            (&self.event_store, &self.snapshot_manager)
        {
            let store = event_store.lock().unwrap();
            let graph = self.graph.read().unwrap();
            let last_event_id = store.next_event_id().saturating_sub(1);

            if last_event_id > 0 {
                drop(store);
                snapshot_manager.create_snapshot_with_backup(
                    last_event_id,
                    &graph.entities,
                    &graph.relations,
                )?;

                let mut store = event_store.lock().unwrap();
                store.snapshot_created(last_event_id);

                return Ok(Some(snapshot_manager.latest_path()));
            }
        }
        Ok(None)
    }

    /// Get Event Store statistics (only in Event Sourcing mode)
    pub fn get_stats(&self) -> Option<crate::event_store::EventStoreStats> {
        if let Some(ref event_store) = self.event_store {
            let store = event_store.lock().unwrap();
            let collector = crate::event_store::StatsCollector::new(store.config().clone());
            collector.collect().ok()
        } else {
            None
        }
    }

    /// Manually rotate event log (archive old events)
    pub fn rotate_event_log(&self) -> McpResult<Option<std::path::PathBuf>> {
        if let (Some(ref event_store), Some(ref rotation)) =
            (&self.event_store, &self.log_rotation)
        {
            // Just need to drop the lock, don't need the config
            let _store = event_store.lock().unwrap();
            drop(_store);

            // Get last snapshot event ID from snapshot manager
            if let Some(ref snapshot_manager) = self.snapshot_manager {
                if let Ok(Some(meta)) = snapshot_manager.load_meta() {
                    return Ok(rotation.rotate_after_snapshot(meta.last_event_id)?);
                }
            }
        }
        Ok(None)
    }

    /// Clean up old archive files, keeping only the most recent N
    pub fn cleanup_archives(&self, keep_count: usize) -> McpResult<usize> {
        if let Some(ref rotation) = self.log_rotation {
            Ok(rotation.cleanup_old_archives(keep_count)?)
        } else {
            Ok(0)
        }
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
