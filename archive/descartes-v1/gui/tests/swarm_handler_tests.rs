//! Unit Tests for GUI Swarm Handler (GuiStreamHandler)
//!
//! This test suite covers:
//! - Stream handler callbacks (on_status_update, on_thought_update, etc.)
//! - Agent auto-creation from stream events
//! - Concurrent agent updates (thread safety)
//! - Error handling and status transitions
//! - CRUD operations (add, remove, get agents)
//! - Heartbeat timestamp updates
//! - Progress and output updates

use chrono::Utc;
use descartes_core::{
    agent_state::AgentError, AgentProgress, AgentRuntimeState, LifecycleEvent, OutputStream,
    RuntimeAgentStatus, StreamHandler,
};
use descartes_gui::swarm_handler::GuiStreamHandler;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a test agent with basic configuration
fn create_test_agent(name: &str, task: &str) -> AgentRuntimeState {
    AgentRuntimeState::new(
        Uuid::new_v4(),
        name.to_string(),
        task.to_string(),
        "test-backend".to_string(),
    )
}

// ============================================================================
// BASIC CRUD OPERATIONS
// ============================================================================

#[test]
fn test_agent_crud_operations() {
    let handler = GuiStreamHandler::new();

    // Test initial state - no agents
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 0);

    // Test add_agent
    let agent1 = create_test_agent("agent-1", "task-1");
    let agent1_id = agent1.agent_id;
    handler.add_agent(agent1.clone());

    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);
    assert!(agents.contains_key(&agent1_id));
    assert_eq!(agents.get(&agent1_id).unwrap().name, "agent-1");

    // Test adding another agent
    let agent2 = create_test_agent("agent-2", "task-2");
    let agent2_id = agent2.agent_id;
    handler.add_agent(agent2);

    let agents = handler.get_agents();
    assert_eq!(agents.len(), 2);
    assert!(agents.contains_key(&agent2_id));

    // Test remove_agent
    handler.remove_agent(&agent1_id);
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);
    assert!(!agents.contains_key(&agent1_id));
    assert!(agents.contains_key(&agent2_id));

    // Test removing last agent
    handler.remove_agent(&agent2_id);
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 0);
}

#[test]
fn test_multiple_agents_tracking() {
    let handler = GuiStreamHandler::new();

    // Add multiple agents
    let mut agent_ids = Vec::new();
    for i in 0..10 {
        let agent = create_test_agent(&format!("agent-{}", i), &format!("task-{}", i));
        agent_ids.push(agent.agent_id);
        handler.add_agent(agent);
    }

    // Verify all agents were added
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 10);

    for (i, agent_id) in agent_ids.iter().enumerate() {
        assert!(agents.contains_key(agent_id));
        let agent = agents.get(agent_id).unwrap();
        assert_eq!(agent.name, format!("agent-{}", i));
        assert_eq!(agent.task, format!("task-{}", i));
    }
}

#[test]
fn test_get_agents_ref() {
    let handler = GuiStreamHandler::new();

    // Add an agent
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;
    handler.add_agent(agent);

    // Get shared reference
    let agents_ref = handler.get_agents_ref();
    let agents = agents_ref.lock().unwrap();
    assert_eq!(agents.len(), 1);
    assert!(agents.contains_key(&agent_id));
}

// ============================================================================
// STREAM HANDLER CALLBACK TESTS
// ============================================================================

#[test]
fn test_on_status_update() {
    let mut handler = GuiStreamHandler::new();

    // Create an agent
    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Call on_status_update - must follow valid transitions: Idle -> Initializing -> Running
    handler.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);

    // Verify agent was created and status was set
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);
    assert!(agents.contains_key(&agent_id));

    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Initializing);
    assert_eq!(agent.agent_id, agent_id);

    // Transition to Running (valid from Initializing)
    handler.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Running);

    // Update status again to Completed (valid from Running)
    handler.on_status_update(agent_id, RuntimeAgentStatus::Completed, timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Completed);
}

#[test]
fn test_thought_update_transitions_status() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();
    let thought = "Analyzing the problem...".to_string();

    // Must first transition to Initializing (valid from Idle), then thought_update can transition to Thinking
    handler.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);

    // Call on_thought_update (should transition from Initializing to Thinking)
    handler.on_thought_update(agent_id, thought.clone(), timestamp);

    // Verify agent has Thinking status
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);

    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Thinking);
    assert_eq!(agent.current_thought, Some(thought.clone()));

    // Update thought again
    let new_thought = "Evaluating solutions...".to_string();
    handler.on_thought_update(agent_id, new_thought.clone(), timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Thinking);
    assert_eq!(agent.current_thought, Some(new_thought));
}

#[test]
fn test_status_update_clears_thought() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // First transition to Initializing, then set agent to Thinking with a thought
    handler.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);
    handler.on_thought_update(agent_id, "Thinking...".to_string(), timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Thinking);
    assert!(agent.current_thought.is_some());

    // Transition to Running (should clear thought) - valid from Thinking
    handler.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Running);
    assert!(agent.current_thought.is_none());
}

#[test]
fn test_progress_update() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Send progress update
    let progress = AgentProgress::new(50.0);
    handler.on_progress_update(agent_id, progress.clone(), timestamp);

    // Verify progress was updated
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);

    let agent = agents.get(&agent_id).unwrap();
    assert!(agent.progress.is_some());
    assert_eq!(agent.progress.as_ref().unwrap().percentage, 50.0);

    // Update progress again
    let new_progress = AgentProgress::with_steps(8, 10);
    handler.on_progress_update(agent_id, new_progress.clone(), timestamp);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert!(agent.progress.is_some());
    assert_eq!(agent.progress.as_ref().unwrap().current_step, Some(8));
    assert_eq!(agent.progress.as_ref().unwrap().total_steps, Some(10));
}

#[test]
fn test_output_update() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Send output (note: output is logged but not stored in agent state)
    // This test just verifies the handler doesn't crash
    handler.on_output(
        agent_id,
        OutputStream::Stdout,
        "Hello".to_string(),
        timestamp,
    );
    handler.on_output(
        agent_id,
        OutputStream::Stderr,
        "Error".to_string(),
        timestamp,
    );

    // Output alone does NOT auto-create agents (by design - output is just logged)
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 0);

    // But if we create an agent first, output calls work fine
    handler.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);
    handler.on_output(
        agent_id,
        OutputStream::Stdout,
        "Hello".to_string(),
        timestamp,
    );

    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);
}

#[test]
fn test_on_error_sets_failed_status() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Start agent in a valid state (Initializing) before it can fail
    handler.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);

    // Create error
    let error = AgentError::new(
        "CONNECTION_ERROR".to_string(),
        "Failed to connect to database".to_string(),
    );

    // Send error event (Failed is valid from Initializing)
    handler.on_error(agent_id, error.clone(), timestamp);

    // Verify agent status is Failed and error is set
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);

    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Failed);
    assert!(agent.error.is_some());
    assert_eq!(agent.error.as_ref().unwrap().code, "CONNECTION_ERROR");
    assert_eq!(
        agent.error.as_ref().unwrap().message,
        "Failed to connect to database"
    );
}

#[test]
fn test_on_lifecycle() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Test various lifecycle events
    let lifecycle_transitions = vec![
        (LifecycleEvent::Spawned, RuntimeAgentStatus::Idle),
        (LifecycleEvent::Started, RuntimeAgentStatus::Initializing),
        (LifecycleEvent::Resumed, RuntimeAgentStatus::Running),
        (LifecycleEvent::Paused, RuntimeAgentStatus::Paused),
        (LifecycleEvent::Resumed, RuntimeAgentStatus::Running),
        (LifecycleEvent::Completed, RuntimeAgentStatus::Completed),
    ];

    for (event, expected_status) in lifecycle_transitions {
        handler.on_lifecycle(agent_id, event.clone(), timestamp);

        let agents = handler.get_agents();
        let agent = agents.get(&agent_id).unwrap();
        assert_eq!(
            agent.status, expected_status,
            "Failed for lifecycle event: {:?}",
            event
        );
    }
}

#[test]
fn test_auto_creates_unknown_agent() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Call on_heartbeat for unknown agent_id
    handler.on_heartbeat(agent_id, timestamp);

    // Verify agent was auto-created
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 1);
    assert!(agents.contains_key(&agent_id));

    let agent = agents.get(&agent_id).unwrap();
    // Check that default name starts with "agent-"
    assert!(agent.name.starts_with("agent-"));
    assert_eq!(agent.task, "Auto-created from stream");
    assert_eq!(agent.model_backend, "unknown");
}

#[test]
fn test_heartbeat_updates_timestamp() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();

    // First heartbeat
    let timestamp1 = Utc::now();
    handler.on_heartbeat(agent_id, timestamp1);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    let first_timestamp = agent.updated_at;

    // Wait a bit
    std::thread::sleep(Duration::from_millis(10));

    // Second heartbeat with newer timestamp
    let timestamp2 = Utc::now();
    handler.on_heartbeat(agent_id, timestamp2);

    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    let second_timestamp = agent.updated_at;

    // Verify timestamp was updated
    assert!(second_timestamp > first_timestamp);
}

// ============================================================================
// CONCURRENT ACCESS TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_agent_updates() {
    let handler = Arc::new(std::sync::Mutex::new(GuiStreamHandler::new()));

    // Spawn multiple tokio tasks updating different agents
    let mut handles = Vec::new();

    for i in 0..10 {
        let handler_clone = Arc::clone(&handler);
        let handle = tokio::spawn(async move {
            let agent_id = Uuid::new_v4();
            let timestamp = Utc::now();

            // Lock handler and perform updates
            let mut h = handler_clone.lock().unwrap();

            // Multiple operations for this agent - follow valid state transitions
            h.on_status_update(agent_id, RuntimeAgentStatus::Initializing, timestamp);
            h.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);
            h.on_progress_update(agent_id, AgentProgress::new(i as f32 * 10.0), timestamp);
            h.on_thought_update(agent_id, format!("Agent {} thinking...", i), timestamp);

            agent_id
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let agent_ids: Vec<Uuid> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify no deadlocks and all updates recorded
    let handler = handler.lock().unwrap();
    let agents = handler.get_agents();

    assert_eq!(agents.len(), 10);

    for agent_id in agent_ids {
        assert!(agents.contains_key(&agent_id));
        let agent = agents.get(&agent_id).unwrap();
        assert_eq!(agent.status, RuntimeAgentStatus::Thinking); // Thought update transitions to Thinking
        assert!(agent.current_thought.is_some());
    }
}

#[tokio::test]
async fn test_concurrent_updates_same_agent() {
    let handler = Arc::new(std::sync::Mutex::new(GuiStreamHandler::new()));
    let agent_id = Uuid::new_v4();

    // Spawn multiple tasks updating the same agent
    let mut handles = Vec::new();

    for i in 0..20 {
        let handler_clone = Arc::clone(&handler);
        let handle = tokio::spawn(async move {
            let timestamp = Utc::now();
            let mut h = handler_clone.lock().unwrap();

            // Each task updates progress
            h.on_progress_update(agent_id, AgentProgress::new(i as f32 * 5.0), timestamp);
        });

        handles.push(handle);
    }

    // Wait for all tasks
    futures::future::join_all(handles).await;

    // Verify agent exists and was updated (no deadlock)
    let handler = handler.lock().unwrap();
    let agents = handler.get_agents();

    assert_eq!(agents.len(), 1);
    assert!(agents.contains_key(&agent_id));

    let agent = agents.get(&agent_id).unwrap();
    assert!(agent.progress.is_some());
    // Progress will be from one of the updates
    assert!(agent.progress.as_ref().unwrap().percentage <= 95.0);
}

// ============================================================================
// CLONE AND DEFAULT TESTS
// ============================================================================

#[test]
fn test_handler_clone() {
    let handler1 = GuiStreamHandler::new();

    // Add an agent
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;
    handler1.add_agent(agent);

    // Clone handler
    let handler2 = handler1.clone();

    // Both handlers should see the same agents (shared Arc)
    let agents1 = handler1.get_agents();
    let agents2 = handler2.get_agents();

    assert_eq!(agents1.len(), 1);
    assert_eq!(agents2.len(), 1);
    assert!(agents1.contains_key(&agent_id));
    assert!(agents2.contains_key(&agent_id));
}

#[test]
fn test_handler_default() {
    let handler = GuiStreamHandler::default();

    let agents = handler.get_agents();
    assert_eq!(agents.len(), 0);
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_complete_agent_lifecycle() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Lifecycle: Spawned -> Initializing -> Running -> Thinking -> Running -> Completed
    handler.on_lifecycle(agent_id, LifecycleEvent::Spawned, timestamp);

    let agents = handler.get_agents();
    assert_eq!(
        agents.get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Idle
    );

    handler.on_lifecycle(agent_id, LifecycleEvent::Started, timestamp);
    assert_eq!(
        handler.get_agents().get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Initializing
    );

    handler.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);
    assert_eq!(
        handler.get_agents().get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Running
    );

    handler.on_thought_update(agent_id, "Processing data...".to_string(), timestamp);
    assert_eq!(
        handler.get_agents().get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Thinking
    );

    handler.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);
    assert_eq!(
        handler.get_agents().get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Running
    );

    handler.on_progress_update(agent_id, AgentProgress::new(100.0), timestamp);
    handler.on_lifecycle(agent_id, LifecycleEvent::Completed, timestamp);
    assert_eq!(
        handler.get_agents().get(&agent_id).unwrap().status,
        RuntimeAgentStatus::Completed
    );
}

#[test]
fn test_error_during_execution() {
    let mut handler = GuiStreamHandler::new();

    let agent_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // Start agent - Lifecycle Started goes to Initializing, then we can go to Running
    handler.on_lifecycle(agent_id, LifecycleEvent::Started, timestamp);
    handler.on_status_update(agent_id, RuntimeAgentStatus::Running, timestamp);
    handler.on_progress_update(agent_id, AgentProgress::new(50.0), timestamp);

    // Error occurs
    let error = AgentError::new(
        "RUNTIME_ERROR".to_string(),
        "Unexpected error occurred".to_string(),
    );
    handler.on_error(agent_id, error, timestamp);

    // Verify agent is in Failed state
    let agents = handler.get_agents();
    let agent = agents.get(&agent_id).unwrap();
    assert_eq!(agent.status, RuntimeAgentStatus::Failed);
    assert!(agent.error.is_some());
    assert_eq!(agent.progress.as_ref().unwrap().percentage, 50.0); // Progress preserved
}

#[test]
fn test_multiple_agents_different_states() {
    let mut handler = GuiStreamHandler::new();
    let timestamp = Utc::now();

    // Create agents in different states - must follow valid state transitions
    // Agent 1: Idle -> Initializing -> Running
    let agent1_id = Uuid::new_v4();
    handler.on_status_update(agent1_id, RuntimeAgentStatus::Initializing, timestamp);
    handler.on_status_update(agent1_id, RuntimeAgentStatus::Running, timestamp);
    handler.on_progress_update(agent1_id, AgentProgress::new(25.0), timestamp);

    // Agent 2: Idle -> Initializing -> Thinking (thought_update auto-transitions to Thinking)
    let agent2_id = Uuid::new_v4();
    handler.on_status_update(agent2_id, RuntimeAgentStatus::Initializing, timestamp);
    handler.on_thought_update(agent2_id, "Analyzing...".to_string(), timestamp);

    // Agent 3: Idle -> Initializing -> Running -> Completed
    let agent3_id = Uuid::new_v4();
    handler.on_lifecycle(agent3_id, LifecycleEvent::Started, timestamp); // -> Initializing
    handler.on_lifecycle(agent3_id, LifecycleEvent::Resumed, timestamp); // -> Running
    handler.on_lifecycle(agent3_id, LifecycleEvent::Completed, timestamp); // -> Completed

    // Agent 4: Idle -> Initializing -> Failed
    let agent4_id = Uuid::new_v4();
    handler.on_status_update(agent4_id, RuntimeAgentStatus::Initializing, timestamp);
    let error = AgentError::new("ERR".to_string(), "Error".to_string());
    handler.on_error(agent4_id, error, timestamp);

    // Verify all agents
    let agents = handler.get_agents();
    assert_eq!(agents.len(), 4);

    assert_eq!(
        agents.get(&agent1_id).unwrap().status,
        RuntimeAgentStatus::Running
    );
    assert_eq!(
        agents.get(&agent2_id).unwrap().status,
        RuntimeAgentStatus::Thinking
    );
    assert_eq!(
        agents.get(&agent3_id).unwrap().status,
        RuntimeAgentStatus::Completed
    );
    assert_eq!(
        agents.get(&agent4_id).unwrap().status,
        RuntimeAgentStatus::Failed
    );
}
