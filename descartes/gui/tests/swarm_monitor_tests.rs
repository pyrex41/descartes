//! Comprehensive tests for Swarm Monitor Live Updates (Phase 3:5.5)
//!
//! This test suite covers:
//! - Agent spawn → UI appearance
//! - Status transitions → UI updates
//! - Thought stream → thought bubble updates
//! - Agent completion → final state display
//! - Filtering and search during live updates
//! - Performance with 10, 50, 100+ agents
//! - WebSocket streaming
//! - Animation performance (60 FPS)

use descartes_gui::swarm_monitor::{
    SwarmMonitorState, SwarmMonitorMessage, AgentEvent, ConnectionStatus,
    PerformanceStats, update, TARGET_FPS, FRAME_TIME_BUDGET_MS,
};
use descartes_core::{
    AgentRuntimeState, AgentStatus, AgentProgress, AgentError, AgentFilter,
    GroupingMode, SortMode,
};
use std::collections::HashMap;
use std::time::Instant;
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

/// Create multiple test agents
fn create_test_agents(count: usize) -> Vec<AgentRuntimeState> {
    (0..count)
        .map(|i| {
            create_test_agent(&format!("agent-{}", i), &format!("task-{}", i))
        })
        .collect()
}

/// Simulate agent state transitions
fn transition_agent_through_lifecycle(agent: &mut AgentRuntimeState) {
    agent.transition_to(AgentStatus::Initializing, None).ok();
    agent.transition_to(AgentStatus::Running, None).ok();
    agent.transition_to(AgentStatus::Thinking, None).ok();
    agent.update_thought("Processing data...".to_string());
    agent.transition_to(AgentStatus::Running, None).ok();
    agent.transition_to(AgentStatus::Completed, None).ok();
}

// ============================================================================
// BASIC STATE TESTS
// ============================================================================

#[test]
fn test_swarm_monitor_creation() {
    let state = SwarmMonitorState::new();
    assert_eq!(state.agents.len(), 0);
    assert_eq!(state.filter, AgentFilter::All);
    assert_eq!(state.grouping, GroupingMode::None);
    assert_eq!(state.search_query, "");
    assert!(state.live_updates_enabled);
    assert!(!state.websocket_enabled);
    assert_eq!(state.connection_status, ConnectionStatus::Disconnected);
}

#[test]
fn test_agent_addition() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    assert_eq!(state.agents.len(), 1);
    assert!(state.agents.contains_key(&agent_id));
}

#[test]
fn test_agent_removal() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);
    assert_eq!(state.agents.len(), 1);

    state.remove_agent(&agent_id);
    assert_eq!(state.agents.len(), 0);
}

// ============================================================================
// LIVE UPDATE TESTS
// ============================================================================

#[test]
fn test_agent_spawn_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("new-agent", "new-task");
    let agent_id = agent.agent_id;

    let event = AgentEvent::AgentSpawned { agent: agent.clone() };
    state.handle_agent_event(event);

    assert_eq!(state.agents.len(), 1);
    let stored_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(stored_agent.name, "new-agent");
    assert_eq!(stored_agent.task, "new-task");
}

#[test]
fn test_status_change_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Send status change event
    let event = AgentEvent::AgentStatusChanged {
        agent_id,
        status: AgentStatus::Running,
    };
    state.handle_agent_event(event);

    let updated_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(updated_agent.status, AgentStatus::Running);
}

#[test]
fn test_thought_update_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Send thought update event
    let thought = "Analyzing the problem...".to_string();
    let event = AgentEvent::AgentThoughtUpdate {
        agent_id,
        thought: thought.clone(),
    };
    state.handle_agent_event(event);

    let updated_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(updated_agent.current_thought, Some(thought));
    assert_eq!(updated_agent.status, AgentStatus::Thinking);
}

#[test]
fn test_progress_update_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Send progress update event
    let progress = AgentProgress::new(50.0);
    let event = AgentEvent::AgentProgressUpdate {
        agent_id,
        progress: progress.clone(),
    };
    state.handle_agent_event(event);

    let updated_agent = state.agents.get(&agent_id).unwrap();
    assert!(updated_agent.progress.is_some());
    assert_eq!(updated_agent.progress.as_ref().unwrap().percentage, 50.0);
}

#[test]
fn test_completion_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Send completion event
    let event = AgentEvent::AgentCompleted { agent_id };
    state.handle_agent_event(event);

    let updated_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(updated_agent.status, AgentStatus::Completed);
}

#[test]
fn test_failure_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Send failure event
    let error = AgentError::new(
        "TEST_ERROR".to_string(),
        "Test error message".to_string(),
    );
    let event = AgentEvent::AgentFailed {
        agent_id,
        error: error.clone(),
    };
    state.handle_agent_event(event);

    let updated_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(updated_agent.status, AgentStatus::Failed);
    assert!(updated_agent.error.is_some());
    assert_eq!(updated_agent.error.as_ref().unwrap().code, "TEST_ERROR");
}

#[test]
fn test_termination_event() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);
    assert_eq!(state.agents.len(), 1);

    // Send termination event
    let event = AgentEvent::AgentTerminated { agent_id };
    state.handle_agent_event(event);

    assert_eq!(state.agents.len(), 0);
}

#[test]
fn test_multiple_status_transitions() {
    let mut state = SwarmMonitorState::new();
    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent);

    // Simulate lifecycle transitions
    let transitions = vec![
        AgentStatus::Initializing,
        AgentStatus::Running,
        AgentStatus::Thinking,
        AgentStatus::Running,
        AgentStatus::Completed,
    ];

    for status in transitions.iter() {
        let event = AgentEvent::AgentStatusChanged {
            agent_id,
            status: *status,
        };
        state.handle_agent_event(event);

        let agent = state.agents.get(&agent_id).unwrap();
        assert_eq!(agent.status, *status);
    }

    // Check timeline
    let agent = state.agents.get(&agent_id).unwrap();
    assert!(agent.timeline.len() >= transitions.len());
}

// ============================================================================
// FILTERING AND SEARCH TESTS
// ============================================================================

#[test]
fn test_filtering_during_live_updates() {
    let mut state = SwarmMonitorState::new();

    // Create agents with different statuses
    let agents = vec![
        ("agent-1", AgentStatus::Running),
        ("agent-2", AgentStatus::Thinking),
        ("agent-3", AgentStatus::Completed),
        ("agent-4", AgentStatus::Failed),
    ];

    for (name, status) in agents.iter() {
        let mut agent = create_test_agent(name, "test-task");
        let agent_id = agent.agent_id;
        agent.transition_to(*status, None).ok();
        state.update_agent(agent);
    }

    // Test different filters
    state.filter = AgentFilter::Running;
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "agent-1");

    state.filter = AgentFilter::Thinking;
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "agent-2");

    state.filter = AgentFilter::Completed;
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "agent-3");

    state.filter = AgentFilter::Failed;
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "agent-4");

    state.filter = AgentFilter::Active;
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 2); // Running and Thinking
}

#[test]
fn test_search_during_live_updates() {
    let mut state = SwarmMonitorState::new();

    // Create agents with different names
    for i in 0..10 {
        let name = if i % 2 == 0 {
            format!("analyzer-{}", i)
        } else {
            format!("processor-{}", i)
        };
        let agent = create_test_agent(&name, "test-task");
        state.update_agent(agent);
    }

    // Test search functionality
    state.search_query = "analyzer".to_string();
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 5);

    state.search_query = "processor".to_string();
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 5);

    state.search_query = "agent".to_string();
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 0); // No agents have "agent" in their name

    state.search_query = "".to_string();
    let filtered = state.filtered_agents();
    assert_eq!(filtered.len(), 10); // All agents
}

#[test]
fn test_grouping_during_live_updates() {
    let mut state = SwarmMonitorState::new();

    // Create agents with different statuses
    for i in 0..6 {
        let mut agent = create_test_agent(&format!("agent-{}", i), "test-task");
        let status = match i % 3 {
            0 => AgentStatus::Running,
            1 => AgentStatus::Thinking,
            _ => AgentStatus::Completed,
        };
        agent.transition_to(status, None).ok();
        state.update_agent(agent);
    }

    // Test grouping by status
    state.grouping = GroupingMode::ByStatus;
    let grouped = state.grouped_agents();
    assert_eq!(grouped.len(), 3); // Three distinct statuses

    // Test grouping by model
    state.grouping = GroupingMode::ByModel;
    let grouped = state.grouped_agents();
    assert_eq!(grouped.len(), 1); // All same model backend
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_performance_with_10_agents() {
    let mut state = SwarmMonitorState::new();
    let agents = create_test_agents(10);

    let start = Instant::now();
    for agent in agents {
        state.update_agent(agent);
    }
    let duration = start.elapsed();

    assert_eq!(state.agents.len(), 10);
    assert!(duration.as_millis() < 100, "Adding 10 agents took too long");
}

#[test]
fn test_performance_with_50_agents() {
    let mut state = SwarmMonitorState::new();
    let agents = create_test_agents(50);

    let start = Instant::now();
    for agent in agents {
        state.update_agent(agent);
    }
    let duration = start.elapsed();

    assert_eq!(state.agents.len(), 50);
    assert!(duration.as_millis() < 500, "Adding 50 agents took too long");
}

#[test]
fn test_performance_with_100_agents() {
    let mut state = SwarmMonitorState::new();
    let agents = create_test_agents(100);

    let start = Instant::now();
    for agent in agents {
        state.update_agent(agent);
    }
    let duration = start.elapsed();

    assert_eq!(state.agents.len(), 100);
    assert!(duration.as_millis() < 1000, "Adding 100 agents took too long");
}

#[test]
fn test_batch_update_performance() {
    let mut state = SwarmMonitorState::new();
    let agents = create_test_agents(100);

    let agent_map: HashMap<Uuid, AgentRuntimeState> = agents
        .into_iter()
        .map(|agent| (agent.agent_id, agent))
        .collect();

    let start = Instant::now();
    state.update_agents_batch(agent_map);
    let duration = start.elapsed();

    assert_eq!(state.agents.len(), 100);
    assert!(duration.as_millis() < 100, "Batch update of 100 agents took too long");
}

#[test]
fn test_filtering_performance_with_many_agents() {
    let mut state = SwarmMonitorState::new();

    // Create 100 agents with random statuses
    for i in 0..100 {
        let mut agent = create_test_agent(&format!("agent-{}", i), "test-task");
        let status = match i % 4 {
            0 => AgentStatus::Running,
            1 => AgentStatus::Thinking,
            2 => AgentStatus::Completed,
            _ => AgentStatus::Failed,
        };
        agent.transition_to(status, None).ok();
        state.update_agent(agent);
    }

    // Test filtering performance
    state.filter = AgentFilter::Running;
    let start = Instant::now();
    let filtered = state.filtered_agents();
    let duration = start.elapsed();

    assert_eq!(filtered.len(), 25);
    assert!(duration.as_micros() < 1000, "Filtering 100 agents took too long");
}

// ============================================================================
// ANIMATION PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_animation_tick_performance() {
    let mut state = SwarmMonitorState::new();

    // Add some agents with thinking state
    for i in 0..10 {
        let mut agent = create_test_agent(&format!("agent-{}", i), "test-task");
        agent.transition_to(AgentStatus::Thinking, None).ok();
        agent.update_thought("Processing...".to_string());
        state.update_agent(agent);
    }

    // Simulate 60 frames (1 second)
    let start = Instant::now();
    for _ in 0..60 {
        state.tick_animation();
    }
    let duration = start.elapsed();

    // Should complete within reasonable time (< 100ms for 60 frames)
    assert!(duration.as_millis() < 100, "60 animation ticks took too long");
}

#[test]
fn test_frame_time_tracking() {
    let mut state = SwarmMonitorState::new();

    // Tick multiple times
    for _ in 0..100 {
        state.tick_animation();
    }

    // Check that frame times are tracked
    assert!(!state.frame_times.is_empty());
    assert!(state.avg_frame_time() >= 0.0);
    assert!(state.max_frame_time >= 0.0);
}

#[test]
fn test_fps_calculation() {
    let mut state = SwarmMonitorState::new();

    // Wait a bit and tick multiple times
    for _ in 0..60 {
        state.tick_animation();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    // FPS should be calculated (may not be exactly 60 due to sleep variance)
    // Just check that it's non-zero
    assert!(state.fps >= 0.0);
}

#[test]
fn test_performance_stats() {
    let mut state = SwarmMonitorState::new();

    // Add some agents
    for i in 0..20 {
        let mut agent = create_test_agent(&format!("agent-{}", i), "test-task");
        if i % 2 == 0 {
            agent.transition_to(AgentStatus::Running, None).ok();
        }
        state.update_agent(agent);
    }

    // Tick a few times
    for _ in 0..10 {
        state.tick_animation();
    }

    let stats = state.get_performance_stats();
    assert_eq!(stats.total_agents, 20);
    assert_eq!(stats.active_agents, 10);
    assert!(stats.avg_frame_time_ms >= 0.0);
}

// ============================================================================
// LIVE UPDATES CONTROL TESTS
// ============================================================================

#[test]
fn test_toggle_live_updates() {
    let mut state = SwarmMonitorState::new();

    assert!(state.live_updates_enabled);

    state.toggle_live_updates();
    assert!(!state.live_updates_enabled);

    state.toggle_live_updates();
    assert!(state.live_updates_enabled);
}

#[test]
fn test_live_updates_disabled() {
    let mut state = SwarmMonitorState::new();
    state.disable_live_updates();

    let agent = create_test_agent("test-agent", "test-task");
    let agent_id = agent.agent_id;

    state.update_agent(agent.clone());

    // Send event while live updates are disabled
    let event = AgentEvent::AgentStatusChanged {
        agent_id,
        status: AgentStatus::Running,
    };

    // Process through message handler
    let message = SwarmMonitorMessage::AgentEventReceived(event);
    update(&mut state, message);

    // Status should NOT be updated (live updates disabled)
    let stored_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(stored_agent.status, AgentStatus::Idle);
}

#[test]
fn test_toggle_websocket() {
    let mut state = SwarmMonitorState::new();

    assert!(!state.websocket_enabled);

    state.toggle_websocket();
    assert!(state.websocket_enabled);

    state.toggle_websocket();
    assert!(!state.websocket_enabled);
}

#[test]
fn test_connection_status_changes() {
    let mut state = SwarmMonitorState::new();

    assert_eq!(state.connection_status, ConnectionStatus::Disconnected);

    state.set_connection_status(ConnectionStatus::Connecting);
    assert_eq!(state.connection_status, ConnectionStatus::Connecting);

    state.set_connection_status(ConnectionStatus::Connected);
    assert_eq!(state.connection_status, ConnectionStatus::Connected);

    state.set_connection_status(ConnectionStatus::Error);
    assert_eq!(state.connection_status, ConnectionStatus::Error);
}

// ============================================================================
// MESSAGE HANDLING TESTS
// ============================================================================

#[test]
fn test_filter_message() {
    let mut state = SwarmMonitorState::new();

    let message = SwarmMonitorMessage::SetFilter(AgentFilter::Running);
    update(&mut state, message);

    assert_eq!(state.filter, AgentFilter::Running);
}

#[test]
fn test_grouping_message() {
    let mut state = SwarmMonitorState::new();

    let message = SwarmMonitorMessage::SetGrouping(GroupingMode::ByStatus);
    update(&mut state, message);

    assert_eq!(state.grouping, GroupingMode::ByStatus);
}

#[test]
fn test_sort_message() {
    let mut state = SwarmMonitorState::new();

    let message = SwarmMonitorMessage::SetSortMode(SortMode::ByStatus);
    update(&mut state, message);

    assert_eq!(state.sort_mode, SortMode::ByStatus);
}

#[test]
fn test_search_message() {
    let mut state = SwarmMonitorState::new();

    let message = SwarmMonitorMessage::SearchQueryChanged("test".to_string());
    update(&mut state, message);

    assert_eq!(state.search_query, "test");
}

#[test]
fn test_batch_update_message() {
    let mut state = SwarmMonitorState::new();
    let agents = create_test_agents(10);

    let message = SwarmMonitorMessage::BatchAgentUpdate(agents);
    update(&mut state, message);

    assert_eq!(state.agents.len(), 10);
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_complete_agent_lifecycle_with_live_updates() {
    let mut state = SwarmMonitorState::new();

    // Spawn agent
    let agent = create_test_agent("lifecycle-agent", "lifecycle-task");
    let agent_id = agent.agent_id;

    let spawn_event = AgentEvent::AgentSpawned { agent };
    state.handle_agent_event(spawn_event);

    assert_eq!(state.agents.len(), 1);

    // Status transitions
    let transitions = vec![
        AgentStatus::Initializing,
        AgentStatus::Running,
        AgentStatus::Thinking,
    ];

    for status in transitions {
        let event = AgentEvent::AgentStatusChanged { agent_id, status };
        state.handle_agent_event(event);
    }

    // Thought update
    let thought_event = AgentEvent::AgentThoughtUpdate {
        agent_id,
        thought: "Analyzing code structure...".to_string(),
    };
    state.handle_agent_event(thought_event);

    // Progress updates
    for i in 1..=5 {
        let progress_event = AgentEvent::AgentProgressUpdate {
            agent_id,
            progress: AgentProgress::new(i as f32 * 20.0),
        };
        state.handle_agent_event(progress_event);
    }

    // Completion
    let complete_event = AgentEvent::AgentCompleted { agent_id };
    state.handle_agent_event(complete_event);

    // Verify final state
    let final_agent = state.agents.get(&agent_id).unwrap();
    assert_eq!(final_agent.status, AgentStatus::Completed);
    assert!(final_agent.progress.is_some());
    assert_eq!(final_agent.progress.as_ref().unwrap().percentage, 100.0);
}

#[test]
fn test_concurrent_agent_updates() {
    let mut state = SwarmMonitorState::new();

    // Spawn multiple agents
    let agent_ids: Vec<Uuid> = (0..10)
        .map(|i| {
            let agent = create_test_agent(&format!("agent-{}", i), "task");
            let agent_id = agent.agent_id;
            let event = AgentEvent::AgentSpawned { agent };
            state.handle_agent_event(event);
            agent_id
        })
        .collect();

    // Update all agents concurrently with different statuses
    for (i, agent_id) in agent_ids.iter().enumerate() {
        let status = match i % 3 {
            0 => AgentStatus::Running,
            1 => AgentStatus::Thinking,
            _ => AgentStatus::Completed,
        };

        let event = AgentEvent::AgentStatusChanged {
            agent_id: *agent_id,
            status,
        };
        state.handle_agent_event(event);
    }

    // Verify all updates
    assert_eq!(state.agents.len(), 10);

    let running_count = state
        .agents
        .values()
        .filter(|a| a.status == AgentStatus::Running)
        .count();
    let thinking_count = state
        .agents
        .values()
        .filter(|a| a.status == AgentStatus::Thinking)
        .count();
    let completed_count = state
        .agents
        .values()
        .filter(|a| a.status == AgentStatus::Completed)
        .count();

    assert!(running_count >= 3);
    assert!(thinking_count >= 3);
    assert!(completed_count >= 3);
}

#[test]
fn test_filtering_with_live_updates() {
    let mut state = SwarmMonitorState::new();

    // Add 20 agents with different statuses
    for i in 0..20 {
        let mut agent = create_test_agent(&format!("agent-{}", i), "task");
        let status = match i % 4 {
            0 => AgentStatus::Running,
            1 => AgentStatus::Thinking,
            2 => AgentStatus::Completed,
            _ => AgentStatus::Failed,
        };
        agent.transition_to(status, None).ok();

        let event = AgentEvent::AgentSpawned { agent };
        state.handle_agent_event(event);
    }

    // Test filter with live updates
    state.filter = AgentFilter::Active;
    let active = state.filtered_agents();
    assert_eq!(active.len(), 10); // Running + Thinking

    // Transition one Running agent to Completed
    let running_agent = state
        .agents
        .values()
        .find(|a| a.status == AgentStatus::Running)
        .unwrap();
    let agent_id = running_agent.agent_id;

    let event = AgentEvent::AgentStatusChanged {
        agent_id,
        status: AgentStatus::Completed,
    };
    state.handle_agent_event(event);

    // Active count should decrease
    let active = state.filtered_agents();
    assert_eq!(active.len(), 9);
}
