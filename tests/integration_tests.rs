//! Integration tests for Memory Graph MCP Server

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use memory_graph::knowledge_base::KnowledgeBase;
use memory_graph::types::{Entity, Observation, Relation};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn setup_test_kb() -> (Arc<KnowledgeBase>, String) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_file = format!(
        "test_memory_{}_{}.jsonl",
        std::process::id(),
        id
    );

    // Create a test file path
    let kb = Arc::new(KnowledgeBase::with_file_path(temp_file.clone()));
    (kb, temp_file)
}

fn cleanup(file_path: &str) {
    let _ = fs::remove_file(file_path);
}

#[test]
fn test_create_entities() {
    let (kb, temp_file) = setup_test_kb();

    let entities = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["Lives in NYC".to_string()],
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

    let created = kb.create_entities(entities).unwrap();
    assert_eq!(created.len(), 2);

    let graph = kb.read_graph(None, None).unwrap();
    assert_eq!(graph.entities.len(), 2);

    cleanup(&temp_file);
}

#[test]
fn test_create_relations() {
    let (kb, temp_file) = setup_test_kb();

    // First create entities
    let entities = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec![],
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
    kb.create_entities(entities).unwrap();

    // Then create relations
    let relations = vec![Relation {
        from: "Alice".to_string(),
        to: "Bob".to_string(),
        relation_type: "knows".to_string(),
        created_by: String::new(),
        created_at: 0,
        valid_from: None,
        valid_to: None,
    }];

    let created = kb.create_relations(relations).unwrap();
    assert_eq!(created.len(), 1);

    let graph = kb.read_graph(None, None).unwrap();
    assert_eq!(graph.relations.len(), 1);

    cleanup(&temp_file);
}

#[test]
fn test_search_nodes() {
    let (kb, temp_file) = setup_test_kb();

    let entities = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["Software Engineer".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
        Entity {
            name: "Bob".to_string(),
            entity_type: "Person".to_string(),
            observations: vec!["Doctor".to_string()],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    ];
    kb.create_entities(entities).unwrap();

    let result = kb.search_nodes("Alice", None, true).unwrap();
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].name, "Alice");

    let result = kb.search_nodes("Engineer", None, true).unwrap();
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].name, "Alice");

    cleanup(&temp_file);
}

#[test]
fn test_delete_entities() {
    let (kb, temp_file) = setup_test_kb();

    let entities = vec![
        Entity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            observations: vec![],
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
    kb.create_entities(entities).unwrap();

    kb.delete_entities(vec!["Alice".to_string()]).unwrap();

    let graph = kb.read_graph(None, None).unwrap();
    assert_eq!(graph.entities.len(), 1);
    assert_eq!(graph.entities[0].name, "Bob");

    cleanup(&temp_file);
}

#[test]
fn test_concurrent_access() {
    let (kb, temp_file) = setup_test_kb();

    // Spawn multiple threads simulating concurrent agents
    let mut handles = vec![];

    for i in 0..10 {
        let kb_clone = Arc::clone(&kb);
        let handle = thread::spawn(move || {
            // Each "agent" creates an entity
            let entity = Entity {
                name: format!("Agent{}", i),
                entity_type: "Person".to_string(),
                observations: vec![format!("Created by thread {}", i)],
                created_by: String::new(),
                updated_by: String::new(),
                created_at: 0,
                updated_at: 0,
            };
            kb_clone.create_entities(vec![entity]).unwrap();

            // Each agent also reads the graph
            let graph = kb_clone.read_graph(None, None).unwrap();
            assert!(graph.entities.len() >= 1);

            // Each agent adds an observation
            let obs = Observation {
                entity_name: format!("Agent{}", i),
                contents: vec![format!("Observation from thread {}", i)],
            };
            let _ = kb_clone.add_observations(vec![obs]);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify final state
    let graph = kb.read_graph(None, None).unwrap();
    assert_eq!(graph.entities.len(), 10, "All 10 entities should exist");

    // Verify all entities have observations
    for entity in &graph.entities {
        assert!(
            entity.observations.len() >= 1,
            "Entity should have observations"
        );
    }

    cleanup(&temp_file);
}

#[test]
fn test_concurrent_read_write() {
    let (kb, temp_file) = setup_test_kb();

    // Pre-populate with some entities
    for i in 0..5 {
        let entity = Entity {
            name: format!("Entity{}", i),
            entity_type: "Module".to_string(),
            observations: vec![],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        };
        kb.create_entities(vec![entity]).unwrap();
    }

    let mut handles = vec![];

    // 5 reader threads
    for _ in 0..5 {
        let kb_clone = Arc::clone(&kb);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let graph = kb_clone.read_graph(None, None).unwrap();
                assert!(graph.entities.len() >= 5);
                let _ = kb_clone.search_nodes("Entity", None, true);
            }
        });
        handles.push(handle);
    }

    // 3 writer threads
    for i in 0..3 {
        let kb_clone = Arc::clone(&kb);
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let obs = Observation {
                    entity_name: format!("Entity{}", i),
                    contents: vec![format!("Update {} from writer {}", j, i)],
                };
                let _ = kb_clone.add_observations(vec![obs]);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify no data corruption
    let graph = kb.read_graph(None, None).unwrap();
    assert_eq!(
        graph.entities.len(),
        5,
        "Original entities should still exist"
    );

    cleanup(&temp_file);
}

#[test]
fn test_semantic_search_synonyms() {
    let (kb, temp_file) = setup_test_kb();

    let entities = vec![Entity {
        name: "Alice".to_string(),
        entity_type: "Person".to_string(),
        observations: vec!["Software developer working on backend".to_string()],
        created_by: String::new(),
        updated_by: String::new(),
        created_at: 0,
        updated_at: 0,
    }];
    kb.create_entities(entities).unwrap();

    // Search with synonym "coder" should find "developer"
    let result = kb.search_nodes("coder", None, true).unwrap();
    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].name, "Alice");

    // Search with synonym "programmer" should also find "developer"
    let result = kb.search_nodes("programmer", None, true).unwrap();
    assert_eq!(result.entities.len(), 1);

    cleanup(&temp_file);
}

#[test]
fn test_pagination() {
    let (kb, temp_file) = setup_test_kb();

    // Create 20 entities
    for i in 0..20 {
        let entity = Entity {
            name: format!("Entity{:02}", i),
            entity_type: "Module".to_string(),
            observations: vec![],
            created_by: String::new(),
            updated_by: String::new(),
            created_at: 0,
            updated_at: 0,
        };
        kb.create_entities(vec![entity]).unwrap();
    }

    // Test limit
    let result = kb.read_graph(Some(5), None).unwrap();
    assert_eq!(result.entities.len(), 5);

    // Test offset
    let result = kb.read_graph(Some(5), Some(10)).unwrap();
    assert_eq!(result.entities.len(), 5);

    // Test beyond range
    let result = kb.read_graph(Some(100), Some(50)).unwrap();
    assert_eq!(result.entities.len(), 0);

    cleanup(&temp_file);
}
