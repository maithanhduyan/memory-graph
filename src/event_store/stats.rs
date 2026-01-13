//! Event Store Statistics and Metrics
//!
//! Provides statistics about the event store including:
//! - Event counts by type
//! - Storage size information
//! - Performance metrics

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use crate::types::EventType;

use super::rotation::LogRotation;
use super::store::{EventStoreConfig, EventStoreResult};

/// Statistics about the Event Store
#[derive(Debug, Clone, Default)]
pub struct EventStoreStats {
    /// Total number of events in active log
    pub active_event_count: usize,
    /// Total number of events in archives
    pub archived_event_count: usize,
    /// Size of active event log in bytes
    pub active_log_size: u64,
    /// Total size of archives in bytes
    pub archive_size: u64,
    /// Size of latest snapshot in bytes
    pub snapshot_size: u64,
    /// Number of archive files
    pub archive_file_count: usize,
    /// Events by type in active log
    pub events_by_type: HashMap<EventType, usize>,
    /// Last event ID
    pub last_event_id: u64,
    /// Last snapshot event ID
    pub last_snapshot_event_id: u64,
    /// Events since last snapshot
    pub events_since_snapshot: usize,
}

impl EventStoreStats {
    /// Calculate total events
    pub fn total_events(&self) -> usize {
        self.active_event_count + self.archived_event_count
    }

    /// Calculate total storage size
    pub fn total_size(&self) -> u64 {
        self.active_log_size + self.archive_size + self.snapshot_size
    }

    /// Format size in human-readable format
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// Collector for Event Store statistics
pub struct StatsCollector {
    config: EventStoreConfig,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new(config: EventStoreConfig) -> Self {
        Self { config }
    }

    /// Collect all statistics
    pub fn collect(&self) -> EventStoreResult<EventStoreStats> {
        let mut stats = EventStoreStats::default();

        // Active log stats
        let events_path = self.config.events_path();
        if events_path.exists() {
            let (count, size, by_type, last_id) = self.analyze_event_file(&events_path)?;
            stats.active_event_count = count;
            stats.active_log_size = size;
            stats.events_by_type = by_type;
            stats.last_event_id = last_id;
        }

        // Archive stats
        let rotation = LogRotation::new(self.config.clone());
        let archives = rotation.list_archives()?;
        stats.archive_file_count = archives.len();
        stats.archived_event_count = archives.iter().map(|a| a.event_count).sum();
        stats.archive_size = archives.iter().map(|a| a.size).sum();

        // Snapshot stats
        let snapshot_path = self.config.latest_snapshot_path();
        if snapshot_path.exists() {
            stats.snapshot_size = fs::metadata(&snapshot_path)?.len();

            // Parse snapshot to get last_snapshot_event_id
            if let Some(meta) = self.parse_snapshot_meta(&snapshot_path)? {
                stats.last_snapshot_event_id = meta;
            }
        }

        // Calculate events since snapshot
        if stats.last_event_id > stats.last_snapshot_event_id {
            stats.events_since_snapshot = (stats.last_event_id - stats.last_snapshot_event_id) as usize;
        }

        Ok(stats)
    }

    /// Analyze an event file
    fn analyze_event_file(&self, path: &Path) -> EventStoreResult<(usize, u64, HashMap<EventType, usize>, u64)> {
        let file = File::open(path)?;
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let reader = BufReader::new(file);

        let mut count = 0;
        let mut by_type: HashMap<EventType, usize> = HashMap::new();
        let mut last_id = 0u64;

        for line_result in reader.lines() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            count += 1;

            // Parse event
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                // Get event ID
                if let Some(id) = value.get("eventId").and_then(|v| v.as_u64()) {
                    if id > last_id {
                        last_id = id;
                    }
                }

                // Get event type
                if let Some(type_str) = value.get("eventType").and_then(|v| v.as_str()) {
                    if let Ok(event_type) = serde_json::from_value::<EventType>(
                        serde_json::Value::String(type_str.to_string())
                    ) {
                        *by_type.entry(event_type).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok((count, size, by_type, last_id))
    }

    /// Parse snapshot metadata to get last event ID
    fn parse_snapshot_meta(&self, path: &Path) -> EventStoreResult<Option<u64>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        if let Some(Ok(line)) = reader.lines().next() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(id) = value.get("lastEventId").and_then(|v| v.as_u64()) {
                    return Ok(Some(id));
                }
            }
        }

        Ok(None)
    }

    /// Benchmark replay performance
    pub fn benchmark_replay(&self, iterations: usize) -> EventStoreResult<ReplayBenchmark> {
        let events_path = self.config.events_path();
        if !events_path.exists() {
            return Ok(ReplayBenchmark::default());
        }

        let mut total_duration = std::time::Duration::ZERO;
        let mut event_count = 0;

        for _ in 0..iterations {
            let start = Instant::now();

            let file = File::open(&events_path)?;
            let reader = BufReader::new(file);

            for line_result in reader.lines() {
                let line = line_result?;
                if line.trim().is_empty() {
                    continue;
                }

                // Parse event (simulating replay)
                let _: serde_json::Value = serde_json::from_str(&line)?;
                event_count += 1;
            }

            total_duration += start.elapsed();
        }

        let avg_duration = total_duration / iterations as u32;
        let events_per_iter = event_count / iterations;
        let events_per_sec = if avg_duration.as_secs_f64() > 0.0 {
            events_per_iter as f64 / avg_duration.as_secs_f64()
        } else {
            0.0
        };

        Ok(ReplayBenchmark {
            iterations,
            events_per_iteration: events_per_iter,
            avg_duration_ms: avg_duration.as_millis() as u64,
            events_per_second: events_per_sec,
        })
    }
}

/// Replay benchmark results
#[derive(Debug, Clone, Default)]
pub struct ReplayBenchmark {
    /// Number of iterations run
    pub iterations: usize,
    /// Events per iteration
    pub events_per_iteration: usize,
    /// Average duration in milliseconds
    pub avg_duration_ms: u64,
    /// Events replayed per second
    pub events_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_store::EventStore;
    use crate::types::EventType;
    use tempfile::TempDir;

    #[test]
    fn test_collect_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({"name": "Alice", "entity_type": "Person", "observations": []}),
        ).unwrap();
        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({"name": "Bob", "entity_type": "Person", "observations": []}),
        ).unwrap();
        store.create_and_append_event(
            EventType::RelationCreated,
            "user".to_string(),
            serde_json::json!({"from": "Alice", "to": "Bob", "relation_type": "knows"}),
        ).unwrap();

        // Collect stats
        let collector = StatsCollector::new(config);
        let stats = collector.collect().unwrap();

        assert_eq!(stats.active_event_count, 3);
        assert_eq!(stats.last_event_id, 3);
        assert!(stats.active_log_size > 0);

        // Verify events by type
        assert_eq!(stats.events_by_type.get(&EventType::EntityCreated), Some(&2));
        assert_eq!(stats.events_by_type.get(&EventType::RelationCreated), Some(&1));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(EventStoreStats::format_size(500), "500 B");
        assert_eq!(EventStoreStats::format_size(1024), "1.00 KB");
        assert_eq!(EventStoreStats::format_size(1536), "1.50 KB");
        assert_eq!(EventStoreStats::format_size(1048576), "1.00 MB");
        assert_eq!(EventStoreStats::format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_benchmark_replay() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::new(temp_dir.path().join("data"));

        // Create some events
        let mut store = EventStore::with_config(config.clone());
        for i in 1..=10 {
            store.create_and_append_event(
                EventType::EntityCreated,
                "user".to_string(),
                serde_json::json!({"name": format!("Entity{}", i), "entity_type": "Test", "observations": []}),
            ).unwrap();
        }

        // Benchmark
        let collector = StatsCollector::new(config);
        let benchmark = collector.benchmark_replay(3).unwrap();

        assert_eq!(benchmark.iterations, 3);
        assert_eq!(benchmark.events_per_iteration, 10);
        assert!(benchmark.events_per_second > 0.0);
    }
}
