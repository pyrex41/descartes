//! Example: Agent Monitoring RPC Integration Usage (Phase 3:5.3)
//!
//! This example demonstrates how to use the agent monitoring system with RPC integration
//! to track and manage multiple agents in real-time.
//!
//! Run with:
//! ```bash
//! cargo run --example agent_monitor_usage --features="agent-monitoring"
//! ```

use descartes_core::{
    agent_state::{
        AgentProgress, AgentRuntimeState, AgentStatus, AgentStreamMessage, LifecycleEvent,
    },
    AgentError as RuntimeAgentError,
};
use descartes_daemon::{
    AgentMonitor, AgentMonitoringRpcImpl, AgentMonitoringRpcServer, AgentStatusFilter, EventBus,
    EventCategory, EventFilter,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Agent Monitoring RPC Integration Example ===\n");

    // ========================================================================
    // 1. Setup Event Bus and Agent Monitor
    // ========================================================================
    println!("1. Setting up event bus and agent monitor...");

    let event_bus = Arc::new(EventBus::new());
    let monitor = Arc::new(AgentMonitor::new(Arc::clone(&event_bus)));
    monitor.register_event_handler().await;

    // Start background monitoring tasks (stale agent cleanup, etc.)
    let _monitor_task = monitor.start().await;

    println!("   ✓ Event bus created");
    println!("   ✓ Agent monitor initialized");
    println!("   ✓ Background tasks started\n");

    // ========================================================================
    // 2. Subscribe to Events
    // ========================================================================
    println!("2. Subscribing to agent events...");

    let filter = EventFilter {
        event_categories: vec![EventCategory::Agent],
        ..Default::default()
    };

    let (_sub_id, mut event_rx) = event_bus.subscribe(Some(filter)).await;

    // Spawn task to print events
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            println!("   📡 Event received: {:?}", event);
        }
    });

    println!("   ✓ Subscribed to agent events\n");

    // ========================================================================
    // 3. Create RPC Implementation
    // ========================================================================
    println!("3. Creating RPC implementation...");

    let rpc = AgentMonitoringRpcImpl::new(Arc::clone(&monitor));

    println!("   ✓ RPC methods ready\n");

    // ========================================================================
    // 4. Simulate Multiple Agents
    // ========================================================================
    println!("4. Simulating agent lifecycle events...\n");

    // Spawn 3 agents
    let agent_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

    for (i, agent_id) in agent_ids.iter().enumerate() {
        println!("   Agent {}: Spawning...", i + 1);

        // Lifecycle: Spawned
        rpc.push_agent_update(AgentStreamMessage::Lifecycle {
            agent_id: *agent_id,
            event: LifecycleEvent::Spawned,
            timestamp: chrono::Utc::now(),
        })
        .await?;

        sleep(Duration::from_millis(100)).await;

        // Status: Initializing
        rpc.push_agent_update(AgentStreamMessage::StatusUpdate {
            agent_id: *agent_id,
            status: AgentStatus::Initializing,
            timestamp: chrono::Utc::now(),
        })
        .await?;

        sleep(Duration::from_millis(100)).await;

        // Thought update
        rpc.push_agent_update(AgentStreamMessage::ThoughtUpdate {
            agent_id: *agent_id,
            thought: format!("Agent {} analyzing task requirements...", i + 1),
            timestamp: chrono::Utc::now(),
        })
        .await?;

        sleep(Duration::from_millis(100)).await;

        // Status: Running
        rpc.push_agent_update(AgentStreamMessage::StatusUpdate {
            agent_id: *agent_id,
            status: AgentStatus::Running,
            timestamp: chrono::Utc::now(),
        })
        .await?;

        println!("   Agent {}: Running", i + 1);
    }

    println!();

    // ========================================================================
    // 5. Simulate Progress Updates
    // ========================================================================
    println!("5. Simulating progress updates...\n");

    for step in 1..=5 {
        for (i, agent_id) in agent_ids.iter().enumerate() {
            rpc.push_agent_update(AgentStreamMessage::ProgressUpdate {
                agent_id: *agent_id,
                progress: AgentProgress::with_steps(step, 5),
                timestamp: chrono::Utc::now(),
            })
            .await?;
        }

        println!(
            "   Progress: Step {}/5 ({:.0}%)",
            step,
            (step as f32 / 5.0) * 100.0
        );
        sleep(Duration::from_millis(500)).await;
    }

    println!();

    // ========================================================================
    // 6. Complete Some Agents, Fail Others
    // ========================================================================
    println!("6. Completing agents...\n");

    // Agent 0: Completed
    rpc.push_agent_update(AgentStreamMessage::Lifecycle {
        agent_id: agent_ids[0],
        event: LifecycleEvent::Completed,
        timestamp: chrono::Utc::now(),
    })
    .await?;
    println!("   Agent 1: ✓ Completed");

    // Agent 1: Failed
    rpc.push_agent_update(AgentStreamMessage::Error {
        agent_id: agent_ids[1],
        error: RuntimeAgentError::new("TASK_ERROR".to_string(), "Simulated error".to_string()),
        timestamp: chrono::Utc::now(),
    })
    .await?;
    println!("   Agent 2: ✗ Failed");

    // Agent 2: Completed
    rpc.push_agent_update(AgentStreamMessage::Lifecycle {
        agent_id: agent_ids[2],
        event: LifecycleEvent::Completed,
        timestamp: chrono::Utc::now(),
    })
    .await?;
    println!("   Agent 3: ✓ Completed\n");

    sleep(Duration::from_secs(1)).await;

    // ========================================================================
    // 7. Query Agent Status via RPC
    // ========================================================================
    println!("7. Querying agent status via RPC...\n");

    // List all agents
    let all_agents = rpc.list_agents(None).await?;
    println!("   Total agents: {}", all_agents.len());

    for agent in &all_agents {
        println!(
            "     - {} ({}) - Status: {}",
            agent.name, agent.agent_id, agent.status
        );
    }

    println!();

    // Filter by status
    let running_filter = AgentStatusFilter {
        status: Some(AgentStatus::Running),
        model_backend: None,
        active_only: None,
    };
    let running_agents = rpc.list_agents(Some(running_filter)).await?;
    println!("   Running agents: {}", running_agents.len());

    let completed_filter = AgentStatusFilter {
        status: Some(AgentStatus::Completed),
        model_backend: None,
        active_only: None,
    };
    let completed_agents = rpc.list_agents(Some(completed_filter)).await?;
    println!("   Completed agents: {}", completed_agents.len());

    let failed_filter = AgentStatusFilter {
        status: Some(AgentStatus::Failed),
        model_backend: None,
        active_only: None,
    };
    let failed_agents = rpc.list_agents(Some(failed_filter)).await?;
    println!("   Failed agents: {}\n", failed_agents.len());

    // ========================================================================
    // 8. Get Statistics
    // ========================================================================
    println!("8. Retrieving statistics...\n");

    let statistics = rpc.get_agent_statistics().await?;
    println!("   Agent Statistics:");
    println!("     Total agents: {}", statistics.total);

    if let Some(stats) = statistics.statistics {
        println!("     Active agents: {}", stats.total_active);
        println!("     Completed agents: {}", stats.total_completed);
        println!("     Failed agents: {}", stats.total_failed);

        if let Some(avg_time) = stats.avg_execution_time {
            println!("     Average execution time: {:.2}s", avg_time);
        }

        println!("\n     Status breakdown:");
        for (status, count) in stats.status_counts {
            println!("       {}: {}", status, count);
        }
    }

    println!();

    // ========================================================================
    // 9. Get Monitoring Health
    // ========================================================================
    println!("9. Checking monitoring system health...\n");

    let health = rpc.get_monitoring_health().await?;
    println!("   Monitoring Health:");
    println!("     Total agents tracked: {}", health.total_agents);
    println!("     Active agents: {}", health.active_agents);
    println!("     Failed agents: {}", health.failed_agents);
    println!("     Completed agents: {}", health.completed_agents);
    println!("     Total discovered: {}", health.total_discovered);
    println!("     Total removed: {}", health.total_removed);
    println!("     Total messages processed: {}", health.total_messages);

    if let Some(avg_time) = health.avg_execution_time_secs {
        println!("     Average execution time: {:.2}s", avg_time);
    }

    println!();

    // ========================================================================
    // 10. Individual Agent Details
    // ========================================================================
    println!("10. Retrieving individual agent details...\n");

    for (i, agent_id) in agent_ids.iter().enumerate() {
        let agent = rpc.get_agent_status(agent_id.to_string()).await?;
        println!("   Agent {}:", i + 1);
        println!("     ID: {}", agent.agent_id);
        println!("     Name: {}", agent.name);
        println!("     Status: {}", agent.status);
        println!("     Task: {}", agent.task);

        if let Some(thought) = &agent.current_thought {
            println!("     Current thought: {}", thought);
        }

        if let Some(progress) = &agent.progress {
            println!("     Progress: {:.1}%", progress.percentage);
        }

        if let Some(error) = &agent.error {
            println!("     Error: {} - {}", error.code, error.message);
        }

        println!("     Timeline events: {}", agent.timeline.len());

        if let Some(exec_time) = agent.execution_time() {
            println!("     Execution time: {}s", exec_time.num_seconds());
        }

        println!();
    }

    // ========================================================================
    // Summary
    // ========================================================================
    println!("=== Summary ===");
    println!("Successfully demonstrated:");
    println!("  ✓ Agent monitoring system initialization");
    println!("  ✓ Event bus integration");
    println!("  ✓ Real-time agent lifecycle tracking");
    println!("  ✓ Progress monitoring");
    println!("  ✓ RPC query methods");
    println!("  ✓ Statistics aggregation");
    println!("  ✓ Health monitoring");
    println!("\nAll systems operational! 🚀\n");

    Ok(())
}
