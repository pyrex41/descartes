// Example usage and integration patterns for SqliteStateStore

#![allow(dead_code)]

use crate::state_store::{SqliteStateStore, AgentState};
use crate::traits::{Event, ActorType, Task, TaskStatus, StateStore};
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;

/// Example: Initialize state store and save agent state
#[allow(dead_code)]
pub async fn example_initialize_state_store() -> Result<(), Box<dyn std::error::Error>> {
    // Create state store
    let mut store = SqliteStateStore::new("/tmp/agent_state.db", false).await?;

    // Initialize database schema
    store.initialize().await?;

    // Save an agent state
    let agent_state = AgentState {
        agent_id: "agent_001".to_string(),
        name: "MyAgent".to_string(),
        status: "running".to_string(),
        metadata: json!({
            "type": "worker",
            "priority": "high",
            "tags": ["production", "critical"]
        }),
        state_data: json!({
            "iteration": 10,
            "tasks_completed": 42,
            "last_action": "process_batch"
        }).to_string(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        version: 1,
    };

    store.save_agent_state(&agent_state).await?;
    println!("Agent state saved!");

    Ok(())
}

/// Example: Load and update agent state
#[allow(dead_code)]
pub async fn example_load_and_update() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStateStore::new("/tmp/agent_state.db", false).await?;

    // Load agent state
    if let Some(mut agent_state) = store.load_agent_state("agent_001").await? {
        println!("Loaded agent: {:?}", agent_state.name);

        // Update agent state
        agent_state.status = "paused".to_string();
        agent_state.updated_at = Utc::now().timestamp();
        agent_state.version += 1;

        store.save_agent_state(&agent_state).await?;
        println!("Agent state updated!");
    }

    Ok(())
}

/// Example: List all agents
#[allow(dead_code)]
pub async fn example_list_all_agents() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStateStore::new("/tmp/agent_state.db", false).await?;

    let agents = store.list_agents().await?;
    for agent in agents {
        println!("Agent: {} ({})", agent.name, agent.status);
    }

    Ok(())
}

/// Example: Record state transitions
#[allow(dead_code)]
pub async fn example_state_transitions() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStateStore::new("/tmp/agent_state.db", false).await?;

    // Record transition
    let state_before = json!({ "status": "idle" }).to_string();
    let state_after = json!({ "status": "running" }).to_string();

    store
        .record_state_transition(
            "agent_001",
            &state_before,
            &state_after,
            Some("User initiated execution".to_string()),
        )
        .await?;

    // Get history
    let history = store.get_state_history("agent_001", 10).await?;
    for transition in history {
        println!(
            "Transition: {} -> {} ({})",
            transition.state_before, transition.state_after,
            transition.reason.unwrap_or_default()
        );
    }

    Ok(())
}

/// Example: Create and restore snapshots
#[allow(dead_code)]
pub async fn example_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStateStore::new("/tmp/agent_state.db", false).await?;

    // Create a snapshot
    let snapshot_id = store
        .create_snapshot("agent_001", Some("Stable checkpoint at iteration 100".to_string()))
        .await?;

    println!("Snapshot created: {}", snapshot_id);

    // List snapshots
    let snapshots = store.list_snapshots("agent_001").await?;
    for (id, desc, created_at) in snapshots {
        println!("Snapshot: {} ({}) - {}", id, desc.unwrap_or_default(), created_at);
    }

    // Restore from snapshot
    store.restore_snapshot(&snapshot_id).await?;
    println!("State restored from snapshot");

    Ok(())
}

/// Example: Save and retrieve events
#[allow(dead_code)]
pub async fn example_event_management() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = SqliteStateStore::new("/tmp/events.db", false).await?;
    store.initialize().await?;

    // Save an event
    let event = Event {
        id: Uuid::new_v4(),
        event_type: "agent_started".to_string(),
        timestamp: Utc::now().timestamp(),
        session_id: "session_001".to_string(),
        actor_type: ActorType::Agent,
        actor_id: "agent_001".to_string(),
        content: "Agent started processing task".to_string(),
        metadata: Some(json!({
            "task_id": "task_123",
            "priority": "high"
        })),
        git_commit: Some("abc123def456".to_string()),
    };

    store.save_event(&event).await?;

    // Retrieve events
    let events = store.get_events("session_001").await?;
    println!("Retrieved {} events", events.len());

    // Search events
    let results = store.search_events("task").await?;
    println!("Found {} matching events", results.len());

    Ok(())
}

/// Example: Task management
#[allow(dead_code)]
pub async fn example_task_management() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = SqliteStateStore::new("/tmp/tasks.db", false).await?;
    store.initialize().await?;

    // Create and save a task
    let task = Task {
        id: Uuid::new_v4(),
        title: "Process batch data".to_string(),
        description: Some("Process incoming batch of transactions".to_string()),
        status: TaskStatus::InProgress,
        assigned_to: Some("agent_001".to_string()),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: Some(json!({
            "batch_size": 1000,
            "priority": "high"
        })),
    };

    store.save_task(&task).await?;

    // Retrieve task
    if let Some(fetched) = store.get_task(&task.id).await? {
        println!("Task: {} ({})", fetched.title, fetched.status as i32);
    }

    // Get all tasks
    let all_tasks = store.get_tasks().await?;
    println!("Total tasks: {}", all_tasks.len());

    Ok(())
}

/// Example: Using state store with key prefix
#[allow(dead_code)]
pub async fn example_prefixed_state_store() -> Result<(), Box<dyn std::error::Error>> {
    // Create separate stores with prefixes for different agent types
    let worker_store = SqliteStateStore::new("/tmp/state.db", false)
        .await?
        .with_prefix("worker".to_string());

    let coordinator_store = SqliteStateStore::new("/tmp/state.db", false)
        .await?
        .with_prefix("coordinator".to_string());

    println!("Using prefixed stores: worker and coordinator");

    Ok(())
}

/// Example: Concurrent access with multiple tasks
#[allow(dead_code)]
pub async fn example_concurrent_access() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;

    let store = Arc::new(SqliteStateStore::new("/tmp/concurrent.db", false).await?);

    let mut handles = vec![];

    // Spawn multiple tasks accessing the store
    for i in 0..5 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let agent_id = format!("agent_{}", i);
            let state = AgentState {
                agent_id: agent_id.clone(),
                name: format!("Agent {}", i),
                status: "running".to_string(),
                metadata: json!({ "task_id": i }),
                state_data: format!("{{ \"count\": {} }}", i),
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                version: 1,
            };

            store_clone.save_agent_state(&state).await.expect("Failed to save");
            println!("Task {} completed", i);
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await?;
    }

    println!("All concurrent tasks completed");

    Ok(())
}

/// Example: Transaction pattern (auto-commit)
#[allow(dead_code)]
pub async fn example_transaction_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let store = SqliteStateStore::new("/tmp/transaction.db", false).await?;

    // Execute multiple operations as a logical group
    let _result = store
        .transact(|| async {
            // Operations here are implicitly grouped
            // In production, you'd use actual transactions with the pool
            Ok("Transaction completed".to_string())
        })
        .await?;

    Ok(())
}

/// Example: Migration verification
#[allow(dead_code)]
pub async fn example_check_migrations() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = SqliteStateStore::new("/tmp/migrations.db", false).await?;
    store.initialize().await?;

    let migrations = store.get_migration_history().await?;
    println!("Applied migrations:");
    for migration in migrations {
        println!(
            "  v{}: {} ({})",
            migration.version,
            migration.name,
            migration.description.unwrap_or_default()
        );
    }

    Ok(())
}
