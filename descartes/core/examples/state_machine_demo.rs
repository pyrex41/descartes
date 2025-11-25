//! Example demonstrating Statig State Machine integration for Descartes workflows
//!
//! This example shows:
//! - Basic workflow state management
//! - Event-driven transitions
//! - Async handler implementation
//! - State persistence and serialization
//! - Workflow orchestration with multiple concurrent workflows
//! - Custom event handling
//! - State history and rollback

use descartes_core::state_machine::*;
use std::sync::Arc;

/// Custom state handler that logs transitions
struct LoggingHandler;

#[async_trait::async_trait]
impl StateHandler for LoggingHandler {
    async fn on_enter(&self, state: WorkflowState) -> StateMachineResult<()> {
        println!("  [HANDLER] Entering state: {}", state);
        Ok(())
    }

    async fn on_exit(&self, state: WorkflowState) -> StateMachineResult<()> {
        println!("  [HANDLER] Exiting state: {}", state);
        Ok(())
    }

    async fn on_event(
        &self,
        state: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        println!("  [HANDLER] Event received in {}: {}", state, event);
        Ok(())
    }

    async fn on_transition(
        &self,
        from: WorkflowState,
        to: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        println!(
            "  [HANDLER] Transition: {} -> {} (via: {})",
            from, to, event
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    println!("=== Descartes State Machine Integration Demo ===\n");

    // Example 1: Basic workflow
    println!("Example 1: Basic Workflow Management");
    println!("====================================\n");
    basic_workflow_example().await;

    println!("\n");

    // Example 2: Multiple concurrent workflows
    println!("Example 2: Concurrent Workflow Orchestration");
    println!("==========================================\n");
    concurrent_workflows_example().await;

    println!("\n");

    // Example 3: Custom event handling
    println!("Example 3: Custom Event Handling");
    println!("================================\n");
    custom_event_example().await;

    println!("\n");

    // Example 4: History and state tracking
    println!("Example 4: State History & Tracking");
    println!("==================================\n");
    history_example().await;

    println!("\n=== Demo Complete ===");
}

async fn basic_workflow_example() {
    let handler = Arc::new(LoggingHandler);
    let sm = Arc::new(WorkflowStateMachine::with_handler(
        "basic-workflow-1".to_string(),
        handler,
    ));

    println!("Initial state: {}\n", sm.current_state().await);

    // Start workflow
    println!("Starting workflow...");
    sm.process_event(WorkflowEvent::Start)
        .await
        .expect("Failed to start");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\nCurrent state: {}", sm.current_state().await);

    // Pause workflow
    println!("\nPausing workflow...");
    sm.process_event(WorkflowEvent::Pause)
        .await
        .expect("Failed to pause");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\nCurrent state: {}", sm.current_state().await);

    // Resume workflow
    println!("\nResuming workflow...");
    sm.process_event(WorkflowEvent::Resume)
        .await
        .expect("Failed to resume");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\nCurrent state: {}", sm.current_state().await);

    // Complete workflow
    println!("\nCompleting workflow...");
    sm.process_event(WorkflowEvent::Complete)
        .await
        .expect("Failed to complete");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\nFinal state: {}", sm.current_state().await);

    // Show metadata
    let metadata = sm.get_metadata().await;
    println!("\nWorkflow Metadata:");
    println!("  ID: {}", metadata.workflow_id);
    println!("  Current State: {}", metadata.current_state);
    println!("  Created: {}", metadata.created_at);
    println!("  History Entries: {}", metadata.history_size);
}

async fn concurrent_workflows_example() {
    let orchestrator = Arc::new(WorkflowOrchestrator::new());

    // Create multiple workflows
    let workflows: Vec<_> = (1..=3)
        .map(|i| {
            let handler = Arc::new(LoggingHandler);
            Arc::new(WorkflowStateMachine::with_handler(
                format!("workflow-{}", i),
                handler,
            ))
        })
        .collect();

    // Register all workflows
    for (i, workflow) in workflows.iter().enumerate() {
        let id = format!("workflow-{}", i + 1);
        orchestrator
            .register_workflow(id, Arc::clone(workflow))
            .await
            .expect("Failed to register");
    }

    println!(
        "Registered workflows: {:?}\n",
        orchestrator.list_workflows().await
    );

    // Run workflows concurrently
    let mut handles = vec![];

    for workflow in workflows.iter() {
        let sm = Arc::clone(workflow);
        let handle = tokio::spawn(async move {
            sm.process_event(WorkflowEvent::Start).await.ok();
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            sm.process_event(WorkflowEvent::Complete).await.ok();
        });
        handles.push(handle);
    }

    // Wait for all workflows to complete
    for handle in handles {
        let _ = handle.await;
    }

    println!();

    // Show final state of all workflows
    for metadata in orchestrator.get_all_metadata().await {
        println!(
            "Workflow {} final state: {}",
            metadata.workflow_id, metadata.current_state
        );
    }
}

async fn custom_event_example() {
    let sm = Arc::new(WorkflowStateMachine::new(
        "custom-event-workflow".to_string(),
    ));

    // Store context data
    sm.set_context("task_name", serde_json::json!("Feature Implementation"))
        .await
        .expect("Failed to set context");

    sm.set_context("priority", serde_json::json!("high"))
        .await
        .expect("Failed to set context");

    sm.set_context("agent", serde_json::json!("architect"))
        .await
        .expect("Failed to set context");

    println!("Workflow context: {:?}\n", sm.get_all_context().await);

    // Start workflow
    println!("Starting workflow...");
    sm.process_event(WorkflowEvent::Start)
        .await
        .expect("Failed to start");

    // Send custom events
    println!("Sending custom events...");

    let planning_event = WorkflowEvent::Custom {
        name: "planning_started".to_string(),
        data: serde_json::json!({ "estimated_hours": 8 }),
    };

    sm.process_event(planning_event)
        .await
        .expect("Failed to process custom event");

    let coding_event = WorkflowEvent::Custom {
        name: "coding_started".to_string(),
        data: serde_json::json!({ "files_created": 5 }),
    };

    sm.process_event(coding_event)
        .await
        .expect("Failed to process custom event");

    // Complete workflow
    println!("Completing workflow...");
    sm.process_event(WorkflowEvent::Complete)
        .await
        .expect("Failed to complete");

    println!("\nFinal state: {}", sm.current_state().await);
}

async fn history_example() {
    let sm = Arc::new(WorkflowStateMachine::new("history-workflow".to_string()));

    // Perform multiple transitions
    println!("Performing state transitions...\n");

    sm.process_event(WorkflowEvent::Start)
        .await
        .expect("Failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    sm.process_event(WorkflowEvent::Pause)
        .await
        .expect("Failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    sm.process_event(WorkflowEvent::Resume)
        .await
        .expect("Failed");
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Get history
    let history = sm.get_history().await;
    println!("Complete History ({} entries):", history.len());
    for (i, entry) in history.iter().enumerate() {
        println!(
            "  [{}] {} -> {} ({}ms) - Event: {}",
            i + 1,
            entry.transition.from_state,
            entry.transition.to_state,
            entry.transition.duration_ms,
            entry.transition.event
        );
    }

    // Get last N entries
    println!("\nLast 2 transitions:");
    let recent = sm.get_history_tail(2).await;
    for entry in recent {
        println!(
            "  {} -> {} via {}",
            entry.transition.from_state, entry.transition.to_state, entry.transition.event
        );
    }

    // Show final state
    println!("\nCurrent state: {}", sm.current_state().await);

    // Complete workflow
    sm.process_event(WorkflowEvent::Complete)
        .await
        .expect("Failed to complete");

    println!("Final state after completion: {}", sm.current_state().await);
}
