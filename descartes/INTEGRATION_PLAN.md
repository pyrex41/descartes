# State Machine Integration Plan

**Task ID**: phase2:7.1
**Status**: Ready for Implementation
**Timeline**: Weeks 1-4 of Phase 2

---

## Quick Reference

- **Selected Library**: Statig (with rust-fsm as alternative)
- **Swarm.toml**: Declarative workflow definitions
- **Output**: Generated Rust state machines + Mermaid diagrams
- **Deployment**: Integrated with AgentRunner and StateStore
- **Testing**: Full test coverage with examples

---

## Phase 2A: Foundation (Week 1-2)

### A.1 Dependency Addition

**File**: `/descartes/Cargo.toml`

```toml
[workspace.dependencies]
# Existing deps...

# State Machine Engine
statig = "0.3"

# TOML parsing
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Code generation (for Mermaid export)
mermaid = "0.1"
```

**File**: `/descartes/core/Cargo.toml`

```toml
[dependencies]
# Existing...
statig = { workspace = true }
toml = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
```

### A.2 Core Module Structure

**Create**: `/descartes/core/src/state_machine.rs`

```rust
//! State Machine Engine Module
//!
//! Provides abstraction over Statig for Descartes workflows.
//! - SwarmConfig: Parse and validate Swarm.toml
//! - WorkflowEngine: Execute state machines
//! - WorkflowState: Generic state machine interface

pub mod config;
pub mod engine;
pub mod executor;
pub mod types;
pub mod validator;
pub mod codegen;

pub use config::{SwarmConfig, WorkflowDefinition};
pub use engine::{WorkflowEngine, WorkflowInstance};
pub use types::{WorkflowEvent, WorkflowState, WorkflowContext};
pub use validator::WorkflowValidator;
pub use codegen::CodeGenerator;
```

### A.3 File Structure

Create the following module files:

1. **`config.rs`** - TOML parsing and data structures
2. **`types.rs`** - Core types (State, Event, Context)
3. **`validator.rs`** - Workflow validation (DAG, reachability)
4. **`engine.rs`** - Workflow execution engine
5. **`executor.rs`** - Agent execution integration
6. **`codegen.rs`** - Rust code generation from Swarm.toml
7. **`diagram.rs`** - Mermaid diagram generation

### A.4 Type Definitions

**`types.rs`** - Define core abstractions:

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Generic workflow state
pub trait WorkflowState: Clone + Debug + Send + Sync {
    fn state_name(&self) -> &'static str;
    fn is_terminal(&self) -> bool;
}

/// Generic workflow event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Workflow context (state data)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub task_id: String,
    pub workflow_name: String,
    pub state_variant: String,
    pub data: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow execution result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub success: bool,
    pub final_state: String,
    pub error: Option<String>,
    pub output: serde_json::Value,
}
```

### A.5 TOML Schema Implementation

**`config.rs`** - Parse Swarm.toml:

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    pub metadata: Metadata,
    pub agents: HashMap<String, AgentDefinition>,
    pub resources: HashMap<String, ResourceDefinition>,
    pub workflows: Vec<WorkflowDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub r#type: String,
    pub endpoint: Option<String>,
    pub auth_required: Option<bool>,
    pub secret_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub description: String,
    pub metadata: WorkflowMetadata,
    pub states: HashMap<String, StateDefinition>,
    pub guards: Option<HashMap<String, String>>,
    pub contracts: Option<HashMap<String, ContractDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub initial_state: String,
    pub completion_timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
    pub retry_backoff_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDefinition {
    pub description: String,
    pub entry_actions: Option<Vec<String>>,
    pub exit_actions: Option<Vec<String>>,
    pub handlers: Option<Vec<HandlerDefinition>>,
    pub agents: Option<Vec<String>>,
    pub parent: Option<String>,
    pub terminal: Option<bool>,
    pub timeout_seconds: Option<u64>,
    pub timeout_target: Option<String>,
    pub parallel_execution: Option<bool>,
    pub required_resources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerDefinition {
    pub event: String,
    pub target: String,
    pub guards: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDefinition {
    pub name: String,
    pub input: Option<HashMap<String, String>>,
    pub output: HashMap<String, String>,
}

impl SwarmConfig {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: SwarmConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn load_from_string(toml_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: SwarmConfig = toml::from_str(toml_str)?;
        Ok(config)
    }
}
```

### A.6 Update Library Root

**File**: `/descartes/core/src/lib.rs`

```rust
pub mod state_machine;

// Re-export common types
pub use state_machine::{
    SwarmConfig, WorkflowEngine, WorkflowInstance,
    WorkflowEvent, WorkflowContext, WorkflowResult,
};
```

### A.7 Initial Tests

**File**: `/descartes/core/tests/state_machine_basic.rs`

```rust
#[test]
fn test_load_swarm_toml_basic() {
    let toml_content = r#"
[metadata]
version = "1.0"
name = "Test Workflow"

[agents.test_agent]
model = "test"
max_tokens = 1000
temperature = 0.5

[[workflows]]
name = "simple"

[workflows.metadata]
initial_state = "Start"

[workflows.states.Start]
description = "Starting state"
terminal = true
"#;

    let config = SwarmConfig::load_from_string(toml_content).unwrap();
    assert_eq!(config.metadata.name, "Test Workflow");
    assert_eq!(config.workflows.len(), 1);
    assert_eq!(config.workflows[0].metadata.initial_state, "Start");
}

#[test]
fn test_workflow_validation() {
    // Load valid workflow
    // Validate it passes checks
    // Assert no errors
}
```

---

## Phase 2B: Validation & Code Generation (Week 3)

### B.1 Workflow Validator

**File**: `/descartes/core/src/state_machine/validator.rs`

```rust
pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validate workflow definition
    pub fn validate(workflow: &WorkflowDefinition) -> Result<(), Vec<ValidationError>> {
        let mut errors = vec![];

        // Check 1: Initial state exists
        if !workflow.states.contains_key(&workflow.metadata.initial_state) {
            errors.push(ValidationError::InitialStateNotFound(
                workflow.metadata.initial_state.clone()
            ));
        }

        // Check 2: All referenced states exist
        for state in workflow.states.values() {
            if let Some(parent) = &state.parent {
                if !workflow.states.contains_key(parent) {
                    errors.push(ValidationError::ParentStateNotFound(parent.clone()));
                }
            }

            if let Some(handlers) = &state.handlers {
                for handler in handlers {
                    if !workflow.states.contains_key(&handler.target) {
                        errors.push(ValidationError::TargetStateNotFound(
                            handler.target.clone()
                        ));
                    }
                }
            }
        }

        // Check 3: No cycles (DAG validation)
        if has_cycles(workflow) {
            errors.push(ValidationError::CycleDetected);
        }

        // Check 4: All states are reachable
        if let Some(unreachable) = find_unreachable_states(workflow) {
            for state in unreachable {
                errors.push(ValidationError::UnreachableState(state));
            }
        }

        // Check 5: All agents exist
        // Check 6: All resources exist
        // Check 7: All guards exist

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    InitialStateNotFound(String),
    ParentStateNotFound(String),
    TargetStateNotFound(String),
    CycleDetected,
    UnreachableState(String),
    InvalidAgent(String),
    InvalidResource(String),
    InvalidGuard(String),
}
```

### B.2 Code Generator

**File**: `/descartes/core/src/state_machine/codegen.rs`

```rust
pub struct CodeGenerator;

impl CodeGenerator {
    /// Generate Rust state machine from Swarm.toml workflow
    pub fn generate_state_machine(
        workflow: &WorkflowDefinition,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        // Generate state enum
        code.push_str(&Self::generate_state_enum(workflow)?);
        code.push('\n');

        // Generate context struct
        code.push_str(&Self::generate_context_struct(workflow)?);
        code.push('\n');

        // Generate event enum
        code.push_str(&Self::generate_event_enum(workflow)?);
        code.push('\n');

        // Generate handlers
        code.push_str(&Self::generate_handlers(workflow)?);

        Ok(code)
    }

    fn generate_state_enum(workflow: &WorkflowDefinition) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::from("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum State {\n");

        for (state_name, _state_def) in &workflow.states {
            code.push_str(&format!("    {},\n", to_rust_identifier(state_name)));
        }

        code.push_str("}\n");
        Ok(code)
    }

    fn generate_context_struct(workflow: &WorkflowDefinition) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::from(
            "#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]\npub struct Context {\n"
        );

        code.push_str("    pub task_id: String,\n");
        code.push_str("    pub workflow_name: String,\n");
        code.push_str("    pub created_at: chrono::DateTime<chrono::Utc>,\n");

        code.push_str("}\n");
        Ok(code)
    }

    fn generate_event_enum(workflow: &WorkflowDefinition) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::from("#[derive(Debug, Clone)]\npub enum Event {\n");

        let mut events = std::collections::HashSet::new();
        for state in workflow.states.values() {
            if let Some(handlers) = &state.handlers {
                for handler in handlers {
                    events.insert(&handler.event);
                }
            }
        }

        for event in events {
            code.push_str(&format!("    {},\n", to_rust_identifier(event)));
        }

        code.push_str("}\n");
        Ok(code)
    }

    fn generate_handlers(_workflow: &WorkflowDefinition) -> Result<String, Box<dyn std::error::Error>> {
        // Generate state transition logic
        Ok("// Handler implementation\n".to_string())
    }
}

fn to_rust_identifier(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
```

### B.3 Mermaid Diagram Generator

**File**: `/descartes/core/src/state_machine/diagram.rs`

```rust
pub struct DiagramGenerator;

impl DiagramGenerator {
    /// Generate Mermaid state diagram from workflow
    pub fn generate_mermaid(
        workflow: &WorkflowDefinition,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut diagram = String::from("stateDiagram-v2\n");

        // Add initial state
        diagram.push_str(&format!("    [*] --> {}\n", workflow.metadata.initial_state));

        // Add state transitions
        for (state_name, state_def) in &workflow.states {
            if let Some(handlers) = &state_def.handlers {
                for handler in handlers {
                    diagram.push_str(&format!(
                        "    {} --> {} : {}\n",
                        state_name, handler.target, handler.event
                    ));
                }
            }

            // Add terminal state arrows
            if state_def.terminal.unwrap_or(false) {
                diagram.push_str(&format!("    {} --> [*]\n", state_name));
            }
        }

        Ok(diagram)
    }
}
```

### B.4 Integration Tests

**File**: `/descartes/core/tests/state_machine_codegen.rs`

```rust
#[test]
fn test_code_generation_produces_valid_rust() {
    let config = load_test_workflow();
    let code = CodeGenerator::generate_state_machine(&config.workflows[0]).unwrap();

    // Verify generated code contains expected patterns
    assert!(code.contains("pub enum State"));
    assert!(code.contains("pub struct Context"));
    assert!(code.contains("pub enum Event"));
}

#[test]
fn test_mermaid_generation() {
    let config = load_test_workflow();
    let diagram = DiagramGenerator::generate_mermaid(&config.workflows[0]).unwrap();

    assert!(diagram.contains("stateDiagram-v2"));
    assert!(diagram.contains("[*] -->"));
}
```

---

## Phase 2C: Execution & Persistence (Week 4)

### C.1 Workflow Engine

**File**: `/descartes/core/src/state_machine/engine.rs`

```rust
use crate::{StateStore, AgentRunner};

pub struct WorkflowEngine {
    config: SwarmConfig,
    state_store: Arc<dyn StateStore>,
    agent_runner: Arc<dyn AgentRunner>,
}

pub struct WorkflowInstance {
    task_id: String,
    workflow_name: String,
    current_state: String,
    context: WorkflowContext,
}

impl WorkflowEngine {
    pub async fn new(
        config: SwarmConfig,
        state_store: Arc<dyn StateStore>,
        agent_runner: Arc<dyn AgentRunner>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Validate config
        WorkflowValidator::validate_all(&config)?;

        Ok(Self {
            config,
            state_store,
            agent_runner,
        })
    }

    /// Execute a workflow
    pub async fn execute(
        &self,
        workflow_name: &str,
        task_id: &str,
        initial_data: serde_json::Value,
    ) -> Result<WorkflowResult, Box<dyn std::error::Error>> {
        let workflow = self.config.get_workflow(workflow_name)?;

        // Create workflow instance
        let mut instance = WorkflowInstance {
            task_id: task_id.to_string(),
            workflow_name: workflow_name.to_string(),
            current_state: workflow.metadata.initial_state.clone(),
            context: WorkflowContext {
                task_id: task_id.to_string(),
                workflow_name: workflow_name.to_string(),
                state_variant: workflow.metadata.initial_state.clone(),
                data: serde_json::from_value(initial_data)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        };

        // Persist initial state
        self.state_store.save_workflow_state(&instance.context).await?;

        // Execute state machine loop
        loop {
            let state_def = workflow.states.get(&instance.current_state)?;

            // Run entry actions
            if let Some(entry_actions) = &state_def.entry_actions {
                for action in entry_actions {
                    self.execute_action(&instance, action).await?;
                }
            }

            // Check if terminal
            if state_def.terminal.unwrap_or(false) {
                return Ok(WorkflowResult {
                    success: true,
                    final_state: instance.current_state,
                    error: None,
                    output: serde_json::to_value(&instance.context.data)?,
                });
            }

            // Wait for event (timeout or agent action)
            let event = self.wait_for_event(&instance).await?;

            // Transition
            let next_state = self.transition(&instance, event).await?;
            instance.current_state = next_state;
            instance.context.updated_at = chrono::Utc::now();

            // Persist state
            self.state_store.save_workflow_state(&instance.context).await?;
        }
    }

    async fn execute_action(
        &self,
        instance: &WorkflowInstance,
        action: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Execute action (run agent, call API, etc.)
        println!("[{}] Executing action: {}", instance.task_id, action);
        Ok(())
    }

    async fn wait_for_event(
        &self,
        _instance: &WorkflowInstance,
    ) -> Result<WorkflowEvent, Box<dyn std::error::Error>> {
        // Wait for event from agents or timeout
        Ok(WorkflowEvent {
            event_type: "test".to_string(),
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        })
    }

    async fn transition(
        &self,
        instance: &WorkflowInstance,
        event: WorkflowEvent,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let workflow = self.config.get_workflow(&instance.workflow_name)?;
        let state_def = workflow.states.get(&instance.current_state)?;

        // Find handler for event
        if let Some(handlers) = &state_def.handlers {
            for handler in handlers {
                if handler.event == event.event_type {
                    return Ok(handler.target.clone());
                }
            }
        }

        Err("No handler for event".into())
    }
}
```

### C.2 State Persistence

Update `StateStore` trait in existing code:

```rust
pub trait StateStore: Send + Sync {
    // Existing methods...

    async fn save_workflow_state(
        &self,
        context: &WorkflowContext,
    ) -> StateStoreResult<()>;

    async fn load_workflow_state(
        &self,
        task_id: &str,
    ) -> StateStoreResult<WorkflowContext>;

    async fn delete_workflow_state(
        &self,
        task_id: &str,
    ) -> StateStoreResult<()>;
}
```

### C.3 Integration with AgentRunner

Update `AgentRunner` to support workflow execution:

```rust
pub trait AgentRunner: Send + Sync {
    // Existing methods...

    async fn execute_workflow_action(
        &self,
        task_id: &str,
        action: &str,
        context: &WorkflowContext,
    ) -> AgentResult<serde_json::Value>;
}
```

### C.4 Comprehensive Tests

**File**: `/descartes/core/tests/state_machine_execution.rs`

```rust
#[tokio::test]
async fn test_workflow_execution() {
    // Load test workflow
    // Execute workflow
    // Verify final state
    // Assert output
}

#[tokio::test]
async fn test_workflow_persistence() {
    // Start workflow
    // Save state
    // Load workflow
    // Verify state matches
}

#[tokio::test]
async fn test_concurrent_workflows() {
    // Start multiple workflows
    // Execute concurrently
    // Verify independence
}
```

---

## Implementation Checklist

### Week 1 - Foundation
- [ ] Add Statig and dependencies to Cargo.toml
- [ ] Create state_machine module structure
- [ ] Implement type definitions (State, Event, Context)
- [ ] Implement SwarmConfig and TOML parsing
- [ ] Create basic tests for parsing
- [ ] Document API

### Week 2 - Parser & Validation
- [ ] Implement WorkflowValidator
  - [ ] Cycle detection
  - [ ] Reachability analysis
  - [ ] Reference validation
- [ ] Add comprehensive validation tests
- [ ] Create integration tests

### Week 3 - Code Generation & Visualization
- [ ] Implement CodeGenerator
  - [ ] State enum generation
  - [ ] Context struct generation
  - [ ] Event enum generation
  - [ ] Handler logic generation
- [ ] Implement DiagramGenerator (Mermaid)
- [ ] Add CLI command for code/diagram export
- [ ] Test generated code compiles

### Week 4 - Execution & Persistence
- [ ] Implement WorkflowEngine
- [ ] Integrate with StateStore
- [ ] Integrate with AgentRunner
- [ ] Implement state persistence
- [ ] Implement workflow resumption
- [ ] Add E2E tests
- [ ] Performance benchmarking

---

## Testing Strategy

### Unit Tests
- TOML parsing and validation
- State machine logic
- Guard evaluation
- Action execution

### Integration Tests
- Workflow execution with mocked agents
- State persistence and resumption
- Multiple concurrent workflows
- Agent communication

### E2E Tests
- Full workflow execution with real agents
- Multi-agent scenarios
- Error handling and recovery

### Performance Tests
- Workflow startup time
- State transition latency
- Memory usage
- Concurrent workflow scaling

---

## Success Criteria

### Phase 2A (Foundation)
- ✅ Statig integrated into workspace
- ✅ Swarm.toml parser fully functional
- ✅ Type system defined and tested
- ✅ 100% API coverage with unit tests

### Phase 2B (Validation & Codegen)
- ✅ Workflow validator catches all error classes
- ✅ Code generator produces compilable Rust
- ✅ Mermaid diagram generation working
- ✅ Example workflows validated and generated

### Phase 2C (Execution & Persistence)
- ✅ Workflows execute end-to-end
- ✅ State persists and resumes correctly
- ✅ Multiple workflows run concurrently
- ✅ Integration with existing Descartes components

---

## Known Limitations & Workarounds

### Limitation: Statig has no built-in serialization
**Workaround**: Serialize context separately, store state as string, reconstruct state from context on resume.

### Limitation: Statig has no built-in Mermaid export
**Workaround**: Generate diagrams from SwarmConfig, not from compiled state machine.

### Limitation: No compile-time deadlock detection
**Workaround**: Implement cycle detection in SwarmConfig validator.

### Limitation: State transitions are synchronous in base Statig
**Workaround**: Use `async` feature to make handlers async.

---

## Dependencies Added

| Crate | Version | Why | Impact |
|-------|---------|-----|--------|
| statig | 0.3 | State machine framework | New capability |
| toml | 0.8 | TOML parsing | New format support |
| serde_json | 1.0 | JSON context serialization | Medium-weight dependency |
| chrono | 0.4 | Timestamps | Already in ecosystem |

**Total size impact**: ~200KB binary size increase

---

## Future Enhancements

1. **Hot Reload** - Update workflows without restart
2. **Workflow Composition** - Nest workflows within states
3. **Distributed Execution** - Run workflows across multiple nodes
4. **Time Travel** - Replay workflows from saved checkpoints
5. **Visual Editor** - GUI for creating Swarm.toml files
6. **Machine Learning** - Learn optimal agent assignments

---

## References

- **Statig GitHub**: https://github.com/mdeloof/statig
- **TOML Spec**: https://toml.io/en/
- **Descartes Master Plan**: `/planning/Descartes_Master_Plan.md`
- **State Machine Evaluation**: `/STATE_MACHINE_EVALUATION.md`
- **Swarm.toml Schema**: `/SWARM_TOML_SCHEMA.md`

---

**Document Status**: Ready for implementation
**Next Step**: Begin Phase 2A with Statig integration
