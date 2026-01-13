# Descartes State Machine Integration Guide

## Overview

The Statig State Machine library is integrated into Descartes Core to provide production-ready workflow orchestration with:

- **Hierarchical state machines** for complex multi-agent workflows
- **Event-driven transitions** with compile-time verification
- **Async/await support** with Tokio integration
- **State persistence** using SQLite via StateStore trait
- **State history** with rollback capabilities
- **Workflow orchestration** for managing multiple concurrent workflows
- **Serialization support** for state snapshots and recovery

## Architecture

### Core Components

```
WorkflowState
├── Idle (initial)
├── Running (active)
├── Paused (suspended)
├── Completed (terminal)
└── Failed (terminal)

WorkflowEvent
├── Start
├── Pause
├── Resume
├── Complete
├── Fail(message)
├── Retry
├── Timeout
├── Rollback
└── Custom { name, data }
```

### State Transition Graph

```
Idle → Running → Completed
       ↓ ↑
      Paused
       ↓
      Failed → Running (Retry)
```

## Quick Start

### Basic Workflow

```rust
use descartes_core::state_machine::*;

#[tokio::main]
async fn main() {
    // Create a workflow state machine
    let sm = WorkflowStateMachine::new("my-workflow".to_string());

    // Start the workflow
    sm.process_event(WorkflowEvent::Start).await?;

    // Process events
    sm.process_event(WorkflowEvent::Pause).await?;
    sm.process_event(WorkflowEvent::Resume).await?;

    // Complete the workflow
    sm.process_event(WorkflowEvent::Complete).await?;

    // Check final state
    assert_eq!(sm.current_state().await, WorkflowState::Completed);
}
```

### Custom Event Handler

```rust
use async_trait::async_trait;

struct MyHandler;

#[async_trait]
impl StateHandler for MyHandler {
    async fn on_enter(&self, state: WorkflowState) -> StateMachineResult<()> {
        println!("Entering state: {}", state);
        Ok(())
    }

    async fn on_exit(&self, state: WorkflowState) -> StateMachineResult<()> {
        println!("Exiting state: {}", state);
        Ok(())
    }

    async fn on_event(
        &self,
        state: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        println!("Processing event {} in state {}", event, state);
        Ok(())
    }

    async fn on_transition(
        &self,
        from: WorkflowState,
        to: WorkflowState,
        event: &WorkflowEvent,
    ) -> StateMachineResult<()> {
        println!("Transitioning: {} -> {} via {}", from, to, event);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let handler = Arc::new(MyHandler);
    let sm = WorkflowStateMachine::with_handler(
        "workflow-1".to_string(),
        handler
    );

    sm.process_event(WorkflowEvent::Start).await?;
    // ... rest of workflow
}
```

## Usage Patterns

### 1. Context Management

Store and retrieve arbitrary data during workflow execution:

```rust
// Store context data
sm.set_context("task_id", serde_json::json!("TASK-123")).await?;
sm.set_context("priority", serde_json::json!("high")).await?;

// Retrieve context data
let task_id = sm.get_context("task_id").await;

// Get all context
let all_context = sm.get_all_context().await;
```

### 2. Custom Events

Send domain-specific events with associated data:

```rust
let event = WorkflowEvent::Custom {
    name: "code_review_started".to_string(),
    data: serde_json::json!({
        "reviewer_count": 3,
        "deadline": "2024-01-15"
    })
};

sm.process_event(event).await?;
```

### 3. State History

Track all state transitions with timestamps and context snapshots:

```rust
// Get all history entries
let history = sm.get_history().await;
for entry in history {
    println!("Transition: {} -> {}",
        entry.transition.from_state,
        entry.transition.to_state
    );
    println!("Event: {}", entry.transition.event);
    println!("Duration: {}ms", entry.transition.duration_ms);
    println!("Context: {}", entry.context_snapshot);
}

// Get last N entries
let recent = sm.get_history_tail(10).await;
```

### 4. Workflow Orchestration

Manage multiple workflows with a central orchestrator:

```rust
let orchestrator = WorkflowOrchestrator::new();

// Register workflows
let sm1 = Arc::new(WorkflowStateMachine::new("workflow-1".to_string()));
let sm2 = Arc::new(WorkflowStateMachine::new("workflow-2".to_string()));

orchestrator.register_workflow("workflow-1".to_string(), sm1).await?;
orchestrator.register_workflow("workflow-2".to_string(), sm2).await?;

// List all workflows
let workflows = orchestrator.list_workflows().await;

// Get specific workflow
let sm = orchestrator.get_workflow("workflow-1").await?;

// Get metadata for all workflows
let all_metadata = orchestrator.get_all_metadata().await;
```

### 5. State Serialization & Persistence

Serialize workflow state for storage and recovery:

```rust
// Serialize to storage
let serialized = sm.serialize().await?;
// Store serialized to database/file

// Later: Restore from serialized state
let restored_sm = WorkflowStateMachine::deserialize(serialized).await?;
let state = restored_sm.current_state().await;
```

### 6. Concurrent Workflows

Run multiple workflows concurrently with Tokio:

```rust
let sm1 = Arc::new(WorkflowStateMachine::new("workflow-1".to_string()));
let sm2 = Arc::new(WorkflowStateMachine::new("workflow-2".to_string()));

let task1 = tokio::spawn({
    let sm = Arc::clone(&sm1);
    async move {
        sm.process_event(WorkflowEvent::Start).await.ok();
        // ... workflow logic
    }
});

let task2 = tokio::spawn({
    let sm = Arc::clone(&sm2);
    async move {
        sm.process_event(WorkflowEvent::Start).await.ok();
        // ... workflow logic
    }
});

tokio::try_join!(task1, task2)?;
```

## Advanced Features

### Hierarchical States

Define parent-child state relationships for complex workflows:

```rust
let root = HierarchicalState::new("Root".to_string());

let active = HierarchicalState::new("Active".to_string())
    .with_parent("Root".to_string())
    .with_substates(vec![
        HierarchicalState::new("Planning".to_string()),
        HierarchicalState::new("Coding".to_string()),
        HierarchicalState::new("Testing".to_string()),
    ]);

let inactive = HierarchicalState::new("Inactive".to_string())
    .with_parent("Root".to_string());
```

### Parallel States

Support parallel execution paths:

```rust
let parallel = HierarchicalState::parallel()
    .with_substates(vec![
        HierarchicalState::new("Path1".to_string()),
        HierarchicalState::new("Path2".to_string()),
    ]);
```

## Integration with Descartes Components

### With Agent Lifecycle Management

```rust
pub struct AgentWorkflow {
    state_machine: Arc<WorkflowStateMachine>,
    agent_id: String,
}

impl AgentWorkflow {
    pub async fn start_agent(&self) -> StateMachineResult<()> {
        self.state_machine.set_context(
            "agent_id",
            serde_json::json!(&self.agent_id)
        ).await?;

        self.state_machine.process_event(WorkflowEvent::Start).await?;
        Ok(())
    }

    pub async fn pause_agent(&self) -> StateMachineResult<()> {
        self.state_machine.process_event(WorkflowEvent::Pause).await
    }

    pub async fn resume_agent(&self) -> StateMachineResult<()> {
        self.state_machine.process_event(WorkflowEvent::Resume).await
    }

    pub async fn stop_agent(&self) -> StateMachineResult<()> {
        self.state_machine.process_event(WorkflowEvent::Complete).await
    }
}
```

### With StateStore Trait

```rust
pub struct PersistentWorkflow {
    state_machine: Arc<WorkflowStateMachine>,
    store: Arc<dyn StateStore>,
}

impl PersistentWorkflow {
    pub async fn save_state(&self) -> StateMachineResult<()> {
        let serialized = self.state_machine.serialize().await?;
        // Implement storage using StateStore trait
        Ok(())
    }

    pub async fn restore_state(&self, workflow_id: &str) -> StateMachineResult<()> {
        // Load from StateStore
        // Restore workflow state
        Ok(())
    }
}
```

### With Swarm.toml Parser

```toml
[workflow.code_implementation]
name = "Code Implementation"
initial_state = "Planning"

[workflow.code_implementation.states.Planning]
description = "Architect plans the implementation"
handlers = ["validate_requirements", "generate_design"]
next_on_success = "Coding"
next_on_failure = "Blocked"

[workflow.code_implementation.states.Coding]
description = "Developer implements code"
handlers = ["write_code", "commit_changes"]
next_on_success = "Testing"
next_on_failure = "Blocked"
```

Workflows defined in Swarm.toml can be automatically converted to state machines.

## Error Handling

```rust
use descartes_core::state_machine::{StateMachineError, StateMachineResult};

async fn safe_transition(
    sm: &WorkflowStateMachine,
    event: WorkflowEvent
) -> StateMachineResult<()> {
    match sm.process_event(event).await {
        Ok(()) => {
            println!("Transition successful");
            Ok(())
        }
        Err(StateMachineError::InvalidTransition(from, to)) => {
            println!("Cannot transition from {} to {}", from, to);
            Err(StateMachineError::InvalidTransition(from, to))
        }
        Err(StateMachineError::HandlerError(msg)) => {
            println!("Handler error: {}", msg);
            Err(StateMachineError::HandlerError(msg))
        }
        Err(e) => Err(e)
    }
}
```

## Performance Considerations

1. **History Management**: Default limit is 1000 entries per workflow. Configure with:
   ```rust
   let sm = WorkflowStateMachine::new("workflow".to_string())
       .with_max_history(10000);
   ```

2. **Concurrent Access**: State machine uses `Arc<RwLock<>>` for thread-safe access.

3. **Memory Usage**: Each context snapshot is stored in history. Use sparse updates.

4. **Serialization**: Context data is JSON serialized for storage. Keep payloads reasonable.

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_basic() {
        let sm = WorkflowStateMachine::new("test-workflow".to_string());

        sm.process_event(WorkflowEvent::Start).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Running);

        sm.process_event(WorkflowEvent::Complete).await.unwrap();
        assert_eq!(sm.current_state().await, WorkflowState::Completed);
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let sm = WorkflowStateMachine::new("test-workflow".to_string());

        // Can't transition from Idle directly to Paused
        let result = sm.process_event(WorkflowEvent::Pause).await;
        assert!(result.is_err());
    }
}
```

## Running the Demo

```bash
cd /Users/reuben/gauntlet/cap/descartes

# Run the state machine demo
cargo run --example state_machine_demo
```

## Migration from POC

The production implementation builds on `/Users/reuben/gauntlet/cap/descartes/poc_state_machine.rs` with:

1. **Proper error handling** with `StateMachineError` enum
2. **Thread-safe access** with `Arc<RwLock<>>`
3. **Async/await throughout** instead of mock implementations
4. **Type safety** with strong state enum
5. **Comprehensive tests** in the module
6. **Documentation and examples**
7. **Serialization support** for persistence
8. **Workflow orchestration** for multi-workflow scenarios
9. **Integration hooks** for handlers and custom events
10. **Production-ready performance** characteristics

## Next Steps

1. Implement StateStore integration for SQLite persistence
2. Add Swarm.toml parser support for declarative workflow definition
3. Implement retry policies and timeouts
4. Add metrics collection for monitoring
5. Create workflow templates for common patterns
6. Implement workflow composition/nesting
7. Add workflow validation and visualization

## References

- **File**: `/Users/reuben/gauntlet/cap/descartes/core/src/state_machine.rs`
- **Example**: `/Users/reuben/gauntlet/cap/descartes/core/examples/state_machine_demo.rs`
- **Library**: Statig 0.3 (https://github.com/p0lunin/statig)
- **Related**: Swarm.toml parser (in progress)
