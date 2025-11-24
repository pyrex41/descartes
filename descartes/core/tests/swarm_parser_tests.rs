//! Comprehensive test suite for Swarm.toml parser
//!
//! Tests cover:
//! - TOML parsing and deserialization
//! - Workflow validation
//! - State reachability analysis
//! - Cycle detection
//! - Code generation
//! - Error handling

#[cfg(test)]
mod tests {
    use descartes_core::swarm_parser::*;

    // ============================================================================
    // Test 1: Parser Creation and Basic Parsing
    // ============================================================================

    #[test]
    fn test_parser_creation() {
        let parser = SwarmParser::new();
        // Parser should be created successfully
        assert!(parser.parse_string("").is_err());
    }

    #[test]
    fn test_empty_string_parsing() {
        let parser = SwarmParser::new();
        let result = parser.parse_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml() {
        let parser = SwarmParser::new();
        let invalid_toml = "invalid [ toml ]{";
        let result = parser.parse_string(invalid_toml);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 2: Simple Workflow Parsing
    // ============================================================================

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

    // ============================================================================
    // Test 3: Metadata Validation
    // ============================================================================

    #[test]
    fn test_missing_metadata_version() {
        let toml_content = r#"
[metadata]
name = "Test"
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
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_metadata_name() {
        let toml_content = r#"
[metadata]
version = "1.0"
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
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_config(&config);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 4: Workflow Validation
    // ============================================================================

    #[test]
    fn test_valid_workflow() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.test]
model = "claude-3-opus"

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
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 5: Agent Reference Validation
    // ============================================================================

    #[test]
    fn test_invalid_agent_reference() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.valid_agent]
model = "claude-3-opus"

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
agents = ["invalid_agent"]
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_agent_reference() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.my_agent]
model = "claude-3-opus"

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
agents = ["my_agent"]
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Test 6: Handler Target Validation
    // ============================================================================

    #[test]
    fn test_invalid_handler_target() {
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
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "go", target = "NonExistent" }
]
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 7: State Reachability Analysis
    // ============================================================================

    #[test]
    fn test_state_reachability_all_reachable() {
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
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = &config.workflows[0];
        let reachable = parser.compute_reachable_states(workflow);

        assert!(reachable.contains("A"));
        assert!(reachable.contains("B"));
        assert!(reachable.contains("C"));
    }

    #[test]
    fn test_state_reachability_unreachable() {
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
    { event = "go_b", target = "B" }
]

[workflows.states.B]
description = "B"
handlers = []

[workflows.states.C]
description = "C (unreachable)"
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = &config.workflows[0];
        let reachable = parser.compute_reachable_states(workflow);

        assert!(reachable.contains("A"));
        assert!(reachable.contains("B"));
        assert!(!reachable.contains("C"));
    }

    #[test]
    fn test_timeout_makes_target_reachable() {
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
timeout_seconds = 100
timeout_target = "Timeout"
handlers = [
    { event = "go", target = "B" }
]

[workflows.states.B]
description = "B"
handlers = []

[workflows.states.Timeout]
description = "Timeout state"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = &config.workflows[0];
        let reachable = parser.compute_reachable_states(workflow);

        assert!(reachable.contains("Timeout"));
    }

    // ============================================================================
    // Test 8: Terminal State Validation
    // ============================================================================

    #[test]
    fn test_terminal_state_with_handlers_invalid() {
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
handlers = [
    { event = "retry", target = "Start" }
]
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 9: Code Generation - State Enum
    // ============================================================================

    #[test]
    fn test_state_enum_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "myworkflow"
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
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        let enum_code = workflow.generate_state_enum();
        assert!(enum_code.contains("Start"));
        assert!(enum_code.contains("End"));
        assert!(enum_code.contains("pub enum"));
    }

    // ============================================================================
    // Test 10: Code Generation - Event Enum
    // ============================================================================

    #[test]
    fn test_event_enum_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "myworkflow"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "process", target = "Processing" },
    { event = "skip", target = "End" }
]

[workflows.states.Processing]
description = "Processing"
handlers = [
    { event = "complete", target = "End" }
]

[workflows.states.End]
description = "End"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        let event_enum = workflow.generate_event_enum();
        assert!(event_enum.contains("process"));
        assert!(event_enum.contains("complete"));
        assert!(event_enum.contains("skip"));
        assert!(event_enum.contains("pub enum"));
    }

    // ============================================================================
    // Test 11: Code Generation - Context Struct
    // ============================================================================

    #[test]
    fn test_context_struct_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "myworkflow"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        let context = workflow.generate_context_struct();
        assert!(context.contains("pub struct"));
        assert!(context.contains("Context"));
        assert!(context.contains("Serialize"));
        assert!(context.contains("metadata"));
    }

    // ============================================================================
    // Test 12: Code Generation - State Machine
    // ============================================================================

    #[test]
    fn test_state_machine_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "myworkflow"
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
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        let state_machine = workflow.generate_state_machine_code();
        assert!(state_machine.contains("impl"));
        assert!(state_machine.contains("on_event"));
        assert!(state_machine.contains("match (self, event)"));
    }

    // ============================================================================
    // Test 13: Code Generation - Mermaid Diagram
    // ============================================================================

    #[test]
    fn test_mermaid_diagram_generation() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[[workflows]]
name = "myworkflow"
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
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        let diagram = workflow.generate_mermaid_diagram();
        assert!(diagram.contains("mermaid"));
        assert!(diagram.contains("stateDiagram"));
        assert!(diagram.contains("Start"));
        assert!(diagram.contains("End"));
    }

    // ============================================================================
    // Test 14: Multiple Agents
    // ============================================================================

    #[test]
    fn test_multiple_agents() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.agent1]
model = "claude-3-opus"

[agents.agent2]
model = "claude-3-sonnet"

[agents.agent3]
model = "claude-3-haiku"

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
agents = ["agent1", "agent2", "agent3"]
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.states["Start"].agents.len(), 3);
    }

    // ============================================================================
    // Test 15: Multiple Resources
    // ============================================================================

    #[test]
    fn test_multiple_resources() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]

[resources]

[resources.github]
type = "http"
endpoint = "https://api.github.com"

[resources.slack]
type = "webhook"
endpoint = "https://hooks.slack.com/services/..."

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
required_resources = ["github", "slack"]
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Test 16: Entry and Exit Actions
    // ============================================================================

    #[test]
    fn test_entry_exit_actions() {
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
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
entry_actions = ["init", "setup"]
exit_actions = ["cleanup", "log"]
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        assert_eq!(workflow.states["Start"].entry_actions.len(), 2);
        assert_eq!(workflow.states["Start"].exit_actions.len(), 2);
    }

    // ============================================================================
    // Test 17: Guard Conditions
    // ============================================================================

    #[test]
    fn test_guard_conditions() {
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
initial_state = "Start"

[workflows.guards]
condition1 = "context.value > 10"
condition2 = "context.approved == true"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "go", target = "End", guards = ["condition1", "condition2"] }
]

[workflows.states.End]
description = "End"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Test 18: Invalid Guard Reference
    // ============================================================================

    #[test]
    fn test_invalid_guard_reference() {
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
initial_state = "Start"

[workflows.guards]
condition1 = "context.value > 10"

[workflows.states]

[workflows.states.Start]
description = "Start"
handlers = [
    { event = "go", target = "End", guards = ["unknown_condition"] }
]

[workflows.states.End]
description = "End"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let result = parser.validate_workflow(&config.workflows[0], &config);
        assert!(result.is_err());
    }

    // ============================================================================
    // Test 19: Parallel Execution Flag
    // ============================================================================

    #[test]
    fn test_parallel_execution() {
        let toml_content = r#"
[metadata]
version = "1.0"
name = "Test"
description = "Test"

[agents]
[agents.a1]
model = "claude-3-opus"

[agents.a2]
model = "claude-3-opus"

[resources]

[[workflows]]
name = "test"
description = "Test"

[workflows.metadata]
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
agents = ["a1", "a2"]
parallel_execution = true
handlers = []
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        assert!(workflow.states["Start"].parallel_execution);
    }

    // ============================================================================
    // Test 20: Timeout Handling
    // ============================================================================

    #[test]
    fn test_timeout_configuration() {
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
initial_state = "Start"

[workflows.states]

[workflows.states.Start]
description = "Start"
timeout_seconds = 3600
timeout_target = "Timeout"
handlers = []

[workflows.states.Timeout]
description = "Timeout state"
terminal = true
"#;

        let parser = SwarmParser::new();
        let config = parser.parse_string(toml_content).unwrap();
        let workflow = parser.validate_workflow(&config.workflows[0], &config).unwrap();

        assert_eq!(workflow.states["Start"].timeout_seconds, Some(3600));
        assert_eq!(workflow.states["Start"].timeout_target, Some("Timeout".to_string()));
    }
}
