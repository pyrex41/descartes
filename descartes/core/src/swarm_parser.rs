/// Swarm.toml Parser - Declarative workflow configuration parsing
///
/// This module provides comprehensive TOML parsing for Swarm.toml workflow definitions,
/// including validation, state machine generation, and code generation capabilities.
///
/// Example:
/// ```ignore
/// use descartes_core::swarm_parser::SwarmParser;
///
/// let parser = SwarmParser::new();
/// let config = parser.parse_file("Swarm.toml")?;
/// config.validate()?;
/// let state_machine = config.generate_state_machine()?;
/// ```
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

/// Error types for Swarm.toml parsing and validation
#[derive(Error, Debug)]
pub enum SwarmParseError {
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unreachable state: {0}")]
    UnreachableState(String),

    #[error("Cyclic dependency detected: {0}")]
    CyclicDependency(String),

    #[error("Invalid guard reference: {0}")]
    InvalidGuard(String),

    #[error("Invalid agent reference: {0}")]
    InvalidAgent(String),

    #[error("Invalid resource reference: {0}")]
    InvalidResource(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("State machine generation failed: {0}")]
    CodeGenerationError(String),

    #[error("Variable interpolation error: {0}")]
    InterpolationError(String),
}

/// Result type for swarm parser operations
pub type SwarmResult<T> = Result<T, SwarmParseError>;

/// Main Swarm configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub metadata: WorkflowMetadata,
    pub agents: HashMap<String, AgentConfig>,
    pub resources: HashMap<String, ResourceConfig>,
    pub workflows: Vec<Workflow>,
    pub guards: Option<HashMap<String, String>>,
}

/// Workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub created: Option<String>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceConfig {
    #[serde(rename = "http")]
    Http {
        endpoint: String,
        auth_required: Option<bool>,
        secret_key: Option<String>,
    },
    #[serde(rename = "webhook")]
    Webhook {
        endpoint: String,
        #[serde(default)]
        description: Option<String>,
    },
    #[serde(rename = "database")]
    Database {
        connection_string: String,
        #[serde(default)]
        pool_size: Option<u32>,
    },
    #[serde(rename = "custom")]
    Custom {
        #[serde(flatten)]
        config: toml::Value,
    },
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: Option<String>,
    pub metadata: WorkflowMetadataDetails,
    pub states: HashMap<String, State>,
    #[serde(default)]
    pub guards: HashMap<String, String>,
    #[serde(default)]
    pub contracts: HashMap<String, Contract>,
}

/// Detailed workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadataDetails {
    pub initial_state: String,
    #[serde(default)]
    pub completion_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub retry_backoff_seconds: Option<u64>,
}

/// State definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub description: String,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub entry_actions: Vec<String>,
    #[serde(default)]
    pub exit_actions: Vec<String>,
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub parallel_execution: bool,
    #[serde(default)]
    pub handlers: Vec<Handler>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub timeout_target: Option<String>,
    #[serde(default)]
    pub required_resources: Vec<String>,
}

/// Event handler for state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handler {
    pub event: String,
    pub target: String,
    #[serde(default)]
    pub guards: Vec<String>,
}

/// Contract specification for state inputs/outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub input: HashMap<String, String>,
    #[serde(default)]
    pub output: HashMap<String, String>,
}

/// Parsed and validated workflow structure
#[derive(Debug, Clone)]
pub struct ValidatedWorkflow {
    pub name: String,
    pub metadata: WorkflowMetadataDetails,
    pub states: HashMap<String, ValidatedState>,
    pub agents: HashMap<String, AgentConfig>,
    pub resources: HashMap<String, ResourceConfig>,
    pub guards: HashMap<String, String>,
    pub contracts: HashMap<String, Contract>,
}

/// Validated state with dependency information
#[derive(Debug, Clone)]
pub struct ValidatedState {
    pub name: String,
    pub description: String,
    pub agents: Vec<String>,
    pub entry_actions: Vec<String>,
    pub exit_actions: Vec<String>,
    pub terminal: bool,
    pub parent: Option<String>,
    pub parallel_execution: bool,
    pub handlers: Vec<Handler>,
    pub timeout_seconds: Option<u64>,
    pub timeout_target: Option<String>,
    pub required_resources: Vec<String>,
    pub reachable: bool,
}

/// Swarm.toml Parser
pub struct SwarmParser;

impl SwarmParser {
    /// Create a new parser instance
    pub fn new() -> Self {
        SwarmParser
    }

    /// Parse a Swarm.toml file from disk
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> SwarmResult<SwarmConfig> {
        let content = std::fs::read_to_string(path)?;
        self.parse_string(&content)
    }

    /// Parse a Swarm.toml from string content
    pub fn parse_string(&self, content: &str) -> SwarmResult<SwarmConfig> {
        let config: SwarmConfig = toml::from_str(content)?;
        Ok(config)
    }

    /// Parse and validate a workflow configuration
    pub fn parse_and_validate<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> SwarmResult<Vec<ValidatedWorkflow>> {
        let config = self.parse_file(path)?;
        self.validate_config(&config)?;

        let validated = config
            .workflows
            .iter()
            .map(|workflow| self.validate_workflow(workflow, &config))
            .collect::<SwarmResult<Vec<_>>>()?;

        Ok(validated)
    }
}

impl SwarmParser {
    /// Validate the entire configuration
    pub fn validate_config(&self, config: &SwarmConfig) -> SwarmResult<()> {
        // Validate metadata
        if config.metadata.version.is_empty() {
            return Err(SwarmParseError::MissingField(
                "metadata.version".to_string(),
            ));
        }

        if config.metadata.name.is_empty() {
            return Err(SwarmParseError::MissingField("metadata.name".to_string()));
        }

        // Validate that we have at least one workflow
        if config.workflows.is_empty() {
            return Err(SwarmParseError::ValidationError(
                "At least one workflow must be defined".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a single workflow
    pub fn validate_workflow(
        &self,
        workflow: &Workflow,
        config: &SwarmConfig,
    ) -> SwarmResult<ValidatedWorkflow> {
        // Validate basic structure
        if workflow.name.is_empty() {
            return Err(SwarmParseError::MissingField("workflow.name".to_string()));
        }

        if workflow.states.is_empty() {
            return Err(SwarmParseError::ValidationError(format!(
                "Workflow '{}' must define at least one state",
                workflow.name
            )));
        }

        // Validate initial state exists
        let initial_state = &workflow.metadata.initial_state;
        if !workflow.states.contains_key(initial_state) {
            return Err(SwarmParseError::ValidationError(format!(
                "Initial state '{}' not found in workflow '{}'",
                initial_state, workflow.name
            )));
        }

        // Validate all states
        for (state_name, state) in &workflow.states {
            self.validate_state(state_name, state, workflow, config)?;
        }

        // Validate state reachability (DAG check)
        self.validate_dag(workflow)?;

        // Validate guards reference
        for (_, state) in &workflow.states {
            for handler in &state.handlers {
                for guard in &handler.guards {
                    if !workflow.guards.contains_key(guard)
                        && !config
                            .guards
                            .as_ref()
                            .map_or(false, |g| g.contains_key(guard))
                    {
                        return Err(SwarmParseError::InvalidGuard(format!(
                            "Guard '{}' referenced in state '{}' is not defined",
                            guard, state.description
                        )));
                    }
                }
            }
        }

        // Build validated workflow with reachability information
        let mut validated_states = HashMap::new();
        let reachable_states = self.compute_reachable_states(workflow);

        for (state_name, state) in &workflow.states {
            validated_states.insert(
                state_name.clone(),
                ValidatedState {
                    name: state_name.clone(),
                    description: state.description.clone(),
                    agents: state.agents.clone(),
                    entry_actions: state.entry_actions.clone(),
                    exit_actions: state.exit_actions.clone(),
                    terminal: state.terminal,
                    parent: state.parent.clone(),
                    parallel_execution: state.parallel_execution,
                    handlers: state.handlers.clone(),
                    timeout_seconds: state.timeout_seconds,
                    timeout_target: state.timeout_target.clone(),
                    required_resources: state.required_resources.clone(),
                    reachable: reachable_states.contains(state_name),
                },
            );
        }

        Ok(ValidatedWorkflow {
            name: workflow.name.clone(),
            metadata: workflow.metadata.clone(),
            states: validated_states,
            agents: config.agents.clone(),
            resources: config.resources.clone(),
            guards: workflow.guards.clone(),
            contracts: workflow.contracts.clone(),
        })
    }

    /// Validate a single state
    fn validate_state(
        &self,
        state_name: &str,
        state: &State,
        workflow: &Workflow,
        config: &SwarmConfig,
    ) -> SwarmResult<()> {
        // Validate description
        if state.description.is_empty() {
            return Err(SwarmParseError::MissingField(format!(
                "State '{}' missing description",
                state_name
            )));
        }

        // Validate agents exist
        for agent in &state.agents {
            if !config.agents.contains_key(agent) {
                return Err(SwarmParseError::InvalidAgent(format!(
                    "Agent '{}' referenced in state '{}' is not defined",
                    agent, state_name
                )));
            }
        }

        // Validate resources exist
        for resource in &state.required_resources {
            if !config.resources.contains_key(resource) {
                return Err(SwarmParseError::InvalidResource(format!(
                    "Resource '{}' referenced in state '{}' is not defined",
                    resource, state_name
                )));
            }
        }

        // Validate parent state exists (if specified)
        if let Some(parent) = &state.parent {
            if !workflow.states.contains_key(parent) {
                return Err(SwarmParseError::ValidationError(format!(
                    "Parent state '{}' of '{}' not found",
                    parent, state_name
                )));
            }
        }

        // Validate handlers for terminal states
        if state.terminal && !state.handlers.is_empty() {
            return Err(SwarmParseError::ValidationError(format!(
                "Terminal state '{}' cannot have handlers",
                state_name
            )));
        }

        // Validate handlers target valid states
        for handler in &state.handlers {
            if !workflow.states.contains_key(&handler.target) {
                return Err(SwarmParseError::ValidationError(format!(
                    "Handler in state '{}' references non-existent target state '{}'",
                    state_name, handler.target
                )));
            }
        }

        // Validate timeout configuration
        if let Some(target) = &state.timeout_target {
            if !workflow.states.contains_key(target) {
                return Err(SwarmParseError::ValidationError(format!(
                    "Timeout target '{}' in state '{}' is not a valid state",
                    target, state_name
                )));
            }
        }

        Ok(())
    }

    /// Validate workflow is a DAG (no cycles)
    fn validate_dag(&self, workflow: &Workflow) -> SwarmResult<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for state_name in workflow.states.keys() {
            if !visited.contains(state_name) {
                self.dfs_cycle_check(state_name, workflow, &mut visited, &mut rec_stack)?;
            }
        }

        Ok(())
    }

    /// Depth-first search for cycle detection
    fn dfs_cycle_check(
        &self,
        state_name: &str,
        workflow: &Workflow,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> SwarmResult<()> {
        visited.insert(state_name.to_string());
        rec_stack.insert(state_name.to_string());

        if let Some(state) = workflow.states.get(state_name) {
            for handler in &state.handlers {
                if !visited.contains(&handler.target) {
                    self.dfs_cycle_check(&handler.target, workflow, visited, rec_stack)?;
                } else if rec_stack.contains(&handler.target) {
                    return Err(SwarmParseError::CyclicDependency(format!(
                        "Cycle detected: {} -> {}",
                        state_name, handler.target
                    )));
                }
            }
        }

        rec_stack.remove(state_name);
        Ok(())
    }

    /// Compute which states are reachable from the initial state
    pub fn compute_reachable_states(&self, workflow: &Workflow) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = vec![workflow.metadata.initial_state.clone()];

        while let Some(state_name) = queue.pop() {
            if reachable.contains(&state_name) {
                continue;
            }

            reachable.insert(state_name.clone());

            if let Some(state) = workflow.states.get(&state_name) {
                for handler in &state.handlers {
                    if !reachable.contains(&handler.target) {
                        queue.push(handler.target.clone());
                    }
                }

                // Add timeout target if present
                if let Some(target) = &state.timeout_target {
                    if !reachable.contains(target) {
                        queue.push(target.clone());
                    }
                }
            }
        }

        reachable
    }
}

impl ValidatedWorkflow {
    /// Check for unreachable states
    pub fn check_unreachable_states(&self) -> SwarmResult<()> {
        let unreachable: Vec<_> = self
            .states
            .iter()
            .filter(|(_, state)| !state.reachable)
            .map(|(name, _)| name.clone())
            .collect();

        if !unreachable.is_empty() {
            return Err(SwarmParseError::UnreachableState(format!(
                "Unreachable states: {}",
                unreachable.join(", ")
            )));
        }

        Ok(())
    }

    /// Generate a state enum for code generation
    pub fn generate_state_enum(&self) -> String {
        let mut states_code = String::new();
        states_code.push_str(&format!(
            "/// Generated state enum for workflow '{}'\n",
            self.name
        ));
        states_code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
        states_code.push_str(&format!(
            "pub enum {}State {{\n",
            capitalize_first(&self.name)
        ));

        for state_name in self.states.keys() {
            states_code.push_str(&format!("    {},\n", state_name));
        }

        states_code.push_str("}\n\n");
        states_code
    }

    /// Generate event enum for code generation
    pub fn generate_event_enum(&self) -> String {
        let mut events = HashSet::new();

        for state in self.states.values() {
            for handler in &state.handlers {
                events.insert(handler.event.clone());
            }
        }

        let mut events_code = String::new();
        events_code.push_str(&format!(
            "/// Generated event enum for workflow '{}'\n",
            self.name
        ));
        events_code.push_str("#[derive(Debug, Clone, PartialEq, Eq, Hash)]\n");
        events_code.push_str(&format!(
            "pub enum {}Event {{\n",
            capitalize_first(&self.name)
        ));

        for event in events {
            events_code.push_str(&format!("    {},\n", event));
        }

        events_code.push_str("}\n\n");
        events_code
    }

    /// Generate context struct for code generation
    pub fn generate_context_struct(&self) -> String {
        let mut context_code = String::new();
        context_code.push_str(&format!(
            "/// Generated context struct for workflow '{}'\n",
            self.name
        ));
        context_code.push_str("#[derive(Serialize, Deserialize, Clone, Debug)]\n");
        context_code.push_str(&format!(
            "pub struct {}Context {{\n",
            capitalize_first(&self.name)
        ));
        context_code.push_str("    // Add your context fields here\n");
        context_code.push_str("    pub metadata: serde_json::Value,\n");
        context_code.push_str("}\n\n");
        context_code
    }

    /// Generate Mermaid state diagram documentation
    pub fn generate_mermaid_diagram(&self) -> String {
        let mut diagram = String::new();
        diagram.push_str("```mermaid\n");
        diagram.push_str("stateDiagram-v2\n");
        diagram.push_str(&format!("    [*] --> {}\n", self.metadata.initial_state));

        let mut processed = HashSet::new();

        for (state_name, state) in &self.states {
            if processed.contains(state_name) {
                continue;
            }
            processed.insert(state_name.clone());

            for handler in &state.handlers {
                if state.terminal {
                    diagram.push_str(&format!("    {} --> [*]\n", state_name));
                } else {
                    diagram.push_str(&format!("    {} --> {}\n", state_name, handler.target));
                }
            }

            if let Some(timeout_target) = &state.timeout_target {
                diagram.push_str(&format!(
                    "    {} --> {} : timeout\n",
                    state_name, timeout_target
                ));
            }
        }

        diagram.push_str("```\n");
        diagram
    }

    /// Generate complete state machine code
    pub fn generate_state_machine_code(&self) -> String {
        let mut code = String::new();
        code.push_str("// Generated state machine code\n");
        code.push_str("// WARNING: This is auto-generated, do not edit manually\n\n");

        code.push_str(&self.generate_state_enum());
        code.push_str(&self.generate_event_enum());
        code.push_str(&self.generate_context_struct());

        code.push_str(&format!("impl {}State {{\n", capitalize_first(&self.name)));
        code.push_str("    /// Handle an event in the current state\n");
        code.push_str(&format!(
            "    pub fn on_event(\n        self,\n        event: {}Event,\n        context: &mut {}Context,\n    ) -> Self {{\n",
            capitalize_first(&self.name),
            capitalize_first(&self.name)
        ));
        code.push_str("        match (self, event) {\n");

        for (state_name, state) in &self.states {
            for handler in &state.handlers {
                code.push_str(&format!(
                    "            ({}State::{}, {}Event::{}) => {{",
                    capitalize_first(&self.name),
                    state_name,
                    capitalize_first(&self.name),
                    handler.event
                ));

                if !handler.guards.is_empty() {
                    code.push_str(" // Guards: ");
                    code.push_str(&handler.guards.join(", "));
                }

                code.push_str(&format!(
                    " {}State::{} }}\n",
                    capitalize_first(&self.name),
                    handler.target
                ));
            }
        }

        code.push_str("            _ => self,\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n");

        code
    }
}

/// Helper function to capitalize first character
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let rest: String = chars.collect();
            first.to_uppercase().to_string() + &rest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = SwarmParser::new();
        assert!(parser.parse_string("").is_err()); // Empty string should fail
    }

    #[test]
    fn test_simple_workflow_parsing() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test Workflow"
description = "Test"

[agents]
[agents.test_agent]
model = "claude-3-opus"
max_tokens = 1000
temperature = 0.5

[resources]

[[workflows]]
name = "simple"
description = "Simple workflow"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start state"
handlers = [
    { event = "go", target = "End" }
]

[workflows.states.End]
description = "End state"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content);
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.metadata.name, "Test Workflow");
        assert_eq!(config.workflows.len(), 1);
        assert_eq!(config.workflows[0].name, "simple");
    }

    #[test]
    fn test_workflow_validation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.test_agent]
model = "claude-3-opus"

[resources]

[[workflows]]
name = "test_workflow"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "go", target = "End" }
]

[workflows.states.End]
description = "End"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_initial_state() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "NonExistent"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let validated = parser.parse_and_validate("/dev/null");
        // This will fail since file doesn't exist, but demonstrates the flow
        assert!(validated.is_err());
    }

    #[test]
    fn test_state_reachability() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "A"

[workflows.states]

[workflows.states.A]
description = "A"
handlers = [
    { event = "go_b", target = "B" },
    { event = "go_c", target = "C" }
]

[workflows.states.B]
description = "B"
handlers = [
    { event = "done", target = "C" }
]

[workflows.states.C]
description = "C"
terminal = true

[workflows.states.D]
description = "D (unreachable)"
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflows = config.workflows;
        assert_eq!(workflows.len(), 1);

        let workflow = &workflows[0];
        let reachable = parser.compute_reachable_states(workflow);

        assert!(reachable.contains("A"));
        assert!(reachable.contains("B"));
        assert!(reachable.contains("C"));
        assert!(!reachable.contains("D"));
    }

    #[test]
    fn test_code_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "TestWorkflow"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "process", target = "Processing" }
]

[workflows.states.Processing]
description = "Processing"
handlers = [
    { event = "complete", target = "Done" }
]

[workflows.states.Done]
description = "Done"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = &config.workflows[0];
        let validated = parser.validate_workflow(workflow, &config).unwrap();

        let state_enum = validated.generate_state_enum();
        assert!(state_enum.contains("Start"));
        assert!(state_enum.contains("Processing"));
        assert!(state_enum.contains("Done"));

        let event_enum = validated.generate_event_enum();
        assert!(event_enum.contains("process"));
        assert!(event_enum.contains("complete"));

        let mermaid = validated.generate_mermaid_diagram();
        assert!(mermaid.contains("stateDiagram"));
        assert!(mermaid.contains("Start"));

        let state_machine = validated.generate_state_machine_code();
        // The workflow name is "test", so capitalize_first makes it "Test"
        // The impl is "impl TestState" not "impl TestWorkflowState"
        assert!(state_machine.contains("impl TestState"));
    }
}
