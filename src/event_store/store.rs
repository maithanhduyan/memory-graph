//! Event Store - Core event sourcing implementation
//!
//! The EventStore manages the append-only event log and provides
//! functionality for replaying events to rebuild state.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::types::{
    Entity, EntityCreatedData, EntityDeletedData, EntityUpdatedData, Event, EventType,
    ObservationAddedData, ObservationRemovedData, Relation, RelationCreatedData,
    RelationDeletedData, SnapshotMeta,
};

/// Configuration for the EventStore
#[derive(Debug, Clone)]
pub struct EventStoreConfig {
    /// Path to the data directory
    pub data_dir: PathBuf,
    /// Threshold for creating snapshots (number of events)
    pub snapshot_threshold: usize,
    /// Whether to archive old event logs
    pub archive_old_events: bool,
    /// Whether to compress archived events
    pub compress_archive: bool,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            snapshot_threshold: 1000,
            archive_old_events: true,
            compress_archive: false, // TODO: implement compression
        }
    }
}

impl EventStoreConfig {
    /// Create config with custom data directory
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Create config with custom data directory (alias for new)
    pub fn with_data_dir<P: AsRef<Path>>(data_dir: P) -> Self {
        Self::new(data_dir)
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get path to events.jsonl
    pub fn events_path(&self) -> PathBuf {
        self.data_dir.join("events.jsonl")
    }

    /// Get path to snapshots directory
    pub fn snapshots_dir(&self) -> PathBuf {
        self.data_dir.join("snapshots")
    }

    /// Get path to latest snapshot
    pub fn latest_snapshot_path(&self) -> PathBuf {
        self.snapshots_dir().join("latest.jsonl")
    }

    /// Get path to previous snapshot (backup)
    pub fn previous_snapshot_path(&self) -> PathBuf {
        self.snapshots_dir().join("previous.jsonl")
    }

    /// Get path to archive directory
    pub fn archive_dir(&self) -> PathBuf {
        self.data_dir.join("archive")
    }
}

/// Result type for EventStore operations
pub type EventStoreResult<T> = Result<T, EventStoreError>;

/// Errors that can occur in EventStore operations
#[derive(Debug)]
pub enum EventStoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidEvent(String),
    SnapshotCorrupted(String),
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStoreError::Io(e) => write!(f, "IO error: {}", e),
            EventStoreError::Json(e) => write!(f, "JSON error: {}", e),
            EventStoreError::InvalidEvent(msg) => write!(f, "Invalid event: {}", msg),
            EventStoreError::SnapshotCorrupted(msg) => write!(f, "Snapshot corrupted: {}", msg),
        }
    }
}

impl std::error::Error for EventStoreError {}

impl From<std::io::Error> for EventStoreError {
    fn from(e: std::io::Error) -> Self {
        EventStoreError::Io(e)
    }
}

impl From<serde_json::Error> for EventStoreError {
    fn from(e: serde_json::Error) -> Self {
        EventStoreError::Json(e)
    }
}

/// The EventStore manages append-only event log and state replay
pub struct EventStore {
    config: EventStoreConfig,
    /// Next event ID to assign
    next_event_id: u64,
    /// Number of events since last snapshot
    events_since_snapshot: usize,
    /// Last event ID included in most recent snapshot
    last_snapshot_event_id: u64,
}

impl EventStore {
    /// Create a new EventStore with default config
    pub fn new() -> Self {
        Self::with_config(EventStoreConfig::default())
    }

    /// Create a new EventStore with custom config
    pub fn with_config(config: EventStoreConfig) -> Self {
        Self {
            config,
            next_event_id: 1,
            events_since_snapshot: 0,
            last_snapshot_event_id: 0,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &EventStoreConfig {
        &self.config
    }

    /// Get the next event ID (without incrementing)
    pub fn next_event_id(&self) -> u64 {
        self.next_event_id
    }

    /// Get events since last snapshot
    pub fn events_since_snapshot(&self) -> usize {
        self.events_since_snapshot
    }

    /// Check if snapshot should be created
    pub fn should_snapshot(&self) -> bool {
        self.events_since_snapshot >= self.config.snapshot_threshold
    }

    /// Append an event to the event log
    ///
    /// This is the core write operation. Events are appended atomically
    /// with fsync to ensure durability.
    pub fn append_event(&mut self, event: Event) -> EventStoreResult<u64> {
        let events_path = self.config.events_path();

        // Ensure parent directory exists
        if let Some(parent) = events_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;

        // Serialize and write
        let json_line = event.to_json_line()?;
        writeln!(file, "{}", json_line)?;

        // Sync to disk for durability
        file.sync_all()?;

        // Update internal state
        let event_id = event.event_id;
        if event_id >= self.next_event_id {
            self.next_event_id = event_id + 1;
        }
        self.events_since_snapshot += 1;

        Ok(event_id)
    }

    /// Create a new event and append it
    pub fn create_and_append_event(
        &mut self,
        event_type: EventType,
        user: String,
        data: serde_json::Value,
    ) -> EventStoreResult<Event> {
        let event_id = self.next_event_id;
        self.next_event_id += 1;

        let event = Event::new(event_type, event_id, user, data);
        self.append_event(event.clone())?;

        Ok(event)
    }

    /// Load all events from the event log
    pub fn load_events(&self) -> EventStoreResult<Vec<Event>> {
        let events_path = self.config.events_path();

        if !events_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&events_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            match Event::from_json_line(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse event at line {}: {}",
                        line_num + 1,
                        e
                    );
                    // Continue loading other events
                }
            }
        }

        Ok(events)
    }

    /// Load events after a specific event ID
    ///
    /// Used for replaying events after loading a snapshot.
    pub fn load_events_after(&self, after_event_id: u64) -> EventStoreResult<Vec<Event>> {
        let all_events = self.load_events()?;
        Ok(all_events
            .into_iter()
            .filter(|e| e.event_id > after_event_id)
            .collect())
    }

    /// Load snapshot metadata from a snapshot file
    pub fn load_snapshot_meta(&self) -> EventStoreResult<Option<SnapshotMeta>> {
        let snapshot_path = self.config.latest_snapshot_path();

        if !snapshot_path.exists() {
            return Ok(None);
        }

        let file = File::open(&snapshot_path)?;
        let reader = BufReader::new(file);

        // First line should be metadata
        if let Some(first_line) = reader.lines().next() {
            let line = first_line?;
            let meta = SnapshotMeta::from_json_line(&line)?;
            Ok(Some(meta))
        } else {
            Err(EventStoreError::SnapshotCorrupted(
                "Empty snapshot file".to_string(),
            ))
        }
    }

    /// Load entities and relations from snapshot
    pub fn load_snapshot(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let snapshot_path = self.config.latest_snapshot_path();

        if !snapshot_path.exists() {
            return Ok(None);
        }

        let file = File::open(&snapshot_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line is metadata
        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty snapshot".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        // Parse remaining lines
        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as entity or relation based on structure
            let value: serde_json::Value = serde_json::from_str(&line)?;

            if value.get("entityType").is_some() && value.get("name").is_some() {
                // It's an entity
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                // It's a relation
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
        }

        Ok(Some((meta, entities, relations)))
    }

    /// Apply a single event to the state
    ///
    /// This is the core state mutation logic. Each event type
    /// corresponds to a specific state change.
    pub fn apply_event(
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        event: &Event,
    ) -> EventStoreResult<()> {
        match event.event_type {
            EventType::EntityCreated => {
                let data: EntityCreatedData = event.parse_data()?;

                // Check if entity already exists
                if entities.iter().any(|e| e.name == data.name) {
                    // Entity already exists, skip (idempotent)
                    return Ok(());
                }

                let entity = Entity {
                    name: data.name,
                    entity_type: data.entity_type,
                    observations: data.observations,
                    created_by: event.user.clone(),
                    updated_by: event.user.clone(),
                    created_at: event.timestamp as u64,
                    updated_at: event.timestamp as u64,
                };
                entities.push(entity);
            }

            EventType::EntityUpdated => {
                let data: EntityUpdatedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.name) {
                    if let Some(new_type) = data.entity_type {
                        entity.entity_type = new_type;
                    }
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::EntityDeleted => {
                let data: EntityDeletedData = event.parse_data()?;

                // Remove entity
                entities.retain(|e| e.name != data.name);

                // Remove relations involving this entity
                relations.retain(|r| r.from != data.name && r.to != data.name);
            }

            EventType::ObservationAdded => {
                let data: ObservationAddedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.entity) {
                    if !entity.observations.contains(&data.observation) {
                        entity.observations.push(data.observation);
                    }
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::ObservationRemoved => {
                let data: ObservationRemovedData = event.parse_data()?;

                if let Some(entity) = entities.iter_mut().find(|e| e.name == data.entity) {
                    entity.observations.retain(|o| o != &data.observation);
                    entity.updated_by = event.user.clone();
                    entity.updated_at = event.timestamp as u64;
                }
            }

            EventType::RelationCreated => {
                let data: RelationCreatedData = event.parse_data()?;

                // Check if relation already exists
                let exists = relations.iter().any(|r| {
                    r.from == data.from && r.to == data.to && r.relation_type == data.relation_type
                });

                if !exists {
                    let relation = Relation {
                        from: data.from,
                        to: data.to,
                        relation_type: data.relation_type,
                        created_by: event.user.clone(),
                        created_at: event.timestamp as u64,
                        valid_from: data.valid_from.map(|v| v as u64),
                        valid_to: data.valid_to.map(|v| v as u64),
                    };
                    relations.push(relation);
                }
            }

            EventType::RelationDeleted => {
                let data: RelationDeletedData = event.parse_data()?;

                relations.retain(|r| {
                    !(r.from == data.from && r.to == data.to && r.relation_type == data.relation_type)
                });
            }
        }

        Ok(())
    }

    /// Replay all events to rebuild state
    ///
    /// This loads and applies all events in order to reconstruct
    /// the current state from scratch.
    pub fn replay_all(&self) -> EventStoreResult<(Vec<Entity>, Vec<Relation>, u64)> {
        let events = self.load_events()?;
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut max_event_id = 0u64;

        for event in &events {
            Self::apply_event(&mut entities, &mut relations, event)?;
            if event.event_id > max_event_id {
                max_event_id = event.event_id;
            }
        }

        Ok((entities, relations, max_event_id))
    }

    /// Replay events after a specific event ID
    pub fn replay_after(
        &self,
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        after_event_id: u64,
    ) -> EventStoreResult<u64> {
        let events = self.load_events_after(after_event_id)?;
        let mut max_event_id = after_event_id;

        for event in &events {
            Self::apply_event(entities, relations, event)?;
            if event.event_id > max_event_id {
                max_event_id = event.event_id;
            }
        }

        Ok(max_event_id)
    }

    /// Initialize from storage (snapshot + replay)
    ///
    /// This is the main startup path:
    /// 1. Try to load latest snapshot
    /// 2. Replay any events after the snapshot
    /// 3. Return the reconstructed state
    pub fn initialize(&mut self) -> EventStoreResult<(Vec<Entity>, Vec<Relation>)> {
        // Try to load snapshot first
        if let Some((meta, mut entities, mut relations)) = self.load_snapshot()? {
            self.last_snapshot_event_id = meta.last_event_id;
            self.next_event_id = meta.last_event_id + 1;

            // Replay events after snapshot
            let max_event_id = self.replay_after(&mut entities, &mut relations, meta.last_event_id)?;

            if max_event_id > self.next_event_id {
                self.next_event_id = max_event_id + 1;
            }

            self.events_since_snapshot = (max_event_id - meta.last_event_id) as usize;

            println!(
                "Loaded snapshot (event_id: {}) + replayed {} events. Total: {} entities, {} relations.",
                meta.last_event_id,
                self.events_since_snapshot,
                entities.len(),
                relations.len()
            );

            Ok((entities, relations))
        } else {
            // No snapshot, replay all events
            let (entities, relations, max_event_id) = self.replay_all()?;

            if max_event_id > 0 {
                self.next_event_id = max_event_id + 1;
                self.events_since_snapshot = max_event_id as usize;
            }

            println!(
                "No snapshot found. Replayed {} events. Total: {} entities, {} relations.",
                max_event_id, entities.len(), relations.len()
            );

            Ok((entities, relations))
        }
    }

    /// Reset snapshot counter (called after snapshot creation)
    pub fn snapshot_created(&mut self, last_event_id: u64) {
        self.last_snapshot_event_id = last_event_id;
        self.events_since_snapshot = 0;
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn create_test_store() -> (EventStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_data_dir(temp_dir.path());

        // Create directories
        std::fs::create_dir_all(config.events_path().parent().unwrap()).unwrap();
        std::fs::create_dir_all(config.snapshots_dir()).unwrap();

        let store = EventStore::with_config(config);
        (store, temp_dir)
    }

    #[test]
    fn test_append_and_load_events() {
        let (mut store, _temp_dir) = create_test_store();

        // Append some events
        let event1 = store
            .create_and_append_event(
                EventType::EntityCreated,
                "test_user".to_string(),
                json!({
                    "name": "Test:Entity",
                    "entity_type": "Test",
                    "observations": ["obs1"]
                }),
            )
            .unwrap();

        let event2 = store
            .create_and_append_event(
                EventType::ObservationAdded,
                "test_user".to_string(),
                json!({
                    "entity": "Test:Entity",
                    "observation": "obs2"
                }),
            )
            .unwrap();

        assert_eq!(event1.event_id, 1);
        assert_eq!(event2.event_id, 2);
        assert_eq!(store.next_event_id(), 3);
        assert_eq!(store.events_since_snapshot(), 2);

        // Load events
        let events = store.load_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
        assert_eq!(events[1].event_type, EventType::ObservationAdded);
    }

    #[test]
    fn test_load_events_after() {
        let (mut store, _temp_dir) = create_test_store();

        // Create 5 events
        for i in 1..=5 {
            store
                .create_and_append_event(
                    EventType::EntityCreated,
                    "user".to_string(),
                    json!({
                        "name": format!("Entity:{}", i),
                        "entity_type": "Test"
                    }),
                )
                .unwrap();
        }

        // Load events after ID 3
        let events = store.load_events_after(3).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, 4);
        assert_eq!(events[1].event_id, 5);
    }

    #[test]
    fn test_apply_entity_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::EntityCreated,
            1,
            "user".to_string(),
            json!({
                "name": "Bug:Login",
                "entity_type": "Bug",
                "observations": ["Login fails"]
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Bug:Login");
        assert_eq!(entities[0].entity_type, "Bug");
        assert_eq!(entities[0].observations, vec!["Login fails"]);
    }

    #[test]
    fn test_apply_observation_added() {
        let mut entities = vec![Entity::new("Bug:X".to_string(), "Bug".to_string())];
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::ObservationAdded,
            1,
            "user".to_string(),
            json!({
                "entity": "Bug:X",
                "observation": "New observation"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities[0].observations, vec!["New observation"]);
    }

    #[test]
    fn test_apply_entity_deleted() {
        let mut entities = vec![
            Entity::new("Bug:X".to_string(), "Bug".to_string()),
            Entity::new("Bug:Y".to_string(), "Bug".to_string()),
        ];
        let mut relations = vec![
            Relation::new("Bug:X".to_string(), "Module:A".to_string(), "affects".to_string()),
            Relation::new("Bug:Y".to_string(), "Module:B".to_string(), "affects".to_string()),
        ];

        let event = Event::new(
            EventType::EntityDeleted,
            1,
            "user".to_string(),
            json!({
                "name": "Bug:X"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Bug:Y");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].from, "Bug:Y");
    }

    #[test]
    fn test_apply_relation_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::RelationCreated,
            1,
            "user".to_string(),
            json!({
                "from": "Bug:X",
                "to": "Module:Auth",
                "relation_type": "affects"
            }),
        );

        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].from, "Bug:X");
        assert_eq!(relations[0].to, "Module:Auth");
        assert_eq!(relations[0].relation_type, "affects");
    }

    #[test]
    fn test_replay_all() {
        let (mut store, _temp_dir) = create_test_store();

        // Create events
        store
            .create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                json!({"name": "A", "entity_type": "Test"}),
            )
            .unwrap();

        store
            .create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                json!({"name": "B", "entity_type": "Test"}),
            )
            .unwrap();

        store
            .create_and_append_event(
                EventType::RelationCreated,
                "user".to_string(),
                json!({"from": "A", "to": "B", "relation_type": "depends_on"}),
            )
            .unwrap();

        // Replay
        let (entities, relations, max_id) = store.replay_all().unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 1);
        assert_eq!(max_id, 3);
    }

    #[test]
    fn test_idempotent_entity_created() {
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        let event = Event::new(
            EventType::EntityCreated,
            1,
            "user".to_string(),
            json!({"name": "A", "entity_type": "Test"}),
        );

        // Apply twice
        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();
        EventStore::apply_event(&mut entities, &mut relations, &event).unwrap();

        // Should still be just 1 entity (idempotent)
        assert_eq!(entities.len(), 1);
    }
}
