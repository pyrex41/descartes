//! Example demonstrating SQLite persistence for Descartes state machines
//!
//! This example shows:
//! - Creating persistent workflow storage
//! - Saving workflow states to SQLite
//! - Loading and recovering workflows
//! - History management and cleanup

use descartes_core::state_machine::*;
use descartes_core::state_machine_store::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Descartes State Machine Persistence Demo ===\n");

    // Create temporary database for demo
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file.path().to_str().unwrap();
    let database_url = format!("sqlite://{}", db_path);

    println!("Using database: {}\n", db_path);

    // Initialize store
    println!("Example 1: Store Initialization");
    println!("==============================\n");
    let store = SqliteWorkflowStore::new(&database_url, StateStoreConfig::default()).await?;
    println!("SQLite store initialized successfully\n");

    // Example 2: Save workflow
    println!("Example 2: Save Workflow State");
    println!("=============================\n");
    save_workflow_example(&store).await?;

    // Example 3: Load workflow
    println!("\nExample 3: Load Workflow State");
    println!("=============================\n");
    load_workflow_example(&store).await?;

    // Example 4: Workflow history
    println!("\nExample 4: Workflow History");
    println!("==========================\n");
    history_example(&store).await?;

    // Example 5: Recovery
    println!("\nExample 5: Workflow Recovery");
    println!("===========================\n");
    recovery_example(&store).await?;

    // Example 6: Multiple workflows
    println!("\nExample 6: Multiple Workflows");
    println!("============================\n");
    multiple_workflows_example(&store).await?;

    println!("\n=== Demo Complete ===");
    Ok(())
}

async fn save_workflow_example(
    store: &SqliteWorkflowStore,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a workflow
    let sm = Arc::new(WorkflowStateMachine::new("demo-workflow-1".to_string()));

    // Perform some transitions
    sm.process_event(WorkflowEvent::Start).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Store context
    sm.set_context("task_id", serde_json::json!("TASK-001"))
        .await?;
    sm.set_context("priority", serde_json::json!("high"))
        .await?;
    sm.set_context("assigned_to", serde_json::json!("agent-1"))
        .await?;

    sm.process_event(WorkflowEvent::Pause).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Save to database
    store.save_workflow(&sm).await?;
    println!("Saved workflow 'demo-workflow-1' to database");

    // Perform more transitions
    sm.process_event(WorkflowEvent::Resume).await?;
    sm.process_event(WorkflowEvent::Complete).await?;

    // Save again
    store.save_workflow(&sm).await?;
    println!("Saved updated workflow state");

    Ok(())
}

async fn load_workflow_example(
    store: &SqliteWorkflowStore,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the workflow
    let serialized = store.load_workflow("demo-workflow-1").await?;

    println!("Loaded workflow: {}", serialized.workflow_id);
    println!("  Current state: {}", serialized.current_state);
    println!("  History entries: {}", serialized.history.len());
    println!("  Context:");
    for (key, value) in serialized.context.as_object().unwrap().iter() {
        println!("    {}: {}", key, value);
    }

    println!("\n  History:");
    for (i, entry) in serialized.history.iter().enumerate() {
        println!(
            "    [{}] {} -> {} (via: {})",
            i + 1,
            entry.transition.from_state,
            entry.transition.to_state,
            entry.transition.event
        );
    }

    // Restore workflow instance
    let restored_sm = WorkflowStateMachine::deserialize(serialized).await?;
    println!(
        "\nRestored workflow current state: {}",
        restored_sm.current_state().await
    );

    Ok(())
}

async fn history_example(store: &SqliteWorkflowStore) -> Result<(), Box<dyn std::error::Error>> {
    // Get history for the workflow
    let history = store.get_workflow_history("demo-workflow-1").await?;

    println!("Complete workflow history ({} entries):", history.len());
    println!();

    for (i, entry) in history.iter().enumerate() {
        println!("Transition #{}", i + 1);
        println!("  ID: {}", entry.transition.transition_id);
        println!("  From state: {}", entry.transition.from_state);
        println!("  To state: {}", entry.transition.to_state);
        println!("  Event: {}", entry.transition.event);
        println!("  Duration: {}ms", entry.transition.duration_ms);
        println!("  Timestamp: {}", entry.transition.timestamp);

        if entry.context_snapshot.get("task_id").is_some() {
            println!(
                "  Context (task_id): {}",
                entry.context_snapshot.get("task_id").unwrap()
            );
        }
        println!();
    }

    Ok(())
}

async fn recovery_example(store: &SqliteWorkflowStore) -> Result<(), Box<dyn std::error::Error>> {
    // Recover a workflow from storage
    let recovered = WorkflowRecovery::recover_workflow(store, "demo-workflow-1").await?;

    println!(
        "Recovered workflow 'demo-workflow-1': {}",
        recovered.current_state().await
    );

    // Verify metadata
    let metadata = recovered.get_metadata().await;
    println!("  Created: {}", metadata.created_at);
    println!("  Last transition: {}", metadata.last_transition_at);
    println!("  History size: {}", metadata.history_size);

    Ok(())
}

async fn multiple_workflows_example(
    store: &SqliteWorkflowStore,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating and saving multiple workflows...\n");

    // Create three workflows
    for i in 2..=4 {
        let workflow_id = format!("demo-workflow-{}", i);
        let sm = Arc::new(WorkflowStateMachine::new(workflow_id.clone()));

        // Do some work
        sm.process_event(WorkflowEvent::Start).await?;
        sm.set_context("sequence", serde_json::json!(i)).await?;

        // Random transitions
        match i % 3 {
            0 => {
                sm.process_event(WorkflowEvent::Pause).await?;
            }
            1 => {
                sm.process_event(WorkflowEvent::Complete).await?;
            }
            _ => {
                sm.process_event(WorkflowEvent::Fail("Demo error".to_string()))
                    .await?;
            }
        }

        // Save
        store.save_workflow(&sm).await?;
        println!("Saved {}", workflow_id);

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // List all workflows
    println!("\nAll saved workflows:");
    let workflows = store.list_workflows().await?;
    for workflow in workflows {
        println!(
            "  - {} (state: {}, updated: {})",
            workflow.workflow_id, workflow.current_state, workflow.updated_at
        );
    }

    // Recover all workflows
    println!("\nRecovering all workflows...");
    let recovered = WorkflowRecovery::recover_all_workflows(store).await?;
    println!("Recovered {} workflows", recovered.len());

    for sm in recovered {
        let metadata = sm.get_metadata().await;
        println!(
            "  - {}: {} (history: {} entries)",
            metadata.workflow_id, metadata.current_state, metadata.history_size
        );
    }

    Ok(())
}
