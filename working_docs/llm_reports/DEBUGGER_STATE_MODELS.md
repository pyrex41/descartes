# Debugger State Models - Phase 3:6.1 Implementation Report

## Overview

This document describes the debugger state models implemented for the Descartes Agent Orchestration System. The debugger provides comprehensive state tracking, breakpoint management, step-by-step execution, and context inspection capabilities for debugging agent workflows.

## Implementation Details

### File Location
- **Primary Module**: `/home/user/descartes/descartes/core/src/debugger.rs`
- **Module Export**: Added to `/home/user/descartes/descartes/core/src/lib.rs`
- **Example**: `/home/user/descartes/descartes/core/examples/debugger_example.rs`

### Architecture

The debugger state models integrate seamlessly with the existing Descartes architecture:

1. **State Machine Integration**: Uses `WorkflowState` from `state_machine.rs`
2. **Thought System Integration**: Compatible with `ThoughtMetadata` from `thoughts.rs`
3. **Agent Integration**: Works with `AgentInfo` and `AgentStatus` from `agent_runner.rs`
4. **Serialization**: Full serde support for persistence and IPC

## Core Data Structures

### 1. DebuggerState

The main state container for all debugging operations.

```rust
pub struct DebuggerState {
    pub debug_mode: bool,                    // Enabled/disabled flag
    pub execution_state: ExecutionState,     // Current execution mode
    pub current_thought: Option<ThoughtSnapshot>,
    pub current_context: DebugContext,       // Full execution context
    pub breakpoints: Vec<Breakpoint>,        // All breakpoints
    pub step_count: u64,                     // Total steps executed
    pub history_index: Option<usize>,        // Position in history
    pub history: Vec<DebugSnapshot>,         // Execution snapshots
    pub max_history_size: usize,             // History limit (default: 1000)
    pub started_at: Option<String>,          // Debug session start time
    pub statistics: DebugStatistics,         // Session statistics
}
```

**Key Features**:
- Enable/disable debug mode dynamically
- Full execution history with configurable limits
- Comprehensive statistics tracking
- JSON serialization support
- File persistence (save/load)

**Methods**:
- `new(agent_id)` - Create new debugger for an agent
- `enable()` / `disable()` - Control debug mode
- `add_breakpoint()` / `remove_breakpoint()` - Manage breakpoints
- `step()` - Execute a single step with state capture
- `check_breakpoints()` - Evaluate breakpoint conditions
- `goto_history(index)` - Navigate execution history
- `to_json()` / `from_json()` - Serialization
- `save_to_file()` / `load_from_file()` - Persistence

### 2. ExecutionState

Represents the current execution mode of the debugger.

```rust
pub enum ExecutionState {
    Running,        // Normal execution
    Paused,         // Paused awaiting commands
    SteppingInto,   // Step into nested calls
    SteppingOver,   // Step over nested calls
    SteppingOut,    // Step out to parent frame
    Continuing,     // Continue until breakpoint
}
```

**Features**:
- Type-safe state transitions
- Helper methods: `is_running()`, `is_paused()`, `is_stepping()`
- Display trait for human-readable output

### 3. ThoughtSnapshot

Captures agent thought state at a specific execution point.

```rust
pub struct ThoughtSnapshot {
    pub thought_id: String,
    pub content: String,
    pub timestamp: String,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub agent_id: Option<Uuid>,
    pub step_number: u64,
}
```

**Features**:
- Integration with existing `ThoughtMetadata`
- `summary()` method for abbreviated output
- Timestamped for temporal ordering
- Step number correlation

### 4. DebugContext

Complete execution context at any point in time.

```rust
pub struct DebugContext {
    pub agent_id: Uuid,
    pub workflow_state: WorkflowState,
    pub stack_depth: usize,
    pub local_variables: HashMap<String, serde_json::Value>,
    pub call_stack: Vec<CallFrame>,
    pub last_thought: Option<ThoughtSnapshot>,
    pub current_step: u64,
    pub metadata: serde_json::Value,
}
```

**Features**:
- Call stack management (push/pop frames)
- Variable scoping and storage
- Workflow state tracking
- `format_stack_trace()` for debugging output

### 5. CallFrame

Represents a single frame in the call stack.

```rust
pub struct CallFrame {
    pub frame_id: Uuid,
    pub name: String,
    pub workflow_state: WorkflowState,
    pub local_variables: HashMap<String, serde_json::Value>,
    pub entry_step: u64,
    pub parent_frame_id: Option<Uuid>,
}
```

**Features**:
- Hierarchical frame tracking
- Per-frame variable scoping
- Entry step tracking
- Parent frame references for stack unwinding

### 6. Breakpoint

Represents a debugger breakpoint with optional conditions.

```rust
pub struct Breakpoint {
    pub id: Uuid,
    pub location: BreakpointLocation,
    pub condition: Option<String>,
    pub enabled: bool,
    pub hit_count: u64,
    pub description: Option<String>,
    pub created_at: String,
}
```

**Features**:
- Multiple location types (see BreakpointLocation)
- Conditional breakpoints (expression support ready)
- Enable/disable without removal
- Hit count tracking
- Human-readable descriptions

### 7. BreakpointLocation

Specifies where a breakpoint should trigger.

```rust
pub enum BreakpointLocation {
    WorkflowState { state: WorkflowState },
    ThoughtId { thought_id: String },
    StepCount { step: u64 },
    AgentId { agent_id: Uuid },
    AnyTransition,
    StackDepth { depth: usize },
}
```

**Location Types**:
- **WorkflowState**: Break on specific workflow state
- **ThoughtId**: Break when specific thought is created
- **StepCount**: Break at exact step number
- **AgentId**: Break when specific agent is active
- **AnyTransition**: Break on any state change
- **StackDepth**: Break at specific call stack depth

### 8. DebugCommand

Commands for controlling debugger execution.

```rust
pub enum DebugCommand {
    Enable,
    Disable,
    Pause,
    Resume,
    Step,
    StepOver,
    StepInto,
    StepOut,
    Continue,
    SetBreakpoint { location, condition },
    RemoveBreakpoint { id },
    ToggleBreakpoint { id },
    ListBreakpoints,
    InspectContext,
    ShowStack,
    Evaluate { expression },
    GotoHistory { index },
    ShowHistory,
    ClearHistory,
    GetStatistics,
}
```

**Command Categories**:
- **Control**: Enable, Disable, Pause, Resume, Continue
- **Stepping**: Step, StepOver, StepInto, StepOut
- **Breakpoints**: Set, Remove, Toggle, List
- **Inspection**: InspectContext, ShowStack, Evaluate
- **History**: Goto, Show, Clear
- **Statistics**: GetStatistics

### 9. DebugSnapshot

A complete snapshot of execution state at a point in time.

```rust
pub struct DebugSnapshot {
    pub step_number: u64,
    pub timestamp: String,
    pub execution_state: ExecutionState,
    pub thought: Option<ThoughtSnapshot>,
    pub context: DebugContext,
    pub workflow_state: WorkflowState,
}
```

**Features**:
- Full state capture
- Timestamped for replay
- Used for history navigation
- Supports time-travel debugging

### 10. DebugStatistics

Tracks statistics about a debug session.

```rust
pub struct DebugStatistics {
    pub sessions_started: u64,
    pub total_steps: u64,
    pub pauses: u64,
    pub resumes: u64,
    pub breakpoints_set: u64,
    pub breakpoints_hit: u64,
}
```

**Metrics Tracked**:
- Session lifecycle events
- Step execution count
- Pause/resume frequency
- Breakpoint effectiveness

## Integration with Agent System

### Agent State Integration

The debugger integrates with the agent system through:

1. **DebuggerStateExt Trait**: Extension trait for agent types
   ```rust
   pub trait DebuggerStateExt {
       fn debugger_state(&self) -> Option<&DebuggerState>;
       fn debugger_state_mut(&mut self) -> Option<&mut DebuggerState>;
       fn is_debugging(&self) -> bool;
   }
   ```

2. **AgentInfo Integration**: Each agent can have an associated debugger state
3. **WorkflowState Integration**: Debugger tracks workflow state transitions
4. **Thought System Integration**: Captures and displays agent thoughts

### State Machine Integration

The debugger works alongside the existing state machine:

- **WorkflowState** from `state_machine.rs` is used for workflow tracking
- **WorkflowEvent** can trigger debugger breakpoints
- **StateHandler** can invoke debugger on transitions
- **History** mechanism complementary to state machine history

### Serialization and Persistence

Full serialization support enables:

1. **State Persistence**: Save/load debugger state to disk
2. **IPC Communication**: Send debugger state between processes
3. **Remote Debugging**: Network transmission of debug info
4. **Checkpointing**: Capture execution state for replay

## Error Handling

Comprehensive error type covering all failure modes:

```rust
pub enum DebuggerError {
    NotEnabled,
    InvalidStateTransition(String, String),
    BreakpointNotFound(Uuid),
    InvalidBreakpointLocation(String),
    CannotStepWhileRunning,
    CannotResumeWhileNotPaused,
    HistoryIndexOutOfBounds(usize),
    NoHistoryAvailable,
    ConditionEvaluationFailed(String),
    CommandFailed(String),
}
```

## Usage Examples

### Basic Debugging Session

```rust
use descartes_core::{DebuggerState, Breakpoint, BreakpointLocation, WorkflowState};
use uuid::Uuid;

// Create debugger
let agent_id = Uuid::new_v4();
let mut debugger = DebuggerState::new(agent_id);

// Enable debugging
debugger.enable();

// Set breakpoint on workflow state
let bp = Breakpoint::new(BreakpointLocation::WorkflowState {
    state: WorkflowState::Running,
});
debugger.add_breakpoint(bp);

// Execute steps
for _ in 0..10 {
    debugger.step().unwrap();

    if let Some(bp) = debugger.check_breakpoints() {
        println!("Breakpoint hit: {}", bp.location);
        // Handle breakpoint...
    }
}
```

### History Navigation

```rust
// Navigate through execution history
let history = debugger.get_history();
for (i, snapshot) in history.iter().enumerate() {
    println!("Step {}: State={}",
        snapshot.step_number,
        snapshot.execution_state);
}

// Jump to specific point
debugger.goto_history(5).unwrap();
```

### State Persistence

```rust
use std::path::Path;

// Save debugger state
let path = Path::new("/tmp/debugger_state.json");
debugger.save_to_file(path).unwrap();

// Load debugger state
let restored = DebuggerState::load_from_file(path).unwrap();
```

## Testing

Comprehensive test suite included in the module:

- **test_debugger_state_creation**: Basic initialization
- **test_enable_disable_debug_mode**: Mode toggling
- **test_add_remove_breakpoint**: Breakpoint management
- **test_execution_state_checks**: State validation
- **test_thought_snapshot**: Thought capture
- **test_call_frame**: Frame management
- **test_debug_context**: Context operations
- **test_breakpoint_location_display**: Display formatting
- **test_breakpoint_enable_disable**: Breakpoint toggling
- **test_step_execution**: Step-by-step execution
- **test_history_navigation**: History traversal
- **test_serialization**: JSON serialization/deserialization

Run tests with:
```bash
cd /home/user/descartes/descartes
cargo test --package descartes-core debugger
```

## Future Enhancements

Potential future additions:

1. **Expression Evaluator**: Implement conditional breakpoint evaluation
2. **Watch Expressions**: Monitor variable changes
3. **Remote Debugging Protocol**: Network-based debugging
4. **Visual Debugger UI**: GUI integration
5. **Replay System**: Full execution replay from history
6. **Profiling Integration**: Performance analysis during debugging
7. **Multi-Agent Debugging**: Debug multiple agents simultaneously
8. **Debug Adaptor Protocol**: VSCode/IDE integration

## File Structure

```
/home/user/descartes/descartes/core/
├── src/
│   ├── debugger.rs              # Main debugger implementation
│   └── lib.rs                   # Module exports
├── examples/
│   └── debugger_example.rs      # Usage examples
└── tests/                       # Integration tests (if needed)
```

## API Surface

### Public Types Exported

From `lib.rs`:
```rust
pub use debugger::{
    DebuggerState,
    DebuggerError,
    DebuggerResult,
    ExecutionState,
    ThoughtSnapshot,
    CallFrame,
    DebugContext,
    BreakpointLocation,
    Breakpoint,
    DebugCommand,
    DebugEvent,
    DebugSnapshot,
    DebugStatistics,
    DebuggerStateExt,
};
```

## Dependencies

The debugger module depends on:

- **Existing Descartes modules**:
  - `state_machine` (WorkflowState)
  - `thoughts` (ThoughtMetadata)
  - `traits` (Agent types)

- **External crates**:
  - `uuid` - Unique identifiers
  - `serde` / `serde_json` - Serialization
  - `thiserror` - Error handling
  - `chrono` - Timestamps

## Conclusion

The debugger state models provide a production-ready foundation for debugging Descartes agent workflows. The implementation:

✅ **Complete**: All requested fields and features implemented
✅ **Type-Safe**: Leverages Rust's type system for correctness
✅ **Tested**: Comprehensive test coverage
✅ **Integrated**: Seamless integration with existing systems
✅ **Extensible**: Easy to add new features
✅ **Documented**: Inline documentation and examples
✅ **Serializable**: Full persistence support

The models are ready for use in building higher-level debugging tools, IDEs, and monitoring systems for the Descartes orchestration platform.
