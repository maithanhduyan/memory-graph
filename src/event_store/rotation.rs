//! Log Rotation and Archive Management
//!
//! Provides functionality for:
//! - Rotating event logs after snapshot
//! - Archiving old events with timestamps
//! - Cleaning up old archives
//! - Compression (future)

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::store::{EventStoreConfig, EventStoreResult};

/// Log rotation manager for event archives
pub struct LogRotation {
    config: EventStoreConfig,
}

impl LogRotation {
    /// Create a new LogRotation manager
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Rotate the current event log after a snapshot
    ///
    /// This moves events up to the snapshot point to an archive file,
    /// keeping only events after the snapshot in the active log.
    ///
    /// # Arguments
    /// * `snapshot_event_id` - The last event ID included in the snapshot
    ///
    /// # Returns
    /// * `Ok(Some(path))` - Path to the archive file if rotation occurred
    /// * `Ok(None)` - No rotation needed (no events to archive)
    pub fn rotate_after_snapshot(&self, snapshot_event_id: u64) -> EventStoreResult<Option<PathBuf>> {
        let events_path = self.config.events_path();

        if !events_path.exists() {
            return Ok(None);
        }

        // Read all events
        let file = File::open(&events_path)?;
        let reader = BufReader::new(file);

        let mut archive_lines = Vec::new();
        let mut keep_lines = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            // Parse event ID from the line
            if let Some(event_id) = self.extract_event_id(&line) {
                if event_id <= snapshot_event_id {
                    archive_lines.push(line);
                } else {
                    keep_lines.push(line);
                }
            } else {
                // If we can't parse, keep it in the active log
                keep_lines.push(line);
            }
        }

        // No events to archive
        if archive_lines.is_empty() {
            return Ok(None);
        }

        // Create archive directory
        let archive_dir = self.config.archive_dir();
        fs::create_dir_all(&archive_dir)?;

        // Generate archive filename with event range
        let archive_filename = format!("events_{}_to_{}.jsonl",
            self.extract_event_id(&archive_lines[0]).unwrap_or(0),
            snapshot_event_id
        );
        let archive_path = archive_dir.join(&archive_filename);

        // Write archive file
        {
            let mut archive_file = File::create(&archive_path)?;
            for line in &archive_lines {
                writeln!(archive_file, "{}", line)?;
            }
            archive_file.sync_all()?;
        }

        // Write remaining events back to active log
        {
            let temp_path = events_path.with_extension("tmp");
            let mut temp_file = File::create(&temp_path)?;
            for line in &keep_lines {
                writeln!(temp_file, "{}", line)?;
            }
            temp_file.sync_all()?;

            // Atomic rename
            fs::rename(&temp_path, &events_path)?;
        }

        println!(
            "Rotated {} events to archive: {}",
            archive_lines.len(),
            archive_path.display()
        );

        Ok(Some(archive_path))
    }

    /// Extract event ID from a JSON line
    fn extract_event_id(&self, line: &str) -> Option<u64> {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        value.get("eventId")?.as_u64()
    }

    /// List all archive files
    pub fn list_archives(&self) -> EventStoreResult<Vec<ArchiveInfo>> {
        let archive_dir = self.config.archive_dir();

        if !archive_dir.exists() {
            return Ok(Vec::new());
        }

        let mut archives = Vec::new();

        for entry in fs::read_dir(&archive_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let metadata = entry.metadata()?;
                let size = metadata.len();

                // Count events in archive
                let event_count = self.count_events(&path)?;

                archives.push(ArchiveInfo {
                    path,
                    size,
                    event_count,
                });
            }
        }

        // Sort by filename (which includes event IDs)
        archives.sort_by(|a, b| a.path.file_name().cmp(&b.path.file_name()));

        Ok(archives)
    }

    /// Count events in a file
    fn count_events(&self, path: &Path) -> EventStoreResult<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let count = reader.lines().filter(|l| l.is_ok()).count();
        Ok(count)
    }

    /// Clean up old archives, keeping only the most recent N
    ///
    /// # Arguments
    /// * `keep_count` - Number of most recent archives to keep
    ///
    /// # Returns
    /// * Number of archives deleted
    pub fn cleanup_old_archives(&self, keep_count: usize) -> EventStoreResult<usize> {
        let mut archives = self.list_archives()?;

        if archives.len() <= keep_count {
            return Ok(0);
        }

        // Sort by filename descending (newest first)
        archives.sort_by(|a, b| b.path.file_name().cmp(&a.path.file_name()));

        let to_delete = &archives[keep_count..];
        let delete_count = to_delete.len();

        for archive in to_delete {
            fs::remove_file(&archive.path)?;
            println!("Deleted old archive: {}", archive.path.display());
        }

        Ok(delete_count)
    }

    /// Get total size of all archives in bytes
    pub fn total_archive_size(&self) -> EventStoreResult<u64> {
        let archives = self.list_archives()?;
        Ok(archives.iter().map(|a| a.size).sum())
    }
}

/// Information about an archive file
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    /// Path to the archive file
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Number of events in the archive
    pub event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_store::EventStore;
    use crate::types::EventType;
    use tempfile::TempDir;

    #[test]
    fn test_rotate_after_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        for i in 1..=5 {
            store.create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                serde_json::json!({
                    "name": format!("Entity{}", i),
                    "entity_type": "Test",
                    "observations": []
                }),
            ).unwrap();
        }

        // Rotate after event 3
        let rotation = LogRotation::new(config.clone());
        let archive_path = rotation.rotate_after_snapshot(3).unwrap();

        assert!(archive_path.is_some());
        let archive_path = archive_path.unwrap();
        assert!(archive_path.exists());

        // Verify archive contains events 1-3
        let archive_count = rotation.count_events(&archive_path).unwrap();
        assert_eq!(archive_count, 3);

        // Verify active log contains events 4-5
        let active_count = rotation.count_events(&config.events_path()).unwrap();
        assert_eq!(active_count, 2);
    }

    #[test]
    fn test_list_archives() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create archive directory and some files
        let archive_dir = config.archive_dir();
        fs::create_dir_all(&archive_dir).unwrap();

        fs::write(archive_dir.join("events_1_to_100.jsonl"), "{}\n{}\n").unwrap();
        fs::write(archive_dir.join("events_101_to_200.jsonl"), "{}\n").unwrap();

        let rotation = LogRotation::new(config);
        let archives = rotation.list_archives().unwrap();

        assert_eq!(archives.len(), 2);
    }

    #[test]
    fn test_cleanup_old_archives() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create archive directory and some files
        let archive_dir = config.archive_dir();
        fs::create_dir_all(&archive_dir).unwrap();

        fs::write(archive_dir.join("events_1_to_100.jsonl"), "{}").unwrap();
        fs::write(archive_dir.join("events_101_to_200.jsonl"), "{}").unwrap();
        fs::write(archive_dir.join("events_201_to_300.jsonl"), "{}").unwrap();

        let rotation = LogRotation::new(config);

        // Keep only 2 archives
        let deleted = rotation.cleanup_old_archives(2).unwrap();
        assert_eq!(deleted, 1);

        // Verify only 2 archives remain
        let remaining = rotation.list_archives().unwrap();
        assert_eq!(remaining.len(), 2);
    }
}
