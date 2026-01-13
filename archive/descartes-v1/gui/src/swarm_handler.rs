//! Real-time Stream Handler for Swarm Monitor
//!
//! This module implements a StreamHandler that bridges the agent stream parser
//! with the Swarm Monitor UI, providing real-time updates of agent states.

use chrono::{DateTime, Utc};
use descartes_core::agent_stream_parser::StreamHandler;
use descartes_core::AgentRuntimeState;
use descartes_core::{
    AgentProgress, LifecycleEvent, OutputStream, RuntimeAgentError, RuntimeAgentStatus,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================================
// GUI STREAM HANDLER
// ============================================================================

/// Stream handler that updates the GUI's swarm monitor state
///
/// This handler receives callbacks from the AgentStreamParser and updates
/// the shared agent state that the GUI reads from.
#[derive(Clone)]
pub struct GuiStreamHandler {
    /// Shared agent state (wrapped for thread-safe updates)
    agents: Arc<Mutex<HashMap<Uuid, AgentRuntimeState>>>,
}

impl GuiStreamHandler {
    /// Create a new GUI stream handler
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a clone of the current agents map
    pub fn get_agents(&self) -> HashMap<Uuid, AgentRuntimeState> {
        self.agents.lock().unwrap().clone()
    }

    /// Get shared reference to agents for real-time access
    pub fn get_agents_ref(&self) -> Arc<Mutex<HashMap<Uuid, AgentRuntimeState>>> {
        Arc::clone(&self.agents)
    }

    /// Add or update an agent
    pub fn add_agent(&self, agent: AgentRuntimeState) {
        self.agents.lock().unwrap().insert(agent.agent_id, agent);
    }

    /// Remove an agent
    pub fn remove_agent(&self, agent_id: &Uuid) {
        self.agents.lock().unwrap().remove(agent_id);
    }

    /// Get a specific agent
    #[allow(dead_code)]
    fn get_or_create_agent(&self, agent_id: Uuid) -> AgentRuntimeState {
        let mut agents = self.agents.lock().unwrap();

        agents
            .entry(agent_id)
            .or_insert_with(|| {
                AgentRuntimeState::new(
                    agent_id,
                    format!("agent-{}", &agent_id.to_string()[..8]),
                    "Auto-created from stream".to_string(),
                    "unknown".to_string(),
                )
            })
            .clone()
    }

    /// Update agent state
    fn update_agent<F>(&self, agent_id: Uuid, updater: F)
    where
        F: FnOnce(&mut AgentRuntimeState),
    {
        let mut agents = self.agents.lock().unwrap();

        if let Some(agent) = agents.get_mut(&agent_id) {
            updater(agent);
        } else {
            // Create new agent if it doesn't exist
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", &agent_id.to_string()[..8]),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            updater(&mut agent);
            agents.insert(agent_id, agent);
        }
    }
}

impl Default for GuiStreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamHandler for GuiStreamHandler {
    fn on_status_update(
        &mut self,
        agent_id: Uuid,
        status: RuntimeAgentStatus,
        _timestamp: DateTime<Utc>,
    ) {
        self.update_agent(agent_id, |agent| {
            agent
                .transition_to(status, Some("Status update from stream".to_string()))
                .ok();

            // Clear thought if transitioning out of Thinking state
            if agent.status != RuntimeAgentStatus::Thinking {
                agent.clear_thought();
            }
        });
    }

    fn on_thought_update(&mut self, agent_id: Uuid, thought: String, _timestamp: DateTime<Utc>) {
        self.update_agent(agent_id, |agent| {
            agent.update_thought(thought);

            // Auto-transition to Thinking state if not already
            if agent.status != RuntimeAgentStatus::Thinking {
                agent
                    .transition_to(
                        RuntimeAgentStatus::Thinking,
                        Some("Thought detected".to_string()),
                    )
                    .ok();
            }
        });
    }

    fn on_progress_update(
        &mut self,
        agent_id: Uuid,
        progress: AgentProgress,
        _timestamp: DateTime<Utc>,
    ) {
        self.update_agent(agent_id, |agent| {
            agent.update_progress(progress);
        });
    }

    fn on_output(
        &mut self,
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        _timestamp: DateTime<Utc>,
    ) {
        // Output is not stored in agent state, but we log it
        tracing::debug!("Agent {} {:?}: {}", agent_id, stream, content);
    }

    fn on_error(&mut self, agent_id: Uuid, error: RuntimeAgentError, _timestamp: DateTime<Utc>) {
        self.update_agent(agent_id, |agent| {
            agent.set_error(error.clone());
            agent
                .transition_to(RuntimeAgentStatus::Failed, Some(error.message.clone()))
                .ok();
        });
    }

    fn on_lifecycle(&mut self, agent_id: Uuid, event: LifecycleEvent, _timestamp: DateTime<Utc>) {
        let status = match event {
            LifecycleEvent::Spawned => RuntimeAgentStatus::Idle,
            LifecycleEvent::Started => RuntimeAgentStatus::Initializing,
            LifecycleEvent::Paused => RuntimeAgentStatus::Paused,
            LifecycleEvent::Resumed => RuntimeAgentStatus::Running,
            LifecycleEvent::Completed => RuntimeAgentStatus::Completed,
            LifecycleEvent::Failed => RuntimeAgentStatus::Failed,
            LifecycleEvent::Terminated => RuntimeAgentStatus::Terminated,
        };

        self.update_agent(agent_id, |agent| {
            agent
                .transition_to(status, Some(format!("Lifecycle event: {:?}", event)))
                .ok();
        });
    }

    fn on_heartbeat(&mut self, agent_id: Uuid, timestamp: DateTime<Utc>) {
        self.update_agent(agent_id, |agent| {
            agent.updated_at = timestamp;
        });
    }
}

// ============================================================================
// DEMO DATA GENERATOR
// ============================================================================

/// Generate sample agent data for demonstration purposes
pub fn generate_sample_agents() -> HashMap<Uuid, AgentRuntimeState> {
    use descartes_core::AgentProgress;

    let mut agents = HashMap::new();

    // Agent 1: Running agent
    let agent1_id = Uuid::new_v4();
    let mut agent1 = AgentRuntimeState::new(
        agent1_id,
        "code-analyzer".to_string(),
        "Analyze codebase for patterns".to_string(),
        "anthropic".to_string(),
    );
    agent1
        .transition_to(
            RuntimeAgentStatus::Running,
            Some("Analyzing files".to_string()),
        )
        .ok();
    agent1.update_progress(AgentProgress::with_steps(15, 30));
    agents.insert(agent1_id, agent1);

    // Agent 2: Thinking agent
    let agent2_id = Uuid::new_v4();
    let mut agent2 = AgentRuntimeState::new(
        agent2_id,
        "problem-solver".to_string(),
        "Solve complex algorithmic problem".to_string(),
        "anthropic".to_string(),
    );
    agent2
        .transition_to(RuntimeAgentStatus::Thinking, Some("Processing".to_string()))
        .ok();
    agent2.update_thought(
        "Evaluating different approaches to optimize the sorting algorithm...".to_string(),
    );
    agent2.update_progress(AgentProgress::new(45.0));
    agents.insert(agent2_id, agent2);

    // Agent 3: Another thinking agent
    let agent3_id = Uuid::new_v4();
    let mut agent3 = AgentRuntimeState::new(
        agent3_id,
        "code-generator".to_string(),
        "Generate API endpoints".to_string(),
        "openai".to_string(),
    );
    agent3
        .transition_to(RuntimeAgentStatus::Thinking, Some("Planning".to_string()))
        .ok();
    agent3.update_thought(
        "Considering RESTful design patterns and best practices for the API structure..."
            .to_string(),
    );
    agents.insert(agent3_id, agent3);

    // Agent 4: Paused agent
    let agent4_id = Uuid::new_v4();
    let mut agent4 = AgentRuntimeState::new(
        agent4_id,
        "test-runner".to_string(),
        "Run integration tests".to_string(),
        "anthropic".to_string(),
    );
    agent4
        .transition_to(RuntimeAgentStatus::Initializing, None)
        .ok();
    agent4.transition_to(RuntimeAgentStatus::Running, None).ok();
    agent4
        .transition_to(
            RuntimeAgentStatus::Paused,
            Some("User requested pause".to_string()),
        )
        .ok();
    agent4.update_progress(AgentProgress::with_steps(5, 20));
    agents.insert(agent4_id, agent4);

    // Agent 5: Completed agent
    let agent5_id = Uuid::new_v4();
    let mut agent5 = AgentRuntimeState::new(
        agent5_id,
        "doc-writer".to_string(),
        "Generate documentation".to_string(),
        "anthropic".to_string(),
    );
    agent5.transition_to(RuntimeAgentStatus::Running, None).ok();
    agent5
        .transition_to(
            RuntimeAgentStatus::Completed,
            Some("Documentation complete".to_string()),
        )
        .ok();
    agent5.update_progress(AgentProgress::new(100.0));
    agents.insert(agent5_id, agent5);

    // Agent 6: Failed agent
    let agent6_id = Uuid::new_v4();
    let mut agent6 = AgentRuntimeState::new(
        agent6_id,
        "database-migrator".to_string(),
        "Migrate database schema".to_string(),
        "anthropic".to_string(),
    );
    agent6.transition_to(RuntimeAgentStatus::Running, None).ok();
    agent6.set_error(RuntimeAgentError::new(
        "CONNECTION_FAILED".to_string(),
        "Failed to connect to database: connection timeout".to_string(),
    ));
    agent6
        .transition_to(
            RuntimeAgentStatus::Failed,
            Some("Connection error".to_string()),
        )
        .ok();
    agents.insert(agent6_id, agent6);

    // Agent 7: Idle agent
    let agent7_id = Uuid::new_v4();
    let agent7 = AgentRuntimeState::new(
        agent7_id,
        "task-scheduler".to_string(),
        "Schedule background tasks".to_string(),
        "openai".to_string(),
    );
    agents.insert(agent7_id, agent7);

    // Agent 8: Initializing agent
    let agent8_id = Uuid::new_v4();
    let mut agent8 = AgentRuntimeState::new(
        agent8_id,
        "data-processor".to_string(),
        "Process large dataset".to_string(),
        "anthropic".to_string(),
    );
    agent8
        .transition_to(
            RuntimeAgentStatus::Initializing,
            Some("Loading data".to_string()),
        )
        .ok();
    agents.insert(agent8_id, agent8);

    // Agent 9: Another running agent with progress
    let agent9_id = Uuid::new_v4();
    let mut agent9 = AgentRuntimeState::new(
        agent9_id,
        "refactorer".to_string(),
        "Refactor legacy code".to_string(),
        "openai".to_string(),
    );
    agent9.transition_to(RuntimeAgentStatus::Running, None).ok();
    agent9.update_progress(AgentProgress::with_steps(8, 12));
    agents.insert(agent9_id, agent9);

    // Agent 10: Thinking with complex thought
    let agent10_id = Uuid::new_v4();
    let mut agent10 = AgentRuntimeState::new(
        agent10_id,
        "security-auditor".to_string(),
        "Audit security vulnerabilities".to_string(),
        "anthropic".to_string(),
    );
    agent10
        .transition_to(RuntimeAgentStatus::Thinking, None)
        .ok();
    agent10.update_thought("Analyzing authentication flows for potential security vulnerabilities, checking for SQL injection, XSS, and CSRF attack vectors...".to_string());
    agent10.update_progress(AgentProgress::new(62.5));
    agents.insert(agent10_id, agent10);

    agents
}
