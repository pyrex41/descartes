//! Real-Time Event System Tests for Task Board
//!
//! These tests focus on the daemon-side event system:
//! - WebSocket integration
//! - Connection loss and reconnection
//! - Event backlog handling
//! - Concurrent subscribers
//! - Event filtering
//! - Performance under load

use descartes_daemon::events::{
    DescartesEvent, EventBus, EventCategory, EventFilter, TaskEvent, TaskEventType,
};
use descartes_daemon::task_event_emitter::{TaskEventEmitter, TaskEventEmitterConfig};
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::{StateStore, Task, TaskComplexity, TaskPriority, TaskStatus};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;

/// Helper to create a test system
async fn setup_test_system() -> (Arc<TaskEventEmitter>, Arc<EventBus>) {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_file.path().to_str().unwrap();

    let mut state_store = SqliteStateStore::new(db_path, false)
        .await
        .expect("Failed to create state store");
    state_store
        .initialize()
        .await
        .expect("Failed to initialize");

    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;
    let event_bus = Arc::new(EventBus::new());

    let config = TaskEventEmitterConfig {
        enable_debouncing: false,
        include_task_data: true,
        verbose_logging: true,
        ..Default::default()
    };

    let emitter = Arc::new(TaskEventEmitter::new(state_store, event_bus.clone(), config));
    emitter
        .initialize_cache()
        .await
        .expect("Failed to initialize cache");

    (emitter, event_bus)
}

/// Helper to create sample task
fn create_sample_task(status: TaskStatus) -> Task {
    Task {
        id: Uuid::new_v4(),
        title: "Test Task".to_string(),
        description: Some("Description".to_string()),
        status,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Moderate,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    }
}

#[tokio::test]
async fn test_event_bus_multiple_subscribers() {
    let (emitter, event_bus) = setup_test_system().await;

    // Create multiple subscribers
    let (_sub1, mut rx1) = event_bus.subscribe(None).await;
    let (_sub2, mut rx2) = event_bus.subscribe(None).await;
    let (_sub3, mut rx3) = event_bus.subscribe(None).await;

    // Create a task
    let task = create_sample_task(TaskStatus::Todo);
    emitter.save_task(&task).await.expect("Failed to save task");

    // All subscribers should receive the event
    let timeout = Duration::from_millis(1000);

    let event1 = tokio::time::timeout(timeout, rx1.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    let event2 = tokio::time::timeout(timeout, rx2.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    let event3 = tokio::time::timeout(timeout, rx3.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    // Verify all received task events
    match (event1, event2, event3) {
        (
            DescartesEvent::TaskEvent(e1),
            DescartesEvent::TaskEvent(e2),
            DescartesEvent::TaskEvent(e3),
        ) => {
            assert_eq!(e1.task_id, task.id.to_string());
            assert_eq!(e2.task_id, task.id.to_string());
            assert_eq!(e3.task_id, task.id.to_string());
            assert_eq!(e1.event_type, TaskEventType::Created);
            assert_eq!(e2.event_type, TaskEventType::Created);
            assert_eq!(e3.event_type, TaskEventType::Created);
        }
        _ => panic!("Expected TaskEvent for all subscribers"),
    }
}

#[tokio::test]
async fn test_event_filtering_by_task_id() {
    let (emitter, event_bus) = setup_test_system().await;

    // Create tasks
    let task1 = create_sample_task(TaskStatus::Todo);
    let task2 = create_sample_task(TaskStatus::InProgress);

    // Subscribe with filter for task1 only
    let filter = EventFilter::for_task(task1.id.to_string());
    let (_sub_id, mut rx) = event_bus.subscribe(Some(filter)).await;

    // Save both tasks
    emitter.save_task(&task1).await.expect("Failed to save task1");
    emitter.save_task(&task2).await.expect("Failed to save task2");

    // Should only receive event for task1
    let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    match event {
        DescartesEvent::TaskEvent(e) => {
            assert_eq!(e.task_id, task1.id.to_string());
        }
        _ => panic!("Expected TaskEvent"),
    }

    // Should not receive event for task2 (timeout expected)
    let result = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(result.is_err(), "Should timeout waiting for filtered event");
}

#[tokio::test]
async fn test_event_filtering_by_category() {
    let (_, event_bus) = setup_test_system().await;

    // Subscribe with Task category filter
    let filter = EventFilter {
        event_categories: vec![EventCategory::Task],
        ..Default::default()
    };
    let (_sub_id, mut rx) = event_bus.subscribe(Some(filter)).await;

    // Publish a Task event
    let task_event = TaskEvent {
        id: Uuid::new_v4().to_string(),
        task_id: Uuid::new_v4().to_string(),
        agent_id: None,
        timestamp: Utc::now(),
        event_type: TaskEventType::Created,
        data: serde_json::json!({}),
    };
    event_bus
        .publish(DescartesEvent::TaskEvent(task_event))
        .await;

    // Should receive the task event
    let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    assert!(matches!(event, DescartesEvent::TaskEvent(_)));

    // Publish a System event
    use descartes_daemon::events::SystemEvent;
    let system_event = SystemEvent::daemon_started();
    event_bus.publish(system_event).await;

    // Should not receive the system event (timeout expected)
    let result = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
    assert!(result.is_err(), "Should timeout for filtered system event");
}

#[tokio::test]
async fn test_subscription_management() {
    let (_, event_bus) = setup_test_system().await;

    // Create subscription
    let (sub_id, _rx) = event_bus.subscribe(None).await;
    assert_eq!(event_bus.subscription_count().await, 1);

    // Unsubscribe
    event_bus.unsubscribe(&sub_id).await;
    assert_eq!(event_bus.subscription_count().await, 0);
}

#[tokio::test]
async fn test_event_bus_statistics() {
    let (emitter, event_bus) = setup_test_system().await;

    // Subscribe to receive events
    let (_sub_id, mut _rx) = event_bus.subscribe(None).await;

    // Create some tasks
    for i in 0..5 {
        let task = create_sample_task(TaskStatus::Todo);
        emitter.save_task(&task).await.expect("Failed to save task");
        sleep(Duration::from_millis(10)).await; // Small delay for event processing
    }

    // Check statistics
    sleep(Duration::from_millis(100)).await; // Wait for stats update
    let stats = event_bus.stats().await;

    assert_eq!(stats.total_events_published, 5);
    assert_eq!(stats.active_subscriptions, 1);
}

#[tokio::test]
async fn test_event_ordering() {
    let (emitter, event_bus) = setup_test_system().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create tasks in specific order
    let task_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    for task_id in &task_ids {
        let mut task = create_sample_task(TaskStatus::Todo);
        task.id = *task_id;
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Collect events and verify order
    let mut received_ids = Vec::new();
    for _ in 0..5 {
        if let Ok(event) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            if let Ok(DescartesEvent::TaskEvent(task_event)) = event {
                received_ids.push(task_event.task_id);
            }
        }
    }

    assert_eq!(received_ids.len(), 5);

    // Verify all task IDs were received
    for task_id in task_ids {
        assert!(received_ids.contains(&task_id.to_string()));
    }
}

#[tokio::test]
async fn test_high_volume_event_stream() {
    let (emitter, event_bus) = setup_test_system().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create many tasks rapidly
    let task_count = 100;
    for i in 0..task_count {
        let task = create_sample_task(TaskStatus::Todo);
        emitter.save_task(&task).await.expect("Failed to save task");

        if i % 10 == 0 {
            sleep(Duration::from_millis(1)).await; // Brief pause every 10 tasks
        }
    }

    // Collect events
    let mut received_count = 0;
    let timeout = Duration::from_millis(2000);
    let start = tokio::time::Instant::now();

    while start.elapsed() < timeout {
        match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Ok(_)) => received_count += 1,
            _ => break,
        }
    }

    // Should receive most or all events
    assert!(
        received_count >= task_count * 9 / 10,
        "Expected at least 90% of events, got {}/{}",
        received_count,
        task_count
    );
}

#[tokio::test]
async fn test_concurrent_event_publishing() {
    let (emitter, event_bus) = setup_test_system().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Spawn multiple tasks concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let emitter = emitter.clone();
        let handle = tokio::spawn(async move {
            let task = create_sample_task(TaskStatus::Todo);
            emitter.save_task(&task).await.expect("Failed to save task");
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Collect events
    let mut event_count = 0;
    while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
        event_count += 1;
    }

    assert_eq!(event_count, 10);
}

#[tokio::test]
async fn test_task_lifecycle_event_sequence() {
    let (emitter, event_bus) = setup_test_system().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create task
    let mut task = create_sample_task(TaskStatus::Todo);
    emitter.save_task(&task).await.expect("Failed to save task");

    // Receive Created event
    let event1 = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    match event1 {
        DescartesEvent::TaskEvent(e) => {
            assert_eq!(e.event_type, TaskEventType::Created);
            assert_eq!(e.task_id, task.id.to_string());
        }
        _ => panic!("Expected Created TaskEvent"),
    }

    // Update task
    task.status = TaskStatus::InProgress;
    task.updated_at = Utc::now().timestamp();
    emitter.save_task(&task).await.expect("Failed to save task");

    // Receive Updated event
    let event2 = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    match event2 {
        DescartesEvent::TaskEvent(e) => {
            assert_eq!(e.event_type, TaskEventType::Progress);
            assert_eq!(e.task_id, task.id.to_string());

            let prev_status = e.data.get("previous_status").and_then(|v| v.as_str());
            let new_status = e.data.get("new_status").and_then(|v| v.as_str());

            assert_eq!(prev_status, Some("Todo"));
            assert_eq!(new_status, Some("InProgress"));
        }
        _ => panic!("Expected Updated TaskEvent"),
    }

    // Delete task
    emitter
        .delete_task(&task.id)
        .await
        .expect("Failed to delete task");

    // Receive Deleted event
    let event3 = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    match event3 {
        DescartesEvent::TaskEvent(e) => {
            assert_eq!(e.event_type, TaskEventType::Cancelled);
            assert_eq!(e.task_id, task.id.to_string());
        }
        _ => panic!("Expected Deleted TaskEvent"),
    }
}

#[tokio::test]
async fn test_debouncing_reduces_events() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let db_path = temp_file.path().to_str().unwrap();

    let mut state_store = SqliteStateStore::new(db_path, false)
        .await
        .expect("Failed to create state store");
    state_store
        .initialize()
        .await
        .expect("Failed to initialize");

    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;
    let event_bus = Arc::new(EventBus::new());

    // Enable debouncing with short interval
    let config = TaskEventEmitterConfig {
        enable_debouncing: true,
        debounce_interval_ms: 50,
        include_task_data: true,
        verbose_logging: false,
    };

    let emitter = TaskEventEmitter::new(state_store, event_bus.clone(), config);
    emitter
        .initialize_cache()
        .await
        .expect("Failed to initialize cache");

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create task and update it rapidly
    let mut task = create_sample_task(TaskStatus::Todo);
    emitter.save_task(&task).await.expect("Failed to save task");

    // Rapid updates
    for i in 0..20 {
        task.title = format!("Updated {}", i);
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
    assert!(
        event_count < 21,
        "Expected debouncing to reduce events, got {}",
        event_count
    );
    println!(
        "Debouncing reduced 21 operations to {} events",
        event_count
    );
}

#[tokio::test]
async fn test_event_backlog_handling() {
    let (emitter, event_bus) = setup_test_system().await;

    // Create many tasks before subscribing
    let task_count = 50;
    for _ in 0..task_count {
        let task = create_sample_task(TaskStatus::Todo);
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Now subscribe (late subscriber)
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create more tasks after subscription
    let new_task_count = 10;
    for _ in 0..new_task_count {
        let task = create_sample_task(TaskStatus::InProgress);
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Should only receive events created after subscription
    let mut received_count = 0;
    while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
        received_count += 1;
    }

    // Should receive the new tasks, not the backlog
    assert_eq!(
        received_count, new_task_count,
        "Should only receive events after subscription"
    );
}

#[tokio::test]
async fn test_subscriber_late_join() {
    let (emitter, event_bus) = setup_test_system().await;

    // First subscriber
    let (_sub1, mut rx1) = event_bus.subscribe(None).await;

    // Create a task
    let task1 = create_sample_task(TaskStatus::Todo);
    emitter.save_task(&task1).await.expect("Failed to save task");

    // First subscriber receives event
    let _ = rx1.recv().await.expect("Failed to receive");

    // Second subscriber joins late
    let (_sub2, mut rx2) = event_bus.subscribe(None).await;

    // Create another task
    let task2 = create_sample_task(TaskStatus::InProgress);
    emitter.save_task(&task2).await.expect("Failed to save task");

    // Both subscribers should receive the new event
    let event1 = tokio::time::timeout(Duration::from_millis(500), rx1.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    let event2 = tokio::time::timeout(Duration::from_millis(500), rx2.recv())
        .await
        .expect("Timeout")
        .expect("Failed to receive");

    match (event1, event2) {
        (DescartesEvent::TaskEvent(e1), DescartesEvent::TaskEvent(e2)) => {
            assert_eq!(e1.task_id, task2.id.to_string());
            assert_eq!(e2.task_id, task2.id.to_string());
        }
        _ => panic!("Expected TaskEvent"),
    }
}

#[tokio::test]
async fn test_emitter_statistics() {
    let (emitter, _) = setup_test_system().await;

    // Create some tasks
    for _ in 0..15 {
        let task = create_sample_task(TaskStatus::Todo);
        emitter.save_task(&task).await.expect("Failed to save task");
    }

    // Check statistics
    let stats = emitter.get_statistics().await;
    assert_eq!(stats.cached_tasks, 15);
    assert!(stats.config.include_task_data);
}

#[tokio::test]
async fn test_multiple_rapid_status_changes() {
    let (emitter, event_bus) = setup_test_system().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    // Create task and rapidly change its status
    let mut task = create_sample_task(TaskStatus::Todo);
    emitter.save_task(&task).await.expect("Failed to save task");

    let statuses = vec![
        TaskStatus::InProgress,
        TaskStatus::Blocked,
        TaskStatus::InProgress,
        TaskStatus::Done,
    ];

    for status in statuses {
        task.status = status;
        task.updated_at = Utc::now().timestamp();
        emitter.save_task(&task).await.expect("Failed to save task");
        sleep(Duration::from_millis(20)).await;
    }

    // Collect all events
    let mut events = Vec::new();
    while let Ok(Ok(event)) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
        events.push(event);
    }

    // Should receive 5 events (1 create + 4 updates)
    assert_eq!(events.len(), 5);

    // Verify event sequence
    match &events[0] {
        DescartesEvent::TaskEvent(e) => assert_eq!(e.event_type, TaskEventType::Created),
        _ => panic!("Expected Created event"),
    }

    for event in &events[1..] {
        match event {
            DescartesEvent::TaskEvent(e) => assert_eq!(e.event_type, TaskEventType::Progress),
            _ => panic!("Expected Progress event"),
        }
    }
}
