//! Example demonstrating the Descartes Debugger State Models
//!
//! This example shows how to:
//! - Create and enable debugger state
//! - Set breakpoints at various locations
//! - Execute step-by-step with state capture
//! - Inspect call stacks and variables
//! - Navigate execution history
//! - Serialize/deserialize debugger state

use descartes_core::{
    Breakpoint, BreakpointLocation, CallFrame, DebugCommand, DebuggerState, ExecutionState,
    ThoughtSnapshot, WorkflowState,
};
use uuid::Uuid;

fn main() {
    println!("=== Descartes Debugger Example ===\n");

    // Create a new agent and debugger state
    let agent_id = Uuid::new_v4();
    let mut debugger = DebuggerState::new(agent_id);

    println!("1. Creating debugger for agent: {}", agent_id);
    println!("   Debug mode enabled: {}\n", debugger.is_enabled());

    // Enable debug mode
    debugger.enable();
    println!("2. Debug mode enabled: {}\n", debugger.is_enabled());

    // Set some breakpoints
    println!("3. Setting breakpoints:");

    // Breakpoint on workflow state transition
    let bp1 = Breakpoint::with_description(
        BreakpointLocation::WorkflowState {
            state: WorkflowState::Running,
        },
        "Break when workflow enters Running state".to_string(),
    );
    debugger.add_breakpoint(bp1);
    println!("   - Breakpoint on WorkflowState::Running");

    // Breakpoint on specific step
    let bp2 = Breakpoint::with_description(
        BreakpointLocation::StepCount { step: 5 },
        "Break at step 5".to_string(),
    );
    debugger.add_breakpoint(bp2);
    println!("   - Breakpoint at step 5");

    // Breakpoint on stack depth
    let bp3 = Breakpoint::new(BreakpointLocation::StackDepth { depth: 3 });
    debugger.add_breakpoint(bp3);
    println!("   - Breakpoint when stack depth reaches 3\n");

    // Simulate execution with steps
    println!("4. Executing steps:");
    for i in 0..10 {
        debugger.step().unwrap();

        // Simulate thought creation at certain steps
        if i % 3 == 0 {
            let thought = ThoughtSnapshot::new(
                format!("thought-{}", i),
                format!("This is thought number {}", i),
                debugger.step_count,
                Some(agent_id),
            );
            debugger.current_thought = Some(thought);
            debugger
                .current_context
                .update_thought(debugger.current_thought.clone().unwrap());
        }

        // Simulate call stack growth
        if i == 2 {
            let frame1 = CallFrame::new(
                "process_task".to_string(),
                WorkflowState::Running,
                debugger.step_count,
                None,
            );
            debugger.current_context.push_frame(frame1);
        }

        if i == 4 {
            let frame2 = CallFrame::new(
                "analyze_data".to_string(),
                WorkflowState::Running,
                debugger.step_count,
                debugger.current_context.current_frame().map(|f| f.frame_id),
            );
            debugger.current_context.push_frame(frame2);
        }

        // Check breakpoints
        if let Some(bp) = debugger.check_breakpoints() {
            println!("   Step {}: BREAKPOINT HIT - {}", i, bp.location);
            debugger.execution_state = ExecutionState::Paused;
        }

        println!(
            "   Step {}: count={}, stack_depth={}, state={}",
            i, debugger.step_count, debugger.current_context.stack_depth, debugger.execution_state
        );
    }
    println!();

    // Display breakpoint statistics
    println!("5. Breakpoint Statistics:");
    for bp in debugger.get_breakpoints() {
        println!(
            "   - {} (hits: {}, enabled: {})",
            bp.location, bp.hit_count, bp.enabled
        );
    }
    println!();

    // Show call stack
    println!("6. Current Call Stack:");
    println!("{}", debugger.current_context.format_stack_trace());

    // Show execution history
    println!("7. Execution History (last 3 entries):");
    let history = debugger.get_history();
    for (i, snapshot) in history.iter().rev().take(3).enumerate() {
        println!(
            "   #{}: Step {}, State: {}, Workflow: {}",
            i, snapshot.step_number, snapshot.execution_state, snapshot.workflow_state
        );
        if let Some(ref thought) = snapshot.thought {
            println!("      Thought: {}", thought.summary());
        }
    }
    println!();

    // Navigate history
    println!("8. History Navigation:");
    if !history.is_empty() {
        let mid_point = history.len() / 2;
        debugger.goto_history(mid_point).unwrap();
        println!("   Navigated to history index: {}", mid_point);
        if let Some(snapshot) = debugger.current_snapshot() {
            println!("   Snapshot at step: {}", snapshot.step_number);
        }
    }
    println!();

    // Show statistics
    println!("9. Debug Session Statistics:");
    let stats = &debugger.statistics;
    println!("   Sessions started: {}", stats.sessions_started);
    println!("   Total steps: {}", stats.total_steps);
    println!("   Breakpoints set: {}", stats.breakpoints_set);
    println!("   Breakpoints hit: {}", stats.breakpoints_hit);
    println!();

    // Demonstrate serialization
    println!("10. Serialization:");
    match debugger.to_json() {
        Ok(json) => {
            println!("   Debugger state serialized successfully");
            println!("   JSON size: {} bytes", json.len());

            // Deserialize it back
            match DebuggerState::from_json(&json) {
                Ok(restored) => {
                    println!("   Debugger state deserialized successfully");
                    println!("   Restored step count: {}", restored.step_count);
                }
                Err(e) => println!("   Deserialization error: {}", e),
            }
        }
        Err(e) => println!("   Serialization error: {}", e),
    }
    println!();

    // Demonstrate debug commands
    println!("11. Debug Commands (examples):");
    println!("   - DebugCommand::Enable");
    println!("   - DebugCommand::Pause");
    println!("   - DebugCommand::Step");
    println!("   - DebugCommand::SetBreakpoint {{ location, condition }}");
    println!("   - DebugCommand::InspectContext");
    println!("   - DebugCommand::ShowStack");
    println!();

    // Summary
    println!("=== Example Complete ===");
    println!("The debugger provides comprehensive state tracking for agent execution,");
    println!("including breakpoints, stepping, history navigation, and context inspection.");
}
