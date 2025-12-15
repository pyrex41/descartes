//! Comprehensive Debugger Tests for Phase 3:6.4
//!
//! This module provides extensive test coverage for the debugger functionality,
//! including:
//! - State management tests
//! - Control operation tests (pause, resume, step, continue)
//! - Breakpoint management tests
//! - Context tracking tests
//! - Integration tests with agent execution
//! - Edge case and error handling tests
//!
//! Test Organization:
//! - Basic functionality tests
//! - Advanced scenario tests
//! - Edge case tests
//! - Integration tests

use descartes_core::debugger::*;
use descartes_core::state_machine::WorkflowState;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a test debugger with enabled state
fn create_enabled_debugger() -> Debugger {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();
    debugger
}

/// Create a test debugger with paused state
fn create_paused_debugger() -> Debugger {
    let mut debugger = create_enabled_debugger();
    debugger.pause_agent().unwrap();
    debugger
}

/// Simulate agent execution steps
fn simulate_execution_steps(debugger: &mut Debugger, steps: usize) {
    for _ in 0..steps {
        debugger.state_mut().step().unwrap();
    }
}

/// Create a test thought snapshot
fn create_test_thought(id: &str, content: &str, step: u64) -> ThoughtSnapshot {
    ThoughtSnapshot::new(id.to_string(), content.to_string(), step, None)
}

// ============================================================================
// STATE MANAGEMENT TESTS
// ============================================================================

#[test]
fn test_debugger_creation() {
    let agent_id = Uuid::new_v4();
    let debugger = Debugger::new(agent_id);

    assert!(!debugger.state().is_enabled());
    assert_eq!(debugger.agent_id(), agent_id);
    assert_eq!(debugger.state().execution_state, ExecutionState::Running);
    assert_eq!(debugger.state().step_count, 0);
    assert_eq!(debugger.state().breakpoints.len(), 0);
}

#[test]
fn test_enable_disable_debugger() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // Initially disabled
    assert!(!debugger.state().is_enabled());

    // Enable
    debugger.state_mut().enable();
    assert!(debugger.state().is_enabled());
    assert!(debugger.state().started_at.is_some());
    assert_eq!(debugger.state().statistics.sessions_started, 1);

    // Disable
    debugger.state_mut().disable();
    assert!(!debugger.state().is_enabled());
    assert_eq!(debugger.state().execution_state, ExecutionState::Running);
}

#[test]
fn test_execution_state_transitions() {
    let mut debugger = create_enabled_debugger();

    // Running -> Paused
    assert_eq!(debugger.state().execution_state, ExecutionState::Running);
    debugger.pause_agent().unwrap();
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);

    // Paused -> Running
    debugger.resume_agent().unwrap();
    assert_eq!(debugger.state().execution_state, ExecutionState::Running);

    // Running -> SteppingInto
    debugger.pause_agent().unwrap();
    debugger.step_into().unwrap();
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
}

#[test]
fn test_history_management() {
    let mut debugger = create_enabled_debugger();

    // Initially no history
    assert_eq!(debugger.state().history.len(), 0);

    // Execute steps and build history
    for i in 0..5 {
        debugger.state_mut().step().unwrap();
    }

    assert_eq!(debugger.state().history.len(), 5);
    assert_eq!(debugger.state().step_count, 5);

    // Navigate to specific point in history
    debugger.state_mut().goto_history(2).unwrap();
    assert_eq!(debugger.state().history_index, Some(2));

    // Clear history
    debugger.state_mut().clear_history();
    assert_eq!(debugger.state().history.len(), 0);
    assert_eq!(debugger.state().history_index, None);
}

#[test]
fn test_history_max_size_limit() {
    let mut debugger = create_enabled_debugger();
    debugger.state_mut().max_history_size = 10;

    // Execute more steps than max history size
    for _ in 0..15 {
        debugger.state_mut().step().unwrap();
    }

    // History should be capped at max size
    assert_eq!(debugger.state().history.len(), 10);
}

#[test]
fn test_history_navigation_errors() {
    let mut debugger = create_enabled_debugger();

    // Navigate with no history
    let result = debugger.state_mut().goto_history(0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DebuggerError::HistoryIndexOutOfBounds(_)
    ));

    // Create some history
    debugger.state_mut().step().unwrap();
    debugger.state_mut().step().unwrap();

    // Navigate beyond bounds
    let result = debugger.state_mut().goto_history(10);
    assert!(result.is_err());
}

// ============================================================================
// PAUSE/RESUME TESTS
// ============================================================================

#[test]
fn test_pause_resume_basic() {
    let mut debugger = create_enabled_debugger();

    // Pause execution
    debugger.pause_agent().unwrap();
    assert!(debugger.should_pause());
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    assert_eq!(debugger.state().statistics.pauses, 1);

    // Resume execution
    debugger.resume_agent().unwrap();
    assert!(!debugger.should_pause());
    assert_eq!(debugger.state().execution_state, ExecutionState::Running);
    assert_eq!(debugger.state().statistics.resumes, 1);
}

#[test]
fn test_pause_already_paused() {
    let mut debugger = create_paused_debugger();
    let initial_pauses = debugger.state().statistics.pauses;

    // Pause again (should succeed as idempotent)
    debugger.pause_agent().unwrap();
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    // Statistics should not increment for redundant pause
    assert_eq!(debugger.state().statistics.pauses, initial_pauses);
}

#[test]
fn test_resume_when_not_paused() {
    let mut debugger = create_enabled_debugger();

    // Try to resume when already running
    let result = debugger.resume_agent();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DebuggerError::CannotResumeWhileNotPaused
    ));
}

#[test]
fn test_pause_without_debug_enabled() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // Try to pause without enabling debug mode
    let result = debugger.pause_agent();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DebuggerError::NotEnabled));
}

#[test]
fn test_pause_resume_callbacks() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);
    debugger.state_mut().enable();

    // Set up pause callback
    let pause_called = Arc::new(Mutex::new(false));
    let pause_called_clone = pause_called.clone();
    debugger.on_pause(move |_ctx| {
        *pause_called_clone.lock().unwrap() = true;
    });

    // Pause should trigger callback
    debugger.pause_agent().unwrap();
    assert!(*pause_called.lock().unwrap());
}

// ============================================================================
// STEP OPERATION TESTS
// ============================================================================

#[test]
fn test_step_basic() {
    let mut debugger = create_enabled_debugger();

    debugger.step_agent().unwrap();

    assert_eq!(debugger.state().step_count, 1);
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    assert_eq!(debugger.state().current_context.current_step, 1);
    assert_eq!(debugger.state().history.len(), 1);
}

#[test]
fn test_step_multiple() {
    let mut debugger = create_enabled_debugger();

    for i in 1..=5 {
        debugger.step_agent().unwrap();
        assert_eq!(debugger.state().step_count, i);
        assert_eq!(debugger.state().current_context.current_step, i);
    }

    assert_eq!(debugger.state().history.len(), 5);
}

#[test]
fn test_step_into() {
    let mut debugger = create_enabled_debugger();

    debugger.step_into().unwrap();

    assert_eq!(debugger.state().step_count, 1);
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
    assert!(debugger.state().statistics.total_steps > 0);
}

#[test]
fn test_step_over_same_depth() {
    let mut debugger = create_enabled_debugger();
    let initial_depth = debugger.state().current_context.stack_depth;

    debugger.step_over().unwrap();

    // Should remain at same or shallower depth
    assert!(debugger.state().current_context.stack_depth <= initial_depth);
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
}

#[test]
fn test_step_over_with_nested_calls() {
    let mut debugger = create_enabled_debugger();

    // Push a call frame to create depth
    debugger.push_call_frame("outer".to_string(), WorkflowState::Running);
    let depth_before = debugger.state().current_context.stack_depth;
    assert_eq!(depth_before, 1);

    // Push another frame (simulating nested call)
    debugger.push_call_frame("inner".to_string(), WorkflowState::Running);
    assert_eq!(debugger.state().current_context.stack_depth, 2);

    // Step over should return to original depth
    debugger.step_over().unwrap();
    assert!(debugger.state().current_context.stack_depth <= depth_before);
}

#[test]
fn test_step_out_at_top_level() {
    let mut debugger = create_enabled_debugger();
    assert_eq!(debugger.state().current_context.stack_depth, 0);

    // Step out at top level should just step once
    debugger.step_out().unwrap();

    assert_eq!(debugger.state().step_count, 1);
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
}

#[test]
fn test_step_out_from_nested_call() {
    let mut debugger = create_enabled_debugger();

    // Push multiple call frames
    debugger.push_call_frame("frame1".to_string(), WorkflowState::Running);
    debugger.push_call_frame("frame2".to_string(), WorkflowState::Running);
    debugger.push_call_frame("frame3".to_string(), WorkflowState::Running);
    assert_eq!(debugger.state().current_context.stack_depth, 3);

    // Step out should reduce depth
    debugger.step_out().unwrap();
    assert!(debugger.state().current_context.stack_depth < 3);
    assert_eq!(debugger.state().execution_state, ExecutionState::Paused);
}

#[test]
fn test_step_without_debug_enabled() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    let result = debugger.step_agent();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DebuggerError::NotEnabled));
}

#[test]
fn test_step_callbacks() {
    let mut debugger = create_enabled_debugger();

    let step_called = Arc::new(Mutex::new(false));
    let step_called_clone = step_called.clone();
    debugger.on_step(move |_snapshot| {
        *step_called_clone.lock().unwrap() = true;
    });

    debugger.step_agent().unwrap();
    assert!(*step_called.lock().unwrap());
}

// ============================================================================
// CONTINUE EXECUTION TESTS
// ============================================================================

#[test]
fn test_continue_execution() {
    let mut debugger = create_paused_debugger();

    debugger.continue_execution().unwrap();

    assert_eq!(debugger.state().execution_state, ExecutionState::Continuing);
}

#[test]
fn test_continue_without_debug_enabled() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    let result = debugger.continue_execution();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DebuggerError::NotEnabled));
}

// ============================================================================
// BREAKPOINT MANAGEMENT TESTS
// ============================================================================

#[test]
fn test_add_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 5 });
    let bp_id = debugger.state_mut().add_breakpoint(bp);

    assert_eq!(debugger.state().breakpoints.len(), 1);
    assert_eq!(debugger.state().statistics.breakpoints_set, 1);

    let added_bp = &debugger.state().breakpoints[0];
    assert_eq!(added_bp.id, bp_id);
    assert!(added_bp.enabled);
    assert_eq!(added_bp.hit_count, 0);
}

#[test]
fn test_add_multiple_breakpoints() {
    let mut debugger = create_enabled_debugger();

    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 5 }));
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::WorkflowState {
            state: WorkflowState::Running,
        }));
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::AnyTransition));

    assert_eq!(debugger.state().breakpoints.len(), 3);
}

#[test]
fn test_remove_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 5 });
    let bp_id = debugger.state_mut().add_breakpoint(bp);

    assert_eq!(debugger.state().breakpoints.len(), 1);

    debugger.state_mut().remove_breakpoint(&bp_id).unwrap();

    assert_eq!(debugger.state().breakpoints.len(), 0);
}

#[test]
fn test_remove_nonexistent_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let fake_id = Uuid::new_v4();
    let result = debugger.state_mut().remove_breakpoint(&fake_id);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DebuggerError::BreakpointNotFound(_)
    ));
}

#[test]
fn test_toggle_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let bp = Breakpoint::new(BreakpointLocation::AnyTransition);
    let bp_id = debugger.state_mut().add_breakpoint(bp);

    // Initially enabled
    assert!(debugger.state().breakpoints[0].enabled);

    // Toggle to disabled
    debugger.state_mut().toggle_breakpoint(&bp_id).unwrap();
    assert!(!debugger.state().breakpoints[0].enabled);

    // Toggle back to enabled
    debugger.state_mut().toggle_breakpoint(&bp_id).unwrap();
    assert!(debugger.state().breakpoints[0].enabled);
}

#[test]
fn test_toggle_nonexistent_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let fake_id = Uuid::new_v4();
    let result = debugger.state_mut().toggle_breakpoint(&fake_id);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DebuggerError::BreakpointNotFound(_)
    ));
}

#[test]
fn test_breakpoint_enable_disable() {
    let mut bp = Breakpoint::new(BreakpointLocation::AnyTransition);

    assert!(bp.enabled);

    bp.disable();
    assert!(!bp.enabled);

    bp.enable();
    assert!(bp.enabled);

    bp.toggle();
    assert!(!bp.enabled);

    bp.toggle();
    assert!(bp.enabled);
}

#[test]
fn test_breakpoint_with_condition() {
    let bp = Breakpoint::with_condition(
        BreakpointLocation::StepCount { step: 10 },
        "x > 5".to_string(),
    );

    assert_eq!(bp.condition, Some("x > 5".to_string()));
}

#[test]
fn test_breakpoint_with_description() {
    let bp = Breakpoint::with_description(
        BreakpointLocation::StepCount { step: 10 },
        "Important checkpoint".to_string(),
    );

    assert_eq!(bp.description, Some("Important checkpoint".to_string()));
}

// ============================================================================
// BREAKPOINT TRIGGERING TESTS
// ============================================================================

#[test]
fn test_breakpoint_trigger_on_step_count() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint at step 3
    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 3 });
    debugger.state_mut().add_breakpoint(bp);

    // Execute steps
    debugger.state_mut().step().unwrap(); // step 1
    assert!(debugger.state_mut().check_breakpoints().is_none());

    debugger.state_mut().step().unwrap(); // step 2
    assert!(debugger.state_mut().check_breakpoints().is_none());

    debugger.state_mut().step().unwrap(); // step 3
    let triggered = debugger.state_mut().check_breakpoints();
    assert!(triggered.is_some());
    assert_eq!(triggered.unwrap().hit_count, 1);
}

#[test]
fn test_breakpoint_trigger_on_workflow_state() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint on Running state
    let bp = Breakpoint::new(BreakpointLocation::WorkflowState {
        state: WorkflowState::Running,
    });
    debugger.state_mut().add_breakpoint(bp);

    // Update workflow state to Running
    debugger.update_workflow_state(WorkflowState::Running);

    // Check breakpoint
    let triggered = debugger.state_mut().check_breakpoints();
    assert!(triggered.is_some());
}

#[test]
fn test_breakpoint_trigger_on_any_transition() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint on any transition
    let bp = Breakpoint::new(BreakpointLocation::AnyTransition);
    debugger.state_mut().add_breakpoint(bp);

    // Any state should trigger
    let triggered = debugger.state_mut().check_breakpoints();
    assert!(triggered.is_some());
}

#[test]
fn test_disabled_breakpoint_no_trigger() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint at step 1
    let mut bp = Breakpoint::new(BreakpointLocation::StepCount { step: 1 });
    bp.disable();
    debugger.state_mut().add_breakpoint(bp);

    // Step to breakpoint location
    debugger.state_mut().step().unwrap();

    // Should not trigger because disabled
    let triggered = debugger.state_mut().check_breakpoints();
    assert!(triggered.is_none());
}

#[test]
fn test_multiple_breakpoints_at_same_location() {
    let mut debugger = create_enabled_debugger();

    // Add multiple breakpoints at step 1
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::with_description(
            BreakpointLocation::StepCount { step: 1 },
            "First breakpoint".to_string(),
        ));
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::with_description(
            BreakpointLocation::StepCount { step: 1 },
            "Second breakpoint".to_string(),
        ));

    debugger.state_mut().step().unwrap();

    // Should trigger the first matching breakpoint
    let triggered = debugger.state_mut().check_breakpoints();
    assert!(triggered.is_some());
}

#[test]
fn test_breakpoint_hit_count() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint on any transition
    let bp = Breakpoint::new(BreakpointLocation::AnyTransition);
    let bp_id = bp.id;
    debugger.state_mut().add_breakpoint(bp);

    // Trigger multiple times
    for _ in 0..5 {
        debugger.state_mut().step().unwrap();
        debugger.state_mut().check_breakpoints();
    }

    // Check hit count
    let bp = debugger
        .state()
        .breakpoints
        .iter()
        .find(|b| b.id == bp_id)
        .unwrap();
    assert_eq!(bp.hit_count, 5);
}

#[test]
fn test_breakpoint_on_stack_depth() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint at stack depth 2
    let bp = Breakpoint::new(BreakpointLocation::StackDepth { depth: 2 });
    debugger.state_mut().add_breakpoint(bp);

    // No trigger at depth 0
    assert!(debugger.state_mut().check_breakpoints().is_none());

    // Push frames to depth 2
    debugger.push_call_frame("frame1".to_string(), WorkflowState::Running);
    assert!(debugger.state_mut().check_breakpoints().is_none());

    debugger.push_call_frame("frame2".to_string(), WorkflowState::Running);
    assert!(debugger.state_mut().check_breakpoints().is_some());
}

#[test]
fn test_breakpoint_callbacks() {
    let mut debugger = create_enabled_debugger();

    let breakpoint_hit = Arc::new(Mutex::new(false));
    let breakpoint_hit_clone = breakpoint_hit.clone();

    debugger.on_breakpoint(move |bp, ctx| {
        *breakpoint_hit_clone.lock().unwrap() = true;
    });

    // Add and trigger breakpoint
    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 1 });
    debugger.state_mut().add_breakpoint(bp);

    debugger.state_mut().step().unwrap();
    let _ = debugger.check_and_handle_breakpoints();

    assert!(*breakpoint_hit.lock().unwrap());
}

// ============================================================================
// CONTEXT TRACKING TESTS
// ============================================================================

#[test]
fn test_thought_snapshot_capture() {
    let mut debugger = create_enabled_debugger();

    let snapshot = debugger.capture_thought_snapshot(
        "thought-123".to_string(),
        "This is a test thought".to_string(),
    );

    assert_eq!(snapshot.thought_id, "thought-123");
    assert_eq!(snapshot.content, "This is a test thought");
    assert_eq!(snapshot.step_number, 0);
    assert!(debugger.state().current_thought.is_some());
}

#[test]
fn test_thought_snapshot_summary() {
    let long_content = "a".repeat(150);
    let snapshot = ThoughtSnapshot::new("test".to_string(), long_content, 0, None);

    let summary = snapshot.summary();
    assert!(summary.len() <= 103); // 100 + "..."
    assert!(summary.ends_with("..."));
}

#[test]
fn test_context_snapshot_capture() {
    let mut debugger = create_enabled_debugger();

    debugger.update_workflow_state(WorkflowState::Running);
    debugger.set_context_variable("test_var".to_string(), serde_json::json!(42));

    let context = debugger.capture_context_snapshot();

    assert_eq!(context.workflow_state, WorkflowState::Running);
    assert_eq!(
        context.local_variables.get("test_var"),
        Some(&serde_json::json!(42))
    );
}

#[test]
fn test_call_frame_management() {
    let mut debugger = create_enabled_debugger();

    assert_eq!(debugger.state().current_context.stack_depth, 0);

    // Push frames
    let frame1_id = debugger.push_call_frame("func1".to_string(), WorkflowState::Running);
    assert_eq!(debugger.state().current_context.stack_depth, 1);

    let frame2_id = debugger.push_call_frame("func2".to_string(), WorkflowState::Running);
    assert_eq!(debugger.state().current_context.stack_depth, 2);

    // Current frame should be func2
    let current = debugger.state().current_context.current_frame();
    assert!(current.is_some());
    assert_eq!(current.unwrap().name, "func2");

    // Pop frames
    let popped = debugger.pop_call_frame();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().name, "func2");
    assert_eq!(debugger.state().current_context.stack_depth, 1);

    debugger.pop_call_frame();
    assert_eq!(debugger.state().current_context.stack_depth, 0);
}

#[test]
fn test_call_frame_variables() {
    let mut frame = CallFrame::new("test_func".to_string(), WorkflowState::Running, 0, None);

    frame.set_variable("x".to_string(), serde_json::json!(10));
    frame.set_variable("y".to_string(), serde_json::json!("hello"));

    assert_eq!(frame.get_variable("x"), Some(&serde_json::json!(10)));
    assert_eq!(frame.get_variable("y"), Some(&serde_json::json!("hello")));
    assert_eq!(frame.get_variable("z"), None);
}

#[test]
fn test_debug_context_stack_trace() {
    let agent_id = Uuid::new_v4();
    let mut context = DebugContext::new(agent_id, WorkflowState::Idle);

    context.push_frame(CallFrame::new(
        "main".to_string(),
        WorkflowState::Running,
        0,
        None,
    ));
    context.push_frame(CallFrame::new(
        "helper".to_string(),
        WorkflowState::Running,
        5,
        None,
    ));

    let trace = context.format_stack_trace();
    assert!(trace.contains("main"));
    assert!(trace.contains("helper"));
    assert!(trace.contains("Call Stack"));
}

#[test]
fn test_context_variable_management() {
    let mut debugger = create_enabled_debugger();

    debugger.set_context_variable("var1".to_string(), serde_json::json!(100));
    debugger.set_context_variable("var2".to_string(), serde_json::json!("test"));

    let context = &debugger.state().current_context;
    assert_eq!(
        context.local_variables.get("var1"),
        Some(&serde_json::json!(100))
    );
    assert_eq!(
        context.local_variables.get("var2"),
        Some(&serde_json::json!("test"))
    );
}

#[test]
fn test_workflow_state_updates() {
    let mut debugger = create_enabled_debugger();

    assert_eq!(
        debugger.state().current_context.workflow_state,
        WorkflowState::Idle
    );

    debugger.update_workflow_state(WorkflowState::Running);
    assert_eq!(
        debugger.state().current_context.workflow_state,
        WorkflowState::Running
    );

    debugger.update_workflow_state(WorkflowState::Paused);
    assert_eq!(
        debugger.state().current_context.workflow_state,
        WorkflowState::Paused
    );
}

// ============================================================================
// COMMAND PROCESSING TESTS
// ============================================================================

#[test]
fn test_process_command_enable() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    let result = debugger.process_command(DebugCommand::Enable);
    assert!(result.is_ok());
    assert!(debugger.state().is_enabled());

    match result.unwrap() {
        CommandResult::Success { message } => {
            assert!(message.contains("enabled"));
        }
        _ => panic!("Expected Success result"),
    }
}

#[test]
fn test_process_command_disable() {
    let mut debugger = create_enabled_debugger();

    let result = debugger.process_command(DebugCommand::Disable);
    assert!(result.is_ok());
    assert!(!debugger.state().is_enabled());
}

#[test]
fn test_process_command_pause() {
    let mut debugger = create_enabled_debugger();

    let result = debugger.process_command(DebugCommand::Pause);
    assert!(result.is_ok());
    assert!(debugger.should_pause());
}

#[test]
fn test_process_command_resume() {
    let mut debugger = create_paused_debugger();

    let result = debugger.process_command(DebugCommand::Resume);
    assert!(result.is_ok());
    assert!(!debugger.should_pause());
}

#[test]
fn test_process_command_step() {
    let mut debugger = create_enabled_debugger();

    let result = debugger.process_command(DebugCommand::Step);
    assert!(result.is_ok());

    match result.unwrap() {
        CommandResult::StepComplete { step_number, .. } => {
            assert_eq!(step_number, 1);
        }
        _ => panic!("Expected StepComplete result"),
    }
}

#[test]
fn test_process_command_set_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let location = BreakpointLocation::StepCount { step: 10 };
    let result = debugger.process_command(DebugCommand::SetBreakpoint {
        location: location.clone(),
        condition: None,
    });

    assert!(result.is_ok());
    assert_eq!(debugger.state().breakpoints.len(), 1);

    match result.unwrap() {
        CommandResult::BreakpointSet { breakpoint_id, .. } => {
            assert_ne!(breakpoint_id, Uuid::nil());
        }
        _ => panic!("Expected BreakpointSet result"),
    }
}

#[test]
fn test_process_command_remove_breakpoint() {
    let mut debugger = create_enabled_debugger();

    let bp = Breakpoint::new(BreakpointLocation::AnyTransition);
    let bp_id = debugger.state_mut().add_breakpoint(bp);

    let result = debugger.process_command(DebugCommand::RemoveBreakpoint { id: bp_id });
    assert!(result.is_ok());
    assert_eq!(debugger.state().breakpoints.len(), 0);
}

#[test]
fn test_process_command_list_breakpoints() {
    let mut debugger = create_enabled_debugger();

    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::AnyTransition));
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 5 }));

    let result = debugger.process_command(DebugCommand::ListBreakpoints);
    assert!(result.is_ok());

    match result.unwrap() {
        CommandResult::BreakpointList { breakpoints } => {
            assert_eq!(breakpoints.len(), 2);
        }
        _ => panic!("Expected BreakpointList result"),
    }
}

#[test]
fn test_process_command_inspect_context() {
    let mut debugger = create_enabled_debugger();

    debugger.set_context_variable("test".to_string(), serde_json::json!("value"));

    let result = debugger.process_command(DebugCommand::InspectContext);
    assert!(result.is_ok());

    match result.unwrap() {
        CommandResult::ContextInspection { context } => {
            assert!(context.local_variables.contains_key("test"));
        }
        _ => panic!("Expected ContextInspection result"),
    }
}

#[test]
fn test_process_command_show_stack() {
    let mut debugger = create_enabled_debugger();

    debugger.push_call_frame("main".to_string(), WorkflowState::Running);
    debugger.push_call_frame("helper".to_string(), WorkflowState::Running);

    let result = debugger.process_command(DebugCommand::ShowStack);
    assert!(result.is_ok());

    match result.unwrap() {
        CommandResult::StackTrace { trace, frames } => {
            assert_eq!(frames.len(), 2);
            assert!(trace.contains("main"));
            assert!(trace.contains("helper"));
        }
        _ => panic!("Expected StackTrace result"),
    }
}

#[test]
fn test_process_command_get_statistics() {
    let mut debugger = create_enabled_debugger();

    // step_agent() puts debugger in Paused state, so we need to
    // resume before pause to actually increment the pause counter
    debugger.step_agent().unwrap();
    debugger.resume_agent().unwrap();
    debugger.pause_agent().unwrap();

    let result = debugger.process_command(DebugCommand::GetStatistics);
    assert!(result.is_ok());

    match result.unwrap() {
        CommandResult::Statistics { stats } => {
            assert_eq!(stats.sessions_started, 1);
            assert!(stats.total_steps > 0);
            assert!(stats.pauses > 0);
        }
        _ => panic!("Expected Statistics result"),
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_breakpoint_in_nonexistent_location() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoint at step 100
    let bp = Breakpoint::new(BreakpointLocation::StepCount { step: 100 });
    debugger.state_mut().add_breakpoint(bp);

    // Execute only 5 steps
    for _ in 0..5 {
        debugger.state_mut().step().unwrap();
        assert!(debugger.state_mut().check_breakpoints().is_none());
    }
}

#[test]
fn test_stepping_when_at_end() {
    let mut debugger = create_enabled_debugger();

    // Step multiple times (no error should occur)
    for _ in 0..100 {
        let result = debugger.step_agent();
        assert!(result.is_ok());
    }

    assert_eq!(debugger.state().step_count, 100);
}

#[test]
fn test_operations_on_disabled_debugger() {
    let agent_id = Uuid::new_v4();
    let mut debugger = Debugger::new(agent_id);

    // All operations should fail when debugger is disabled
    assert!(debugger.pause_agent().is_err());
    assert!(debugger.step_agent().is_err());
    assert!(debugger.step_into().is_err());
    assert!(debugger.step_over().is_err());
    assert!(debugger.step_out().is_err());
    assert!(debugger.continue_execution().is_err());
}

#[test]
fn test_serialization_deserialization() {
    let agent_id = Uuid::new_v4();
    let mut state = DebuggerState::new(agent_id);
    state.enable();
    state.add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 5 }));

    let json = state.to_json().unwrap();
    let deserialized = DebuggerState::from_json(&json).unwrap();

    assert_eq!(state.debug_mode, deserialized.debug_mode);
    assert_eq!(state.step_count, deserialized.step_count);
    assert_eq!(state.breakpoints.len(), deserialized.breakpoints.len());
}

#[test]
fn test_empty_history_navigation() {
    let mut debugger = create_enabled_debugger();

    let result = debugger.state_mut().goto_history(0);
    assert!(result.is_err());
}

#[test]
fn test_max_history_wraparound() {
    let mut debugger = create_enabled_debugger();
    debugger.state_mut().max_history_size = 5;

    // Fill beyond max
    for _ in 0..10 {
        debugger.state_mut().step().unwrap();
    }

    assert_eq!(debugger.state().history.len(), 5);

    // Latest entries should be kept
    let last_snapshot = debugger.state().history.last().unwrap();
    assert_eq!(last_snapshot.step_number, 10);
}

#[test]
fn test_concurrent_breakpoint_modifications() {
    let mut debugger = create_enabled_debugger();

    // Add multiple breakpoints
    let id1 = debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 1 }));
    let id2 = debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 2 }));
    let id3 = debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 3 }));

    assert_eq!(debugger.state().breakpoints.len(), 3);

    // Remove middle one
    debugger.state_mut().remove_breakpoint(&id2).unwrap();
    assert_eq!(debugger.state().breakpoints.len(), 2);

    // Remaining should be id1 and id3
    let ids: Vec<Uuid> = debugger
        .state()
        .breakpoints
        .iter()
        .map(|bp| bp.id)
        .collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id3));
    assert!(!ids.contains(&id2));
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_full_debugging_session() {
    let mut debugger = create_enabled_debugger();

    // Set breakpoints
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 3 }));
    debugger
        .state_mut()
        .add_breakpoint(Breakpoint::new(BreakpointLocation::StepCount { step: 7 }));

    // Run until first breakpoint
    for i in 1..=3 {
        debugger.state_mut().step().unwrap();
        if i == 3 {
            assert!(debugger.state_mut().check_breakpoints().is_some());
        }
    }

    // Continue to next breakpoint
    for i in 4..=7 {
        debugger.state_mut().step().unwrap();
        if i == 7 {
            assert!(debugger.state_mut().check_breakpoints().is_some());
        }
    }

    assert_eq!(debugger.state().step_count, 7);
    assert_eq!(debugger.state().statistics.breakpoints_hit, 2);
}

#[test]
fn test_debugging_with_call_stack() {
    let mut debugger = create_enabled_debugger();

    // Simulate nested function calls
    debugger.push_call_frame("main".to_string(), WorkflowState::Running);
    debugger.set_context_variable("main_var".to_string(), serde_json::json!(1));

    debugger.push_call_frame("helper".to_string(), WorkflowState::Running);
    debugger.set_context_variable("helper_var".to_string(), serde_json::json!(2));

    debugger.push_call_frame("inner".to_string(), WorkflowState::Running);

    // Step through
    debugger.step_agent().unwrap();

    // Verify context
    let context = debugger.capture_context_snapshot();
    assert_eq!(context.stack_depth, 3);
    assert_eq!(context.call_stack.len(), 3);

    // Step out
    debugger.step_out().unwrap();
    assert!(debugger.state().current_context.stack_depth < 3);
}

#[test]
fn test_thought_tracking_during_execution() {
    let mut debugger = create_enabled_debugger();

    // Capture thoughts at different steps
    debugger.capture_thought_snapshot("thought-1".to_string(), "First thought".to_string());
    debugger.step_agent().unwrap();

    debugger.capture_thought_snapshot("thought-2".to_string(), "Second thought".to_string());
    debugger.step_agent().unwrap();

    // History should contain snapshots with thoughts
    assert_eq!(debugger.state().history.len(), 2);
}

#[test]
fn test_statistics_tracking() {
    let mut debugger = create_enabled_debugger();

    // Perform various operations
    // Note: step_agent() puts debugger into Paused state at the end,
    // so pause_agent() called after step_agent() won't increment pause count
    // (already paused). We need to resume first to properly test pause.
    debugger.step_agent().unwrap();
    debugger.step_agent().unwrap();
    debugger.resume_agent().unwrap(); // Resume so we can actually pause
    debugger.pause_agent().unwrap();  // This should now increment pauses
    debugger.resume_agent().unwrap();
    debugger.step_agent().unwrap();

    let stats = &debugger.state().statistics;
    assert_eq!(stats.sessions_started, 1);
    assert!(stats.total_steps >= 3);
    assert_eq!(stats.pauses, 1);
    assert_eq!(stats.resumes, 2);
}

#[test]
fn test_error_recovery() {
    let mut debugger = create_enabled_debugger();

    // Try invalid operation
    let result = debugger.resume_agent();
    assert!(result.is_err());

    // Debugger should still be functional
    debugger.pause_agent().unwrap();
    debugger.resume_agent().unwrap();
    assert!(debugger.state().is_enabled());
}
