//! Migration Tool for Event Sourcing
//!
//! Converts existing legacy `memory.jsonl` data to Event Sourcing format:
//! 1. Reads entities and relations from memory.jsonl
//! 2. Creates entity_created and relation_created events
//! 3. Writes events to events.jsonl
//! 4. Creates initial snapshot

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::types::{
    Entity, EntityCreatedData, Event, EventSource, EventType, Relation, RelationCreatedData,
};
use crate::utils::current_timestamp;

use super::store::{EventStoreConfig, EventStoreError, EventStoreResult};
use super::SnapshotManager;

/// Result of a migration operation
#[derive(Debug)]
pub struct MigrationResult {
    pub entities_migrated: usize,
    pub relations_migrated: usize,
    pub events_created: usize,
    pub snapshot_created: bool,
}

/// Migration tool for converting legacy data to Event Sourcing
pub struct MigrationTool {
    config: EventStoreConfig,
}

impl MigrationTool {
    /// Create a new migration tool with default config
    pub fn new() -> Self {
        Self {
            config: EventStoreConfig::default(),
        }
    }

    /// Create a new migration tool with custom config
    pub fn with_config(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Migrate legacy memory.jsonl to Event Sourcing format
    ///
    /// # Arguments
    /// * `legacy_path` - Path to the legacy memory.jsonl file
    ///
    /// # Returns
    /// * `MigrationResult` with counts of migrated items
    pub fn migrate_from_legacy<P: AsRef<Path>>(
        &self,
        legacy_path: P,
    ) -> EventStoreResult<MigrationResult> {
        let legacy_path = legacy_path.as_ref();

        if !legacy_path.exists() {
            return Err(EventStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Legacy file not found: {}", legacy_path.display()),
            )));
        }

        // Read legacy data
        let (entities, relations) = self.read_legacy_file(legacy_path)?;

        // Create events directory
        fs::create_dir_all(self.config.data_dir())?;

        // Create events from entities and relations
        let events = self.create_migration_events(&entities, &relations)?;
        let event_count = events.len();

        // Write events to events.jsonl
        let events_path = self.config.events_path();
        self.write_events(&events_path, &events)?;

        // Create initial snapshot
        let snapshot_manager = SnapshotManager::new(self.config.clone());
        let last_event_id = events.last().map(|e| e.event_id).unwrap_or(0);

        snapshot_manager.create_snapshot_with_backup(last_event_id, &entities, &relations)?;

        // Backup original file
        let backup_path = legacy_path.with_extension("jsonl.migrated");
        if !backup_path.exists() {
            fs::copy(legacy_path, &backup_path)?;
        }

        Ok(MigrationResult {
            entities_migrated: entities.len(),
            relations_migrated: relations.len(),
            events_created: event_count,
            snapshot_created: true,
        })
    }

    /// Read entities and relations from legacy file
    fn read_legacy_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> EventStoreResult<(Vec<Entity>, Vec<Relation>)> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as Entity first
            if let Ok(entity) = serde_json::from_str::<Entity>(trimmed) {
                entities.push(entity);
                continue;
            }

            // Try to parse as Relation
            if let Ok(relation) = serde_json::from_str::<Relation>(trimmed) {
                relations.push(relation);
                continue;
            }

            // Log warning for unknown line format
            eprintln!(
                "[Migration] Warning: Could not parse line: {}",
                if trimmed.len() > 50 {
                    format!("{}...", &trimmed[..50])
                } else {
                    trimmed.to_string()
                }
            );
        }

        Ok((entities, relations))
    }

    /// Create migration events from entities and relations
    fn create_migration_events(
        &self,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<Vec<Event>> {
        let mut events = Vec::new();
        let mut event_id: u64 = 1;
        let timestamp = current_timestamp() as i64;

        // Create entity_created events
        for entity in entities {
            let data = EntityCreatedData {
                name: entity.name.clone(),
                entity_type: entity.entity_type.clone(),
                observations: entity.observations.clone(),
            };

            let user = if entity.created_by.is_empty() {
                "migration".to_string()
            } else {
                entity.created_by.clone()
            };

            let event = Event {
                event_id,
                event_type: EventType::EntityCreated,
                timestamp,
                user,
                agent: Some("MigrationTool".to_string()),
                source: EventSource::Migration,
                data: serde_json::to_value(&data)?,
            };

            events.push(event);
            event_id += 1;
        }

        // Create relation_created events
        for relation in relations {
            let data = RelationCreatedData {
                from: relation.from.clone(),
                to: relation.to.clone(),
                relation_type: relation.relation_type.clone(),
                valid_from: relation.valid_from.map(|v| v as i64),
                valid_to: relation.valid_to.map(|v| v as i64),
            };

            let user = if relation.created_by.is_empty() {
                "migration".to_string()
            } else {
                relation.created_by.clone()
            };

            let event = Event {
                event_id,
                event_type: EventType::RelationCreated,
                timestamp,
                user,
                agent: Some("MigrationTool".to_string()),
                source: EventSource::Migration,
                data: serde_json::to_value(&data)?,
            };

            events.push(event);
            event_id += 1;
        }

        Ok(events)
    }

    /// Write events to file
    fn write_events<P: AsRef<Path>>(
        &self,
        path: P,
        events: &[Event],
    ) -> EventStoreResult<()> {
        let path = path.as_ref();

        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        {
            let mut file = File::create(&temp_path)?;
            for event in events {
                writeln!(file, "{}", serde_json::to_string(event)?)?;
            }
            file.sync_all()?;
        }

        // Atomic rename
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// Check if migration is needed
    pub fn needs_migration<P: AsRef<Path>>(&self, legacy_path: P) -> bool {
        let legacy_path = legacy_path.as_ref();
        let events_path = self.config.events_path();
        let snapshot_path = self.config.latest_snapshot_path();

        // Migration needed if:
        // 1. Legacy file exists
        // 2. AND neither events nor snapshot exist
        legacy_path.exists() && !events_path.exists() && !snapshot_path.exists()
    }
}

impl Default for MigrationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_legacy_file(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("memory.jsonl");
        let mut file = File::create(&path).unwrap();

        // Write some entities
        writeln!(file, r#"{{"name":"Alice","entityType":"Person","observations":["Developer"]}}"#).unwrap();
        writeln!(file, r#"{{"name":"Bob","entityType":"Person","observations":["Designer"]}}"#).unwrap();

        // Write some relations
        writeln!(file, r#"{{"from":"Alice","to":"Bob","relationType":"knows"}}"#).unwrap();

        file.sync_all().unwrap();
        path
    }

    #[test]
    fn test_read_legacy_file() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = create_legacy_file(temp_dir.path());

        let tool = MigrationTool::new();
        let (entities, relations) = tool.read_legacy_file(&legacy_path).unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 1);

        assert_eq!(entities[0].name, "Alice");
        assert_eq!(entities[1].name, "Bob");
        assert_eq!(relations[0].from, "Alice");
        assert_eq!(relations[0].to, "Bob");
    }

    #[test]
    fn test_create_migration_events() {
        let entities = vec![
            Entity {
                name: "TestEntity".to_string(),
                entity_type: "Test".to_string(),
                observations: vec!["observation1".to_string()],
                created_by: "tester".to_string(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];

        let relations = vec![
            Relation {
                from: "A".to_string(),
                to: "B".to_string(),
                relation_type: "test".to_string(),
                created_by: "tester".to_string(),
                created_at: 0,
                valid_from: None,
                valid_to: None,
            },
        ];

        let tool = MigrationTool::new();
        let events = tool.create_migration_events(&entities, &relations).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EventType::EntityCreated);
        assert_eq!(events[1].event_type, EventType::RelationCreated);
        assert!(matches!(events[0].source, EventSource::Migration));
    }

    #[test]
    fn test_migrate_from_legacy() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = create_legacy_file(temp_dir.path());

        // Configure migration to use temp directory
        let config = EventStoreConfig::new(temp_dir.path().join("data"));
        let tool = MigrationTool::with_config(config.clone());

        let result = tool.migrate_from_legacy(&legacy_path).unwrap();

        assert_eq!(result.entities_migrated, 2);
        assert_eq!(result.relations_migrated, 1);
        assert_eq!(result.events_created, 3);
        assert!(result.snapshot_created);

        // Verify events file was created
        assert!(config.events_path().exists());

        // Verify snapshot was created
        assert!(config.latest_snapshot_path().exists());

        // Verify backup was created
        let backup_path = legacy_path.with_extension("jsonl.migrated");
        assert!(backup_path.exists());
    }

    #[test]
    fn test_needs_migration() {
        let temp_dir = TempDir::new().unwrap();
        let legacy_path = temp_dir.path().join("memory.jsonl");

        let config = EventStoreConfig::new(temp_dir.path().join("data"));
        let tool = MigrationTool::with_config(config.clone());

        // No legacy file - no migration needed
        assert!(!tool.needs_migration(&legacy_path));

        // Create legacy file
        File::create(&legacy_path).unwrap();
        assert!(tool.needs_migration(&legacy_path));

        // Create events file - no migration needed
        fs::create_dir_all(config.data_dir()).unwrap();
        File::create(config.events_path()).unwrap();
        assert!(!tool.needs_migration(&legacy_path));
    }
}
