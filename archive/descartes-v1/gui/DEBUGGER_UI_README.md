# Debugger UI Components - Phase 3:6.3

## Overview

This document describes the Iced UI components for the Descartes Agent Debugger, implemented as part of Phase 3:6.3. The debugger UI provides a comprehensive interface for debugging agent execution, including pause/resume controls, thought inspection, context viewing, and breakpoint management.

## Architecture

The debugger UI is built using the Iced framework and integrates with the core debugger logic from `descartes-core`. The architecture follows a clean separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                    Main Application                      │
│                  (gui/src/main.rs)                      │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ├─── Message::Debugger(DebuggerMessage)
                    │
┌───────────────────▼─────────────────────────────────────┐
│              Debugger UI Module                         │
│           (gui/src/debugger_ui.rs)                      │
│                                                          │
│  ┌──────────────────┐  ┌──────────────────┐           │
│  │ DebuggerUiState  │  │  View Functions  │           │
│  └──────────────────┘  └──────────────────┘           │
│                                                          │
│  Components:                                            │
│  - DebuggerControls (Pause, Resume, Step buttons)      │
│  - ThoughtView (Current thought display)               │
│  - ContextView (Variables, Call Stack, Workflow)       │
│  - BreakpointPanel (Breakpoint management)             │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ├─── DebugCommand
                    │
┌───────────────────▼─────────────────────────────────────┐
│              Core Debugger Logic                        │
│           (core/src/debugger.rs)                        │
│                                                          │
│  - DebuggerState                                        │
│  - Debugger Controller                                  │
│  - Breakpoint Management                                │
│  - History Navigation                                   │
└─────────────────────────────────────────────────────────┘
```

## Components

### 1. DebuggerControls Widget

The control panel provides buttons for controlling execution:

**Features:**
- **Pause/Resume Button**: Toggle execution state (⏸/▶)
- **Step Button**: Execute a single step (⏭)
- **Step Over (F10)**: Step over function calls (⤵)
- **Step Into (F11)**: Step into function calls (⤓)
- **Step Out (Shift+F11)**: Step out of current frame (⤴)
- **Continue (F5)**: Continue until next breakpoint (▶▶)
- **Status Indicator**: Shows current execution state
- **Step Counter**: Displays current step number

**Visual Design:**
- Buttons are color-coded: green for Resume, blue for Pause
- Disabled state when not paused (for step buttons)
- Status indicator changes color based on state (yellow for paused, green for running)

### 2. ThoughtView Widget

Displays the current agent thought being executed:

**Features:**
- **Thought Metadata**: ID, step number, timestamp, tags
- **Content Display**: Scrollable view of thought content
- **Syntax Highlighting**: Optional syntax highlighting for code
- **Line Numbers**: Optional line number display
- **Tag Display**: Colored tags for categorization

**Layout:**
```
┌─────────────────────────────────────────┐
│ Current Thought                         │
├─────────────────────────────────────────┤
│ ID: thought-123                         │
│ Step: 42                                │
│ Timestamp: 2024-01-15 10:30:45 UTC     │
│ Tags: planning, analysis                │
├─────────────────────────────────────────┤
│ Content:                                │
│ ┌───────────────────────────────────┐  │
│ │ Analyzing task requirements...    │  │
│ │ - Check dependencies               │  │
│ │ - Validate inputs                  │  │
│ │ - Generate execution plan          │  │
│ └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### 3. ContextView Widget

Displays the current execution context with multiple tabs:

**Tabs:**

#### Variables Tab
- Lists all local variables in current scope
- Shows variable names and values
- Formatted JSON values with type indicators
- Scrollable for large variable sets

#### Call Stack Tab
- Shows complete call stack (innermost first)
- Frame information: name, state, entry step
- Local variable count per frame
- Clickable frames for navigation (future)

#### Workflow State Tab
- Current workflow state with visual indicator
- State machine diagram
- Stack depth and current step info
- Agent ID

#### Metadata Tab
- JSON view of context metadata
- Scrollable formatted JSON

**Layout:**
```
┌─────────────────────────────────────────┐
│ Debug Context                           │
├─────────────────────────────────────────┤
│ [Variables] [Call Stack] [Workflow]    │
├─────────────────────────────────────────┤
│                                         │
│ [Tab Content Area]                      │
│                                         │
│                                         │
└─────────────────────────────────────────┘
```

### 4. BreakpointPanel Widget

Manages breakpoints and displays breakpoint list:

**Features:**
- **Breakpoint List**: Shows all configured breakpoints
- **Enable/Disable**: Checkbox to toggle breakpoints
- **Hit Counter**: Shows how many times breakpoint was hit
- **Delete Button**: Remove breakpoints
- **Add Button**: Open breakpoint creation form
- **Location Display**: Shows where breakpoint is set
- **Description**: Optional breakpoint description

**Breakpoint Types:**
- Step Count: Break at specific step number
- Workflow State: Break when entering specific state
- Agent ID: Break for specific agent
- Any Transition: Break on any state transition
- Stack Depth: Break at specific call stack depth

**Layout:**
```
┌─────────────────────────────────────────┐
│ Breakpoints                      [+ Add]│
├─────────────────────────────────────────┤
│ ☑ Step: 100        Hits: 5        [✕]  │
│ ☐ Workflow: Running Hits: 12      [✕]  │
│ ☑ Any Transition   Hits: 47       [✕]  │
└─────────────────────────────────────────┘
```

## State Management

### DebuggerUiState

The main state structure for the debugger UI:

```rust
pub struct DebuggerUiState {
    pub debugger_state: Option<DebuggerState>,
    pub connected: bool,
    pub agent_id: Option<Uuid>,
    pub ui_settings: DebuggerUiSettings,
    pub show_call_stack: bool,
    pub show_variables: bool,
    pub show_breakpoints: bool,
    pub show_thought_view: bool,
    pub thought_context_split: f32,
    pub context_tab: ContextTab,
    pub breakpoint_form: BreakpointFormState,
}
```

### DebuggerMessage

Messages for debugger interactions:

```rust
pub enum DebuggerMessage {
    // Connection
    ConnectToAgent(Uuid),
    Disconnect,

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

    // UI controls
    ToggleCallStack,
    ToggleVariables,
    SetContextTab(ContextTab),

    // Keyboard shortcuts
    KeyboardShortcut(DebuggerKeyboardShortcut),
}
```

## Keyboard Shortcuts

The debugger UI includes comprehensive keyboard shortcuts for efficient debugging:

| Shortcut      | Action                          |
|---------------|---------------------------------|
| **F5**        | Continue execution              |
| **F9**        | Toggle breakpoint at current    |
| **F10**       | Step Over                       |
| **F11**       | Step Into                       |
| **Shift+F11** | Step Out                        |
| **←/→**       | Navigate time travel history    |
| **Space**     | Play/Pause time travel          |
| **+/-**       | Zoom timeline                   |
| **1-4**       | Set playback speed              |
| **L**         | Toggle loop mode                |

## Integration with Core Debugger

The UI integrates with the core debugger through `DebugCommand`:

```rust
// UI sends commands to core debugger
let command = DebugCommand::Pause;
debugger.process_command(command)?;

// Core debugger returns updated state
let new_state = debugger.state().clone();
ui_state.debugger_state = Some(new_state);
```

## Time Travel Integration

The debugger UI integrates with the time travel component from phase3:7.4:

- Timeline slider for navigating through execution history
- Playback controls for automatic replay
- Event markers synchronized with debugger state
- Git commit markers for version correlation

## Visual Design

### Color Scheme

- **Primary Actions**: Blue (#3264C8)
- **Success/Resume**: Green (#32C864)
- **Warning/Pause**: Amber (#FFC832)
- **Danger/Error**: Red (#C83232)
- **Background**: Dark (#1E1E28)
- **Text**: Light Gray (#DCDCDC)
- **Disabled**: Dark Gray (#3C3C46)

### Typography

- **Title**: 32px
- **Section Headers**: 18px
- **Labels**: 14px
- **Content**: 13px
- **Metadata**: 11-12px

## Usage Example

```rust
// Initialize debugger UI state
let mut debugger_ui_state = DebuggerUiState::default();

// Connect to an agent
let agent_id = Uuid::new_v4();
debugger_ui::update(
    &mut debugger_ui_state,
    DebuggerMessage::ConnectToAgent(agent_id)
);

// Render the UI
let view = debugger_ui::view(&debugger_ui_state);

// Handle user interaction
let command = debugger_ui::update(
    &mut debugger_ui_state,
    DebuggerMessage::Pause
);

// Send command to core debugger
if let Some(cmd) = command {
    debugger.process_command(cmd)?;
}
```

## Future Enhancements

### Phase 3:6.4 - Advanced Features

- **Expression Evaluator**: Evaluate expressions in current context
- **Conditional Breakpoints**: Break only when condition is true
- **Watch Variables**: Monitor specific variables
- **Memory Inspector**: View memory contents
- **Performance Profiler**: CPU and memory profiling

### Phase 3:7 - Time Travel Enhancements

- **Bi-directional Debugging**: Navigate forward and backward in time
- **State Diffing**: Compare states at different points
- **Record & Replay**: Save and replay debugging sessions
- **Omniscient Debugging**: Query any point in execution history

## Testing

### Manual Testing

1. **Launch GUI**: `cargo run --bin descartes-gui`
2. **Navigate to Debugger**: Click "Debugger" in sidebar
3. **Test Controls**:
   - Click Pause/Resume buttons
   - Try step buttons (when paused)
   - Add/remove breakpoints
4. **Test Keyboard Shortcuts**: Press F5, F10, F11
5. **Test Tabs**: Switch between Variables, Call Stack, Workflow

### Integration Testing

```rust
#[test]
fn test_debugger_ui_integration() {
    let mut state = DebuggerUiState::default();
    let agent_id = Uuid::new_v4();

    // Connect
    let cmd = debugger_ui::update(
        &mut state,
        DebuggerMessage::ConnectToAgent(agent_id)
    );
    assert!(state.connected);

    // Pause
    let cmd = debugger_ui::update(
        &mut state,
        DebuggerMessage::Pause
    );
    assert!(matches!(cmd, Some(DebugCommand::Pause)));
}
```

## Performance Considerations

- **Lazy Rendering**: Only render visible components
- **Efficient State Updates**: Minimal cloning, use references where possible
- **Throttled Updates**: Limit UI updates during rapid stepping
- **Memory Management**: Limit history size to prevent unbounded growth

## Accessibility

- **Keyboard Navigation**: All functions accessible via keyboard
- **Screen Reader Support**: Proper labels and ARIA attributes (future)
- **High Contrast Mode**: Support for high contrast themes
- **Configurable UI**: Adjustable font sizes and panel layouts

## Dependencies

- `iced = "0.13"`: UI framework
- `descartes-core`: Core debugger logic
- `uuid`: Agent and breakpoint IDs
- `serde_json`: Variable value serialization
- `chrono`: Timestamp formatting (via core)

## Files

- `/home/user/descartes/descartes/gui/src/debugger_ui.rs`: Main implementation
- `/home/user/descartes/descartes/gui/src/main.rs`: Integration with main app
- `/home/user/descartes/descartes/core/src/debugger.rs`: Core debugger logic

## References

- Iced Framework: https://iced.rs/
- Phase 3:6.1 - Debugger State Models
- Phase 3:6.2 - Debugger Logic
- Phase 3:7.4 - Time Travel Slider

## License

Copyright (c) 2024 Descartes Project
Licensed under the same terms as the main project.
