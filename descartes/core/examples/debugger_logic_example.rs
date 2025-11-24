// Example: Debugger Core Logic Usage
//
// This example demonstrates the core debugger logic features including:
// - Debugger control (pause, resume, step variants)
// - Breakpoint management and handling
// - State capture and history
// - Command processing
// - Agent runtime integration

use descartes_core::{
    Debugger, DebugCommand, CommandResult, DebuggerResult,
    Breakpoint, BreakpointLocation, WorkflowState,
    DebuggableAgent, run_with_debugging,
};
use uuid::Uuid;

// ============================================================================
// EXAMPLE AGENT IMPLEMENTATION
// ============================================================================

/// A simple example agent that implements DebuggableAgent
struct ExampleAgent {
    debugger: Option<Debugger>,
    step_count: u32,
    max_steps: u32,
}

impl ExampleAgent {
    fn new(agent_id: Uuid, max_steps: u32) -> Self {
        Self {
            debugger: Some(Debugger::new(agent_id)),
            step_count: 0,
            max_steps,
        }
    }

    fn simulate_work(&mut self) {
        println!("  [Agent] Executing step {}...", self.step_count + 1);

        if let Some(debugger) = &mut self.debugger {
            // Update workflow state
            debugger.update_workflow_state(WorkflowState::Running);

            // Simulate capturing a thought
            let thought_content = format!("Thinking about step {}", self.step_count + 1);
            debugger.capture_thought_snapshot(
                format!("thought-{}", self.step_count),
                thought_content,
            );

            // Simulate setting a variable
            debugger.set_context_variable(
                format!("step_{}", self.step_count),
                serde_json::json!({"completed": true, "value": self.step_count})
            );
        }
    }
}

impl DebuggableAgent for ExampleAgent {
    fn debugger(&self) -> Option<&Debugger> {
        self.debugger.as_ref()
    }

    fn debugger_mut(&mut self) -> Option<&mut Debugger> {
        self.debugger.as_mut()
    }

    fn execute_step(&mut self) -> DebuggerResult<()> {
        if self.step_count >= self.max_steps {
            println!("  [Agent] Max steps reached, completing...");
            return Ok(());
        }

        self.simulate_work();
        self.step_count += 1;
        Ok(())
    }
}

// ============================================================================
// DEMONSTRATION FUNCTIONS
// ============================================================================

fn demo_basic_control() {
    println!("\n=== Demo: Basic Control Operations ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // Enable debugging
    println!("1. Enabling debug mode...");
    debugger.state_mut().enable();
    assert!(debugger.state().is_enabled());
    println!("   ✓ Debug mode enabled");

    // Pause
    println!("\n2. Pausing agent...");
    debugger.pause_agent().unwrap();
    assert!(debugger.should_pause());
    println!("   ✓ Agent paused");

    // Resume
    println!("\n3. Resuming agent...");
    debugger.resume_agent().unwrap();
    assert!(!debugger.should_pause());
    println!("   ✓ Agent resumed");

    // Step
    println!("\n4. Executing single step...");
    debugger.step_agent().unwrap();
    assert_eq!(debugger.state().step_count, 1);
    println!("   ✓ Step completed (step count: {})", debugger.state().step_count);

    // Disable
    println!("\n5. Disabling debug mode...");
    debugger.state_mut().disable();
    assert!(!debugger.state().is_enabled());
    println!("   ✓ Debug mode disabled");
}

fn demo_stepping_modes() {
    println!("\n=== Demo: Stepping Modes ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();

    // Step into
    println!("1. Step into (enters nested calls)...");
    debugger.step_into().unwrap();
    println!("   ✓ Step into completed (step: {})", debugger.state().step_count);

    // Step over
    println!("\n2. Step over (skips nested calls)...");
    debugger.resume_agent().unwrap();
    debugger.step_over().unwrap();
    println!("   ✓ Step over completed (step: {})", debugger.state().step_count);

    // Push a call frame to demonstrate step out
    println!("\n3. Pushing call frame...");
    debugger.push_call_frame("nested_function".to_string(), WorkflowState::Running);
    println!("   ✓ Call frame pushed (depth: {})", debugger.state().current_context.stack_depth);

    // Step out
    println!("\n4. Step out (exits current frame)...");
    debugger.resume_agent().unwrap();
    debugger.step_out().unwrap();
    println!("   ✓ Step out completed (depth: {})", debugger.state().current_context.stack_depth);
}

fn demo_breakpoints() {
    println!("\n=== Demo: Breakpoint Management ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();

    // Set breakpoint at step 5
    println!("1. Setting breakpoint at step 5...");
    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 5 });
    let bp_id = debugger.state_mut().add_breakpoint(bp);
    println!("   ✓ Breakpoint set (ID: {})", bp_id);

    // Set breakpoint on workflow state
    println!("\n2. Setting breakpoint on Running state...");
    let bp2 = Breakpoint::new(BreakpointLocation::WorkflowState {
        state: WorkflowState::Running,
    });
    let bp2_id = debugger.state_mut().add_breakpoint(bp2);
    println!("   ✓ Breakpoint set (ID: {})", bp2_id);

    // List breakpoints
    println!("\n3. Listing all breakpoints...");
    let breakpoints = debugger.state().get_breakpoints();
    println!("   Total breakpoints: {}", breakpoints.len());
    for (i, bp) in breakpoints.iter().enumerate() {
        println!("   {}. {} (enabled: {})", i + 1, bp.location, bp.enabled);
    }

    // Disable a breakpoint
    println!("\n4. Disabling breakpoint {}...", bp_id);
    debugger.state_mut().toggle_breakpoint(&bp_id).unwrap();
    println!("   ✓ Breakpoint disabled");

    // Remove a breakpoint
    println!("\n5. Removing breakpoint {}...", bp2_id);
    debugger.state_mut().remove_breakpoint(&bp2_id).unwrap();
    println!("   ✓ Breakpoint removed");

    println!("\n   Final breakpoint count: {}", debugger.state().get_breakpoints().len());
}

fn demo_state_capture() {
    println!("\n=== Demo: State Capture ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();

    // Capture thought snapshot
    println!("1. Capturing thought snapshot...");
    let thought = debugger.capture_thought_snapshot(
        "example-thought-001".to_string(),
        "Analyzing the problem domain...".to_string(),
    );
    println!("   ✓ Thought captured:");
    println!("     ID: {}", thought.thought_id);
    println!("     Content: {}", thought.content);
    println!("     Step: {}", thought.step_number);

    // Update workflow state
    println!("\n2. Updating workflow state...");
    debugger.update_workflow_state(WorkflowState::Running);
    println!("   ✓ State updated to: {}", debugger.state().current_context.workflow_state);

    // Set context variables
    println!("\n3. Setting context variables...");
    debugger.set_context_variable("task_id".to_string(), serde_json::json!("task-123"));
    debugger.set_context_variable("priority".to_string(), serde_json::json!(5));
    debugger.set_context_variable("tags".to_string(), serde_json::json!(["important", "urgent"]));
    println!("   ✓ Variables set: {}", debugger.state().current_context.local_variables.len());

    // Capture context snapshot
    println!("\n4. Capturing context snapshot...");
    let context = debugger.capture_context_snapshot();
    println!("   ✓ Context captured:");
    println!("     Agent ID: {}", context.agent_id);
    println!("     Workflow state: {}", context.workflow_state);
    println!("     Variables: {}", context.local_variables.len());

    // Save to history
    println!("\n5. Saving to history...");
    debugger.save_to_history();
    println!("   ✓ Saved (history size: {})", debugger.state().history.len());
}

fn demo_call_stack() {
    println!("\n=== Demo: Call Stack Management ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();

    // Push multiple frames
    println!("1. Building call stack...");
    let frame1_id = debugger.push_call_frame("main".to_string(), WorkflowState::Running);
    println!("   ✓ Pushed 'main' (depth: {})", debugger.state().current_context.stack_depth);

    let frame2_id = debugger.push_call_frame("process_task".to_string(), WorkflowState::Running);
    println!("   ✓ Pushed 'process_task' (depth: {})", debugger.state().current_context.stack_depth);

    let frame3_id = debugger.push_call_frame("validate_input".to_string(), WorkflowState::Running);
    println!("   ✓ Pushed 'validate_input' (depth: {})", debugger.state().current_context.stack_depth);

    // Show stack trace
    println!("\n2. Current stack trace:");
    println!("{}", debugger.state().current_context.format_stack_trace());

    // Pop frames
    println!("3. Popping frames...");
    if let Some(frame) = debugger.pop_call_frame() {
        println!("   ✓ Popped '{}' (depth: {})", frame.name, debugger.state().current_context.stack_depth);
    }
    if let Some(frame) = debugger.pop_call_frame() {
        println!("   ✓ Popped '{}' (depth: {})", frame.name, debugger.state().current_context.stack_depth);
    }

    println!("\n4. Final stack trace:");
    println!("{}", debugger.state().current_context.format_stack_trace());
}

fn demo_command_processing() {
    println!("\n=== Demo: Command Processing ===\n");

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // Enable via command
    println!("1. Processing Enable command...");
    let result = debugger.process_command(DebugCommand::Enable);
    match result {
        Ok(CommandResult::Success { message }) => println!("   ✓ {}", message),
        _ => println!("   ✗ Unexpected result"),
    }

    // Set breakpoint via command
    println!("\n2. Processing SetBreakpoint command...");
    let result = debugger.process_command(DebugCommand::SetBreakpoint {
        location: BreakpointLocation::StepCount { step: 10 },
        condition: None,
    });
    match result {
        Ok(CommandResult::BreakpointSet { breakpoint_id, location }) => {
            println!("   ✓ Breakpoint set at {}", location);
            println!("     ID: {}", breakpoint_id);
        }
        _ => println!("   ✗ Unexpected result"),
    }

    // Step via command
    println!("\n3. Processing Step command...");
    let result = debugger.process_command(DebugCommand::Step);
    match result {
        Ok(CommandResult::StepComplete { step_number, .. }) => {
            println!("   ✓ Step completed (step: {})", step_number);
        }
        _ => println!("   ✗ Unexpected result"),
    }

    // Inspect context via command
    println!("\n4. Processing InspectContext command...");
    debugger.set_context_variable("test".to_string(), serde_json::json!("value"));
    let result = debugger.process_command(DebugCommand::InspectContext);
    match result {
        Ok(CommandResult::ContextInspection { context }) => {
            println!("   ✓ Context inspected:");
            println!("     Agent ID: {}", context.agent_id);
            println!("     Variables: {}", context.local_variables.len());
        }
        _ => println!("   ✗ Unexpected result"),
    }

    // Get statistics via command
    println!("\n5. Processing GetStatistics command...");
    let result = debugger.process_command(DebugCommand::GetStatistics);
    match result {
        Ok(CommandResult::Statistics { stats }) => {
            println!("   ✓ Statistics:");
            println!("     Sessions: {}", stats.sessions_started);
            println!("     Steps: {}", stats.total_steps);
            println!("     Breakpoints set: {}", stats.breakpoints_set);
        }
        _ => println!("   ✗ Unexpected result"),
    }
}

fn demo_callbacks() {
    println!("\n=== Demo: Event Callbacks ===\n");

    use std::sync::{Arc, Mutex};

    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // Set up callbacks
    let pause_count = Arc::new(Mutex::new(0));
    let pause_count_clone = pause_count.clone();
    debugger.on_pause(move |ctx| {
        *pause_count_clone.lock().unwrap() += 1;
        println!("   [Callback] Agent paused at step {}", ctx.current_step);
    });

    let step_count = Arc::new(Mutex::new(0));
    let step_count_clone = step_count.clone();
    debugger.on_step(move |snapshot| {
        *step_count_clone.lock().unwrap() += 1;
        println!("   [Callback] Step executed: {}", snapshot.step_number);
    });

    let bp_count = Arc::new(Mutex::new(0));
    let bp_count_clone = bp_count.clone();
    debugger.on_breakpoint(move |bp, ctx| {
        *bp_count_clone.lock().unwrap() += 1;
        println!("   [Callback] Breakpoint hit: {} at step {}", bp.location, ctx.current_step);
    });

    // Trigger callbacks
    println!("1. Setting up and triggering callbacks...\n");
    debugger.state_mut().enable();

    // Trigger pause callback
    debugger.pause_agent().unwrap();

    // Trigger step callback
    debugger.resume_agent().unwrap();
    debugger.step_agent().unwrap();

    // Trigger breakpoint callback
    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 2 });
    debugger.state_mut().add_breakpoint(bp.clone());
    debugger.resume_agent().unwrap();
    debugger.step_agent().unwrap();
    let _ = debugger.check_and_handle_breakpoints();

    println!("\n2. Callback statistics:");
    println!("   Pause callbacks: {}", *pause_count.lock().unwrap());
    println!("   Step callbacks: {}", *step_count.lock().unwrap());
    println!("   Breakpoint callbacks: {}", *bp_count.lock().unwrap());
}

#[tokio::main]
async fn demo_agent_integration() {
    println!("\n=== Demo: Agent Integration ===\n");

    let agent_id = Uuid::new_v4();
    let mut agent = ExampleAgent::new(agent_id, 5);

    // Enable debugging
    if let Some(debugger) = agent.debugger_mut() {
        debugger.state_mut().enable();
        println!("1. Debug mode enabled for agent {}", agent_id);

        // Set a breakpoint at step 3
        let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 3 });
        debugger.state_mut().add_breakpoint(bp);
        println!("2. Breakpoint set at step 3");
    }

    // Execute a few steps manually
    println!("\n3. Executing steps with debugging:\n");
    for i in 0..3 {
        if agent.before_step() {
            agent.execute_step().unwrap();
            agent.after_step();

            if let Some(debugger) = agent.debugger() {
                if debugger.should_pause() {
                    println!("\n  [Debug] Agent paused at step {}", debugger.state().step_count);
                    break;
                }
            }
        } else {
            println!("\n  [Debug] Agent paused before step");
            break;
        }
    }

    // Show final statistics
    if let Some(debugger) = agent.debugger() {
        println!("\n4. Final debugger statistics:");
        let stats = &debugger.state().statistics;
        println!("   Total steps: {}", stats.total_steps);
        println!("   Breakpoints hit: {}", stats.breakpoints_hit);
        println!("   History entries: {}", debugger.state().history.len());
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Descartes Debugger Core Logic Example               ║");
    println!("║   Comprehensive demonstration of debugger features     ║");
    println!("╚════════════════════════════════════════════════════════╝");

    demo_basic_control();
    demo_stepping_modes();
    demo_breakpoints();
    demo_state_capture();
    demo_call_stack();
    demo_command_processing();
    demo_callbacks();
    demo_agent_integration().await;

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║   All demonstrations completed successfully!           ║");
    println!("╚════════════════════════════════════════════════════════╝\n");
}
