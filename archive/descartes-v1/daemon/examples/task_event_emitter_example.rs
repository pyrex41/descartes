//! Task Event Emitter Example
//!
//! Demonstrates real-time task change notifications with WebSocket streaming
//!
//! Run with: cargo run --package descartes-daemon --example task_event_emitter_example

use chrono::Utc;
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::{StateStore, Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_daemon::events::{DescartesEvent, EventBus};
use descartes_daemon::task_event_emitter::{TaskEventEmitter, TaskEventEmitterConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Task Event Emitter Example ===\n");

    // Setup: Create database and event bus
    let temp_db = tempfile::NamedTempFile::new()?;
    let db_path = temp_db.path().to_str().unwrap();

    println!("1. Initializing SQLite database at: {}", db_path);
    let mut state_store = SqliteStateStore::new(db_path, false).await?;
    state_store.initialize().await?;
    let state_store = Arc::new(state_store) as Arc<dyn StateStore>;

    println!("2. Creating EventBus");
    let event_bus = Arc::new(EventBus::new());

    println!("3. Creating TaskEventEmitter with configuration");
    let config = TaskEventEmitterConfig {
        enable_debouncing: true,
        debounce_interval_ms: 100,
        include_task_data: true,
        verbose_logging: true,
    };

    let emitter = Arc::new(TaskEventEmitter::new(
        state_store,
        event_bus.clone(),
        config,
    ));

    println!("4. Initializing task cache\n");
    emitter.initialize_cache().await?;

    // Subscribe to events in a separate task
    println!("5. Setting up event subscriber");
    let (_sub_id, mut event_receiver) = event_bus.subscribe(None).await;

    let event_handler = tokio::spawn(async move {
        println!("\n[Event Subscriber] Listening for events...\n");

        while let Ok(event) = event_receiver.recv().await {
            match event {
                DescartesEvent::TaskEvent(task_event) => {
                    let change_type = task_event
                        .data
                        .get("change_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    match change_type {
                        "created" => {
                            println!(
                                "🆕 [Event] Task CREATED: {} ({})",
                                task_event.task_id, task_event.event_type as u8
                            );

                            if let Some(task) = task_event.data.get("task") {
                                if let Ok(task) = serde_json::from_value::<Task>(task.clone()) {
                                    println!("   Title: {}", task.title);
                                    println!("   Status: {:?}", task.status);
                                    println!("   Priority: {:?}", task.priority);
                                }
                            }
                        }
                        "updated" => {
                            let prev_status = task_event
                                .data
                                .get("previous_status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let new_status = task_event
                                .data
                                .get("new_status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");

                            println!("📝 [Event] Task UPDATED: {}", task_event.task_id);
                            println!("   Status: {} → {}", prev_status, new_status);
                        }
                        "deleted" => {
                            println!("🗑️  [Event] Task DELETED: {}", task_event.task_id);
                        }
                        _ => {}
                    }
                    println!();
                }
                _ => {}
            }
        }
    });

    // Demo: Create and update tasks
    println!("\n6. Running demo scenarios\n");

    // Scenario 1: Create a task
    println!("--- Scenario 1: Create a new task ---");
    let task1 = Task {
        id: Uuid::new_v4(),
        title: "Implement authentication".to_string(),
        description: Some("Add JWT-based authentication to API".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Complex,
        assigned_to: Some("agent-1".to_string()),
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    emitter.save_task(&task1).await?;
    sleep(Duration::from_millis(300)).await;

    // Scenario 2: Update task status
    println!("--- Scenario 2: Update task status ---");
    let mut task1_updated = task1.clone();
    task1_updated.status = TaskStatus::InProgress;
    task1_updated.updated_at = Utc::now().timestamp();

    emitter.save_task(&task1_updated).await?;
    sleep(Duration::from_millis(300)).await;

    // Scenario 3: Create multiple tasks concurrently
    println!("--- Scenario 3: Create 5 tasks concurrently ---");
    let mut handles = vec![];

    for i in 1..=5 {
        let emitter = emitter.clone();
        let handle = tokio::spawn(async move {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task #{}", i),
                description: Some(format!("Concurrent task number {}", i)),
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            emitter.save_task(&task).await.unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    sleep(Duration::from_millis(500)).await;

    // Scenario 4: Rapid updates (demonstrate debouncing)
    println!("--- Scenario 4: Rapid updates (debouncing demo) ---");
    let mut rapid_task = Task {
        id: Uuid::new_v4(),
        title: "Debounce test".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: TaskPriority::Low,
        complexity: TaskComplexity::Simple,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    // First save (create)
    emitter.save_task(&rapid_task).await?;

    // Rapid updates
    println!("Performing 10 rapid updates...");
    for i in 1..=10 {
        rapid_task.title = format!("Debounce test - update {}", i);
        rapid_task.updated_at = Utc::now().timestamp();
        emitter.save_task(&rapid_task).await?;
        sleep(Duration::from_millis(20)).await;
    }

    sleep(Duration::from_millis(500)).await;

    // Scenario 5: Task lifecycle
    println!("--- Scenario 5: Full task lifecycle ---");
    let mut lifecycle_task = Task {
        id: Uuid::new_v4(),
        title: "Complete lifecycle task".to_string(),
        description: Some("Watch this task go through all states".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::Critical,
        complexity: TaskComplexity::Epic,
        assigned_to: Some("agent-2".to_string()),
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    };

    // Create
    println!("Creating task...");
    emitter.save_task(&lifecycle_task).await?;
    sleep(Duration::from_millis(300)).await;

    // Move to InProgress
    println!("Starting task...");
    lifecycle_task.status = TaskStatus::InProgress;
    lifecycle_task.updated_at = Utc::now().timestamp();
    emitter.save_task(&lifecycle_task).await?;
    sleep(Duration::from_millis(300)).await;

    // Move to Blocked
    println!("Blocking task...");
    lifecycle_task.status = TaskStatus::Blocked;
    lifecycle_task.updated_at = Utc::now().timestamp();
    emitter.save_task(&lifecycle_task).await?;
    sleep(Duration::from_millis(300)).await;

    // Unblock and continue
    println!("Unblocking task...");
    lifecycle_task.status = TaskStatus::InProgress;
    lifecycle_task.updated_at = Utc::now().timestamp();
    emitter.save_task(&lifecycle_task).await?;
    sleep(Duration::from_millis(300)).await;

    // Complete
    println!("Completing task...");
    lifecycle_task.status = TaskStatus::Done;
    lifecycle_task.updated_at = Utc::now().timestamp();
    emitter.save_task(&lifecycle_task).await?;
    sleep(Duration::from_millis(300)).await;

    // Display statistics
    println!("\n--- Statistics ---");
    let stats = emitter.get_statistics().await;
    println!("Cached tasks: {}", stats.cached_tasks);
    println!("Debounce entries: {}", stats.debounce_entries);
    println!(
        "Pending debounced events: {}",
        stats.pending_debounced_events
    );
    println!("Debouncing enabled: {}", stats.config.enable_debouncing);
    println!("Debounce interval: {}ms", stats.config.debounce_interval_ms);

    // Retrieve all tasks
    println!("\n--- All Tasks in Database ---");
    let all_tasks = emitter.get_tasks().await?;
    println!("Total tasks: {}", all_tasks.len());
    for task in &all_tasks {
        println!("  - {} [{}] {:?}", task.title, task.id, task.status);
    }

    println!("\n7. Waiting for final events to process...");
    sleep(Duration::from_secs(1)).await;

    println!("\n=== Demo Complete ===");
    println!("The event handler will continue listening until you press Ctrl+C");

    // Keep running to show events
    event_handler.await?;

    Ok(())
}
