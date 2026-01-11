//! Event Sourcing Integration Tests
//!
//! Tests for the complete Event Sourcing flow including:
//! - Event emission on CRUD operations
//! - Snapshot creation and loading
//! - State recovery from snapshot + events
//! - Migration from legacy format

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use memory_graph::event_store::{EventStore, EventStoreConfig, MigrationTool, SnapshotManager};
use memory_graph::knowledge_base::KnowledgeBase;
use memory_graph::types::{Entity, EventType, Relation};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_data_dir() -> std::path::PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::path::PathBuf::from(format!(
        "target/test_event_sourcing_{}_{}",
        std::process::id(),
        id
    ))
}

fn cleanup_dir(path: &std::path::Path) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_event_store_append_and_load() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);

    // Create event store
    let mut store = EventStore::with_config(config.clone());

    // Append some events
    let event1 = store.create_and_append_event(
        EventType::EntityCreated,
        "test_user".to_string(),
        serde_json::json!({
            "name": "Alice",
            "entity_type": "Person",
            "observations": []
        }),
    ).expect("Failed to append event 1");

    let event2 = store.create_and_append_event(
        EventType::EntityCreated,
        "test_user".to_string(),
        serde_json::json!({
            "name": "Bob",
            "entity_type": "Person",
            "observations": []
        }),
    ).expect("Failed to append event 2");

    assert_eq!(event1.event_id, 1);
    assert_eq!(event2.event_id, 2);

    // Load events
    let loaded_events = store.load_events().expect("Failed to load events");
    assert_eq!(loaded_events.len(), 2);

    cleanup_dir(&data_dir);
}

#[test]
fn test_snapshot_creation_and_loading() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);
    let snapshot_manager = SnapshotManager::new(config.clone());

    // Create some entities and relations
    let entities = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["Developer".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
        Entity {
            name: "Bob".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["Designer".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    ];

    let relations = vec![
        Relation::new("Alice".to_string(), "Bob".to_string(), "knows".to_string()),
    ];

    // Create snapshot
    let meta = snapshot_manager
        .create_snapshot_with_backup(100, &entities, &relations)
        .expect("Failed to create snapshot");

    assert_eq!(meta.entity_count, 2);
    assert_eq!(meta.relation_count, 1);
    assert_eq!(meta.last_event_id, 100);

    // Load snapshot
    let (loaded_meta, loaded_entities, loaded_relations) = snapshot_manager
        .load_full()
        .expect("Failed to load snapshot")
        .expect("Snapshot not found");

    assert_eq!(loaded_meta.last_event_id, 100);
    assert_eq!(loaded_entities.len(), 2);
    assert_eq!(loaded_relations.len(), 1);
    assert_eq!(loaded_entities[0].name, "Alice");
    assert_eq!(loaded_entities[1].name, "Bob");
    assert_eq!(loaded_relations[0].from, "Alice");

    cleanup_dir(&data_dir);
}

#[test]
fn test_state_recovery_from_snapshot_and_events() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);

    // Create initial state with events
    {
        let mut store = EventStore::with_config(config.clone());

        // Create entities via events
        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({
                "name": "Alice",
                "entity_type": "Person",
                "observations": ["Developer"]
            }),
        ).unwrap();

        store.create_and_append_event(
            EventType::EntityCreated,
            "user".to_string(),
            serde_json::json!({
                "name": "Bob",
                "entity_type": "Person",
                "observations": []
            }),
        ).unwrap();

        // Create snapshot at event 2
        let snapshot_manager = SnapshotManager::new(config.clone());
        let entities = vec![
            Entity {
                name: "Alice".to_string(),
                entity_type: "Person".to_string(),
                observations: vec!["Developer".to_string()],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
            Entity {
                name: "Bob".to_string(),
                entity_type: "Person".to_string(),
                observations: vec![],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        snapshot_manager.create_snapshot_with_backup(2, &entities, &[]).unwrap();
        store.snapshot_created(2);

        // Add more events after snapshot
        store.create_and_append_event(
            EventType::RelationCreated,
            "user".to_string(),
            serde_json::json!({
                "from": "Alice",
                "to": "Bob",
                "relation_type": "knows"
            }),
        ).unwrap();

        store.create_and_append_event(
            EventType::ObservationAdded,
            "user".to_string(),
            serde_json::json!({
                "entity": "Bob",
                "observation": "Nice person"
            }),
        ).unwrap();
    }

    // Simulate restart - initialize from snapshot + replay
    {
        let mut store = EventStore::with_config(config.clone());
        let snapshot_manager = SnapshotManager::new(config.clone());

        // Load snapshot
        let (meta, mut entities, mut relations) = snapshot_manager
            .load_full()
            .unwrap()
            .unwrap();

        assert_eq!(meta.last_event_id, 2);
        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 0);

        // Replay events after snapshot
        let new_events = store.load_events_after(meta.last_event_id).unwrap();
        assert_eq!(new_events.len(), 2); // relation_created + observation_added

        // Apply events to state
        for event in &new_events {
            EventStore::apply_event(&mut entities, &mut relations, event).unwrap();
        }

        // Verify final state
        assert_eq!(entities.len(), 2);
        assert_eq!(relations.len(), 1);

        let bob = entities.iter().find(|e| e.name == "Bob").unwrap();
        assert!(bob.observations.contains(&"Nice person".to_string()));
    }

    cleanup_dir(&data_dir);
}

#[test]
fn test_migration_from_legacy() {
    let data_dir = test_data_dir();
    fs::create_dir_all(&data_dir).unwrap();

    // Create a legacy memory.jsonl file
    let legacy_path = data_dir.join("memory.jsonl");
    let legacy_content = r#"{"name":"Alice","entityType":"Person","observations":["Developer"]}
{"name":"Bob","entityType":"Person","observations":["Designer"]}
{"from":"Alice","to":"Bob","relationType":"knows"}"#;

    fs::write(&legacy_path, legacy_content).unwrap();

    // Run migration
    let migration_config = EventStoreConfig::new(data_dir.join("event_sourcing"));
    let migration_tool = MigrationTool::with_config(migration_config.clone());

    let result = migration_tool.migrate_from_legacy(&legacy_path).unwrap();

    assert_eq!(result.entities_migrated, 2);
    assert_eq!(result.relations_migrated, 1);
    assert_eq!(result.events_created, 3);
    assert!(result.snapshot_created);

    // Verify events were created
    assert!(migration_config.events_path().exists());

    // Verify snapshot was created
    assert!(migration_config.latest_snapshot_path().exists());

    // Load and verify snapshot content
    let snapshot_manager = SnapshotManager::new(migration_config);
    let (meta, entities, relations) = snapshot_manager.load_full().unwrap().unwrap();

    assert_eq!(meta.last_event_id, 3);
    assert_eq!(entities.len(), 2);
    assert_eq!(relations.len(), 1);

    cleanup_dir(&data_dir);
}

#[test]
fn test_backup_snapshot_recovery() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);
    let snapshot_manager = SnapshotManager::new(config.clone());

    // Create entities
    let entities_v1 = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["v1".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    ];

    let entities_v2 = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["v2".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
        Entity {
            name: "Bob".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["new".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    ];

    // Create first snapshot
    snapshot_manager.create_snapshot_with_backup(10, &entities_v1, &[]).unwrap();

    // Create second snapshot (first becomes backup)
    snapshot_manager.create_snapshot_with_backup(20, &entities_v2, &[]).unwrap();

    // Verify latest has v2
    let (meta, entities, _) = snapshot_manager.load_full().unwrap().unwrap();
    assert_eq!(meta.last_event_id, 20);
    assert_eq!(entities.len(), 2);

    // Verify backup (previous.jsonl) exists
    assert!(config.previous_snapshot_path().exists());

    // Simulate corruption of latest - delete it
    fs::remove_file(config.latest_snapshot_path()).unwrap();

    // Recover from backup - returns the recovered data directly
    let (meta, entities, _) = snapshot_manager
        .recover_from_backup()
        .expect("Failed to recover from backup")
        .expect("No backup found");

    // Verify recovery restored v1
    assert_eq!(meta.last_event_id, 10);
    assert_eq!(entities.len(), 1);
    // After recovery, the entity should have the v1 observation
    assert!(!entities[0].observations.is_empty(), "Entity should have observations");
    assert_eq!(entities[0].observations[0], "v1");

    cleanup_dir(&data_dir);
}

#[test]
fn test_event_store_replay_all() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);
    let mut store = EventStore::with_config(config.clone());

    // Create events
    store.create_and_append_event(
        EventType::EntityCreated,
        "user".to_string(),
        serde_json::json!({
            "name": "Alice",
            "entity_type": "Person",
            "observations": []
        }),
    ).unwrap();

    store.create_and_append_event(
        EventType::ObservationAdded,
        "user".to_string(),
        serde_json::json!({
            "entity": "Alice",
            "observation": "Knows Rust"
        }),
    ).unwrap();

    store.create_and_append_event(
        EventType::EntityCreated,
        "user".to_string(),
        serde_json::json!({
            "name": "Bob",
            "entity_type": "Person",
            "observations": []
        }),
    ).unwrap();

    store.create_and_append_event(
        EventType::RelationCreated,
        "user".to_string(),
        serde_json::json!({
            "from": "Alice",
            "to": "Bob",
            "relation_type": "mentors"
        }),
    ).unwrap();

    // Replay all events to rebuild state
    let (entities, relations, _last_event_id) = store.replay_all().unwrap();

    assert_eq!(entities.len(), 2);
    assert_eq!(relations.len(), 1);

    let alice = entities.iter().find(|e| e.name == "Alice").unwrap();
    assert!(alice.observations.contains(&"Knows Rust".to_string()));

    let relation = &relations[0];
    assert_eq!(relation.from, "Alice");
    assert_eq!(relation.to, "Bob");
    assert_eq!(relation.relation_type, "mentors");

    cleanup_dir(&data_dir);
}

#[test]
fn test_event_store_replay_after_snapshot() {
    let data_dir = test_data_dir();
    let config = EventStoreConfig::new(&data_dir);
    let mut store = EventStore::with_config(config.clone());

    // Create events 1-3
    for i in 1..=3 {
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

    // Simulate snapshot at event 2
    store.snapshot_created(2);

    // Create more events 4-5
    for i in 4..=5 {
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

    // Replay only events after snapshot
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    store.replay_after(&mut entities, &mut relations, 2).unwrap();

    // Should only have entities from events 3, 4, 5 (after snapshot)
    assert_eq!(entities.len(), 3);
    assert!(entities.iter().any(|e| e.name == "Entity3"));
    assert!(entities.iter().any(|e| e.name == "Entity4"));
    assert!(entities.iter().any(|e| e.name == "Entity5"));

    cleanup_dir(&data_dir);
}
