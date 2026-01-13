//! Comprehensive Debugger UI Tests for Phase 3:6.4
//!
//! This module provides extensive test coverage for the debugger UI components,
//! including:
//! - UI state management tests
//! - Message handling tests
//! - UI component rendering tests
//! - UI-to-command translation tests
//! - Panel visibility and interaction tests
//! - Breakpoint form tests
//!
//! Note: These are unit tests for the UI logic. Integration tests with actual
//! rendering would require a different testing approach with iced test utilities.

use descartes_core::debugger::{
    Breakpoint, BreakpointLocation, DebugCommand, DebuggerState, ExecutionState, ThoughtSnapshot,
};
use descartes_core::state_machine::WorkflowState;
use descartes_gui::debugger_ui::*;
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a default UI state for testing
fn create_test_ui_state() -> DebuggerUiState {
    DebuggerUiState::default()
}

/// Create a connected UI state with debugger
fn create_connected_ui_state() -> DebuggerUiState {
    let agent_id = Uuid::new_v4();
    let mut state = DebuggerUiState::default();
    state.agent_id = Some(agent_id);
    state.connected = true;
    state.debugger_state = Some(DebuggerState::new(agent_id));
    state
}

/// Create a UI state with enabled debugger
fn create_enabled_ui_state() -> DebuggerUiState {
    let mut state = create_connected_ui_state();
    if let Some(ref mut debugger) = state.debugger_state {
        debugger.enable();
    }
    state
}

/// Create a UI state with paused debugger
fn create_paused_ui_state() -> DebuggerUiState {
    let mut state = create_enabled_ui_state();
    if let Some(ref mut debugger) = state.debugger_state {
        debugger.execution_state = ExecutionState::Paused;
    }
    state
}

// ============================================================================
// UI STATE MANAGEMENT TESTS
// ============================================================================

#[test]
fn test_ui_state_default() {
    let state = DebuggerUiState::default();

    assert_eq!(state.debugger_state, None);
    assert!(!state.connected);
    assert_eq!(state.agent_id, None);
    assert!(state.show_call_stack);
    assert!(state.show_variables);
    assert!(state.show_breakpoints);
    assert!(state.show_thought_view);
    assert_eq!(state.thought_context_split, 0.5);
    assert_eq!(state.context_tab, ContextTab::Variables);
}

#[test]
fn test_ui_settings_default() {
    let settings = DebuggerUiSettings::default();

    assert!(settings.show_line_numbers);
    assert!(settings.syntax_highlighting);
    assert!(settings.auto_scroll);
    assert!(!settings.compact_mode);
    assert!(settings.dark_mode);
}

#[test]
fn test_breakpoint_form_default() {
    let form = BreakpointFormState::default();

    assert_eq!(form.location_type, BreakpointLocationType::StepCount);
    assert!(form.step_count.is_empty());
    assert!(form.agent_id.is_empty());
    assert!(form.condition.is_empty());
    assert!(form.description.is_empty());
    assert!(!form.show_form);
}

// ============================================================================
// CONNECTION TESTS
// ============================================================================

#[test]
fn test_connect_to_agent() {
    let mut state = create_test_ui_state();
    let agent_id = Uuid::new_v4();

    let command = update(&mut state, DebuggerMessage::ConnectToAgent(agent_id));

    assert!(state.connected);
    assert_eq!(state.agent_id, Some(agent_id));
    assert!(state.debugger_state.is_some());
    assert_eq!(command, None); // Connection doesn't generate a command
}

#[test]
fn test_disconnect() {
    let mut state = create_connected_ui_state();

    let command = update(&mut state, DebuggerMessage::Disconnect);

    assert!(!state.connected);
    assert_eq!(state.agent_id, None);
    assert_eq!(state.debugger_state, None);
    assert_eq!(command, None);
}

#[test]
fn test_debugger_state_updated() {
    let mut state = create_test_ui_state();
    let agent_id = Uuid::new_v4();
    let new_debugger_state = DebuggerState::new(agent_id);

    let command = update(
        &mut state,
        DebuggerMessage::DebuggerStateUpdated(new_debugger_state.clone()),
    );

    assert!(state.debugger_state.is_some());
    assert_eq!(command, None);
}

// ============================================================================
// CONTROL COMMAND TESTS
// ============================================================================

#[test]
fn test_pause_command() {
    let mut state = create_enabled_ui_state();

    let command = update(&mut state, DebuggerMessage::Pause);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::Pause => {} // Expected
        _ => panic!("Expected Pause command"),
    }
}

#[test]
fn test_resume_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::Resume);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::Resume => {} // Expected
        _ => panic!("Expected Resume command"),
    }
}

#[test]
fn test_step_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::Step);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::Step => {} // Expected
        _ => panic!("Expected Step command"),
    }
}

#[test]
fn test_step_over_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::StepOver);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepOver => {} // Expected
        _ => panic!("Expected StepOver command"),
    }
}

#[test]
fn test_step_into_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::StepInto);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepInto => {} // Expected
        _ => panic!("Expected StepInto command"),
    }
}

#[test]
fn test_step_out_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::StepOut);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepOut => {} // Expected
        _ => panic!("Expected StepOut command"),
    }
}

#[test]
fn test_continue_command() {
    let mut state = create_paused_ui_state();

    let command = update(&mut state, DebuggerMessage::Continue);

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::Continue => {} // Expected
        _ => panic!("Expected Continue command"),
    }
}

// ============================================================================
// BREAKPOINT MANAGEMENT TESTS
// ============================================================================

#[test]
fn test_show_breakpoint_form() {
    let mut state = create_enabled_ui_state();

    assert!(!state.breakpoint_form.show_form);

    let command = update(&mut state, DebuggerMessage::ShowBreakpointForm);

    assert!(state.breakpoint_form.show_form);
    assert_eq!(command, None);
}

#[test]
fn test_hide_breakpoint_form() {
    let mut state = create_enabled_ui_state();
    state.breakpoint_form.show_form = true;

    let command = update(&mut state, DebuggerMessage::HideBreakpointForm);

    assert!(!state.breakpoint_form.show_form);
    assert_eq!(command, None);
}

#[test]
fn test_add_breakpoint() {
    let mut state = create_enabled_ui_state();
    state.breakpoint_form.show_form = true;
    state.breakpoint_form.step_count = "5".to_string();

    let command = update(&mut state, DebuggerMessage::AddBreakpoint);

    assert!(!state.breakpoint_form.show_form); // Form should be hidden after adding
    assert!(command.is_some());

    match command.unwrap() {
        DebugCommand::SetBreakpoint {
            location,
            condition,
        } => {
            match location {
                BreakpointLocation::StepCount { step } => {
                    assert_eq!(step, 5);
                }
                _ => panic!("Expected StepCount location"),
            }
            assert_eq!(condition, None);
        }
        _ => panic!("Expected SetBreakpoint command"),
    }
}

#[test]
fn test_add_breakpoint_invalid_step() {
    let mut state = create_enabled_ui_state();
    state.breakpoint_form.show_form = true;
    state.breakpoint_form.step_count = "invalid".to_string();

    let command = update(&mut state, DebuggerMessage::AddBreakpoint);

    // Should default to step 1 for invalid input
    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::SetBreakpoint { location, .. } => match location {
            BreakpointLocation::StepCount { step } => {
                assert_eq!(step, 1); // Defaults to 1
            }
            _ => panic!("Expected StepCount location"),
        },
        _ => panic!("Expected SetBreakpoint command"),
    }
}

#[test]
fn test_remove_breakpoint() {
    let mut state = create_enabled_ui_state();
    let bp_id = Uuid::new_v4();

    let command = update(&mut state, DebuggerMessage::RemoveBreakpoint(bp_id));

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::RemoveBreakpoint { id } => {
            assert_eq!(id, bp_id);
        }
        _ => panic!("Expected RemoveBreakpoint command"),
    }
}

#[test]
fn test_toggle_breakpoint() {
    let mut state = create_enabled_ui_state();
    let bp_id = Uuid::new_v4();

    let command = update(&mut state, DebuggerMessage::ToggleBreakpoint(bp_id));

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::ToggleBreakpoint { id } => {
            assert_eq!(id, bp_id);
        }
        _ => panic!("Expected ToggleBreakpoint command"),
    }
}

#[test]
fn test_breakpoint_hovered() {
    let mut state = create_enabled_ui_state();
    let bp_id = Uuid::new_v4();

    assert_eq!(state.hovered_breakpoint, None);

    let command = update(&mut state, DebuggerMessage::BreakpointHovered(Some(bp_id)));

    assert_eq!(state.hovered_breakpoint, Some(bp_id));
    assert_eq!(command, None);

    // Unhover
    let command = update(&mut state, DebuggerMessage::BreakpointHovered(None));
    assert_eq!(state.hovered_breakpoint, None);
    assert_eq!(command, None);
}

#[test]
fn test_update_breakpoint_form_step_count() {
    let mut state = create_enabled_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::UpdateBreakpointForm(BreakpointFormField::StepCount("10".to_string())),
    );

    assert_eq!(state.breakpoint_form.step_count, "10");
    assert_eq!(command, None);
}

#[test]
fn test_update_breakpoint_form_location_type() {
    let mut state = create_enabled_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::UpdateBreakpointForm(BreakpointFormField::LocationType(
            BreakpointLocationType::AnyTransition,
        )),
    );

    assert_eq!(
        state.breakpoint_form.location_type,
        BreakpointLocationType::AnyTransition
    );
    assert_eq!(command, None);
}

#[test]
fn test_update_breakpoint_form_condition() {
    let mut state = create_enabled_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::UpdateBreakpointForm(BreakpointFormField::Condition("x > 5".to_string())),
    );

    assert_eq!(state.breakpoint_form.condition, "x > 5");
    assert_eq!(command, None);
}

#[test]
fn test_update_breakpoint_form_description() {
    let mut state = create_enabled_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::UpdateBreakpointForm(BreakpointFormField::Description(
            "Test breakpoint".to_string(),
        )),
    );

    assert_eq!(state.breakpoint_form.description, "Test breakpoint");
    assert_eq!(command, None);
}

// ============================================================================
// NAVIGATION TESTS
// ============================================================================

#[test]
fn test_goto_history() {
    let mut state = create_enabled_ui_state();

    let command = update(&mut state, DebuggerMessage::GoToHistory(5));

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::GotoHistory { index } => {
            assert_eq!(index, 5);
        }
        _ => panic!("Expected GotoHistory command"),
    }
}

#[test]
fn test_select_call_frame() {
    let mut state = create_enabled_ui_state();

    let command = update(&mut state, DebuggerMessage::SelectCallFrame(2));

    // Currently doesn't generate a command
    assert_eq!(command, None);
}

// ============================================================================
// UI CONTROL TESTS
// ============================================================================

#[test]
fn test_toggle_call_stack() {
    let mut state = create_enabled_ui_state();

    assert!(state.show_call_stack);

    let command = update(&mut state, DebuggerMessage::ToggleCallStack);

    assert!(!state.show_call_stack);
    assert_eq!(command, None);

    let command = update(&mut state, DebuggerMessage::ToggleCallStack);

    assert!(state.show_call_stack);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_variables() {
    let mut state = create_enabled_ui_state();

    assert!(state.show_variables);

    let command = update(&mut state, DebuggerMessage::ToggleVariables);

    assert!(!state.show_variables);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_breakpoints() {
    let mut state = create_enabled_ui_state();

    assert!(state.show_breakpoints);

    let command = update(&mut state, DebuggerMessage::ToggleBreakpoints);

    assert!(!state.show_breakpoints);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_thought_view() {
    let mut state = create_enabled_ui_state();

    assert!(state.show_thought_view);

    let command = update(&mut state, DebuggerMessage::ToggleThoughtView);

    assert!(!state.show_thought_view);
    assert_eq!(command, None);
}

#[test]
fn test_set_context_tab() {
    let mut state = create_enabled_ui_state();

    assert_eq!(state.context_tab, ContextTab::Variables);

    let command = update(
        &mut state,
        DebuggerMessage::SetContextTab(ContextTab::CallStack),
    );

    assert_eq!(state.context_tab, ContextTab::CallStack);
    assert_eq!(command, None);

    let command = update(
        &mut state,
        DebuggerMessage::SetContextTab(ContextTab::WorkflowState),
    );

    assert_eq!(state.context_tab, ContextTab::WorkflowState);
    assert_eq!(command, None);

    let command = update(
        &mut state,
        DebuggerMessage::SetContextTab(ContextTab::Metadata),
    );

    assert_eq!(state.context_tab, ContextTab::Metadata);
    assert_eq!(command, None);
}

#[test]
fn test_set_thought_context_split() {
    let mut state = create_enabled_ui_state();

    assert_eq!(state.thought_context_split, 0.5);

    let command = update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.7));

    assert_eq!(state.thought_context_split, 0.7);
    assert_eq!(command, None);

    // Test clamping
    let command = update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.9));

    assert_eq!(state.thought_context_split, 0.8); // Clamped to max
    assert_eq!(command, None);

    let command = update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.1));

    assert_eq!(state.thought_context_split, 0.2); // Clamped to min
    assert_eq!(command, None);
}

// ============================================================================
// SETTINGS TESTS
// ============================================================================

#[test]
fn test_toggle_show_line_numbers() {
    let mut state = create_enabled_ui_state();

    assert!(state.ui_settings.show_line_numbers);

    let command = update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::ShowLineNumbers),
    );

    assert!(!state.ui_settings.show_line_numbers);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_syntax_highlighting() {
    let mut state = create_enabled_ui_state();

    assert!(state.ui_settings.syntax_highlighting);

    let command = update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::SyntaxHighlighting),
    );

    assert!(!state.ui_settings.syntax_highlighting);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_auto_scroll() {
    let mut state = create_enabled_ui_state();

    assert!(state.ui_settings.auto_scroll);

    let command = update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::AutoScroll),
    );

    assert!(!state.ui_settings.auto_scroll);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_compact_mode() {
    let mut state = create_enabled_ui_state();

    assert!(!state.ui_settings.compact_mode);

    let command = update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::CompactMode),
    );

    assert!(state.ui_settings.compact_mode);
    assert_eq!(command, None);
}

#[test]
fn test_toggle_dark_mode() {
    let mut state = create_enabled_ui_state();

    assert!(state.ui_settings.dark_mode);

    let command = update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::DarkMode),
    );

    assert!(!state.ui_settings.dark_mode);
    assert_eq!(command, None);
}

// ============================================================================
// KEYBOARD SHORTCUT TESTS
// ============================================================================

#[test]
fn test_keyboard_shortcut_continue() {
    let mut state = create_paused_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::KeyboardShortcut(DebuggerKeyboardShortcut::Continue),
    );

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::Continue => {} // Expected
        _ => panic!("Expected Continue command"),
    }
}

#[test]
fn test_keyboard_shortcut_step_over() {
    let mut state = create_paused_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::KeyboardShortcut(DebuggerKeyboardShortcut::StepOver),
    );

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepOver => {} // Expected
        _ => panic!("Expected StepOver command"),
    }
}

#[test]
fn test_keyboard_shortcut_step_into() {
    let mut state = create_paused_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::KeyboardShortcut(DebuggerKeyboardShortcut::StepInto),
    );

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepInto => {} // Expected
        _ => panic!("Expected StepInto command"),
    }
}

#[test]
fn test_keyboard_shortcut_step_out() {
    let mut state = create_paused_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::KeyboardShortcut(DebuggerKeyboardShortcut::StepOut),
    );

    assert!(command.is_some());
    match command.unwrap() {
        DebugCommand::StepOut => {} // Expected
        _ => panic!("Expected StepOut command"),
    }
}

#[test]
fn test_keyboard_shortcut_toggle_breakpoint() {
    let mut state = create_enabled_ui_state();

    let command = update(
        &mut state,
        DebuggerMessage::KeyboardShortcut(DebuggerKeyboardShortcut::ToggleBreakpoint),
    );

    // Currently doesn't generate a command
    assert_eq!(command, None);
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_full_ui_workflow() {
    let mut state = create_test_ui_state();
    let agent_id = Uuid::new_v4();

    // Connect
    update(&mut state, DebuggerMessage::ConnectToAgent(agent_id));
    assert!(state.connected);

    // Update state to enabled and paused
    let mut debugger_state = DebuggerState::new(agent_id);
    debugger_state.enable();
    debugger_state.execution_state = ExecutionState::Paused;
    update(
        &mut state,
        DebuggerMessage::DebuggerStateUpdated(debugger_state),
    );

    // Add breakpoint
    state.breakpoint_form.step_count = "10".to_string();
    let cmd = update(&mut state, DebuggerMessage::AddBreakpoint);
    assert!(matches!(cmd, Some(DebugCommand::SetBreakpoint { .. })));

    // Step
    let cmd = update(&mut state, DebuggerMessage::Step);
    assert!(matches!(cmd, Some(DebugCommand::Step)));

    // Continue
    let cmd = update(&mut state, DebuggerMessage::Continue);
    assert!(matches!(cmd, Some(DebugCommand::Continue)));

    // Disconnect
    update(&mut state, DebuggerMessage::Disconnect);
    assert!(!state.connected);
}

#[test]
fn test_ui_state_with_multiple_toggles() {
    let mut state = create_enabled_ui_state();

    // Toggle various panels
    update(&mut state, DebuggerMessage::ToggleCallStack);
    update(&mut state, DebuggerMessage::ToggleVariables);
    update(&mut state, DebuggerMessage::ToggleBreakpoints);

    assert!(!state.show_call_stack);
    assert!(!state.show_variables);
    assert!(!state.show_breakpoints);

    // Toggle back
    update(&mut state, DebuggerMessage::ToggleCallStack);
    update(&mut state, DebuggerMessage::ToggleVariables);
    update(&mut state, DebuggerMessage::ToggleBreakpoints);

    assert!(state.show_call_stack);
    assert!(state.show_variables);
    assert!(state.show_breakpoints);
}

#[test]
fn test_ui_settings_persistence() {
    let mut state = create_enabled_ui_state();

    // Change multiple settings
    update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::ShowLineNumbers),
    );
    update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::SyntaxHighlighting),
    );
    update(
        &mut state,
        DebuggerMessage::ToggleSetting(UiSetting::CompactMode),
    );

    // Verify all changes persisted
    assert!(!state.ui_settings.show_line_numbers);
    assert!(!state.ui_settings.syntax_highlighting);
    assert!(state.ui_settings.compact_mode);
}

#[test]
fn test_context_tab_navigation() {
    let mut state = create_enabled_ui_state();

    // Navigate through all tabs
    let tabs = vec![
        ContextTab::Variables,
        ContextTab::CallStack,
        ContextTab::WorkflowState,
        ContextTab::Metadata,
    ];

    for tab in tabs {
        update(&mut state, DebuggerMessage::SetContextTab(tab));
        assert_eq!(state.context_tab, tab);
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_commands_on_disconnected_state() {
    let mut state = create_test_ui_state();

    // All commands should still be generated even when disconnected
    // (they'll be rejected by the backend)
    let cmd = update(&mut state, DebuggerMessage::Pause);
    assert!(matches!(cmd, Some(DebugCommand::Pause)));
}

#[test]
fn test_empty_breakpoint_form_submission() {
    let mut state = create_enabled_ui_state();
    state.breakpoint_form.step_count = "".to_string();

    let cmd = update(&mut state, DebuggerMessage::AddBreakpoint);

    // Should create breakpoint at step 1 (default)
    assert!(cmd.is_some());
}

#[test]
fn test_split_ratio_boundary_values() {
    let mut state = create_enabled_ui_state();

    // Test exact boundaries
    update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.2));
    assert_eq!(state.thought_context_split, 0.2);

    update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.8));
    assert_eq!(state.thought_context_split, 0.8);

    // Test outside boundaries
    update(&mut state, DebuggerMessage::SetThoughtContextSplit(0.0));
    assert_eq!(state.thought_context_split, 0.2); // Clamped

    update(&mut state, DebuggerMessage::SetThoughtContextSplit(1.0));
    assert_eq!(state.thought_context_split, 0.8); // Clamped
}

#[test]
fn test_rapid_state_changes() {
    let mut state = create_enabled_ui_state();

    // Rapidly change context tabs
    for _ in 0..100 {
        update(
            &mut state,
            DebuggerMessage::SetContextTab(ContextTab::Variables),
        );
        update(
            &mut state,
            DebuggerMessage::SetContextTab(ContextTab::CallStack),
        );
    }

    assert_eq!(state.context_tab, ContextTab::CallStack);
}

#[test]
fn test_concurrent_panel_toggles() {
    let mut state = create_enabled_ui_state();

    // Toggle all panels simultaneously
    update(&mut state, DebuggerMessage::ToggleCallStack);
    update(&mut state, DebuggerMessage::ToggleVariables);
    update(&mut state, DebuggerMessage::ToggleBreakpoints);
    update(&mut state, DebuggerMessage::ToggleThoughtView);

    assert!(!state.show_call_stack);
    assert!(!state.show_variables);
    assert!(!state.show_breakpoints);
    assert!(!state.show_thought_view);
}
