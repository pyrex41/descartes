//! Agent registry for tracking and managing spawned agents
//!
//! Provides:
//! - Agent handle tracking with task associations
//! - Terminal type abstraction (Zellij, Tmux, Kitty, Headless)
//! - Spawn, focus, and status management

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Lifecycle status of a spawned agent in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryStatus {
    /// Agent is being spawned
    Spawning,
    /// Agent is running
    Running,
    /// Agent is idle/waiting
    Idle,
    /// Agent completed successfully
    Completed,
    /// Agent failed
    Failed,
    /// Agent was terminated
    Terminated,
}

impl std::fmt::Display for RegistryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryStatus::Spawning => write!(f, "spawning"),
            RegistryStatus::Running => write!(f, "running"),
            RegistryStatus::Idle => write!(f, "idle"),
            RegistryStatus::Completed => write!(f, "completed"),
            RegistryStatus::Failed => write!(f, "failed"),
            RegistryStatus::Terminated => write!(f, "terminated"),
        }
    }
}

/// Terminal multiplexer type for agent panes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TerminalType {
    /// Zellij terminal multiplexer
    Zellij,
    /// Tmux terminal multiplexer
    Tmux,
    /// Kitty terminal with splits
    Kitty,
    /// Headless mode (no visible terminal)
    #[default]
    Headless,
}

impl std::fmt::Display for TerminalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalType::Zellij => write!(f, "zellij"),
            TerminalType::Tmux => write!(f, "tmux"),
            TerminalType::Kitty => write!(f, "kitty"),
            TerminalType::Headless => write!(f, "headless"),
        }
    }
}

impl TerminalType {
    /// Detect terminal type from environment
    pub fn detect() -> Self {
        if std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok() {
            TerminalType::Zellij
        } else if std::env::var("TMUX").is_ok() {
            TerminalType::Tmux
        } else if std::env::var("KITTY_WINDOW_ID").is_ok() {
            TerminalType::Kitty
        } else {
            TerminalType::Headless
        }
    }
}

/// Handle to a spawned agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandle {
    /// Unique agent ID
    pub id: String,
    /// Associated task ID (e.g., "7.3.1")
    pub task_id: Option<String>,
    /// Pane/window name in terminal multiplexer
    pub pane_name: String,
    /// Current status
    pub status: RegistryStatus,
    /// When the agent was spawned
    pub spawned_at: DateTime<Utc>,
    /// Terminal type used
    pub terminal_type: TerminalType,
    /// Category of the agent (e.g., "builder", "searcher")
    #[serde(default)]
    pub category: Option<String>,
    /// Output/result when completed
    #[serde(default)]
    pub output: Option<String>,
}

impl AgentHandle {
    /// Create a new agent handle
    pub fn new(id: impl Into<String>, pane_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            task_id: None,
            pane_name: pane_name.into(),
            status: RegistryStatus::Spawning,
            spawned_at: Utc::now(),
            terminal_type: TerminalType::detect(),
            category: None,
            output: None,
        }
    }

    /// Set the task ID
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Set the terminal type
    pub fn with_terminal_type(mut self, terminal_type: TerminalType) -> Self {
        self.terminal_type = terminal_type;
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Check if the agent is active (running or idle)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            RegistryStatus::Spawning | RegistryStatus::Running | RegistryStatus::Idle
        )
    }

    /// Check if the agent has finished (completed or failed)
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            RegistryStatus::Completed | RegistryStatus::Failed | RegistryStatus::Terminated
        )
    }

    /// Get duration since spawn
    pub fn elapsed(&self) -> chrono::Duration {
        Utc::now() - self.spawned_at
    }
}

/// Registry for tracking spawned agents
pub struct AgentRegistry {
    /// Agents by ID
    agents: HashMap<String, AgentHandle>,
    /// Task ID to agent ID mapping
    task_index: HashMap<String, String>,
    /// Current terminal type
    terminal_type: TerminalType,
    /// Counter for generating unique IDs
    next_id: u64,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            task_index: HashMap::new(),
            terminal_type: TerminalType::detect(),
            next_id: 1,
        }
    }

    /// Create with specific terminal type
    pub fn with_terminal_type(terminal_type: TerminalType) -> Self {
        Self {
            agents: HashMap::new(),
            task_index: HashMap::new(),
            terminal_type,
            next_id: 1,
        }
    }

    /// Generate a unique agent ID
    fn generate_id(&mut self) -> String {
        let id = format!("agent-{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Spawn a new agent and track it
    ///
    /// This creates a handle for tracking; the actual spawning is done externally.
    pub fn spawn(&mut self, pane_name: impl Into<String>, task_id: Option<String>) -> AgentHandle {
        let id = self.generate_id();
        let pane_name = pane_name.into();

        let handle = AgentHandle {
            id: id.clone(),
            task_id: task_id.clone(),
            pane_name,
            status: RegistryStatus::Spawning,
            spawned_at: Utc::now(),
            terminal_type: self.terminal_type,
            category: None,
            output: None,
        };

        // Index by task ID if provided
        if let Some(tid) = &task_id {
            self.task_index.insert(tid.clone(), id.clone());
        }

        self.agents.insert(id, handle.clone());
        handle
    }

    /// Spawn with a specific category
    pub fn spawn_with_category(
        &mut self,
        pane_name: impl Into<String>,
        task_id: Option<String>,
        category: impl Into<String>,
    ) -> AgentHandle {
        let mut handle = self.spawn(pane_name, task_id);
        handle.category = Some(category.into());
        self.agents.insert(handle.id.clone(), handle.clone());
        handle
    }

    /// Get an agent by ID
    pub fn get(&self, id: &str) -> Option<&AgentHandle> {
        self.agents.get(id)
    }

    /// Get a mutable agent by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut AgentHandle> {
        self.agents.get_mut(id)
    }

    /// Get agent by task ID
    pub fn get_by_task(&self, task_id: &str) -> Option<&AgentHandle> {
        self.task_index
            .get(task_id)
            .and_then(|agent_id| self.agents.get(agent_id))
    }

    /// Get mutable agent by task ID
    pub fn get_by_task_mut(&mut self, task_id: &str) -> Option<&mut AgentHandle> {
        if let Some(agent_id) = self.task_index.get(task_id).cloned() {
            self.agents.get_mut(&agent_id)
        } else {
            None
        }
    }

    /// Update an agent's status
    pub fn update_status(&mut self, id: &str, status: RegistryStatus) -> Result<()> {
        let handle = self
            .agents
            .get_mut(id)
            .ok_or_else(|| Error::Subagent(format!("Agent not found: {}", id)))?;

        handle.status = status;
        Ok(())
    }

    /// Update status and set output
    pub fn update_status_with_output(
        &mut self,
        id: &str,
        status: RegistryStatus,
        output: impl Into<String>,
    ) -> Result<()> {
        let handle = self
            .agents
            .get_mut(id)
            .ok_or_else(|| Error::Subagent(format!("Agent not found: {}", id)))?;

        handle.status = status;
        handle.output = Some(output.into());
        Ok(())
    }

    /// List all running agents
    pub fn list_running(&self) -> Vec<&AgentHandle> {
        self.agents.values().filter(|h| h.is_active()).collect()
    }

    /// List all agents
    pub fn list_all(&self) -> Vec<&AgentHandle> {
        self.agents.values().collect()
    }

    /// List agents by status
    pub fn list_by_status(&self, status: RegistryStatus) -> Vec<&AgentHandle> {
        self.agents
            .values()
            .filter(|h| h.status == status)
            .collect()
    }

    /// Focus on an agent's pane (switch to it in the terminal)
    ///
    /// Returns the command that should be run to focus the pane.
    pub fn focus(&self, id: &str) -> Result<FocusCommand> {
        let handle = self
            .agents
            .get(id)
            .ok_or_else(|| Error::Subagent(format!("Agent not found: {}", id)))?;

        Ok(FocusCommand::new(handle.terminal_type, &handle.pane_name))
    }

    /// Focus on agent by task ID
    pub fn focus_by_task(&self, task_id: &str) -> Result<FocusCommand> {
        let agent_id = self
            .task_index
            .get(task_id)
            .ok_or_else(|| Error::Subagent(format!("No agent for task: {}", task_id)))?;

        self.focus(agent_id)
    }

    /// Remove a finished agent from tracking
    pub fn remove(&mut self, id: &str) -> Option<AgentHandle> {
        if let Some(handle) = self.agents.remove(id) {
            // Also remove from task index
            if let Some(task_id) = &handle.task_id {
                self.task_index.remove(task_id);
            }
            Some(handle)
        } else {
            None
        }
    }

    /// Clean up finished agents older than the given duration
    pub fn cleanup_old(&mut self, max_age: chrono::Duration) -> Vec<AgentHandle> {
        let now = Utc::now();
        let mut removed = Vec::new();

        let ids_to_remove: Vec<String> = self
            .agents
            .iter()
            .filter(|(_, h)| h.is_finished() && (now - h.spawned_at) > max_age)
            .map(|(id, _)| id.clone())
            .collect();

        for id in ids_to_remove {
            if let Some(handle) = self.remove(&id) {
                removed.push(handle);
            }
        }

        removed
    }

    /// Get count of agents by status
    pub fn count_by_status(&self) -> HashMap<RegistryStatus, usize> {
        let mut counts = HashMap::new();
        for handle in self.agents.values() {
            *counts.entry(handle.status).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Command to focus on an agent's pane
#[derive(Debug, Clone)]
pub struct FocusCommand {
    /// Terminal type
    pub terminal_type: TerminalType,
    /// Pane name
    pub pane_name: String,
}

impl FocusCommand {
    /// Create a new focus command
    pub fn new(terminal_type: TerminalType, pane_name: impl Into<String>) -> Self {
        Self {
            terminal_type,
            pane_name: pane_name.into(),
        }
    }

    /// Get the shell command to execute for focusing
    pub fn to_shell_command(&self) -> String {
        match self.terminal_type {
            TerminalType::Zellij => {
                format!("zellij action focus-pane --name {}", self.pane_name)
            }
            TerminalType::Tmux => {
                format!("tmux select-pane -t {}", self.pane_name)
            }
            TerminalType::Kitty => {
                format!("kitty @ focus-window --match title:{}", self.pane_name)
            }
            TerminalType::Headless => {
                // In headless mode, there's no pane to focus
                String::new()
            }
        }
    }

    /// Check if focus is supported
    pub fn is_supported(&self) -> bool {
        !matches!(self.terminal_type, TerminalType::Headless)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_handle_new() {
        let handle = AgentHandle::new("agent-1", "builder-pane");
        assert_eq!(handle.id, "agent-1");
        assert_eq!(handle.pane_name, "builder-pane");
        assert_eq!(handle.status, RegistryStatus::Spawning);
        assert!(handle.task_id.is_none());
    }

    #[test]
    fn test_agent_handle_with_task() {
        let handle = AgentHandle::new("agent-1", "builder-pane")
            .with_task_id("7.3.1")
            .with_category("builder");

        assert_eq!(handle.task_id, Some("7.3.1".to_string()));
        assert_eq!(handle.category, Some("builder".to_string()));
    }

    #[test]
    fn test_agent_handle_is_active() {
        let mut handle = AgentHandle::new("agent-1", "pane");
        assert!(handle.is_active());
        assert!(!handle.is_finished());

        handle.status = RegistryStatus::Running;
        assert!(handle.is_active());

        handle.status = RegistryStatus::Completed;
        assert!(!handle.is_active());
        assert!(handle.is_finished());
    }

    #[test]
    fn test_registry_spawn() {
        let mut registry = AgentRegistry::new();
        let handle = registry.spawn("pane-1", Some("task-1".to_string()));

        assert_eq!(handle.pane_name, "pane-1");
        assert_eq!(handle.task_id, Some("task-1".to_string()));

        // Should be retrievable
        assert!(registry.get(&handle.id).is_some());
        assert!(registry.get_by_task("task-1").is_some());
    }

    #[test]
    fn test_registry_update_status() {
        let mut registry = AgentRegistry::new();
        let handle = registry.spawn("pane-1", None);

        registry
            .update_status(&handle.id, RegistryStatus::Running)
            .unwrap();
        assert_eq!(
            registry.get(&handle.id).unwrap().status,
            RegistryStatus::Running
        );

        registry
            .update_status_with_output(&handle.id, RegistryStatus::Completed, "done!")
            .unwrap();
        let updated = registry.get(&handle.id).unwrap();
        assert_eq!(updated.status, RegistryStatus::Completed);
        assert_eq!(updated.output, Some("done!".to_string()));
    }

    #[test]
    fn test_registry_list_running() {
        let mut registry = AgentRegistry::new();

        let h1 = registry.spawn("pane-1", None);
        let h2 = registry.spawn("pane-2", None);
        registry.spawn("pane-3", None);

        registry
            .update_status(&h1.id, RegistryStatus::Running)
            .unwrap();
        registry
            .update_status(&h2.id, RegistryStatus::Completed)
            .unwrap();

        let running = registry.list_running();
        // h1 is Running (active), h3 is still Spawning (active), h2 is Completed (not active)
        assert_eq!(running.len(), 2);
    }

    #[test]
    fn test_registry_focus() {
        let mut registry = AgentRegistry::with_terminal_type(TerminalType::Zellij);
        let handle = registry.spawn("my-pane", Some("task-1".to_string()));

        let focus = registry.focus(&handle.id).unwrap();
        assert!(focus.is_supported());
        assert!(focus.to_shell_command().contains("zellij"));
        assert!(focus.to_shell_command().contains("my-pane"));

        let focus_by_task = registry.focus_by_task("task-1").unwrap();
        assert_eq!(focus_by_task.pane_name, "my-pane");
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = AgentRegistry::new();
        let handle = registry.spawn("pane-1", Some("task-1".to_string()));
        let id = handle.id.clone();

        assert!(registry.get(&id).is_some());
        assert!(registry.get_by_task("task-1").is_some());

        let removed = registry.remove(&id);
        assert!(removed.is_some());
        assert!(registry.get(&id).is_none());
        assert!(registry.get_by_task("task-1").is_none());
    }

    #[test]
    fn test_terminal_type_display() {
        assert_eq!(format!("{}", TerminalType::Zellij), "zellij");
        assert_eq!(format!("{}", TerminalType::Tmux), "tmux");
        assert_eq!(format!("{}", TerminalType::Kitty), "kitty");
        assert_eq!(format!("{}", TerminalType::Headless), "headless");
    }

    #[test]
    fn test_agent_status_display() {
        assert_eq!(format!("{}", RegistryStatus::Running), "running");
        assert_eq!(format!("{}", RegistryStatus::Completed), "completed");
        assert_eq!(format!("{}", RegistryStatus::Failed), "failed");
    }

    #[test]
    fn test_focus_command_headless() {
        let focus = FocusCommand::new(TerminalType::Headless, "pane");
        assert!(!focus.is_supported());
        assert!(focus.to_shell_command().is_empty());
    }

    #[test]
    fn test_count_by_status() {
        let mut registry = AgentRegistry::new();

        let h1 = registry.spawn("pane-1", None);
        let h2 = registry.spawn("pane-2", None);
        registry.spawn("pane-3", None);

        registry
            .update_status(&h1.id, RegistryStatus::Running)
            .unwrap();
        registry
            .update_status(&h2.id, RegistryStatus::Running)
            .unwrap();

        let counts = registry.count_by_status();
        assert_eq!(counts.get(&RegistryStatus::Running), Some(&2));
        assert_eq!(counts.get(&RegistryStatus::Spawning), Some(&1));
    }
}
