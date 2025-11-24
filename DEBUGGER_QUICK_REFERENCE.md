# Debugger State Models - Quick Reference Card

## Quick Start

```rust
use descartes_core::{DebuggerState, Breakpoint, BreakpointLocation};
use uuid::Uuid;

let mut debugger = DebuggerState::new(Uuid::new_v4());
debugger.enable();
```

## Core Types

| Type | Purpose | Example |
|------|---------|---------|
| `DebuggerState` | Main debugger container | `DebuggerState::new(agent_id)` |
| `ExecutionState` | Current execution mode | `ExecutionState::Paused` |
| `Breakpoint` | Breakpoint definition | `Breakpoint::new(location)` |
| `ThoughtSnapshot` | Captured thought | `ThoughtSnapshot::new(...)` |
| `DebugContext` | Execution context | `DebugContext::new(agent_id, state)` |
| `CallFrame` | Stack frame | `CallFrame::new(name, state, ...)` |

## Execution States

- `Running` - Normal execution
- `Paused` - Awaiting debugger commands
- `SteppingInto` - Step into calls
- `SteppingOver` - Step over calls
- `SteppingOut` - Step out to parent
- `Continuing` - Continue to next breakpoint

## Breakpoint Locations

```rust
BreakpointLocation::WorkflowState { state }     // Break on workflow state
BreakpointLocation::ThoughtId { thought_id }    // Break on thought ID
BreakpointLocation::StepCount { step }          // Break at step N
BreakpointLocation::AgentId { agent_id }        // Break for specific agent
BreakpointLocation::AnyTransition               // Break on any transition
BreakpointLocation::StackDepth { depth }        // Break at stack depth
```

## Common Operations

### Enable Debugging
```rust
debugger.enable();
```

### Set Breakpoint
```rust
let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 10 });
let bp_id = debugger.add_breakpoint(bp);
```

### Execute Step
```rust
debugger.step()?;
```

### Check Breakpoints
```rust
if let Some(bp) = debugger.check_breakpoints() {
    println!("Hit breakpoint: {}", bp.location);
}
```

### Navigate History
```rust
debugger.goto_history(5)?;
let snapshot = debugger.current_snapshot();
```

### Inspect Context
```rust
let context = &debugger.current_context;
println!("Stack depth: {}", context.stack_depth);
println!("{}", context.format_stack_trace());
```

### Save/Load State
```rust
debugger.save_to_file(Path::new("debug.json"))?;
let restored = DebuggerState::load_from_file(Path::new("debug.json"))?;
```

## Debug Commands

| Command | Purpose |
|---------|---------|
| `Enable` | Enable debug mode |
| `Disable` | Disable debug mode |
| `Pause` | Pause execution |
| `Resume` | Resume execution |
| `Step` | Single step |
| `StepOver` | Step over call |
| `StepInto` | Step into call |
| `StepOut` | Step out of frame |
| `Continue` | Continue to breakpoint |
| `SetBreakpoint` | Add breakpoint |
| `RemoveBreakpoint` | Remove breakpoint |
| `InspectContext` | Show context |
| `ShowStack` | Display call stack |
| `GetStatistics` | Get debug stats |

## Error Types

```rust
DebuggerError::NotEnabled                      // Debug mode not enabled
DebuggerError::InvalidStateTransition          // Invalid state change
DebuggerError::BreakpointNotFound              // Breakpoint ID not found
DebuggerError::CannotStepWhileRunning          // Step while running
DebuggerError::HistoryIndexOutOfBounds         // Invalid history index
```

## Integration Points

### With State Machine
```rust
use descartes_core::WorkflowState;

debugger.current_context.workflow_state = WorkflowState::Running;
```

### With Thoughts
```rust
use descartes_core::ThoughtMetadata;

let snapshot = ThoughtSnapshot::from_thought_metadata(&thought, step_num);
debugger.current_thought = Some(snapshot);
```

### With Agent Runner
```rust
use descartes_core::AgentInfo;

let debugger = DebuggerState::new(agent_info.id);
```

## File Locations

| File | Path |
|------|------|
| Implementation | `/home/user/descartes/descartes/core/src/debugger.rs` |
| Example | `/home/user/descartes/descartes/core/examples/debugger_example.rs` |
| Documentation | `/home/user/descartes/DEBUGGER_STATE_MODELS.md` |

## Run Tests

```bash
cd /home/user/descartes/descartes
cargo test --package descartes-core debugger
```

## Run Example

```bash
cd /home/user/descartes/descartes
cargo run --example debugger_example
```

---

**Phase 3:6.1 Implementation Complete** ✅
