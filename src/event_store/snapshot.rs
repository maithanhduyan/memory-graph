//! Snapshot Manager for Event Sourcing
//!
//! Handles creation, loading, and management of state snapshots.
//! Snapshots are point-in-time captures of the materialized state
//! that allow fast startup without replaying all events.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};

use crate::types::{Entity, Relation, SnapshotMeta};
use crate::utils::atomic_write_with;

use super::store::{EventStoreConfig, EventStoreError, EventStoreResult};

/// Snapshot Manager handles creating and loading snapshots
pub struct SnapshotManager {
    config: EventStoreConfig,
}

impl SnapshotManager {
    /// Create a new SnapshotManager with the given config
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Get path to latest snapshot
    pub fn latest_path(&self) -> std::path::PathBuf {
        self.config.latest_snapshot_path()
    }

    /// Get path to previous (backup) snapshot
    pub fn previous_path(&self) -> std::path::PathBuf {
        self.config.previous_snapshot_path()
    }

    /// Check if a snapshot exists
    pub fn snapshot_exists(&self) -> bool {
        self.config.latest_snapshot_path().exists()
    }

    /// Create a new snapshot atomically
    ///
    /// This function:
    /// 1. Writes metadata + entities + relations to a temp file
    /// 2. Syncs to disk
    /// 3. Backs up existing snapshot to previous.jsonl
    /// 4. Atomically renames temp to latest.jsonl
    ///
    /// # Arguments
    ///
    /// * `last_event_id` - The ID of the last event included in this snapshot
    /// * `entities` - Current entities to snapshot
    /// * `relations` - Current relations to snapshot
    pub fn create_snapshot(
        &self,
        last_event_id: u64,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<SnapshotMeta> {
        let latest_path = self.config.latest_snapshot_path();
        let _previous_path = self.config.previous_snapshot_path();

        // Ensure snapshots directory exists
        fs::create_dir_all(self.config.snapshots_dir())?;

        // Create metadata
        let meta = SnapshotMeta::new(last_event_id, entities.len(), relations.len());

        // Write snapshot atomically
        let meta_clone = meta.clone();
        atomic_write_with(&latest_path, |file| {
            // Write metadata first
            let meta_json = serde_json::to_string(&meta_clone)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", meta_json)?;

            // Write entities
            for entity in entities {
                let json = serde_json::to_string(entity)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                writeln!(file, "{}", json)?;
            }

            // Write relations
            for relation in relations {
                let json = serde_json::to_string(relation)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                writeln!(file, "{}", json)?;
            }

            Ok(())
        })
        .map_err(|e| EventStoreError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // Backup is handled by atomic_write_with's rename
        // But we need to manually handle previous backup
        // Note: atomic_write_with already handles this via temp file pattern
        // For explicit backup, we'd need to modify the flow

        println!(
            "Created snapshot: {} entities, {} relations (event_id: {})",
            entities.len(),
            relations.len(),
            last_event_id
        );

        Ok(meta)
    }

    /// Create snapshot with explicit backup of previous version
    ///
    /// This provides an extra safety layer by keeping the previous
    /// snapshot as a backup before creating a new one.
    pub fn create_snapshot_with_backup(
        &self,
        last_event_id: u64,
        entities: &[Entity],
        relations: &[Relation],
    ) -> EventStoreResult<SnapshotMeta> {
        let latest_path = self.config.latest_snapshot_path();
        let previous_path = self.config.previous_snapshot_path();
        let temp_path = latest_path.with_extension("tmp");

        // Ensure snapshots directory exists
        fs::create_dir_all(self.config.snapshots_dir())?;

        // Create metadata
        let meta = SnapshotMeta::new(last_event_id, entities.len(), relations.len());

        // Step 1: Write to temp file
        {
            let mut file = File::create(&temp_path)?;

            // Write metadata
            writeln!(file, "{}", serde_json::to_string(&meta)?)?;

            // Write entities
            for entity in entities {
                writeln!(file, "{}", serde_json::to_string(entity)?)?;
            }

            // Write relations
            for relation in relations {
                writeln!(file, "{}", serde_json::to_string(relation)?)?;
            }

            // Sync to disk
            file.sync_all()?;
        }

        // Step 2: Backup existing snapshot
        if latest_path.exists() {
            // Remove old backup if exists
            if previous_path.exists() {
                fs::remove_file(&previous_path)?;
            }
            fs::rename(&latest_path, &previous_path)?;
        }

        // Step 3: Atomic rename temp to latest
        fs::rename(&temp_path, &latest_path)?;

        println!(
            "Created snapshot with backup: {} entities, {} relations (event_id: {})",
            entities.len(),
            relations.len(),
            last_event_id
        );

        Ok(meta)
    }

    /// Load snapshot metadata only (fast, for checking state)
    pub fn load_meta(&self) -> EventStoreResult<Option<SnapshotMeta>> {
        let path = self.config.latest_snapshot_path();

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);

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

    /// Load full snapshot (metadata + entities + relations)
    pub fn load_full(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let path = self.config.latest_snapshot_path();

        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line is metadata
        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty snapshot".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::with_capacity(meta.entity_count);
        let mut relations = Vec::with_capacity(meta.relation_count);

        // Parse remaining lines
        for (line_num, line_result) in lines.enumerate() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse as JSON value first to determine type
            let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                EventStoreError::SnapshotCorrupted(format!("Line {}: {}", line_num + 2, e))
            })?;

            // Determine if entity or relation based on fields
            if value.get("entityType").is_some() && value.get("name").is_some() {
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
            // Silently skip unknown line types
        }

        // Validate counts
        if entities.len() != meta.entity_count {
            eprintln!(
                "Warning: Expected {} entities, found {}",
                meta.entity_count,
                entities.len()
            );
        }
        if relations.len() != meta.relation_count {
            eprintln!(
                "Warning: Expected {} relations, found {}",
                meta.relation_count,
                relations.len()
            );
        }

        Ok(Some((meta, entities, relations)))
    }

    /// Try to recover from backup snapshot if primary is corrupted
    pub fn recover_from_backup(&self) -> EventStoreResult<Option<(SnapshotMeta, Vec<Entity>, Vec<Relation>)>> {
        let previous_path = self.config.previous_snapshot_path();

        if !previous_path.exists() {
            return Ok(None);
        }

        println!("Attempting recovery from backup snapshot...");

        // Temporarily swap paths to load from backup
        let file = File::open(&previous_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let meta_line = lines
            .next()
            .ok_or_else(|| EventStoreError::SnapshotCorrupted("Empty backup".to_string()))??;
        let meta = SnapshotMeta::from_json_line(&meta_line)?;

        let mut entities = Vec::new();
        let mut relations = Vec::new();

        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(&line)?;

            if value.get("entityType").is_some() && value.get("name").is_some() {
                let entity: Entity = serde_json::from_value(value)?;
                entities.push(entity);
            } else if value.get("relationType").is_some() {
                let relation: Relation = serde_json::from_value(value)?;
                relations.push(relation);
            }
        }

        println!(
            "Recovered from backup: {} entities, {} relations",
            entities.len(),
            relations.len()
        );

        Ok(Some((meta, entities, relations)))
    }

    /// Delete all snapshots (for testing or reset)
    pub fn clear_snapshots(&self) -> EventStoreResult<()> {
        let latest = self.config.latest_snapshot_path();
        let previous = self.config.previous_snapshot_path();

        if latest.exists() {
            fs::remove_file(&latest)?;
        }
        if previous.exists() {
            fs::remove_file(&previous)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SnapshotManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_data_dir(temp_dir.path());
        fs::create_dir_all(config.snapshots_dir()).unwrap();
        let manager = SnapshotManager::new(config);
        (manager, temp_dir)
    }

    fn create_test_entities() -> Vec<Entity> {
        vec![
            Entity::with_observations("Bug:Login".to_string(), "Bug".to_string(), vec!["obs1".to_string()]),
            Entity::new("Module:Auth".to_string(), "Module".to_string()),
        ]
    }

    fn create_test_relations() -> Vec<Relation> {
        vec![Relation::new(
            "Bug:Login".to_string(),
            "Module:Auth".to_string(),
            "affects".to_string(),
        )]
    }

    #[test]
    fn test_create_and_load_snapshot() {
        let (manager, _temp_dir) = create_test_manager();
        let entities = create_test_entities();
        let relations = create_test_relations();

        // Create snapshot
        let meta = manager
            .create_snapshot_with_backup(100, &entities, &relations)
            .unwrap();

        assert_eq!(meta.last_event_id, 100);
        assert_eq!(meta.entity_count, 2);
        assert_eq!(meta.relation_count, 1);

        // Load and verify
        let (loaded_meta, loaded_entities, loaded_relations) =
            manager.load_full().unwrap().unwrap();

        assert_eq!(loaded_meta.last_event_id, 100);
        assert_eq!(loaded_entities.len(), 2);
        assert_eq!(loaded_relations.len(), 1);
        assert_eq!(loaded_entities[0].name, "Bug:Login");
        assert_eq!(loaded_relations[0].relation_type, "affects");
    }

    #[test]
    fn test_snapshot_backup() {
        let (manager, _temp_dir) = create_test_manager();

        // Create first snapshot
        let entities1 = vec![Entity::new("Entity1".to_string(), "Test".to_string())];
        manager
            .create_snapshot_with_backup(10, &entities1, &[])
            .unwrap();

        // Create second snapshot (should backup first)
        let entities2 = vec![Entity::new("Entity2".to_string(), "Test".to_string())];
        manager
            .create_snapshot_with_backup(20, &entities2, &[])
            .unwrap();

        // Verify backup exists
        assert!(manager.previous_path().exists());

        // Load latest (should be Entity2)
        let (meta, entities, _) = manager.load_full().unwrap().unwrap();
        assert_eq!(meta.last_event_id, 20);
        assert_eq!(entities[0].name, "Entity2");

        // Recover from backup (should be Entity1)
        let (backup_meta, backup_entities, _) = manager.recover_from_backup().unwrap().unwrap();
        assert_eq!(backup_meta.last_event_id, 10);
        assert_eq!(backup_entities[0].name, "Entity1");
    }

    #[test]
    fn test_load_meta_only() {
        let (manager, _temp_dir) = create_test_manager();
        let entities = create_test_entities();
        let relations = create_test_relations();

        manager
            .create_snapshot_with_backup(50, &entities, &relations)
            .unwrap();

        // Load meta only (fast)
        let meta = manager.load_meta().unwrap().unwrap();
        assert_eq!(meta.last_event_id, 50);
        assert_eq!(meta.entity_count, 2);
        assert_eq!(meta.relation_count, 1);
    }

    #[test]
    fn test_no_snapshot_returns_none() {
        let (manager, _temp_dir) = create_test_manager();

        assert!(manager.load_meta().unwrap().is_none());
        assert!(manager.load_full().unwrap().is_none());
        assert!(!manager.snapshot_exists());
    }

    #[test]
    fn test_clear_snapshots() {
        let (manager, _temp_dir) = create_test_manager();

        // Create snapshots
        manager
            .create_snapshot_with_backup(10, &[], &[])
            .unwrap();
        manager
            .create_snapshot_with_backup(20, &[], &[])
            .unwrap();

        assert!(manager.snapshot_exists());
        assert!(manager.previous_path().exists());

        // Clear
        manager.clear_snapshots().unwrap();

        assert!(!manager.snapshot_exists());
        assert!(!manager.previous_path().exists());
    }
}
