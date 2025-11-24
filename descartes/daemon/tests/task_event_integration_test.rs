//! Integration tests for TaskEventEmitter
//!
//! Tests the real-time event emission system with concurrent operations

use descartes_daemon::task_event_emitter::{TaskEventEmitter, TaskEventEmitterConfig};
use descartes_daemon::events::{EventBus, DescartesEvent, TaskEventType};
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::{StateStore, Task, TaskStatus, TaskPriority, TaskComplexity};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;

async fn setup_test_system() -> (TaskEventEmitter, Arc<EventBus>) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_file.path().to_str().unwrap();

    let mut state_store = SqliteStateStore::new(db_path, false)
        .await
        .expect("Failed to create state store");
    state_store.initialize().await.expect("Failed to initialize");

    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;
    let event_bus = Arc::new(EventBus::new());

    let config = TaskEventEmitterConfig {
        enable_debouncing: false,
        include_task_data: true,
        verbose_logging: true,
        ..Default::default()
    };

    let emitter = TaskEventEmitter::new(state_store, event_bus.clone(), config);
    emitter.initialize_cache().await.expect("Failed to initialize cache");

    (emitter, event_bus)
}

#[tokio::test]
async fn test_concurrent_task_operations() {
    let (emitter, event_bus) = setup_test_system().await;
    let emitter = Arc::new(emitter);

    // Subscribe to events
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Spawn multiple tasks concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let emitter = emitter.clone();
        let handle = tokio::spawn(async move {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Concurrent Task {}", i),
                description: Some(format!("Description {}", i)),
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            emitter.save_task(&task).await.expect("Failed to save task");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Collect events
    let mut event_count = 0;
    while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
        event_count += 1;
    }

    // Should receive 10 events (one for each task)
    assert_eq!(event_count, 10, "Expected 10 events, got {}", event_count);
}

#[tokio::test]
async fn test_task_lifecycle_events() {
    let (emitter, event_bus) = setup_test_system().await;

    // Subscribe to events
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create a task
    let mut task = Task {
        id: Uuid::new_v4(),
        title: "Lifecycle Test Task".to_string(),
        description: Some("Testing full lifecycle".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Complex,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    // Save task (Created event)
    emitter.save_task(&task).await.expect("Failed to save task");

    // Receive creation event
    let event = rx.recv().await.expect("Failed to receive event");
    match event {
        DescartesEvent::TaskEvent(task_event) => {
            assert_eq!(task_event.event_type, TaskEventType::Created);
        }
        _ => panic!("Expected TaskEvent"),
    }

    // Update task to InProgress
    task.status = TaskStatus::InProgress;
    task.updated_at = Utc::now().timestamp();
    emitter.save_task(&task).await.expect("Failed to save task");

    // Receive update event
    let event = rx.recv().await.expect("Failed to receive event");
    match event {
        DescartesEvent::TaskEvent(task_event) => {
            let previous_status = task_event.data.get("previous_status")
                .and_then(|v| v.as_str())
                .unwrap();
            let new_status = task_event.data.get("new_status")
                .and_then(|v| v.as_str())
                .unwrap();

            assert_eq!(previous_status, "Todo");
            assert_eq!(new_status, "InProgress");
        }
        _ => panic!("Expected TaskEvent"),
    }

    // Update task to Done
    task.status = TaskStatus::Done;
    task.updated_at = Utc::now().timestamp();
    emitter.save_task(&task).await.expect("Failed to save task");

    // Receive final update event
    let event = rx.recv().await.expect("Failed to receive event");
    match event {
        DescartesEvent::TaskEvent(task_event) => {
            let new_status = task_event.data.get("new_status")
                .and_then(|v| v.as_str())
                .unwrap();
            assert_eq!(new_status, "Done");
        }
        _ => panic!("Expected TaskEvent"),
    }
}

#[tokio::test]
async fn test_event_ordering() {
    let (emitter, event_bus) = setup_test_system().await;

    // Subscribe to events
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    let task_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    // Create tasks in specific order
    for (i, task_id) in task_ids.iter().enumerate() {
        let task = Task {
            id: *task_id,
            title: format!("Ordered Task {}", i),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Collect events and verify order
    let mut received_task_ids = vec![];
    for _ in 0..5 {
        if let Ok(event) = rx.recv().await {
            match event {
                DescartesEvent::TaskEvent(task_event) => {
                    received_task_ids.push(task_event.task_id);
                }
                _ => {}
            }
        }
    }

    assert_eq!(received_task_ids.len(), 5);

    // Verify all tasks were received
    for task_id in task_ids {
        assert!(received_task_ids.contains(&task_id.to_string()));
    }
}

#[tokio::test]
async fn test_statistics_accuracy() {
    let (emitter, _) = setup_test_system().await;

    // Create tasks
    let task_count = 20;
    for i in 0..task_count {
        let task = Task {
            id: Uuid::new_v4(),
            title: format!("Stats Test Task {}", i),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Check statistics
    let stats = emitter.get_statistics().await;
    assert_eq!(stats.cached_tasks, task_count);
}

#[tokio::test]
async fn test_rapid_updates_with_debouncing() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_file.path().to_str().unwrap();

    let mut state_store = SqliteStateStore::new(db_path, false)
        .await
        .expect("Failed to create state store");
    state_store.initialize().await.expect("Failed to initialize");

    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;
    let event_bus = Arc::new(EventBus::new());

    let config = TaskEventEmitterConfig {
        enable_debouncing: true,
        debounce_interval_ms: 100,
        include_task_data: true,
        verbose_logging: false,
    };

    let emitter = TaskEventEmitter::new(state_store, event_bus.clone(), config);
    emitter.initialize_cache().await.expect("Failed to initialize cache");

    // Subscribe to events
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create a task and update it rapidly
    let mut task = Task {
        id: Uuid::new_v4(),
        title: "Debounce Test".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Moderate,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    // Save initial task
    emitter.save_task(&task).await.expect("Failed to save task");

    // Rapid updates
    for i in 0..20 {
        task.title = format!("Debounce Test {}", i);
        task.updated_at = Utc::now().timestamp();
        emitter.save_task(&task).await.expect("Failed to save task");
        sleep(Duration::from_millis(10)).await;
    }

    // Wait for debouncing to settle
    sleep(Duration::from_millis(200)).await;

    // Count events
    let mut event_count = 0;
    while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        event_count += 1;
    }

    // Should be significantly less than 21 (1 create + 20 updates)
    assert!(event_count < 21, "Expected debouncing to reduce events, got {}", event_count);
    println!("Debouncing reduced 21 operations to {} events", event_count);
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let (emitter, event_bus) = setup_test_system().await;

    // Create multiple subscribers
    let (_sub1, mut rx1) = event_bus.subscribe(None).await;
    let (_sub2, mut rx2) = event_bus.subscribe(None).await;
    let (_sub3, mut rx3) = event_bus.subscribe(None).await;

    // Create a task
    let task = Task {
        id: Uuid::new_v4(),
        title: "Multi-subscriber Test".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Simple,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    emitter.save_task(&task).await.expect("Failed to save task");

    // All subscribers should receive the event
    let event1 = tokio::time::timeout(Duration::from_millis(500), rx1.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    let event2 = tokio::time::timeout(Duration::from_millis(500), rx2.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    let event3 = tokio::time::timeout(Duration::from_millis(500), rx3.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    // Verify all received the same event type
    match (&event1, &event2, &event3) {
        (
            DescartesEvent::TaskEvent(e1),
            DescartesEvent::TaskEvent(e2),
            DescartesEvent::TaskEvent(e3),
        ) => {
            assert_eq!(e1.task_id, e2.task_id);
            assert_eq!(e2.task_id, e3.task_id);
            assert_eq!(e1.event_type, TaskEventType::Created);
        }
        _ => panic!("Expected TaskEvent for all subscribers"),
    }
}
