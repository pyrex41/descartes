use chrono::Utc;
/// Comprehensive integration tests for SqliteStateStore
use descartes_core::{
    ActorType, AgentState, Event, SqliteStateStore, StateStore, Task, TaskStatus,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use uuid::Uuid;

fn setup_test_db(name: &str) -> String {
    let db_path = format!("/tmp/test_state_{}.db", name);
    let _ = fs::remove_file(&db_path);
    db_path
}

#[tokio::test]
async fn test_store_creation_and_initialization() {
    let db_path = setup_test_db("create_init");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    // Verify file was created
    assert!(Path::new(&db_path).exists());
}

#[tokio::test]
async fn test_save_and_load_agent_state() {
    let db_path = setup_test_db("save_load");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let state = AgentState {
        agent_id: "test_agent_1".to_string(),
        name: "TestAgent".to_string(),
        status: "running".to_string(),
        metadata: json!({
            "type": "worker",
            "capacity": 100
        }),
        state_data: json!({
            "iteration": 42,
            "tasks": 10
        })
        .to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    store
        .save_agent_state(&state)
        .await
        .expect("Failed to save");

    let loaded = store
        .load_agent_state("test_agent_1")
        .await
        .expect("Failed to load")
        .expect("State not found");

    assert_eq!(loaded.agent_id, state.agent_id);
    assert_eq!(loaded.name, state.name);
    assert_eq!(loaded.status, state.status);
    assert_eq!(loaded.version, state.version);
}

#[tokio::test]
async fn test_list_agents() {
    let db_path = setup_test_db("list_agents");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    // Save multiple agents
    for i in 0..5 {
        let state = AgentState {
            agent_id: format!("agent_{}", i),
            name: format!("Agent {}", i),
            status: "idle".to_string(),
            metadata: json!({ "index": i }),
            state_data: "{}".to_string(),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            version: 1,
        };

        store
            .save_agent_state(&state)
            .await
            .expect("Failed to save");
    }

    let agents = store.list_agents().await.expect("Failed to list");
    assert_eq!(agents.len(), 5);

    for (i, agent) in agents.iter().enumerate() {
        assert_eq!(agent.agent_id, format!("agent_{}", i));
    }
}

#[tokio::test]
async fn test_update_agent_status() {
    let db_path = setup_test_db("update_status");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let state = AgentState {
        agent_id: "agent_status_test".to_string(),
        name: "StatusTest".to_string(),
        status: "running".to_string(),
        metadata: json!({}),
        state_data: "{}".to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    store
        .save_agent_state(&state)
        .await
        .expect("Failed to save");

    store
        .update_agent_status("agent_status_test", "paused")
        .await
        .expect("Failed to update");

    let updated = store
        .load_agent_state("agent_status_test")
        .await
        .expect("Failed to load")
        .expect("State not found");

    assert_eq!(updated.status, "paused");
}

#[tokio::test]
async fn test_delete_agent() {
    let db_path = setup_test_db("delete_agent");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let state = AgentState {
        agent_id: "agent_delete".to_string(),
        name: "DeleteTest".to_string(),
        status: "running".to_string(),
        metadata: json!({}),
        state_data: "{}".to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    store
        .save_agent_state(&state)
        .await
        .expect("Failed to save");

    store
        .delete_agent("agent_delete")
        .await
        .expect("Failed to delete");

    let loaded = store
        .load_agent_state("agent_delete")
        .await
        .expect("Failed to load");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_state_transitions() {
    let db_path = setup_test_db("transitions");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let agent_id = "agent_transition";

    // Record transitions
    store
        .record_state_transition(
            agent_id,
            r#"{"status":"idle"}"#,
            r#"{"status":"running"}"#,
            Some("Started by user".to_string()),
        )
        .await
        .expect("Failed to record");

    store
        .record_state_transition(
            agent_id,
            r#"{"status":"running"}"#,
            r#"{"status":"completed"}"#,
            Some("Task finished".to_string()),
        )
        .await
        .expect("Failed to record");

    // Get history
    let history = store
        .get_state_history(agent_id, 10)
        .await
        .expect("Failed to get history");

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].state_after, r#"{"status":"completed"}"#);
    assert_eq!(history[1].state_after, r#"{"status":"running"}"#);
}

#[tokio::test]
async fn test_snapshots() {
    let db_path = setup_test_db("snapshots");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let agent_id = "agent_snapshot";

    // Create initial state
    let state = AgentState {
        agent_id: agent_id.to_string(),
        name: "SnapshotTest".to_string(),
        status: "running".to_string(),
        metadata: json!({}),
        state_data: r#"{"iteration":100}"#.to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    store
        .save_agent_state(&state)
        .await
        .expect("Failed to save");

    // Create snapshot
    let snapshot_id = store
        .create_snapshot(agent_id, Some("Test snapshot".to_string()))
        .await
        .expect("Failed to create snapshot");

    assert!(!snapshot_id.is_empty());

    // List snapshots
    let snapshots = store
        .list_snapshots(agent_id)
        .await
        .expect("Failed to list snapshots");

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].0, snapshot_id);

    // Restore from snapshot
    store
        .restore_snapshot(&snapshot_id)
        .await
        .expect("Failed to restore");

    let restored = store
        .load_agent_state(agent_id)
        .await
        .expect("Failed to load")
        .expect("State not found");

    assert_eq!(restored.state_data, r#"{"iteration":100}"#);
}

#[tokio::test]
async fn test_save_and_retrieve_events() {
    let db_path = setup_test_db("events");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let session_id = "session_123";
    let event = Event {
        id: Uuid::new_v4(),
        event_type: "test_event".to_string(),
        timestamp: Utc::now().timestamp(),
        session_id: session_id.to_string(),
        actor_type: ActorType::Agent,
        actor_id: "agent_1".to_string(),
        content: "Test event content".to_string(),
        metadata: Some(json!({ "key": "value" })),
        git_commit: Some("abc123".to_string()),
    };

    store
        .save_event(&event)
        .await
        .expect("Failed to save event");

    let events = store
        .get_events(session_id)
        .await
        .expect("Failed to get events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "test_event");
    assert_eq!(events[0].actor_id, "agent_1");
}

#[tokio::test]
async fn test_get_events_by_type() {
    let db_path = setup_test_db("events_by_type");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    // Save events of different types
    for i in 0..3 {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "type_a".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "session_1".to_string(),
            actor_type: ActorType::Agent,
            actor_id: format!("agent_{}", i),
            content: format!("Event {}", i),
            metadata: None,
            git_commit: None,
        };

        store.save_event(&event).await.expect("Failed to save");
    }

    for i in 0..2 {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "type_b".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "session_2".to_string(),
            actor_type: ActorType::System,
            actor_id: format!("system_{}", i),
            content: format!("System event {}", i),
            metadata: None,
            git_commit: None,
        };

        store.save_event(&event).await.expect("Failed to save");
    }

    let type_a_events = store
        .get_events_by_type("type_a")
        .await
        .expect("Failed to get type_a events");
    assert_eq!(type_a_events.len(), 3);

    let type_b_events = store
        .get_events_by_type("type_b")
        .await
        .expect("Failed to get type_b events");
    assert_eq!(type_b_events.len(), 2);
}

#[tokio::test]
async fn test_search_events() {
    let db_path = setup_test_db("search_events");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let event1 = Event {
        id: Uuid::new_v4(),
        event_type: "search_test".to_string(),
        timestamp: Utc::now().timestamp(),
        session_id: "session_1".to_string(),
        actor_type: ActorType::Agent,
        actor_id: "agent_1".to_string(),
        content: "Transaction completed successfully".to_string(),
        metadata: None,
        git_commit: None,
    };

    let event2 = Event {
        id: Uuid::new_v4(),
        event_type: "search_test".to_string(),
        timestamp: Utc::now().timestamp(),
        session_id: "session_1".to_string(),
        actor_type: ActorType::Agent,
        actor_id: "agent_2".to_string(),
        content: "No transaction found".to_string(),
        metadata: None,
        git_commit: None,
    };

    store.save_event(&event1).await.expect("Failed to save");
    store.save_event(&event2).await.expect("Failed to save");

    let results = store
        .search_events("transaction")
        .await
        .expect("Failed to search");

    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_save_and_retrieve_tasks() {
    let db_path = setup_test_db("tasks");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let task_id = Uuid::new_v4();
    let task = Task {
        id: task_id,
        title: "Test Task".to_string(),
        description: Some("A test task".to_string()),
        status: TaskStatus::InProgress,
        assigned_to: Some("agent_1".to_string()),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: Some(json!({ "priority": "high" })),
    };

    store.save_task(&task).await.expect("Failed to save task");

    let fetched = store
        .get_task(&task_id)
        .await
        .expect("Failed to get task")
        .expect("Task not found");

    assert_eq!(fetched.title, "Test Task");
    assert_eq!(fetched.status, TaskStatus::InProgress);
    assert_eq!(fetched.assigned_to, Some("agent_1".to_string()));
}

#[tokio::test]
async fn test_get_all_tasks() {
    let db_path = setup_test_db("all_tasks");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    for i in 0..3 {
        let task = Task {
            id: Uuid::new_v4(),
            title: format!("Task {}", i),
            description: None,
            status: TaskStatus::Todo,
            assigned_to: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };

        store.save_task(&task).await.expect("Failed to save");
    }

    let tasks = store.get_tasks().await.expect("Failed to get tasks");

    assert_eq!(tasks.len(), 3);
}

#[tokio::test]
async fn test_key_prefix() {
    let db_path = setup_test_db("key_prefix");

    let store1 = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store")
        .with_prefix("worker".to_string());

    let store2 = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store")
        .with_prefix("coordinator".to_string());

    // Note: Without actually using the stores with different prefixes,
    // we're just verifying they can be created with prefixes
    let _s1 = store1;
    let _s2 = store2;
}

#[tokio::test]
async fn test_concurrent_operations() {
    let db_path = setup_test_db("concurrent");
    let store = std::sync::Arc::new(
        SqliteStateStore::new(&db_path, false)
            .await
            .expect("Failed to create store"),
    );

    {
        let mut store_init = SqliteStateStore::new(&db_path, false)
            .await
            .expect("Failed to create store");
        store_init.initialize().await.expect("Failed to initialize");
    }

    let mut handles = vec![];

    for i in 0..5 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let state = AgentState {
                agent_id: format!("concurrent_agent_{}", i),
                name: format!("ConcurrentAgent{}", i),
                status: "running".to_string(),
                metadata: json!({ "task_id": i }),
                state_data: format!(r#"{{"count":{}}}"#, i),
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                version: 1,
            };

            store_clone
                .save_agent_state(&state)
                .await
                .expect("Failed to save");

            let loaded = store_clone
                .load_agent_state(&format!("concurrent_agent_{}", i))
                .await
                .expect("Failed to load");

            assert!(loaded.is_some());
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }
}

#[tokio::test]
async fn test_migrations_applied() {
    let db_path = setup_test_db("migrations");
    let mut store = SqliteStateStore::new(&db_path, false)
        .await
        .expect("Failed to create store");

    store.initialize().await.expect("Failed to initialize");

    let migrations = store
        .get_migration_history()
        .await
        .expect("Failed to get migrations");

    assert!(migrations.len() >= 4);
    assert_eq!(migrations[0].version, 1);
    assert_eq!(migrations[1].version, 2);
    assert_eq!(migrations[2].version, 3);
    assert_eq!(migrations[3].version, 4);
}
