//! Debugger UI Components for Descartes Agent Orchestration
//!
//! This module provides comprehensive UI components for debugging agent execution,
//! including:
//! - Control panel with Pause, Resume, Step buttons
//! - Thought viewer for current agent thought
//! - Context viewer for workflow state and variables
//! - Call stack display
//! - Breakpoint management panel
//! - Integration with time-travel debugging
//!
//! Phase 3:6.3 - Build Iced UI Components

use descartes_core::debugger::{
    Breakpoint, BreakpointLocation, CallFrame, DebugCommand, DebugContext, DebuggerState,
    ExecutionState, ThoughtSnapshot,
};
use descartes_core::state_machine::WorkflowState;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, Column, Row, Space};
use iced::{
    alignment::{Horizontal, Vertical},
    border, Color, Element, Length, Theme,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Main debugger UI state
#[derive(Debug, Clone)]
pub struct DebuggerUiState {
    /// The underlying debugger state from core
    pub debugger_state: Option<DebuggerState>,

    /// Whether the debugger is connected to an agent
    pub connected: bool,

    /// Agent ID being debugged
    pub agent_id: Option<Uuid>,

    /// UI-specific settings
    pub ui_settings: DebuggerUiSettings,

    /// Currently hovered breakpoint (for highlighting)
    pub hovered_breakpoint: Option<Uuid>,

    /// Show/hide panels
    pub show_call_stack: bool,
    pub show_variables: bool,
    pub show_breakpoints: bool,
    pub show_thought_view: bool,

    /// Split panel sizes (percentage)
    pub thought_context_split: f32,

    /// Selected tab in context view
    pub context_tab: ContextTab,

    /// Breakpoint form state
    pub breakpoint_form: BreakpointFormState,
}

impl Default for DebuggerUiState {
    fn default() -> Self {
        Self {
            debugger_state: None,
            connected: false,
            agent_id: None,
            ui_settings: DebuggerUiSettings::default(),
            hovered_breakpoint: None,
            show_call_stack: true,
            show_variables: true,
            show_breakpoints: true,
            show_thought_view: true,
            thought_context_split: 0.5,
            context_tab: ContextTab::Variables,
            breakpoint_form: BreakpointFormState::default(),
        }
    }
}

/// UI-specific settings for the debugger
#[derive(Debug, Clone)]
pub struct DebuggerUiSettings {
    /// Show line numbers in thought view
    pub show_line_numbers: bool,

    /// Enable syntax highlighting
    pub syntax_highlighting: bool,

    /// Auto-scroll to current step
    pub auto_scroll: bool,

    /// Compact view mode
    pub compact_mode: bool,

    /// Theme preference
    pub dark_mode: bool,
}

impl Default for DebuggerUiSettings {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            syntax_highlighting: true,
            auto_scroll: true,
            compact_mode: false,
            dark_mode: true,
        }
    }
}

/// Tabs in the context view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextTab {
    Variables,
    CallStack,
    WorkflowState,
    Metadata,
}

/// State for the breakpoint creation form
#[derive(Debug, Clone)]
pub struct BreakpointFormState {
    pub location_type: BreakpointLocationType,
    pub step_count: String,
    pub agent_id: String,
    pub condition: String,
    pub description: String,
    pub show_form: bool,
}

impl Default for BreakpointFormState {
    fn default() -> Self {
        Self {
            location_type: BreakpointLocationType::StepCount,
            step_count: String::new(),
            agent_id: String::new(),
            condition: String::new(),
            description: String::new(),
            show_form: false,
        }
    }
}

/// Simplified breakpoint location types for UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointLocationType {
    StepCount,
    WorkflowState,
    AgentId,
    AnyTransition,
}

// ============================================================================
// MESSAGES
// ============================================================================

/// Messages for debugger UI interactions
#[derive(Debug, Clone)]
pub enum DebuggerMessage {
    // Connection
    ConnectToAgent(Uuid),
    Disconnect,
    DebuggerStateUpdated(DebuggerState),

    // Control commands
    Pause,
    Resume,
    Step,
    StepOver,
    StepInto,
    StepOut,
    Continue,

    // Breakpoint management
    AddBreakpoint,
    RemoveBreakpoint(Uuid),
    ToggleBreakpoint(Uuid),
    BreakpointHovered(Option<Uuid>),
    ShowBreakpointForm,
    HideBreakpointForm,
    UpdateBreakpointForm(BreakpointFormField),

    // Navigation
    GoToHistory(usize),
    SelectCallFrame(usize),

    // UI controls
    ToggleCallStack,
    ToggleVariables,
    ToggleBreakpoints,
    ToggleThoughtView,
    SetContextTab(ContextTab),
    SetThoughtContextSplit(f32),

    // Settings
    ToggleSetting(UiSetting),

    // Keyboard shortcuts
    KeyboardShortcut(DebuggerKeyboardShortcut),
}

/// Breakpoint form fields
#[derive(Debug, Clone)]
pub enum BreakpointFormField {
    LocationType(BreakpointLocationType),
    StepCount(String),
    AgentId(String),
    Condition(String),
    Description(String),
}

/// UI settings that can be toggled
#[derive(Debug, Clone, Copy)]
pub enum UiSetting {
    ShowLineNumbers,
    SyntaxHighlighting,
    AutoScroll,
    CompactMode,
    DarkMode,
}

/// Keyboard shortcuts for debugger
#[derive(Debug, Clone, Copy)]
pub enum DebuggerKeyboardShortcut {
    Continue,       // F5
    StepOver,       // F10
    StepInto,       // F11
    StepOut,        // Shift+F11
    ToggleBreakpoint, // F9
}

// ============================================================================
// MAIN DEBUGGER UI VIEW
// ============================================================================

/// Create the main debugger UI view
pub fn view(state: &DebuggerUiState) -> Element<DebuggerMessage> {
    if !state.connected || state.debugger_state.is_none() {
        return view_disconnected();
    }

    let debugger_state = state.debugger_state.as_ref().unwrap();

    // Top control bar
    let control_bar = view_control_bar(debugger_state, &state.ui_settings);

    // Main content area: thought view + context view
    let main_content = view_main_content(state, debugger_state);

    // Bottom panel: breakpoints
    let bottom_panel = if state.show_breakpoints {
        view_breakpoint_panel(state, debugger_state)
    } else {
        column![].into()
    };

    let layout = column![
        control_bar,
        Space::with_height(10),
        main_content,
        Space::with_height(10),
        bottom_panel,
    ]
    .spacing(0)
    .padding(10);

    scrollable(layout).into()
}

/// View when disconnected
fn view_disconnected() -> Element<'static, DebuggerMessage> {
    container(
        column![
            text("Debugger Not Connected").size(24),
            Space::with_height(20),
            text("Connect to an agent to start debugging").size(16),
        ]
        .align_x(Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

// ============================================================================
// CONTROL BAR
// ============================================================================

/// View the control bar with debugger controls
fn view_control_bar(
    debugger_state: &DebuggerState,
    settings: &DebuggerUiSettings,
) -> Element<DebuggerMessage> {
    let execution_state = &debugger_state.execution_state;
    let is_paused = execution_state.is_paused();
    let is_running = execution_state.is_running();

    // Pause/Resume button (toggle style)
    let pause_resume_btn = if is_paused {
        button(
            row![
                text("▶").size(16),
                Space::with_width(5),
                text("Resume").size(14),
            ]
            .align_y(Vertical::Center),
        )
        .on_press(DebuggerMessage::Resume)
        .padding(10)
        .style(button_style_success)
    } else {
        button(
            row![
                text("⏸").size(16),
                Space::with_width(5),
                text("Pause").size(14),
            ]
            .align_y(Vertical::Center),
        )
        .on_press(DebuggerMessage::Pause)
        .padding(10)
        .style(button_style_primary)
    };

    // Step buttons (only enabled when paused)
    let step_btn = create_control_button("Step", "⏭", is_paused, DebuggerMessage::Step);
    let step_over_btn = create_control_button("Step Over (F10)", "⤵", is_paused, DebuggerMessage::StepOver);
    let step_into_btn = create_control_button("Step Into (F11)", "⤓", is_paused, DebuggerMessage::StepInto);
    let step_out_btn = create_control_button("Step Out (Shift+F11)", "⤴", is_paused, DebuggerMessage::StepOut);

    // Continue button
    let continue_btn = create_control_button("Continue (F5)", "▶▶", is_paused, DebuggerMessage::Continue);

    // Status indicator
    let status_text = format!("Status: {}", execution_state);
    let status_color = if is_paused {
        Color::from_rgb8(255, 200, 50)
    } else if is_running {
        Color::from_rgb8(100, 255, 100)
    } else {
        Color::from_rgb8(200, 200, 200)
    };

    let status_indicator = container(
        text(status_text).size(14).style(status_color)
    )
    .padding(10)
    .style(move |theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: border::rounded(4),
        ..Default::default()
    });

    // Step counter
    let step_counter = text(format!("Step: {}", debugger_state.step_count))
        .size(14)
        .style(Color::from_rgb8(200, 200, 200));

    let controls_row = row![
        pause_resume_btn,
        Space::with_width(10),
        step_btn,
        step_over_btn,
        step_into_btn,
        step_out_btn,
        Space::with_width(10),
        continue_btn,
        Space::with_width(Length::Fill),
        step_counter,
        Space::with_width(10),
        status_indicator,
    ]
    .spacing(5)
    .align_y(Vertical::Center);

    container(controls_row)
        .width(Length::Fill)
        .padding(10)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: border::rounded(8),
            ..Default::default()
        })
        .into()
}

/// Helper to create a control button
fn create_control_button(
    label: &str,
    icon: &str,
    enabled: bool,
    message: DebuggerMessage,
) -> Element<DebuggerMessage> {
    let btn_content = row![
        text(icon).size(14),
        Space::with_width(5),
        text(label).size(12),
    ]
    .align_y(Vertical::Center);

    if enabled {
        button(btn_content)
            .on_press(message)
            .padding(8)
            .into()
    } else {
        button(btn_content)
            .padding(8)
            .style(button_style_disabled)
            .into()
    }
}

// ============================================================================
// MAIN CONTENT AREA
// ============================================================================

/// View the main content area with thought and context views
fn view_main_content(
    state: &DebuggerUiState,
    debugger_state: &DebuggerState,
) -> Element<DebuggerMessage> {
    // Split view: Thought on left, Context on right
    let thought_view = if state.show_thought_view {
        view_thought_panel(debugger_state, &state.ui_settings)
    } else {
        column![].into()
    };

    let context_view = view_context_panel(state, debugger_state);

    // Use split percentage
    let thought_width = Length::FillPortion((state.thought_context_split * 100.0) as u16);
    let context_width = Length::FillPortion(((1.0 - state.thought_context_split) * 100.0) as u16);

    container(
        row![
            container(thought_view)
                .width(thought_width)
                .height(Length::Fill),
            Space::with_width(10),
            container(context_view)
                .width(context_width)
                .height(Length::Fill),
        ]
        .spacing(0),
    )
    .height(400)
    .into()
}

// ============================================================================
// THOUGHT VIEW
// ============================================================================

/// View the current thought panel
fn view_thought_panel(
    debugger_state: &DebuggerState,
    settings: &DebuggerUiSettings,
) -> Element<DebuggerMessage> {
    let title = text("Current Thought").size(18);

    let content = if let Some(ref thought) = debugger_state.current_thought {
        view_thought_content(thought, settings)
    } else {
        container(
            text("No thought currently active")
                .size(14)
                .style(Color::from_rgb8(150, 150, 150)),
        )
        .padding(20)
        .center(Length::Fill)
        .into()
    };

    container(
        column![
            title,
            Space::with_height(10),
            content,
        ]
        .spacing(5),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(15)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: border::rounded(8),
        ..Default::default()
    })
    .into()
}

/// View thought content with metadata
fn view_thought_content(
    thought: &ThoughtSnapshot,
    settings: &DebuggerUiSettings,
) -> Element<DebuggerMessage> {
    // Metadata section
    let metadata = column![
        row![
            text("ID:").size(11).style(Color::from_rgb8(150, 150, 150)),
            Space::with_width(10),
            text(&thought.thought_id).size(11),
        ],
        row![
            text("Step:").size(11).style(Color::from_rgb8(150, 150, 150)),
            Space::with_width(10),
            text(format!("{}", thought.step_number)).size(11),
        ],
        row![
            text("Timestamp:").size(11).style(Color::from_rgb8(150, 150, 150)),
            Space::with_width(10),
            text(&thought.timestamp).size(11),
        ],
        if !thought.tags.is_empty() {
            row![
                text("Tags:").size(11).style(Color::from_rgb8(150, 150, 150)),
                Space::with_width(10),
                text(thought.tags.join(", ")).size(11).style(Color::from_rgb8(100, 200, 255)),
            ]
        } else {
            row![]
        },
    ]
    .spacing(5);

    // Content section
    let content_text = if settings.syntax_highlighting {
        // Basic syntax highlighting (simplified)
        text(&thought.content)
            .size(13)
            .style(Color::from_rgb8(220, 220, 220))
    } else {
        text(&thought.content).size(13)
    };

    let content_box = container(
        scrollable(content_text)
    )
    .padding(10)
    .style(|theme: &Theme| container::Style {
        background: Some(Color::from_rgb8(20, 20, 30).into()),
        border: border::rounded(4),
        ..Default::default()
    });

    column![
        metadata,
        Space::with_height(10),
        text("Content:").size(12).style(Color::from_rgb8(150, 150, 150)),
        Space::with_height(5),
        content_box,
    ]
    .spacing(5)
    .into()
}

// ============================================================================
// CONTEXT VIEW
// ============================================================================

/// View the context panel with tabs
fn view_context_panel(
    state: &DebuggerUiState,
    debugger_state: &DebuggerState,
) -> Element<DebuggerMessage> {
    let title = text("Debug Context").size(18);

    // Tab buttons
    let tabs = row![
        create_tab_button("Variables", ContextTab::Variables, state.context_tab),
        create_tab_button("Call Stack", ContextTab::CallStack, state.context_tab),
        create_tab_button("Workflow", ContextTab::WorkflowState, state.context_tab),
        create_tab_button("Metadata", ContextTab::Metadata, state.context_tab),
    ]
    .spacing(5);

    // Tab content
    let content = match state.context_tab {
        ContextTab::Variables => view_variables_tab(&debugger_state.current_context),
        ContextTab::CallStack => view_call_stack_tab(&debugger_state.current_context),
        ContextTab::WorkflowState => view_workflow_state_tab(&debugger_state.current_context),
        ContextTab::Metadata => view_metadata_tab(&debugger_state.current_context),
    };

    container(
        column![
            title,
            Space::with_height(10),
            tabs,
            Space::with_height(10),
            content,
        ]
        .spacing(5),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(15)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: border::rounded(8),
        ..Default::default()
    })
    .into()
}

/// Create a tab button
fn create_tab_button(
    label: &str,
    tab: ContextTab,
    current_tab: ContextTab,
) -> Element<DebuggerMessage> {
    let is_active = tab == current_tab;

    let btn = button(text(label).size(12))
        .on_press(DebuggerMessage::SetContextTab(tab))
        .padding(8);

    if is_active {
        btn.style(button_style_primary).into()
    } else {
        btn.into()
    }
}

/// View variables tab
fn view_variables_tab(context: &DebugContext) -> Element<DebuggerMessage> {
    if context.local_variables.is_empty() {
        return container(
            text("No variables in current scope")
                .size(14)
                .style(Color::from_rgb8(150, 150, 150)),
        )
        .padding(20)
        .center(Length::Fill)
        .into();
    }

    let variables: Vec<Element<DebuggerMessage>> = context
        .local_variables
        .iter()
        .map(|(name, value)| {
            let value_str = format_json_value(value);
            row![
                text(name).size(13).style(Color::from_rgb8(100, 200, 255)),
                text(": ").size(13),
                text(value_str).size(13),
            ]
            .spacing(5)
            .into()
        })
        .collect();

    scrollable(
        column(variables)
            .spacing(5)
            .padding(10)
    )
    .into()
}

/// View call stack tab
fn view_call_stack_tab(context: &DebugContext) -> Element<DebuggerMessage> {
    if context.call_stack.is_empty() {
        return container(
            text("Call stack is empty")
                .size(14)
                .style(Color::from_rgb8(150, 150, 150)),
        )
        .padding(20)
        .center(Length::Fill)
        .into();
    }

    let frames: Vec<Element<DebuggerMessage>> = context
        .call_stack
        .iter()
        .rev()
        .enumerate()
        .map(|(i, frame)| view_call_frame(i, frame))
        .collect();

    scrollable(
        column(frames)
            .spacing(5)
            .padding(10)
    )
    .into()
}

/// View a single call frame
fn view_call_frame(index: usize, frame: &CallFrame) -> Element<DebuggerMessage> {
    let frame_header = row![
        text(format!("#{}", index)).size(12).style(Color::from_rgb8(150, 150, 150)),
        Space::with_width(10),
        text(&frame.name).size(13).style(Color::from_rgb8(255, 200, 100)),
    ]
    .align_y(Vertical::Center);

    let frame_info = column![
        text(format!("State: {}", frame.workflow_state)).size(11),
        text(format!("Entry Step: {}", frame.entry_step)).size(11),
        if !frame.local_variables.is_empty() {
            text(format!("{} local variables", frame.local_variables.len())).size(11)
        } else {
            text("No local variables").size(11)
        },
    ]
    .spacing(2)
    .padding([0, 0, 0, 25]);

    container(
        column![frame_header, Space::with_height(5), frame_info]
            .spacing(5)
    )
    .padding(8)
    .style(|theme: &Theme| container::Style {
        background: Some(Color::from_rgb8(30, 30, 40).into()),
        border: border::rounded(4),
        ..Default::default()
    })
    .into()
}

/// View workflow state tab
fn view_workflow_state_tab(context: &DebugContext) -> Element<DebuggerMessage> {
    let current_state = &context.workflow_state;
    let state_color = workflow_state_color(current_state);

    column![
        row![
            text("Current State:").size(14),
            Space::with_width(10),
            text(format!("{}", current_state))
                .size(16)
                .style(state_color),
        ]
        .align_y(Vertical::Center),
        Space::with_height(15),
        text(format!("Stack Depth: {}", context.stack_depth)).size(13),
        text(format!("Current Step: {}", context.current_step)).size(13),
        text(format!("Agent ID: {}", context.agent_id)).size(13),
        Space::with_height(15),
        text("State Machine Diagram:").size(14),
        Space::with_height(5),
        view_workflow_diagram(current_state),
    ]
    .spacing(5)
    .padding(10)
    .into()
}

/// View a simple workflow state diagram
fn view_workflow_diagram(current_state: &WorkflowState) -> Element<DebuggerMessage> {
    let states = vec![
        WorkflowState::Idle,
        WorkflowState::Running,
        WorkflowState::Paused,
        WorkflowState::Completed,
        WorkflowState::Failed,
        WorkflowState::Cancelled,
    ];

    let state_buttons: Vec<Element<DebuggerMessage>> = states
        .iter()
        .map(|state| {
            let is_current = state == current_state;
            let state_color = workflow_state_color(state);

            let style = if is_current {
                move |theme: &Theme| container::Style {
                    background: Some(state_color.into()),
                    border: border::rounded(4).width(2.0).color(Color::WHITE),
                    ..Default::default()
                }
            } else {
                move |theme: &Theme| container::Style {
                    background: Some(Color::from_rgb8(40, 40, 50).into()),
                    border: border::rounded(4),
                    ..Default::default()
                }
            };

            container(
                text(format!("{}", state))
                    .size(11)
                    .align_x(Horizontal::Center)
            )
            .padding(6)
            .width(Length::Fill)
            .style(style)
            .into()
        })
        .collect();

    column(state_buttons)
        .spacing(5)
        .padding(5)
        .into()
}

/// View metadata tab
fn view_metadata_tab(context: &DebugContext) -> Element<DebuggerMessage> {
    let metadata_str = serde_json::to_string_pretty(&context.metadata)
        .unwrap_or_else(|_| "{}".to_string());

    scrollable(
        container(
            text(metadata_str).size(12)
        )
        .padding(10)
        .style(|theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(20, 20, 30).into()),
            border: border::rounded(4),
            ..Default::default()
        })
    )
    .into()
}

// ============================================================================
// BREAKPOINT PANEL
// ============================================================================

/// View the breakpoint management panel
fn view_breakpoint_panel(
    state: &DebuggerUiState,
    debugger_state: &DebuggerState,
) -> Element<DebuggerMessage> {
    let title = row![
        text("Breakpoints").size(18),
        Space::with_width(Length::Fill),
        button(text("+ Add").size(12))
            .on_press(DebuggerMessage::ShowBreakpointForm)
            .padding(8),
    ]
    .align_y(Vertical::Center);

    let breakpoints_list = if debugger_state.breakpoints.is_empty() {
        container(
            text("No breakpoints set")
                .size(14)
                .style(Color::from_rgb8(150, 150, 150)),
        )
        .padding(20)
        .center(Length::Fill)
        .into()
    } else {
        let items: Vec<Element<DebuggerMessage>> = debugger_state
            .breakpoints
            .iter()
            .map(|bp| view_breakpoint_item(bp, state.hovered_breakpoint.as_ref()))
            .collect();

        scrollable(
            column(items)
                .spacing(5)
                .padding(10)
        )
        .height(150)
        .into()
    };

    let content = column![
        title,
        Space::with_height(10),
        breakpoints_list,
        if state.breakpoint_form.show_form {
            view_breakpoint_form(&state.breakpoint_form)
        } else {
            column![].into()
        },
    ]
    .spacing(10);

    container(content)
        .width(Length::Fill)
        .padding(15)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: border::rounded(8),
            ..Default::default()
        })
        .into()
}

/// View a single breakpoint item
fn view_breakpoint_item(
    breakpoint: &Breakpoint,
    hovered: Option<&Uuid>,
) -> Element<DebuggerMessage> {
    let is_hovered = hovered == Some(&breakpoint.id);
    let enabled_color = if breakpoint.enabled {
        Color::from_rgb8(100, 255, 100)
    } else {
        Color::from_rgb8(150, 150, 150)
    };

    let checkbox_elem = checkbox("", breakpoint.enabled)
        .on_toggle(move |_| DebuggerMessage::ToggleBreakpoint(breakpoint.id))
        .size(16);

    let location_text = text(format!("{}", breakpoint.location))
        .size(13)
        .style(enabled_color);

    let hit_count = text(format!("Hits: {}", breakpoint.hit_count))
        .size(11)
        .style(Color::from_rgb8(150, 150, 150));

    let description = if let Some(ref desc) = breakpoint.description {
        text(desc).size(11).style(Color::from_rgb8(200, 200, 200))
    } else {
        text("").size(11)
    };

    let delete_btn = button(text("✕").size(14))
        .on_press(DebuggerMessage::RemoveBreakpoint(breakpoint.id))
        .padding(4)
        .style(button_style_danger);

    let content = row![
        checkbox_elem,
        Space::with_width(10),
        column![
            location_text,
            description,
        ]
        .spacing(2),
        Space::with_width(Length::Fill),
        hit_count,
        Space::with_width(10),
        delete_btn,
    ]
    .align_y(Vertical::Center)
    .spacing(5);

    let style = if is_hovered {
        |theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(50, 50, 70).into()),
            border: border::rounded(4),
            ..Default::default()
        }
    } else {
        |theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(30, 30, 40).into()),
            border: border::rounded(4),
            ..Default::default()
        }
    };

    container(content)
        .padding(8)
        .width(Length::Fill)
        .style(style)
        .into()
}

/// View the breakpoint creation form
fn view_breakpoint_form(form_state: &BreakpointFormState) -> Element<DebuggerMessage> {
    // Location type selector (simplified - only step count for now)
    let form_content = column![
        text("Add New Breakpoint").size(16),
        Space::with_height(10),
        text("Break at step:").size(12),
        text("(Enter step number)").size(10).style(Color::from_rgb8(150, 150, 150)),
        Space::with_height(5),
        row![
            button(text("Add").size(12))
                .on_press(DebuggerMessage::AddBreakpoint)
                .padding(8),
            Space::with_width(10),
            button(text("Cancel").size(12))
                .on_press(DebuggerMessage::HideBreakpointForm)
                .padding(8),
        ]
        .spacing(10),
    ]
    .spacing(5)
    .padding(15);

    container(form_content)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(40, 40, 50).into()),
            border: border::rounded(8),
            ..Default::default()
        })
        .into()
}

// ============================================================================
// UPDATE LOGIC
// ============================================================================

/// Update the debugger UI state based on messages
pub fn update(state: &mut DebuggerUiState, message: DebuggerMessage) -> Option<DebugCommand> {
    match message {
        DebuggerMessage::ConnectToAgent(agent_id) => {
            state.agent_id = Some(agent_id);
            state.connected = true;
            state.debugger_state = Some(DebuggerState::new(agent_id));
            None
        }

        DebuggerMessage::Disconnect => {
            state.connected = false;
            state.debugger_state = None;
            state.agent_id = None;
            None
        }

        DebuggerMessage::DebuggerStateUpdated(new_state) => {
            state.debugger_state = Some(new_state);
            None
        }

        // Control commands - return DebugCommand to be sent to debugger
        DebuggerMessage::Pause => Some(DebugCommand::Pause),
        DebuggerMessage::Resume => Some(DebugCommand::Resume),
        DebuggerMessage::Step => Some(DebugCommand::Step),
        DebuggerMessage::StepOver => Some(DebugCommand::StepOver),
        DebuggerMessage::StepInto => Some(DebugCommand::StepInto),
        DebuggerMessage::StepOut => Some(DebugCommand::StepOut),
        DebuggerMessage::Continue => Some(DebugCommand::Continue),

        // Breakpoint management
        DebuggerMessage::AddBreakpoint => {
            // Create a simple step count breakpoint
            let step = state.breakpoint_form.step_count.parse::<u64>().unwrap_or(1);
            state.breakpoint_form.show_form = false;
            Some(DebugCommand::SetBreakpoint {
                location: BreakpointLocation::StepCount { step },
                condition: None,
            })
        }

        DebuggerMessage::RemoveBreakpoint(id) => {
            Some(DebugCommand::RemoveBreakpoint { id })
        }

        DebuggerMessage::ToggleBreakpoint(id) => {
            Some(DebugCommand::ToggleBreakpoint { id })
        }

        DebuggerMessage::BreakpointHovered(id) => {
            state.hovered_breakpoint = id;
            None
        }

        DebuggerMessage::ShowBreakpointForm => {
            state.breakpoint_form.show_form = true;
            None
        }

        DebuggerMessage::HideBreakpointForm => {
            state.breakpoint_form.show_form = false;
            None
        }

        DebuggerMessage::UpdateBreakpointForm(field) => {
            match field {
                BreakpointFormField::LocationType(loc_type) => {
                    state.breakpoint_form.location_type = loc_type;
                }
                BreakpointFormField::StepCount(val) => {
                    state.breakpoint_form.step_count = val;
                }
                BreakpointFormField::AgentId(val) => {
                    state.breakpoint_form.agent_id = val;
                }
                BreakpointFormField::Condition(val) => {
                    state.breakpoint_form.condition = val;
                }
                BreakpointFormField::Description(val) => {
                    state.breakpoint_form.description = val;
                }
            }
            None
        }

        // Navigation
        DebuggerMessage::GoToHistory(index) => {
            Some(DebugCommand::GotoHistory { index })
        }

        DebuggerMessage::SelectCallFrame(_index) => {
            // Could navigate to specific call frame
            None
        }

        // UI controls
        DebuggerMessage::ToggleCallStack => {
            state.show_call_stack = !state.show_call_stack;
            None
        }

        DebuggerMessage::ToggleVariables => {
            state.show_variables = !state.show_variables;
            None
        }

        DebuggerMessage::ToggleBreakpoints => {
            state.show_breakpoints = !state.show_breakpoints;
            None
        }

        DebuggerMessage::ToggleThoughtView => {
            state.show_thought_view = !state.show_thought_view;
            None
        }

        DebuggerMessage::SetContextTab(tab) => {
            state.context_tab = tab;
            None
        }

        DebuggerMessage::SetThoughtContextSplit(split) => {
            state.thought_context_split = split.clamp(0.2, 0.8);
            None
        }

        // Settings
        DebuggerMessage::ToggleSetting(setting) => {
            match setting {
                UiSetting::ShowLineNumbers => {
                    state.ui_settings.show_line_numbers = !state.ui_settings.show_line_numbers;
                }
                UiSetting::SyntaxHighlighting => {
                    state.ui_settings.syntax_highlighting = !state.ui_settings.syntax_highlighting;
                }
                UiSetting::AutoScroll => {
                    state.ui_settings.auto_scroll = !state.ui_settings.auto_scroll;
                }
                UiSetting::CompactMode => {
                    state.ui_settings.compact_mode = !state.ui_settings.compact_mode;
                }
                UiSetting::DarkMode => {
                    state.ui_settings.dark_mode = !state.ui_settings.dark_mode;
                }
            }
            None
        }

        // Keyboard shortcuts
        DebuggerMessage::KeyboardShortcut(shortcut) => {
            match shortcut {
                DebuggerKeyboardShortcut::Continue => Some(DebugCommand::Continue),
                DebuggerKeyboardShortcut::StepOver => Some(DebugCommand::StepOver),
                DebuggerKeyboardShortcut::StepInto => Some(DebugCommand::StepInto),
                DebuggerKeyboardShortcut::StepOut => Some(DebugCommand::StepOut),
                DebuggerKeyboardShortcut::ToggleBreakpoint => {
                    // Would toggle breakpoint at current step
                    None
                }
            }
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format a JSON value for display
fn format_json_value(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => format!("\"{}\"", s),
        JsonValue::Array(arr) => format!("[{} items]", arr.len()),
        JsonValue::Object(obj) => format!("{{ {} fields }}", obj.len()),
    }
}

/// Get color for workflow state
fn workflow_state_color(state: &WorkflowState) -> Color {
    match state {
        WorkflowState::Idle => Color::from_rgb8(150, 150, 150),
        WorkflowState::Running => Color::from_rgb8(100, 255, 100),
        WorkflowState::Paused => Color::from_rgb8(255, 200, 50),
        WorkflowState::Completed => Color::from_rgb8(100, 200, 255),
        WorkflowState::Failed => Color::from_rgb8(255, 100, 100),
        WorkflowState::Cancelled => Color::from_rgb8(200, 100, 200),
        _ => Color::from_rgb8(150, 150, 150),
    }
}

// ============================================================================
// BUTTON STYLES
// ============================================================================

fn button_style_primary(theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Color::from_rgb8(50, 100, 200).into()),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}

fn button_style_success(theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Color::from_rgb8(50, 200, 100).into()),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}

fn button_style_danger(theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Color::from_rgb8(200, 50, 50).into()),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}

fn button_style_disabled(theme: &Theme, status: iced::widget::button::Status) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(Color::from_rgb8(60, 60, 70).into()),
        text_color: Color::from_rgb8(100, 100, 110),
        border: border::rounded(4),
        ..Default::default()
    }
}
