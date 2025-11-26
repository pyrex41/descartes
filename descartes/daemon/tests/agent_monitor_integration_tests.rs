//! Integration Tests for Phase 3:5.3 - Agent Monitoring RPC Integration
//!
//! This test suite validates the complete integration of:
//! - Agent monitoring system
//! - JSON stream parser
//! - RPC methods
//! - Event bus integration

use descartes_core::agent_state::{
    AgentError, AgentProgress, AgentRuntimeState, AgentStatus, AgentStreamMessage, LifecycleEvent,
    OutputStream,
};
use descartes_daemon::{
    AgentMonitor, AgentMonitorConfig, AgentMonitoringRpcImpl, AgentMonitoringRpcServer,
    AgentStatusFilter, EventBus,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_monitor() -> Arc<AgentMonitor> {
    let event_bus = Arc::new(EventBus::new());
    Arc::new(AgentMonitor::new(event_bus))
}

async fn setup_rpc_impl() -> AgentMonitoringRpcImpl {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;
    AgentMonitoringRpcImpl::new(monitor)
}

// ============================================================================
// BASIC MONITORING TESTS
// ============================================================================

#[tokio::test]
async fn test_basic_agent_registration() {
    let monitor = create_test_monitor();

    let agent = AgentRuntimeState::new(
        Uuid::new_v4(),
        "test-agent".to_string(),
        "test task".to_string(),
        "claude".to_string(),
    );

    let agent_id = agent.agent_id;
    monitor.register_agent(agent).await;

    // Verify agent is tracked
    let retrieved = monitor.get_agent_status(&agent_id).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test-agent");
}

#[tokio::test]
async fn test_agent_auto_discovery() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;

    let agent_id = Uuid::new_v4();

    // Send a status update without pre-registering the agent
    let message = AgentStreamMessage::StatusUpdate {
        agent_id,
        status: AgentStatus::Running,
        timestamp: chrono::Utc::now(),
    };

    let result = monitor.process_stream_message(message).await;
    assert!(result.is_ok());

    // Agent should be auto-discovered
    let agent = monitor.get_agent_status(&agent_id).await;
    assert!(agent.is_some());
    assert_eq!(agent.unwrap().status, AgentStatus::Running);
}

#[tokio::test]
async fn test_multiple_message_processing() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;

    let agent_id = Uuid::new_v4();

    // Process multiple messages for the same agent
    let messages = vec![
        AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Initializing,
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ThoughtUpdate {
            agent_id,
            thought: "Analyzing the task...".to_string(),
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::new(25.0),
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Running,
            timestamp: chrono::Utc::now(),
        },
    ];

    for message in messages {
        monitor.process_stream_message(message).await.unwrap();
    }

    // Verify final state
    let agent = monitor.get_agent_status(&agent_id).await.unwrap();
    assert_eq!(agent.status, AgentStatus::Thinking); // Thought update sets to Thinking
    assert!(agent.current_thought.is_some());
    assert!(agent.progress.is_some());
    assert_eq!(agent.progress.unwrap().percentage, 25.0);
}

// ============================================================================
// RPC METHOD TESTS
// ============================================================================

#[tokio::test]
async fn test_rpc_list_agents() {
    let rpc = setup_rpc_impl().await;

    // Register several agents
    for i in 0..5 {
        let agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            format!("agent-{}", i),
            format!("task-{}", i),
            "claude".to_string(),
        );
        rpc.register_agent(agent).await.unwrap();
    }

    // List all agents
    let agents = rpc.list_agents(None).await.unwrap();
    assert_eq!(agents.len(), 5);
}

#[tokio::test]
async fn test_rpc_filter_by_status() {
    let rpc = setup_rpc_impl().await;

    // Register agents with different statuses
    let mut agent1 = AgentRuntimeState::new(
        Uuid::new_v4(),
        "running-agent".to_string(),
        "task1".to_string(),
        "claude".to_string(),
    );
    agent1
        .transition_to(AgentStatus::Initializing, None)
        .unwrap();
    agent1.transition_to(AgentStatus::Running, None).unwrap();

    let mut agent2 = AgentRuntimeState::new(
        Uuid::new_v4(),
        "completed-agent".to_string(),
        "task2".to_string(),
        "claude".to_string(),
    );
    agent2
        .transition_to(AgentStatus::Initializing, None)
        .unwrap();
    agent2.transition_to(AgentStatus::Running, None).unwrap();
    agent2.transition_to(AgentStatus::Completed, None).unwrap();

    rpc.register_agent(agent1).await.unwrap();
    rpc.register_agent(agent2).await.unwrap();

    // Filter by Running status
    let filter = AgentStatusFilter {
        status: Some(AgentStatus::Running),
        model_backend: None,
        active_only: None,
    };
    let running_agents = rpc.list_agents(Some(filter)).await.unwrap();
    assert_eq!(running_agents.len(), 1);
    assert_eq!(running_agents[0].name, "running-agent");

    // Filter by Completed status
    let filter = AgentStatusFilter {
        status: Some(AgentStatus::Completed),
        model_backend: None,
        active_only: None,
    };
    let completed_agents = rpc.list_agents(Some(filter)).await.unwrap();
    assert_eq!(completed_agents.len(), 1);
    assert_eq!(completed_agents[0].name, "completed-agent");
}

#[tokio::test]
async fn test_rpc_get_statistics() {
    let rpc = setup_rpc_impl().await;

    // Register agents with various states
    let mut agent1 = AgentRuntimeState::new(
        Uuid::new_v4(),
        "active-1".to_string(),
        "task1".to_string(),
        "claude".to_string(),
    );
    agent1
        .transition_to(AgentStatus::Initializing, None)
        .unwrap();
    agent1.transition_to(AgentStatus::Running, None).unwrap();

    let mut agent2 = AgentRuntimeState::new(
        Uuid::new_v4(),
        "active-2".to_string(),
        "task2".to_string(),
        "claude".to_string(),
    );
    agent2
        .transition_to(AgentStatus::Initializing, None)
        .unwrap();
    agent2.transition_to(AgentStatus::Thinking, None).unwrap();

    let mut agent3 = AgentRuntimeState::new(
        Uuid::new_v4(),
        "failed".to_string(),
        "task3".to_string(),
        "claude".to_string(),
    );
    agent3
        .transition_to(AgentStatus::Initializing, None)
        .unwrap();
    agent3.transition_to(AgentStatus::Failed, None).unwrap();

    rpc.register_agent(agent1).await.unwrap();
    rpc.register_agent(agent2).await.unwrap();
    rpc.register_agent(agent3).await.unwrap();

    // Get statistics
    let stats = rpc.get_agent_statistics().await.unwrap();
    assert_eq!(stats.total, 3);
    assert!(stats.statistics.is_some());

    let stats_data = stats.statistics.unwrap();
    assert_eq!(stats_data.total_active, 2);
    assert_eq!(stats_data.total_failed, 1);
}

#[tokio::test]
async fn test_rpc_push_agent_updates() {
    let rpc = setup_rpc_impl().await;

    let agent_id = Uuid::new_v4();

    // Push various updates
    let updates = vec![
        AgentStreamMessage::Lifecycle {
            agent_id,
            event: LifecycleEvent::Spawned,
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Initializing,
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ThoughtUpdate {
            agent_id,
            thought: "Initializing context...".to_string(),
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::with_steps(1, 5),
            timestamp: chrono::Utc::now(),
        },
    ];

    for update in updates {
        let result = rpc.push_agent_update(update).await;
        assert!(result.is_ok());
    }

    // Verify agent state
    let agent = rpc.get_agent_status(agent_id.to_string()).await.unwrap();
    assert!(agent.current_thought.is_some());
    assert!(agent.progress.is_some());
}

#[tokio::test]
async fn test_rpc_monitoring_health() {
    let rpc = setup_rpc_impl().await;

    // Initial health
    let health = rpc.get_monitoring_health().await.unwrap();
    assert_eq!(health.total_agents, 0);

    // Register some agents
    for i in 0..3 {
        let mut agent = AgentRuntimeState::new(
            Uuid::new_v4(),
            format!("agent-{}", i),
            format!("task-{}", i),
            "claude".to_string(),
        );
        agent
            .transition_to(AgentStatus::Initializing, None)
            .unwrap();
        agent.transition_to(AgentStatus::Running, None).unwrap();
        rpc.register_agent(agent).await.unwrap();
    }

    // Check health again
    let health = rpc.get_monitoring_health().await.unwrap();
    assert_eq!(health.total_agents, 3);
    assert_eq!(health.active_agents, 3);
    assert_eq!(health.total_discovered, 3);
}

// ============================================================================
// EVENT BUS INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_event_bus_integration() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(Arc::clone(&event_bus)));
    monitor.register_event_handler().await;

    // Subscribe to events
    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    let agent_id = Uuid::new_v4();

    // Send a status update
    let message = AgentStreamMessage::StatusUpdate {
        agent_id,
        status: AgentStatus::Running,
        timestamp: chrono::Utc::now(),
    };

    monitor.process_stream_message(message).await.unwrap();

    // Wait for event to be published
    tokio::select! {
        event = rx.recv() => {
            assert!(event.is_ok());
            // Event should be published
        }
        _ = sleep(Duration::from_secs(1)) => {
            panic!("Timeout waiting for event");
        }
    }
}

#[tokio::test]
async fn test_lifecycle_event_publishing() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(Arc::clone(&event_bus)));
    monitor.register_event_handler().await;

    let (_sub_id, mut rx) = event_bus.subscribe(None).await;

    let agent_id = Uuid::new_v4();

    // Send lifecycle events
    let events = vec![
        LifecycleEvent::Spawned,
        LifecycleEvent::Started,
        LifecycleEvent::Completed,
    ];

    for event in events {
        let message = AgentStreamMessage::Lifecycle {
            agent_id,
            event,
            timestamp: chrono::Utc::now(),
        };
        monitor.process_stream_message(message).await.unwrap();
    }

    // Receive at least one event
    let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
    assert!(result.is_ok());
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_invalid_json_handling() {
    let monitor = create_test_monitor();
    monitor.register_event_handler().await;

    // Process invalid JSON
    let result = monitor.process_message("invalid json {").await;
    assert!(result.is_err());

    // Monitor should still be functional
    let stats = monitor.get_monitor_stats().await;
    assert_eq!(stats.total_errors, 0); // Errors in parsing don't count as monitor errors
}

#[tokio::test]
async fn test_agent_error_handling() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;

    let agent_id = Uuid::new_v4();

    // Send error message
    let error_msg = AgentStreamMessage::Error {
        agent_id,
        error: AgentError::new("TEST_ERROR".to_string(), "Test error message".to_string()),
        timestamp: chrono::Utc::now(),
    };

    monitor.process_stream_message(error_msg).await.unwrap();

    // Verify agent is marked as failed
    let agent = monitor.get_agent_status(&agent_id).await.unwrap();
    assert_eq!(agent.status, AgentStatus::Failed);
    assert!(agent.error.is_some());
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_updates() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;

    let agent_ids: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();

    // Send concurrent updates for multiple agents
    let mut handles = vec![];

    for agent_id in agent_ids.iter() {
        let monitor_clone = Arc::clone(&monitor);
        let agent_id = *agent_id;

        let handle = tokio::spawn(async move {
            for i in 0..10 {
                let message = AgentStreamMessage::ProgressUpdate {
                    agent_id,
                    progress: AgentProgress::new((i * 10) as f32),
                    timestamp: chrono::Utc::now(),
                };
                monitor_clone.process_stream_message(message).await.unwrap();
                sleep(Duration::from_millis(10)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all agents were tracked
    let agents = monitor.list_agents().await;
    assert_eq!(agents.len(), 10);
}

#[tokio::test]
async fn test_high_message_throughput() {
    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(event_bus));
    monitor.register_event_handler().await;

    let agent_id = Uuid::new_v4();
    let message_count = 1000;

    let start = std::time::Instant::now();

    for i in 0..message_count {
        let message = AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::new((i as f32 / message_count as f32) * 100.0),
            timestamp: chrono::Utc::now(),
        };
        monitor.process_stream_message(message).await.unwrap();
    }

    let elapsed = start.elapsed();

    // Verify stats
    let stats = monitor.get_monitor_stats().await;
    assert_eq!(stats.total_messages, message_count);

    println!(
        "Processed {} messages in {:?} ({:.2} msg/sec)",
        message_count,
        elapsed,
        message_count as f64 / elapsed.as_secs_f64()
    );
}

// ============================================================================
// INTEGRATION SCENARIO TESTS
// ============================================================================

#[tokio::test]
async fn test_full_agent_lifecycle() {
    let rpc = setup_rpc_impl().await;
    let agent_id = Uuid::new_v4();

    // Simulate complete agent lifecycle
    let lifecycle_messages = vec![
        // 1. Agent spawned
        AgentStreamMessage::Lifecycle {
            agent_id,
            event: LifecycleEvent::Spawned,
            timestamp: chrono::Utc::now(),
        },
        // 2. Agent initializing
        AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Initializing,
            timestamp: chrono::Utc::now(),
        },
        // 3. Agent thinking
        AgentStreamMessage::ThoughtUpdate {
            agent_id,
            thought: "Analyzing requirements...".to_string(),
            timestamp: chrono::Utc::now(),
        },
        // 4. Agent running
        AgentStreamMessage::StatusUpdate {
            agent_id,
            status: AgentStatus::Running,
            timestamp: chrono::Utc::now(),
        },
        // 5. Progress updates
        AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::with_steps(1, 3),
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::with_steps(2, 3),
            timestamp: chrono::Utc::now(),
        },
        AgentStreamMessage::ProgressUpdate {
            agent_id,
            progress: AgentProgress::with_steps(3, 3),
            timestamp: chrono::Utc::now(),
        },
        // 6. Agent completed
        AgentStreamMessage::Lifecycle {
            agent_id,
            event: LifecycleEvent::Completed,
            timestamp: chrono::Utc::now(),
        },
    ];

    for message in lifecycle_messages {
        rpc.push_agent_update(message).await.unwrap();
        sleep(Duration::from_millis(50)).await;
    }

    // Verify final state
    let agent = rpc.get_agent_status(agent_id.to_string()).await.unwrap();
    assert_eq!(agent.status, AgentStatus::Completed);
    assert!(agent.started_at.is_some());
    assert!(agent.completed_at.is_some());
    assert!(agent.timeline.len() > 0);
}
