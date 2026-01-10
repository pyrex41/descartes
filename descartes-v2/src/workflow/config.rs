//! Workflow configuration parsing and structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::{Error, Result};

/// Main workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Workflow metadata
    pub workflow: WorkflowMeta,
    /// Gate configurations
    #[serde(default)]
    pub gates: HashMap<String, GateConfig>,
    /// Transition configurations
    #[serde(default)]
    pub transitions: HashMap<String, TransitionConfig>,
    /// Notification channel configurations
    #[serde(default)]
    pub notifications: HashMap<String, NotificationConfig>,
}

/// Workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMeta {
    /// Workflow name
    pub name: String,
    /// Ordered list of stages
    pub stages: Vec<String>,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

/// Gate configuration for a transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    /// Gate type: auto, manual, notify
    #[serde(rename = "type")]
    pub gate_type: GateType,
    /// Timeout duration (for notify gates)
    #[serde(default, with = "humantime_serde")]
    pub timeout: Option<Duration>,
    /// What to do on timeout
    #[serde(default)]
    pub timeout_action: TimeoutAction,
    /// Notification channels to use
    #[serde(default)]
    pub notify: Vec<String>,
    /// Custom message for notifications
    #[serde(default)]
    pub message: Option<String>,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            gate_type: GateType::Auto,
            timeout: None,
            timeout_action: TimeoutAction::Continue,
            notify: Vec::new(),
            message: None,
        }
    }
}

/// Types of gates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateType {
    /// Automatically continue without waiting
    Auto,
    /// Always wait for manual approval
    Manual,
    /// Notify and wait for response or timeout
    Notify,
}

impl Default for GateType {
    fn default() -> Self {
        GateType::Auto
    }
}

/// What to do when a notify gate times out
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeoutAction {
    /// Continue to next stage
    Continue,
    /// Stay paused, require explicit approval
    Pause,
}

impl Default for TimeoutAction {
    fn default() -> Self {
        TimeoutAction::Continue
    }
}

/// Transition configuration between stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionConfig {
    /// Source stage (optional, inferred from key if not set)
    #[serde(default)]
    pub from: Option<String>,
    /// Target stage (optional, inferred from key if not set)
    #[serde(default)]
    pub to: Option<String>,
    /// Command to run when entering target stage
    #[serde(default)]
    pub command: Option<String>,
    /// Handoff template (inline or file path)
    #[serde(default)]
    pub handoff_template: Option<String>,
    /// Hooks to run before the transition
    #[serde(default)]
    pub pre_hooks: Vec<String>,
    /// Hooks to run after the transition
    #[serde(default)]
    pub post_hooks: Vec<String>,
    /// Context to automatically include
    #[serde(default)]
    pub auto_context: Vec<AutoContext>,
}

/// Types of context that can be auto-included in handoffs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoContext {
    /// Include SCUD tasks
    ScudTasks,
    /// Include SCUD waves
    ScudWaves,
    /// Include SCUD dependencies
    ScudDeps,
    /// Include git diff
    GitDiff,
    /// Include git status
    GitStatus,
    /// Include recent transcript summary
    TranscriptSummary,
    /// Custom context from a command
    #[serde(untagged)]
    Custom(String),
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NotificationConfig {
    /// Telegram bot notification
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    /// Slack webhook notification
    Slack {
        webhook_url: String,
        #[serde(default)]
        channel: Option<String>,
    },
    /// Email notification
    Email {
        smtp_host: String,
        smtp_port: u16,
        from: String,
        to: String,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
    },
    /// Desktop notification (local only)
    Desktop,
    /// Just log (for testing/debugging)
    Log,
}

impl WorkflowConfig {
    /// Load workflow configuration from a file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read workflow config: {}", e)))?;
        Self::parse(&content)
    }

    /// Parse workflow configuration from TOML string
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content)
            .map_err(|e| Error::Config(format!("Failed to parse workflow config: {}", e)))
    }

    /// Get gate configuration for a transition
    pub fn get_gate(&self, from: &str, to: &str) -> GateConfig {
        let key = format!("{}_to_{}", from, to);
        self.gates.get(&key).cloned().unwrap_or_default()
    }

    /// Get transition configuration
    pub fn get_transition(&self, from: &str, to: &str) -> Option<&TransitionConfig> {
        let key = format!("{}_to_{}", from, to);
        self.transitions.get(&key)
    }

    /// Get the next stage after the given stage
    pub fn next_stage(&self, current: &str) -> Option<&str> {
        let stages = &self.workflow.stages;
        stages
            .iter()
            .position(|s| s == current)
            .and_then(|i| stages.get(i + 1))
            .map(|s| s.as_str())
    }

    /// Check if a stage exists
    pub fn has_stage(&self, stage: &str) -> bool {
        self.workflow.stages.iter().any(|s| s == stage)
    }

    /// Get all stages
    pub fn stages(&self) -> &[String] {
        &self.workflow.stages
    }
}

/// Create a default workflow configuration
pub fn default_workflow() -> WorkflowConfig {
    WorkflowConfig {
        workflow: WorkflowMeta {
            name: "default".to_string(),
            stages: vec![
                "research".to_string(),
                "plan".to_string(),
                "implement".to_string(),
                "validate".to_string(),
            ],
            description: Some("Default development workflow".to_string()),
        },
        gates: {
            let mut gates = HashMap::new();
            gates.insert(
                "research_to_plan".to_string(),
                GateConfig {
                    gate_type: GateType::Notify,
                    timeout: Some(Duration::from_secs(300)), // 5 minutes
                    timeout_action: TimeoutAction::Continue,
                    notify: vec!["telegram".to_string()],
                    message: Some("Research complete. Review handoff before planning?".to_string()),
                },
            );
            gates.insert(
                "plan_to_implement".to_string(),
                GateConfig {
                    gate_type: GateType::Manual,
                    timeout: None,
                    timeout_action: TimeoutAction::Pause,
                    notify: vec!["telegram".to_string()],
                    message: Some("Plan complete. Ready to implement?".to_string()),
                },
            );
            gates.insert(
                "implement_to_validate".to_string(),
                GateConfig {
                    gate_type: GateType::Auto,
                    ..Default::default()
                },
            );
            gates
        },
        transitions: {
            let mut transitions = HashMap::new();
            transitions.insert(
                "research_to_plan".to_string(),
                TransitionConfig {
                    from: Some("research".to_string()),
                    to: Some("plan".to_string()),
                    command: Some("/create_plan".to_string()),
                    handoff_template: Some(
                        r#"## Research Summary
{{summary}}

## Key Findings
{{findings}}

## Recommended Approach
{{recommendations}}"#
                            .to_string(),
                    ),
                    pre_hooks: vec![],
                    post_hooks: vec![],
                    auto_context: vec![],
                },
            );
            transitions.insert(
                "plan_to_implement".to_string(),
                TransitionConfig {
                    from: Some("plan".to_string()),
                    to: Some("implement".to_string()),
                    command: Some("/implement_plan".to_string()),
                    handoff_template: Some(
                        r#"## Implementation Plan
{{summary}}

## SCUD Tasks by Wave
{{scud_waves}}

## Dependencies
{{scud_deps}}

## Constraints
{{constraints}}

Work through waves in parallel where dependencies allow."#
                            .to_string(),
                    ),
                    pre_hooks: vec!["scud parse {{plan_file}}".to_string()],
                    post_hooks: vec![],
                    auto_context: vec![AutoContext::ScudTasks, AutoContext::ScudWaves],
                },
            );
            transitions
        },
        notifications: HashMap::new(),
    }
}

// Custom serde module for Duration using humantime
mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => {
                let s = humantime::format_duration(*d).to_string();
                serializer.serialize_some(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            Some(s) => humantime::parse_duration(&s)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workflow_config() {
        let config = r##"
[workflow]
name = "test-workflow"
stages = ["research", "plan", "implement"]

[gates.research_to_plan]
type = "notify"
timeout = "5m"
notify = ["telegram"]

[gates.plan_to_implement]
type = "manual"

[transitions.research_to_plan]
command = "/create_plan"
handoff_template = "Summary: {{summary}}"

[notifications.telegram]
type = "telegram"
bot_token = "token123"
chat_id = "12345"
"##;

        let parsed = WorkflowConfig::parse(config).unwrap();
        assert_eq!(parsed.workflow.name, "test-workflow");
        assert_eq!(parsed.workflow.stages.len(), 3);
        assert!(parsed.gates.contains_key("research_to_plan"));
        assert!(parsed.notifications.contains_key("telegram"));
    }

    #[test]
    fn test_next_stage() {
        let config = default_workflow();
        assert_eq!(config.next_stage("research"), Some("plan"));
        assert_eq!(config.next_stage("plan"), Some("implement"));
        assert_eq!(config.next_stage("validate"), None);
    }

    #[test]
    fn test_get_gate() {
        let config = default_workflow();
        let gate = config.get_gate("research", "plan");
        assert_eq!(gate.gate_type, GateType::Notify);
    }
}
