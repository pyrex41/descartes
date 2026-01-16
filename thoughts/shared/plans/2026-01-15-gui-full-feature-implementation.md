# Descartes GUI Full Feature Implementation Plan

## Overview

Transform the minimal Descartes GUI from a basic execution monitor into a feature-complete interface that exposes all CLI functionality. This includes refactoring the architecture, implementing core agent execution, and adding Settings, Skills, SCUD, and Transcripts views.

## Current State Analysis

The GUI currently provides:
- 3 view tabs: Waves, Agents, Output
- Wave visualization from SCUD (read-only)
- Single-task start buttons (but agent spawning is TODO)
- Basic pause/resume/cancel controls
- Live output buffer

**Critical Gap**: `main.rs:127` has `// TODO: Actually spawn the agent via RalphExecutor` - clicking Start doesn't actually run anything.

### Key Discoveries:
- All view logic is in `main.rs` (933 lines) with empty stub modules in `views/`
- Uses Iced 0.14 with tokio async, mpsc channels for control
- Config is loaded but never exposed for editing
- No form inputs currently used (text_input, pick_list, checkbox not imported)

## Desired End State

A fully-featured GUI with:
1. **6 view tabs**: Waves, Agents, Output, Settings, Skills, SCUD, Transcripts
2. **Working agent execution** with auto-wave orchestration like CLI
3. **Full configuration** of guidance, backpressure, harness, models
4. **Skills browser** with execution and variable input
5. **SCUD management** including tag switching, status editing, PRD import
6. **Transcript browser** with search and replay

### Verification:
- All CLI commands have GUI equivalents
- Can run complete swarm loop from GUI
- Can configure all settings without editing TOML files
- Can browse and execute skills
- Can manage SCUD tasks and tags

## What We're NOT Doing

- Mobile/touch optimization
- Custom theming beyond dark mode
- Plugin/extension system
- Multi-project support (single project at a time)
- Agent definition editor (can browse but not create)

## Implementation Approach

1. **Phase 1**: Refactor architecture - extract views, add new ViewModes
2. **Phase 2**: Fix core functionality - actually spawn agents, auto-execute
3. **Phase 3**: Settings view - all configuration options
4. **Phase 4**: Skills view - browser and runner
5. **Phase 5**: SCUD view - task management
6. **Phase 6**: Transcripts view - history browser
7. **Phase 7**: Polish - keyboard shortcuts, command palette

---

## Phase 1: Architecture Refactor

### Overview
Extract view logic from main.rs into proper modules, establish shared state patterns, and prepare for new views.

### Changes Required:

#### 1.1 Expand ViewMode Enum

**File**: `descartes-gui/src/main.rs`
**Changes**: Add new view modes

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    Waves,
    Agents,
    Output,
    Settings,  // NEW
    Skills,    // NEW
    Scud,      // NEW
    Transcripts, // NEW
}
```

#### 1.2 Expand AppState

**File**: `descartes-gui/src/state.rs`
**Changes**: Add state for new features

```rust
use descartes::{Config, Skill, Transcript};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    // Existing
    pub waves: Vec<Vec<TaskInfo>>,
    pub agent_status: AgentStatus,
    pub current_task: Option<String>,
    pub output_buffer: String,

    // NEW: Configuration state
    pub config: Option<Config>,
    pub config_dirty: bool,

    // NEW: Skills state
    pub skills: Vec<SkillInfo>,
    pub selected_skill: Option<String>,
    pub skill_args: HashMap<String, String>,

    // NEW: SCUD state
    pub scud_tags: Vec<String>,
    pub active_tag: Option<String>,
    pub tasks: Vec<TaskDetail>,
    pub selected_task: Option<String>,

    // NEW: Transcripts state
    pub transcripts: Vec<TranscriptInfo>,
    pub selected_transcript: Option<String>,
    pub transcript_content: Option<String>,

    // NEW: Swarm execution state
    pub swarm_running: bool,
    pub current_wave: usize,
    pub current_round: usize,
    pub validation_results: Vec<ValidationResult>,
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub aliases: Vec<String>,
    pub variables: Vec<SkillVariable>,
}

#[derive(Debug, Clone)]
pub struct SkillVariable {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub dependencies: Vec<String>,
    pub wave: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct TranscriptInfo {
    pub id: String,
    pub timestamp: String,
    pub task_id: Option<String>,
    pub category: String,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub command: String,
    pub success: bool,
    pub output: String,
    pub timestamp: String,
}
```

#### 1.3 Extract Waves View

**File**: `descartes-gui/src/views/waves.rs`
**Changes**: Move view_waves() logic from main.rs

```rust
use iced::widget::{button, column, row, scrollable, text, Column};
use iced::{Alignment, Element, Length};

use crate::state::{AgentStatus, AppState, TaskInfo};
use crate::Message;

pub fn view(state: &AppState, swarm_running: bool) -> Element<Message> {
    let mut waves_column = Column::new().spacing(15);

    if state.waves.is_empty() {
        waves_column = waves_column.push(
            text("No tasks loaded. Select a SCUD tag in Settings or click Refresh.")
        );
    } else {
        for (wave_idx, wave) in state.waves.iter().enumerate() {
            let is_current = swarm_running && wave_idx == state.current_wave;
            let wave_header = row![
                text(format!("Wave {}", wave_idx + 1)).size(18),
                if is_current {
                    text(" (executing)").style(|_| text::Style {
                        color: Some(iced::Color::from_rgb(0.4, 0.8, 0.4)),
                    })
                } else {
                    text("")
                }
            ];

            let mut task_column = Column::new().spacing(5);
            for task in wave {
                let status_color = match task.status.as_str() {
                    "Done" => iced::Color::from_rgb(0.4, 0.8, 0.4),
                    "Failed" => iced::Color::from_rgb(1.0, 0.4, 0.4),
                    "InProgress" => iced::Color::from_rgb(0.4, 0.6, 1.0),
                    _ => iced::Color::from_rgb(0.7, 0.7, 0.7),
                };

                let task_row = row![
                    text(&task.id).width(Length::Fixed(100.0)),
                    text(&task.title).width(Length::Fill),
                    text(&task.status)
                        .width(Length::Fixed(100.0))
                        .style(move |_| text::Style { color: Some(status_color) }),
                    button("Start").on_press(Message::StartTask(task.id.clone())),
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                task_column = task_column.push(task_row);
            }

            waves_column = waves_column.push(wave_header).push(task_column);
        }
    }

    let controls = row![
        button("Refresh").on_press(Message::LoadWaves),
        button("Start Swarm").on_press(Message::StartSwarm),
        if swarm_running {
            button("Stop Swarm").on_press(Message::StopSwarm)
        } else {
            button("Stop Swarm")  // Disabled appearance
        },
    ]
    .spacing(10);

    column![controls, scrollable(waves_column).height(Length::Fill)]
        .spacing(10)
        .into()
}
```

#### 1.4 Extract Agents View

**File**: `descartes-gui/src/views/agents.rs`
**Changes**: Move view_agents() logic, add multi-agent support

```rust
use iced::widget::{button, column, row, scrollable, text, Column};
use iced::{Alignment, Element, Length};

use crate::state::{AgentStatus, AppState};
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    let status_text = match state.agent_status {
        AgentStatus::Idle => "No agent running",
        AgentStatus::Running => "Agent is running...",
        AgentStatus::Paused => "Agent is paused",
    };

    let mut controls = row![].spacing(10);
    match state.agent_status {
        AgentStatus::Idle => {}
        AgentStatus::Running => {
            controls = controls
                .push(button("Pause").on_press(Message::PauseAgent))
                .push(button("Cancel").on_press(Message::CancelAgent));
        }
        AgentStatus::Paused => {
            controls = controls
                .push(button("Resume").on_press(Message::ResumeAgent))
                .push(button("Cancel").on_press(Message::CancelAgent));
        }
    }

    let current_task = if let Some(ref task_id) = state.current_task {
        text(format!("Current task: {}", task_id))
    } else {
        text("No task selected")
    };

    // Validation results section
    let mut validation_column = Column::new().spacing(5);
    validation_column = validation_column.push(text("Validation Results:").size(16));

    if state.validation_results.is_empty() {
        validation_column = validation_column.push(text("No validation runs yet"));
    } else {
        for result in state.validation_results.iter().rev().take(5) {
            let color = if result.success {
                iced::Color::from_rgb(0.4, 0.8, 0.4)
            } else {
                iced::Color::from_rgb(1.0, 0.4, 0.4)
            };
            let status = if result.success { "PASS" } else { "FAIL" };
            validation_column = validation_column.push(
                row![
                    text(format!("[{}]", status)).style(move |_| text::Style { color: Some(color) }),
                    text(&result.command),
                ]
                .spacing(10)
            );
        }
    }

    column![
        text(status_text).size(18),
        current_task,
        controls,
        validation_column,
    ]
    .spacing(15)
    .into()
}
```

#### 1.5 Extract Output View

**File**: `descartes-gui/src/views/output.rs`
**Changes**: Move view_output() logic

```rust
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::state::AppState;
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    let output_text = if state.output_buffer.is_empty() {
        text("No output yet. Start an agent to see output here.")
    } else {
        text(&state.output_buffer)
    };

    let controls = row![
        button("Clear").on_press(Message::ClearOutput),
        button("Copy").on_press(Message::CopyOutput),
    ]
    .spacing(10);

    column![
        controls,
        scrollable(
            container(output_text)
                .padding(10)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.1, 0.1, 0.12,
                    ))),
                    ..Default::default()
                }),
        )
        .height(Length::Fill),
    ]
    .spacing(10)
    .into()
}
```

#### 1.6 Create Settings View Stub

**File**: `descartes-gui/src/views/settings.rs`
**Changes**: Create new file with placeholder

```rust
use iced::widget::{column, text};
use iced::Element;

use crate::state::AppState;
use crate::Message;

pub fn view(_state: &AppState) -> Element<Message> {
    column![
        text("Settings").size(24),
        text("Settings view coming in Phase 3..."),
    ]
    .spacing(20)
    .into()
}
```

#### 1.7 Create Skills View Stub

**File**: `descartes-gui/src/views/skills.rs`
**Changes**: Create new file with placeholder

```rust
use iced::widget::{column, text};
use iced::Element;

use crate::state::AppState;
use crate::Message;

pub fn view(_state: &AppState) -> Element<Message> {
    column![
        text("Skills").size(24),
        text("Skills view coming in Phase 4..."),
    ]
    .spacing(20)
    .into()
}
```

#### 1.8 Create SCUD View Stub

**File**: `descartes-gui/src/views/scud.rs`
**Changes**: Create new file with placeholder

```rust
use iced::widget::{column, text};
use iced::Element;

use crate::state::AppState;
use crate::Message;

pub fn view(_state: &AppState) -> Element<Message> {
    column![
        text("SCUD Tasks").size(24),
        text("SCUD management view coming in Phase 5..."),
    ]
    .spacing(20)
    .into()
}
```

#### 1.9 Create Transcripts View Stub

**File**: `descartes-gui/src/views/transcripts.rs`
**Changes**: Create new file with placeholder

```rust
use iced::widget::{column, text};
use iced::Element;

use crate::state::AppState;
use crate::Message;

pub fn view(_state: &AppState) -> Element<Message> {
    column![
        text("Transcripts").size(24),
        text("Transcript browser coming in Phase 6..."),
    ]
    .spacing(20)
    .into()
}
```

#### 1.10 Update Views Module

**File**: `descartes-gui/src/views/mod.rs`
**Changes**: Export all view modules

```rust
pub mod agents;
pub mod output;
pub mod scud;
pub mod settings;
pub mod skills;
pub mod transcripts;
pub mod waves;
```

#### 1.11 Refactor main.rs

**File**: `descartes-gui/src/main.rs`
**Changes**: Remove inlined view logic, use modules, update navigation

```rust
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use tokio::sync::mpsc;

use descartes::{scud, Config};

mod state;
mod theme;
mod views;

use state::{AgentStatus, AppState, TaskInfo};

// ... (keep DescartesGui struct, Message enum updates below)

impl DescartesGui {
    fn view(&self) -> Element<Message> {
        let header = self.view_header();

        let content: Element<Message> = match self.view {
            ViewMode::Waves => views::waves::view(&self.state, self.state.swarm_running),
            ViewMode::Agents => views::agents::view(&self.state),
            ViewMode::Output => views::output::view(&self.state),
            ViewMode::Settings => views::settings::view(&self.state),
            ViewMode::Skills => views::skills::view(&self.state),
            ViewMode::Scud => views::scud::view(&self.state),
            ViewMode::Transcripts => views::transcripts::view(&self.state),
        };

        let main_column = if let Some(ref error) = self.error {
            let error_banner = self.view_error_banner(error);
            column![error_banner, header, content].spacing(10)
        } else {
            column![header, content].spacing(10)
        };

        container(main_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    fn view_header(&self) -> Element<Message> {
        let nav_buttons = row![
            self.nav_button("Waves", ViewMode::Waves),
            self.nav_button("Agents", ViewMode::Agents),
            self.nav_button("Output", ViewMode::Output),
            self.nav_button("Settings", ViewMode::Settings),
            self.nav_button("Skills", ViewMode::Skills),
            self.nav_button("SCUD", ViewMode::Scud),
            self.nav_button("Transcripts", ViewMode::Transcripts),
        ]
        .spacing(5);

        let status = text(format!("Status: {:?}", self.state.agent_status));

        row![nav_buttons, status]
            .spacing(20)
            .align_y(Alignment::Center)
            .into()
    }

    fn nav_button(&self, label: &str, mode: ViewMode) -> Element<Message> {
        button(text(label))
            .on_press(Message::SwitchView(mode))
            .style(if self.view == mode {
                button::primary
            } else {
                button::secondary
            })
            .into()
    }

    fn view_error_banner(&self, error: &str) -> Element<Message> {
        container(
            row![
                text(error).style(|_| text::Style {
                    color: Some(iced::Color::from_rgb(1.0, 0.4, 0.4)),
                }),
                button("Dismiss").on_press(Message::DismissError),
            ]
            .spacing(10),
        )
        .padding(10)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.2, 0.1, 0.1,
            ))),
            ..Default::default()
        })
        .into()
    }
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy lint check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify ViewMode enum has all 7 variants
grep -E "^\s*(Waves|Agents|Output|Settings|Skills|Scud|Transcripts)," descartes-gui/src/main.rs | wc -l
# Expected: 7

# 5. Verify all view modules exist
ls descartes-gui/src/views/{waves,agents,output,settings,skills,scud,transcripts}.rs 2>/dev/null | wc -l
# Expected: 7

# 6. Verify views/mod.rs exports all modules
grep -c "^pub mod" descartes-gui/src/views/mod.rs
# Expected: 7
```

**Verification Gate**: All commands must succeed before proceeding to Phase 2.

---

## Phase 2: Core Agent Execution

### Overview
Implement actual agent spawning, auto-execution of waves, and multi-agent support. This fixes the critical TODO and makes the GUI actually functional.

### Changes Required:

#### 2.1 Add New Messages

**File**: `descartes-gui/src/main.rs`
**Changes**: Expand Message enum for swarm control

```rust
#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchView(ViewMode),

    // Task/Wave management
    LoadWaves,
    WavesLoaded(Result<Vec<Vec<TaskInfo>>, String>),
    LoadConfig,
    ConfigLoaded(Result<Config, String>),

    // Single task execution
    StartTask(String),

    // Swarm execution (NEW)
    StartSwarm,
    StopSwarm,
    SwarmProgress { wave: usize, round: usize, task_id: String },
    SwarmTaskComplete { task_id: String, success: bool },
    SwarmWaveComplete { wave: usize },
    SwarmValidation(ValidationResult),
    SwarmComplete(Result<(), String>),

    // Agent control
    AgentOutput(String),
    AgentComplete(Result<(), String>),
    PauseAgent,
    ResumeAgent,
    CancelAgent,

    // Output
    ClearOutput,
    CopyOutput,

    // UI
    DismissError,
    Tick,
}
```

#### 2.2 Implement Swarm Execution

**File**: `descartes-gui/src/executor.rs` (NEW)
**Changes**: Create executor module that wraps SwarmExecutor

```rust
use descartes::{Config, SwarmExecutor, SpecConfig};
use tokio::sync::mpsc;

pub enum ExecutorCommand {
    Start {
        config: Config,
        scud_tag: String,
        spec_config: SpecConfig,
    },
    Pause,
    Resume,
    Cancel,
}

pub enum ExecutorEvent {
    Progress { wave: usize, round: usize, task_id: String },
    TaskComplete { task_id: String, success: bool },
    WaveComplete { wave: usize },
    Validation { command: String, success: bool, output: String },
    Output(String),
    Complete(Result<(), String>),
}

pub struct GuiExecutor {
    command_tx: mpsc::Sender<ExecutorCommand>,
    event_rx: mpsc::Receiver<ExecutorEvent>,
}

impl GuiExecutor {
    pub fn new() -> (Self, mpsc::Sender<ExecutorCommand>, mpsc::Receiver<ExecutorEvent>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (evt_tx, evt_rx) = mpsc::channel(256);

        // Spawn executor task
        tokio::spawn(Self::run_executor(cmd_rx, evt_tx));

        (
            Self {
                command_tx: cmd_tx.clone(),
                event_rx: evt_rx,
            },
            cmd_tx,
            evt_rx,
        )
    }

    async fn run_executor(
        mut cmd_rx: mpsc::Receiver<ExecutorCommand>,
        evt_tx: mpsc::Sender<ExecutorEvent>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                ExecutorCommand::Start { config, scud_tag, spec_config } => {
                    let evt_tx = evt_tx.clone();

                    // Create executor with GUI callbacks
                    let executor = SwarmExecutor::new(
                        config.clone(),
                        scud_tag,
                        spec_config,
                    );

                    // Run with progress callbacks
                    match executor.run_with_callbacks(
                        |wave, round, task_id| {
                            let _ = evt_tx.blocking_send(ExecutorEvent::Progress {
                                wave,
                                round,
                                task_id: task_id.to_string(),
                            });
                        },
                        |task_id, success| {
                            let _ = evt_tx.blocking_send(ExecutorEvent::TaskComplete {
                                task_id: task_id.to_string(),
                                success,
                            });
                        },
                        |wave| {
                            let _ = evt_tx.blocking_send(ExecutorEvent::WaveComplete { wave });
                        },
                        |cmd, success, output| {
                            let _ = evt_tx.blocking_send(ExecutorEvent::Validation {
                                command: cmd.to_string(),
                                success,
                                output: output.to_string(),
                            });
                        },
                        |text| {
                            let _ = evt_tx.blocking_send(ExecutorEvent::Output(text.to_string()));
                        },
                    ).await {
                        Ok(()) => {
                            let _ = evt_tx.send(ExecutorEvent::Complete(Ok(()))).await;
                        }
                        Err(e) => {
                            let _ = evt_tx.send(ExecutorEvent::Complete(Err(e.to_string()))).await;
                        }
                    }
                }
                ExecutorCommand::Pause => {
                    // TODO: Implement pause in SwarmExecutor
                }
                ExecutorCommand::Resume => {
                    // TODO: Implement resume in SwarmExecutor
                }
                ExecutorCommand::Cancel => {
                    // TODO: Implement cancel in SwarmExecutor
                }
            }
        }
    }
}
```

#### 2.3 Add Callback Support to SwarmExecutor

**File**: `descartes/src/swarm_executor.rs`
**Changes**: Add `run_with_callbacks` method

```rust
impl SwarmExecutor {
    /// Run swarm with GUI callbacks for progress updates
    pub async fn run_with_callbacks<F1, F2, F3, F4, F5>(
        &self,
        on_task_start: F1,
        on_task_complete: F2,
        on_wave_complete: F3,
        on_validation: F4,
        on_output: F5,
    ) -> Result<()>
    where
        F1: Fn(usize, usize, &str) + Send + Sync,
        F2: Fn(&str, bool) + Send + Sync,
        F3: Fn(usize) + Send + Sync,
        F4: Fn(&str, bool, &str) + Send + Sync,
        F5: Fn(&str) + Send + Sync,
    {
        // Similar to run() but calls callbacks at each step
        // ... implementation details
    }
}
```

#### 2.4 Update Main App for Swarm Integration

**File**: `descartes-gui/src/main.rs`
**Changes**: Handle swarm messages, use subscription for events

```rust
use crate::executor::{ExecutorCommand, ExecutorEvent, GuiExecutor};

struct DescartesGui {
    view: ViewMode,
    state: AppState,
    error: Option<String>,

    // Executor integration
    executor_tx: Option<mpsc::Sender<ExecutorCommand>>,
}

impl DescartesGui {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ... existing handlers ...

            Message::StartSwarm => {
                if let (Some(config), Some(tag)) = (&self.state.config, &self.state.active_tag) {
                    self.state.swarm_running = true;
                    self.state.current_wave = 0;
                    self.state.current_round = 0;
                    self.state.validation_results.clear();

                    let config = config.clone();
                    let tag = tag.clone();

                    return Task::perform(
                        async move {
                            // Start executor
                            let (_, cmd_tx, mut evt_rx) = GuiExecutor::new();

                            let spec_config = SpecConfig::default();
                            let _ = cmd_tx.send(ExecutorCommand::Start {
                                config,
                                scud_tag: tag,
                                spec_config,
                            }).await;

                            // Return the event receiver for subscription
                            Ok(())
                        },
                        |result| match result {
                            Ok(()) => Message::Tick,
                            Err(e) => Message::SwarmComplete(Err(e)),
                        }
                    );
                }
                Task::none()
            }

            Message::StopSwarm => {
                if let Some(ref tx) = self.executor_tx {
                    let tx = tx.clone();
                    return Task::perform(
                        async move { let _ = tx.send(ExecutorCommand::Cancel).await; },
                        |_| Message::Tick,
                    );
                }
                self.state.swarm_running = false;
                Task::none()
            }

            Message::SwarmProgress { wave, round, task_id } => {
                self.state.current_wave = wave;
                self.state.current_round = round;
                self.state.current_task = Some(task_id);
                self.state.agent_status = AgentStatus::Running;
                Task::none()
            }

            Message::SwarmTaskComplete { task_id, success } => {
                // Update task status in waves
                for wave in &mut self.state.waves {
                    for task in wave {
                        if task.id == task_id {
                            task.status = if success { "Done".to_string() } else { "Failed".to_string() };
                        }
                    }
                }
                Task::none()
            }

            Message::SwarmWaveComplete { wave } => {
                self.state.output_buffer.push_str(&format!("\n=== Wave {} complete ===\n", wave + 1));
                Task::none()
            }

            Message::SwarmValidation(result) => {
                self.state.validation_results.push(result);
                Task::none()
            }

            Message::SwarmComplete(result) => {
                self.state.swarm_running = false;
                self.state.agent_status = AgentStatus::Idle;
                match result {
                    Ok(()) => {
                        self.state.output_buffer.push_str("\n=== Swarm complete ===\n");
                    }
                    Err(e) => {
                        self.error = Some(format!("Swarm failed: {}", e));
                    }
                }
                // Reload waves to get updated statuses
                Task::perform(load_waves_from_scud(), Message::WavesLoaded)
            }

            // ... rest of handlers
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Poll executor events if running
        if self.state.swarm_running {
            // Use iced::time::every for polling
            iced::time::every(std::time::Duration::from_millis(100))
                .map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }
}
```

#### 2.5 Implement Single Task Execution

**File**: `descartes-gui/src/main.rs`
**Changes**: Fix the TODO - actually spawn agents

```rust
Message::StartTask(task_id) => {
    self.state.agent_status = AgentStatus::Running;
    self.state.current_task = Some(task_id.clone());
    self.state.output_buffer.clear();

    if let Some(config) = &self.state.config {
        let config = config.clone();
        let task_id = task_id.clone();

        return Task::perform(
            async move {
                // Load task from SCUD
                let storage = scud::storage_from_config(&config)?;
                let task = storage.load_task(&task_id)?;

                // Build prompt
                let spec = descartes::spec::build_prompt(&task, &config, None)?;

                // Spawn agent
                let harness = descartes::harness::create_harness(&config)?;
                let result = harness.run_prompt(&spec).await?;

                Ok(result)
            },
            |result: Result<String, String>| match result {
                Ok(output) => Message::AgentOutput(output),
                Err(e) => Message::AgentComplete(Err(e)),
            }
        );
    }

    Task::none()
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile GUI
cargo build -p descartes-gui

# 2. Compile descartes (includes executor changes)
cargo build -p descartes

# 3. Run all workspace tests
cargo test --workspace

# 4. Clippy check on both packages
cargo clippy -p descartes-gui -p descartes -- -D warnings

# 5. Verify executor.rs exists and has GuiExecutor
test -f descartes-gui/src/executor.rs && grep -q "pub struct GuiExecutor" descartes-gui/src/executor.rs && echo "OK"
# Expected: OK

# 6. Verify SwarmExecutor has callback method
grep -q "run_with_callbacks" descartes/src/swarm_executor.rs && echo "OK"
# Expected: OK

# 7. Verify Message enum has swarm messages
grep -E "^\s*(StartSwarm|StopSwarm|SwarmProgress|SwarmTaskComplete|SwarmWaveComplete|SwarmComplete)" descartes-gui/src/main.rs | wc -l
# Expected: 6

# 8. Verify StartTask handler no longer has TODO
! grep -q "// TODO: Actually spawn the agent" descartes-gui/src/main.rs && echo "TODO removed"
# Expected: TODO removed
```

**Verification Gate**: All commands must succeed before proceeding to Phase 3.

---

## Phase 3: Settings View

### Overview
Implement the full Settings view with all configuration options: SCUD tag, harness, model, guidance, backpressure, and execution settings.

### Changes Required:

#### 3.1 Add Form Widget Imports

**File**: `descartes-gui/src/main.rs`
**Changes**: Add Iced form imports

```rust
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable,
    text, text_input, toggler, Column, TextInput,
};
```

#### 3.2 Add Settings State

**File**: `descartes-gui/src/state.rs`
**Changes**: Add settings-specific state

```rust
#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    // SCUD
    pub scud_tag_input: String,

    // Harness/Model
    pub available_harnesses: Vec<String>,
    pub selected_harness: Option<String>,
    pub available_models: Vec<String>,
    pub selected_model: Option<String>,

    // Execution
    pub round_size: u32,
    pub validate_enabled: bool,
    pub verify_command: String,

    // Guidance
    pub guidance_global: String,
    pub guidance_builder: String,
    pub guidance_review: String,
    pub guidance_validator: String,

    // Backpressure
    pub backpressure_commands: Vec<String>,
    pub backpressure_stop_on_failure: bool,
    pub backpressure_timeout: u32,
}

impl AppState {
    // Add settings field
    pub settings: SettingsState,
}
```

#### 3.3 Add Settings Messages

**File**: `descartes-gui/src/main.rs`
**Changes**: Add settings-related messages

```rust
enum Message {
    // ... existing ...

    // Settings: SCUD
    ScudTagInputChanged(String),
    ScudTagSelected(String),
    LoadScudTags,
    ScudTagsLoaded(Result<Vec<String>, String>),

    // Settings: Harness/Model
    HarnessSelected(String),
    ModelSelected(String),

    // Settings: Execution
    RoundSizeChanged(String),
    ValidateToggled(bool),
    VerifyCommandChanged(String),

    // Settings: Guidance
    GuidanceGlobalChanged(String),
    GuidanceBuilderChanged(String),
    GuidanceReviewChanged(String),
    GuidanceValidatorChanged(String),

    // Settings: Backpressure
    BackpressureCommandAdded(String),
    BackpressureCommandRemoved(usize),
    BackpressureStopOnFailureToggled(bool),
    BackpressureTimeoutChanged(String),

    // Settings: Save
    SaveSettings,
    SettingsSaved(Result<(), String>),
}
```

#### 3.4 Implement Settings View

**File**: `descartes-gui/src/views/settings.rs`
**Changes**: Full settings UI implementation

```rust
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable,
    text, text_input, toggler, Column, Row,
};
use iced::{Alignment, Element, Length};

use crate::state::AppState;
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    let settings = &state.settings;

    // SCUD Tag Section
    let scud_section = section(
        "SCUD Configuration",
        column![
            labeled_row(
                "Active Tag:",
                pick_list(
                    &state.scud_tags[..],
                    state.active_tag.as_ref(),
                    Message::ScudTagSelected,
                )
                .placeholder("Select tag...")
                .width(Length::Fixed(200.0)),
            ),
            labeled_row(
                "New Tag:",
                row![
                    text_input("tag-name", &settings.scud_tag_input)
                        .on_input(Message::ScudTagInputChanged)
                        .width(Length::Fixed(150.0)),
                    button("Create").on_press(Message::CreateScudTag),
                ]
                .spacing(10),
            ),
        ]
        .spacing(10),
    );

    // Harness/Model Section
    let harness_section = section(
        "Harness & Model",
        column![
            labeled_row(
                "Harness:",
                pick_list(
                    &["claude-code", "opencode", "codex"][..],
                    settings.selected_harness.as_deref(),
                    |s: &str| Message::HarnessSelected(s.to_string()),
                )
                .width(Length::Fixed(200.0)),
            ),
            labeled_row(
                "Model:",
                pick_list(
                    &settings.available_models[..],
                    settings.selected_model.as_ref(),
                    Message::ModelSelected,
                )
                .width(Length::Fixed(200.0)),
            ),
        ]
        .spacing(10),
    );

    // Execution Section
    let execution_section = section(
        "Execution Settings",
        column![
            labeled_row(
                "Round Size:",
                text_input("5", &settings.round_size.to_string())
                    .on_input(Message::RoundSizeChanged)
                    .width(Length::Fixed(80.0)),
            ),
            labeled_row(
                "Enable Validation:",
                toggler(settings.validate_enabled)
                    .on_toggle(Message::ValidateToggled),
            ),
            labeled_row(
                "Verify Command:",
                text_input("cargo test", &settings.verify_command)
                    .on_input(Message::VerifyCommandChanged)
                    .width(Length::Fill),
            ),
        ]
        .spacing(10),
    );

    // Guidance Section
    let guidance_section = section(
        "Guidance (Prompt Augmentation)",
        column![
            text("Global:").size(14),
            text_input("Global guidance for all agents...", &settings.guidance_global)
                .on_input(Message::GuidanceGlobalChanged)
                .width(Length::Fill),
            text("Builder:").size(14),
            text_input("Guidance for builder agents...", &settings.guidance_builder)
                .on_input(Message::GuidanceBuilderChanged)
                .width(Length::Fill),
            text("Review:").size(14),
            text_input("Guidance for review agents...", &settings.guidance_review)
                .on_input(Message::GuidanceReviewChanged)
                .width(Length::Fill),
            text("Validator:").size(14),
            text_input("Guidance for validator agents...", &settings.guidance_validator)
                .on_input(Message::GuidanceValidatorChanged)
                .width(Length::Fill),
        ]
        .spacing(5),
    );

    // Backpressure Section
    let mut bp_commands = Column::new().spacing(5);
    for (idx, cmd) in settings.backpressure_commands.iter().enumerate() {
        bp_commands = bp_commands.push(
            row![
                text(cmd).width(Length::Fill),
                button("Remove").on_press(Message::BackpressureCommandRemoved(idx)),
            ]
            .spacing(10)
        );
    }

    let backpressure_section = section(
        "Backpressure Validation",
        column![
            text("Commands (run after each round):").size(14),
            bp_commands,
            row![
                text_input("cargo test", "")
                    .on_input(Message::BackpressureCommandAdded)
                    .width(Length::Fill),
                button("Add").on_press(Message::BackpressureCommandAdded("".to_string())),
            ]
            .spacing(10),
            labeled_row(
                "Stop on Failure:",
                toggler(settings.backpressure_stop_on_failure)
                    .on_toggle(Message::BackpressureStopOnFailureToggled),
            ),
            labeled_row(
                "Timeout (sec):",
                text_input("300", &settings.backpressure_timeout.to_string())
                    .on_input(Message::BackpressureTimeoutChanged)
                    .width(Length::Fixed(80.0)),
            ),
        ]
        .spacing(10),
    );

    // Save Button
    let save_section = row![
        button("Save Settings").on_press(Message::SaveSettings),
        button("Reset to Defaults").on_press(Message::ResetSettings),
    ]
    .spacing(10);

    // Main Layout
    scrollable(
        column![
            text("Settings").size(24),
            scud_section,
            harness_section,
            execution_section,
            guidance_section,
            backpressure_section,
            save_section,
        ]
        .spacing(20)
        .padding(10)
    )
    .height(Length::Fill)
    .into()
}

fn section<'a>(title: &str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        column![
            text(title).size(18),
            content.into(),
        ]
        .spacing(10)
    )
    .padding(15)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.15, 0.15, 0.18))),
        border: iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

fn labeled_row<'a>(label: &str, widget: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(150.0)),
        widget.into(),
    ]
    .align_y(Alignment::Center)
    .spacing(10)
    .into()
}
```

#### 3.5 Implement Settings Handlers

**File**: `descartes-gui/src/main.rs`
**Changes**: Handle all settings messages

```rust
// In update() match:

Message::LoadScudTags => {
    Task::perform(
        async {
            let storage = scud::Storage::discover()?;
            Ok(storage.list_tags()?)
        },
        Message::ScudTagsLoaded,
    )
}

Message::ScudTagsLoaded(result) => {
    match result {
        Ok(tags) => self.state.scud_tags = tags,
        Err(e) => self.error = Some(format!("Failed to load tags: {}", e)),
    }
    Task::none()
}

Message::ScudTagSelected(tag) => {
    self.state.active_tag = Some(tag.clone());
    self.state.settings.scud_tag_input = tag;
    // Reload waves for new tag
    Task::perform(load_waves_from_scud(), Message::WavesLoaded)
}

Message::HarnessSelected(harness) => {
    self.state.settings.selected_harness = Some(harness.clone());
    // Update available models based on harness
    self.state.settings.available_models = match harness.as_str() {
        "claude-code" => vec!["opus".to_string(), "sonnet".to_string(), "haiku".to_string()],
        "opencode" => vec![
            "xai/grok-code-fast-1".to_string(),
            "xai/grok-3-fast".to_string(),
            "anthropic/claude-sonnet".to_string(),
        ],
        "codex" => vec!["gpt-4o".to_string(), "gpt-4".to_string()],
        _ => vec![],
    };
    self.state.config_dirty = true;
    Task::none()
}

Message::ModelSelected(model) => {
    self.state.settings.selected_model = Some(model);
    self.state.config_dirty = true;
    Task::none()
}

Message::RoundSizeChanged(value) => {
    if let Ok(n) = value.parse() {
        self.state.settings.round_size = n;
        self.state.config_dirty = true;
    }
    Task::none()
}

Message::ValidateToggled(enabled) => {
    self.state.settings.validate_enabled = enabled;
    self.state.config_dirty = true;
    Task::none()
}

Message::VerifyCommandChanged(cmd) => {
    self.state.settings.verify_command = cmd;
    self.state.config_dirty = true;
    Task::none()
}

Message::GuidanceGlobalChanged(text) => {
    self.state.settings.guidance_global = text;
    self.state.config_dirty = true;
    Task::none()
}

Message::GuidanceBuilderChanged(text) => {
    self.state.settings.guidance_builder = text;
    self.state.config_dirty = true;
    Task::none()
}

Message::GuidanceReviewChanged(text) => {
    self.state.settings.guidance_review = text;
    self.state.config_dirty = true;
    Task::none()
}

Message::GuidanceValidatorChanged(text) => {
    self.state.settings.guidance_validator = text;
    self.state.config_dirty = true;
    Task::none()
}

Message::BackpressureCommandAdded(cmd) => {
    if !cmd.is_empty() {
        self.state.settings.backpressure_commands.push(cmd);
        self.state.config_dirty = true;
    }
    Task::none()
}

Message::BackpressureCommandRemoved(idx) => {
    if idx < self.state.settings.backpressure_commands.len() {
        self.state.settings.backpressure_commands.remove(idx);
        self.state.config_dirty = true;
    }
    Task::none()
}

Message::BackpressureStopOnFailureToggled(enabled) => {
    self.state.settings.backpressure_stop_on_failure = enabled;
    self.state.config_dirty = true;
    Task::none()
}

Message::BackpressureTimeoutChanged(value) => {
    if let Ok(n) = value.parse() {
        self.state.settings.backpressure_timeout = n;
        self.state.config_dirty = true;
    }
    Task::none()
}

Message::SaveSettings => {
    let settings = self.state.settings.clone();
    Task::perform(
        async move {
            let mut config = Config::load(None)?;

            // Update config from settings
            config.harness.kind = settings.selected_harness.unwrap_or_default();
            config.guidance.global = if settings.guidance_global.is_empty() {
                None
            } else {
                Some(settings.guidance_global)
            };
            config.guidance.builder = if settings.guidance_builder.is_empty() {
                None
            } else {
                Some(settings.guidance_builder)
            };
            config.guidance.review = if settings.guidance_review.is_empty() {
                None
            } else {
                Some(settings.guidance_review)
            };
            config.guidance.validator = if settings.guidance_validator.is_empty() {
                None
            } else {
                Some(settings.guidance_validator)
            };

            // Save config
            config.save()?;
            Ok(())
        },
        Message::SettingsSaved,
    )
}

Message::SettingsSaved(result) => {
    match result {
        Ok(()) => {
            self.state.config_dirty = false;
            self.state.output_buffer.push_str("Settings saved.\n");
        }
        Err(e) => {
            self.error = Some(format!("Failed to save settings: {}", e));
        }
    }
    Task::none()
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify SettingsState struct exists with all fields
grep -A 20 "pub struct SettingsState" descartes-gui/src/state.rs | grep -E "(scud_tag_input|selected_harness|selected_model|round_size|validate_enabled|guidance_global|backpressure_commands)" | wc -l
# Expected: 7 (at minimum)

# 5. Verify settings view has all sections
grep -E "(SCUD Configuration|Harness & Model|Execution Settings|Guidance|Backpressure)" descartes-gui/src/views/settings.rs | wc -l
# Expected: 5

# 6. Verify settings messages exist
grep -cE "^\s*(ScudTagInputChanged|HarnessSelected|ModelSelected|RoundSizeChanged|ValidateToggled|GuidanceGlobalChanged|BackpressureCommandAdded|SaveSettings)" descartes-gui/src/main.rs
# Expected: 8 or more

# 7. Verify form widgets imported
grep -q "pick_list" descartes-gui/src/main.rs && grep -q "toggler" descartes-gui/src/main.rs && echo "Form widgets imported"
# Expected: Form widgets imported

# 8. Verify SaveSettings handler saves to config
grep -A 10 "Message::SaveSettings =>" descartes-gui/src/main.rs | grep -q "config.save()" && echo "Config save implemented"
# Expected: Config save implemented
```

**Verification Gate**: All commands must succeed before proceeding to Phase 4.

---

## Phase 4: Skills View

### Overview
Implement the Skills browser with skill listing, detail view, variable input, and execution.

### Changes Required:

#### 4.1 Add Skills Messages

**File**: `descartes-gui/src/main.rs`
**Changes**: Add skills-related messages

```rust
enum Message {
    // ... existing ...

    // Skills
    LoadSkills,
    SkillsLoaded(Result<Vec<SkillInfo>, String>),
    SelectSkill(String),
    SkillArgChanged(String, String),  // (variable_name, value)
    RunSkill,
    SkillOutput(String),
    SkillComplete(Result<(), String>),
}
```

#### 4.2 Implement Skills View

**File**: `descartes-gui/src/views/skills.rs`
**Changes**: Full skills UI implementation

```rust
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column,
};
use iced::{Alignment, Element, Length};

use crate::state::{AppState, SkillInfo};
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    // Skills list panel
    let mut skills_list = Column::new().spacing(5);

    for skill in &state.skills {
        let is_selected = state.selected_skill.as_ref() == Some(&skill.name);
        let style = if is_selected {
            button::primary
        } else {
            button::secondary
        };

        skills_list = skills_list.push(
            button(
                column![
                    text(&skill.name).size(14),
                    text(&skill.description).size(12).style(|_| text::Style {
                        color: Some(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                    }),
                ]
                .spacing(2)
            )
            .on_press(Message::SelectSkill(skill.name.clone()))
            .style(style)
            .width(Length::Fill)
        );
    }

    let skills_panel = container(
        column![
            row![
                text("Skills").size(18),
                button("Refresh").on_press(Message::LoadSkills),
            ]
            .spacing(10),
            scrollable(skills_list).height(Length::Fill),
        ]
        .spacing(10)
    )
    .width(Length::Fixed(250.0))
    .height(Length::Fill)
    .padding(10)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.12, 0.12, 0.14))),
        ..Default::default()
    });

    // Detail panel
    let detail_panel = if let Some(skill_name) = &state.selected_skill {
        if let Some(skill) = state.skills.iter().find(|s| &s.name == skill_name) {
            view_skill_detail(skill, &state.skill_args)
        } else {
            text("Skill not found").into()
        }
    } else {
        container(
            text("Select a skill from the list").style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            })
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    };

    row![skills_panel, detail_panel]
        .spacing(10)
        .into()
}

fn view_skill_detail<'a>(skill: &SkillInfo, args: &std::collections::HashMap<String, String>) -> Element<'a, Message> {
    let mut content = Column::new().spacing(15);

    // Header
    content = content.push(
        column![
            text(&skill.name).size(24),
            text(&skill.description).size(14).style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.7, 0.7, 0.7)),
            }),
        ]
        .spacing(5)
    );

    // Metadata
    let mut meta = Column::new().spacing(5);
    if let Some(category) = &skill.category {
        meta = meta.push(
            row![
                text("Category:").width(Length::Fixed(100.0)),
                text(category),
            ]
        );
    }
    if !skill.aliases.is_empty() {
        meta = meta.push(
            row![
                text("Aliases:").width(Length::Fixed(100.0)),
                text(skill.aliases.join(", ")),
            ]
        );
    }
    content = content.push(meta);

    // Variables input
    if !skill.variables.is_empty() {
        let mut vars_section = Column::new().spacing(10);
        vars_section = vars_section.push(text("Variables:").size(16));

        for var in &skill.variables {
            let current_value = args.get(&var.name).cloned().unwrap_or_default();
            let placeholder = var.default.clone().unwrap_or_else(|| {
                if var.required { "Required" } else { "Optional" }.to_string()
            });

            let mut var_row = row![
                text(&var.name).width(Length::Fixed(120.0)),
                text_input(&placeholder, &current_value)
                    .on_input(move |v| Message::SkillArgChanged(var.name.clone(), v))
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);

            if var.required {
                var_row = var_row.push(
                    text("*").style(|_| text::Style {
                        color: Some(iced::Color::from_rgb(1.0, 0.4, 0.4)),
                    })
                );
            }

            vars_section = vars_section.push(var_row);

            if let Some(desc) = &var.description {
                vars_section = vars_section.push(
                    text(desc).size(12).style(|_| text::Style {
                        color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
                    })
                );
            }
        }

        content = content.push(vars_section);
    }

    // Run button
    content = content.push(
        button("Run Skill")
            .on_press(Message::RunSkill)
            .style(button::primary)
    );

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}
```

#### 4.3 Implement Skills Handlers

**File**: `descartes-gui/src/main.rs`
**Changes**: Handle skills messages

```rust
use descartes::interactive::skills::SkillRegistry;

// In update():

Message::LoadSkills => {
    Task::perform(
        async {
            let registry = SkillRegistry::new()?;
            let skills: Vec<SkillInfo> = registry
                .list()
                .into_iter()
                .map(|s| SkillInfo {
                    name: s.name.clone(),
                    description: s.description.clone(),
                    category: s.category.clone(),
                    aliases: s.aliases.clone(),
                    variables: s.variables.iter().map(|v| crate::state::SkillVariable {
                        name: v.name.clone(),
                        description: v.description.clone(),
                        required: v.required,
                        default: v.default.clone(),
                    }).collect(),
                })
                .collect();
            Ok(skills)
        },
        Message::SkillsLoaded,
    )
}

Message::SkillsLoaded(result) => {
    match result {
        Ok(skills) => self.state.skills = skills,
        Err(e) => self.error = Some(format!("Failed to load skills: {}", e)),
    }
    Task::none()
}

Message::SelectSkill(name) => {
    self.state.selected_skill = Some(name);
    self.state.skill_args.clear();
    Task::none()
}

Message::SkillArgChanged(name, value) => {
    self.state.skill_args.insert(name, value);
    Task::none()
}

Message::RunSkill => {
    if let Some(skill_name) = &self.state.selected_skill {
        let skill_name = skill_name.clone();
        let args = self.state.skill_args.clone();
        let config = self.state.config.clone();

        self.state.agent_status = AgentStatus::Running;
        self.state.output_buffer.clear();
        self.view = ViewMode::Output;  // Switch to output view

        return Task::perform(
            async move {
                let config = config.ok_or("No config loaded")?;
                let registry = SkillRegistry::new()?;
                let skill = registry.get(&skill_name).ok_or("Skill not found")?;

                // Load prompt with variable substitution
                let prompt = skill.load_prompt(&args)?;

                // Determine category and spawn agent
                let category = skill.category.as_deref().unwrap_or("builder");
                let harness = descartes::harness::create_harness(&config)?;

                let output = harness.run_prompt(&prompt).await?;
                Ok(output)
            },
            |result: Result<String, String>| match result {
                Ok(output) => Message::SkillOutput(output),
                Err(e) => Message::SkillComplete(Err(e)),
            }
        );
    }
    Task::none()
}

Message::SkillOutput(output) => {
    self.state.output_buffer.push_str(&output);
    Task::none()
}

Message::SkillComplete(result) => {
    self.state.agent_status = AgentStatus::Idle;
    match result {
        Ok(()) => {
            self.state.output_buffer.push_str("\n=== Skill complete ===\n");
        }
        Err(e) => {
            self.error = Some(format!("Skill failed: {}", e));
        }
    }
    Task::none()
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify SkillInfo and SkillVariable structs exist
grep -q "pub struct SkillInfo" descartes-gui/src/state.rs && grep -q "pub struct SkillVariable" descartes-gui/src/state.rs && echo "Skill structs defined"
# Expected: Skill structs defined

# 5. Verify skills messages exist
grep -cE "^\s*(LoadSkills|SkillsLoaded|SelectSkill|SkillArgChanged|RunSkill|SkillOutput|SkillComplete)" descartes-gui/src/main.rs
# Expected: 7

# 6. Verify skills view has list and detail panels
grep -q "skills_panel" descartes-gui/src/views/skills.rs && grep -q "detail_panel" descartes-gui/src/views/skills.rs && echo "Skills view has both panels"
# Expected: Skills view has both panels

# 7. Verify view_skill_detail function handles variables
grep -q "fn view_skill_detail" descartes-gui/src/views/skills.rs && grep -q "skill.variables" descartes-gui/src/views/skills.rs && echo "Variable handling implemented"
# Expected: Variable handling implemented

# 8. Verify SkillRegistry integration
grep -q "SkillRegistry" descartes-gui/src/main.rs && echo "SkillRegistry integrated"
# Expected: SkillRegistry integrated
```

**Verification Gate**: All commands must succeed before proceeding to Phase 5.

---

## Phase 5: SCUD Management View

### Overview
Implement SCUD task management including task list, detail view, status editing, tag management, and PRD import.

### Changes Required:

#### 5.1 Add SCUD Messages

**File**: `descartes-gui/src/main.rs`
**Changes**: Add SCUD-related messages

```rust
enum Message {
    // ... existing ...

    // SCUD Tasks
    LoadTasks,
    TasksLoaded(Result<Vec<TaskDetail>, String>),
    SelectTask(String),
    UpdateTaskStatus(String, String),  // (task_id, new_status)
    TaskStatusUpdated(Result<(), String>),

    // SCUD Tags
    CreateScudTag,
    DeleteScudTag(String),
    TagCreated(Result<(), String>),
    TagDeleted(Result<(), String>),

    // PRD Import
    SelectPrdFile,
    PrdFileSelected(Option<PathBuf>),
    ImportPrd,
    PrdImported(Result<usize, String>),  // Returns task count

    // Task Generation
    ExpandTask(String),
    TaskExpanded(Result<(), String>),
}
```

#### 5.2 Implement SCUD View

**File**: `descartes-gui/src/views/scud.rs`
**Changes**: Full SCUD management UI

```rust
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Column,
};
use iced::{Alignment, Element, Length};

use crate::state::{AppState, TaskDetail};
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    // Task list panel
    let mut task_list = Column::new().spacing(3);

    for task in &state.tasks {
        let is_selected = state.selected_task.as_ref() == Some(&task.id);
        let status_color = status_to_color(&task.status);

        let task_btn = button(
            row![
                container(text(&task.status).size(10))
                    .width(Length::Fixed(80.0))
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(status_color)),
                        border: iced::Border { radius: 4.0.into(), ..Default::default() },
                        ..Default::default()
                    })
                    .padding(2),
                text(&task.id).size(12).width(Length::Fixed(80.0)),
                text(&task.title).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        )
        .on_press(Message::SelectTask(task.id.clone()))
        .style(if is_selected { button::primary } else { button::secondary })
        .width(Length::Fill);

        task_list = task_list.push(task_btn);
    }

    let task_panel = container(
        column![
            row![
                text("Tasks").size(18),
                button("Refresh").on_press(Message::LoadTasks),
            ]
            .spacing(10),
            scrollable(task_list).height(Length::Fill),
        ]
        .spacing(10)
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill)
    .padding(10)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.12, 0.12, 0.14))),
        ..Default::default()
    });

    // Detail/Action panel
    let detail_panel = if let Some(task_id) = &state.selected_task {
        if let Some(task) = state.tasks.iter().find(|t| &t.id == task_id) {
            view_task_detail(task)
        } else {
            text("Task not found").into()
        }
    } else {
        view_actions_panel(state)
    };

    row![task_panel, detail_panel]
        .spacing(10)
        .into()
}

fn view_task_detail<'a>(task: &TaskDetail) -> Element<'a, Message> {
    let status_options = ["pending", "in-progress", "done", "failed", "blocked"];

    container(
        column![
            // Header
            text(&task.title).size(20),
            text(&task.id).size(12).style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            }),

            // Status editor
            row![
                text("Status:").width(Length::Fixed(100.0)),
                pick_list(
                    &status_options[..],
                    Some(task.status.as_str()),
                    |s: &str| Message::UpdateTaskStatus(task.id.clone(), s.to_string()),
                )
                .width(Length::Fixed(150.0)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),

            // Wave info
            if let Some(wave) = task.wave {
                row![
                    text("Wave:").width(Length::Fixed(100.0)),
                    text(format!("{}", wave + 1)),
                ]
            } else {
                row![text("")]
            },

            // Description
            text("Description:").size(14),
            container(
                text(&task.description)
            )
            .padding(10)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(0.1, 0.1, 0.12))),
                ..Default::default()
            })
            .width(Length::Fill),

            // Dependencies
            text(format!("Dependencies: {}",
                if task.dependencies.is_empty() {
                    "None".to_string()
                } else {
                    task.dependencies.join(", ")
                }
            )).size(12),

            // Actions
            row![
                button("Expand Task").on_press(Message::ExpandTask(task.id.clone())),
                button("Start Agent").on_press(Message::StartTask(task.id.clone())),
            ]
            .spacing(10),
        ]
        .spacing(15)
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill)
    .padding(20)
    .into()
}

fn view_actions_panel<'a>(state: &AppState) -> Element<'a, Message> {
    container(
        column![
            text("SCUD Actions").size(20),

            // Tag management
            section("Tag Management", column![
                row![
                    text("Active:").width(Length::Fixed(80.0)),
                    pick_list(
                        &state.scud_tags[..],
                        state.active_tag.as_ref(),
                        Message::ScudTagSelected,
                    )
                    .width(Length::Fixed(150.0)),
                ]
                .spacing(10),
                row![
                    text_input("new-tag", &state.settings.scud_tag_input)
                        .on_input(Message::ScudTagInputChanged)
                        .width(Length::Fixed(150.0)),
                    button("Create Tag").on_press(Message::CreateScudTag),
                ]
                .spacing(10),
            ].spacing(10)),

            // PRD Import
            section("Import from PRD", column![
                row![
                    button("Select PRD File").on_press(Message::SelectPrdFile),
                    text(state.prd_file.as_ref().map(|p| p.display().to_string()).unwrap_or_default()),
                ]
                .spacing(10),
                button("Import & Generate Tasks").on_press(Message::ImportPrd),
            ].spacing(10)),

            // Statistics
            section("Statistics", column![
                text(format!("Total Tasks: {}", state.tasks.len())),
                text(format!("Pending: {}", state.tasks.iter().filter(|t| t.status == "pending").count())),
                text(format!("Done: {}", state.tasks.iter().filter(|t| t.status == "done").count())),
                text(format!("Failed: {}", state.tasks.iter().filter(|t| t.status == "failed").count())),
            ].spacing(5)),
        ]
        .spacing(20)
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill)
    .padding(20)
    .into()
}

fn section<'a>(title: &str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        column![
            text(title).size(16),
            content.into(),
        ]
        .spacing(10)
    )
    .padding(15)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.15, 0.15, 0.18))),
        border: iced::Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

fn status_to_color(status: &str) -> iced::Color {
    match status {
        "done" => iced::Color::from_rgb(0.2, 0.6, 0.2),
        "in-progress" => iced::Color::from_rgb(0.2, 0.4, 0.8),
        "failed" => iced::Color::from_rgb(0.8, 0.2, 0.2),
        "blocked" => iced::Color::from_rgb(0.6, 0.4, 0.1),
        _ => iced::Color::from_rgb(0.4, 0.4, 0.4),
    }
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify TaskDetail struct exists
grep -q "pub struct TaskDetail" descartes-gui/src/state.rs && echo "TaskDetail defined"
# Expected: TaskDetail defined

# 5. Verify SCUD messages exist
grep -cE "^\s*(LoadTasks|TasksLoaded|SelectTask|UpdateTaskStatus|TaskStatusUpdated|CreateScudTag|DeleteScudTag|SelectPrdFile|ImportPrd|ExpandTask)" descartes-gui/src/main.rs
# Expected: 10

# 6. Verify SCUD view has task list and detail panels
grep -q "task_panel" descartes-gui/src/views/scud.rs && grep -q "detail_panel" descartes-gui/src/views/scud.rs && echo "SCUD view has both panels"
# Expected: SCUD view has both panels

# 7. Verify status color mapping function
grep -q "fn status_to_color" descartes-gui/src/views/scud.rs && echo "Status colors implemented"
# Expected: Status colors implemented

# 8. Verify view_task_detail has status picker
grep -A 20 "fn view_task_detail" descartes-gui/src/views/scud.rs | grep -q "pick_list" && echo "Status picker in detail view"
# Expected: Status picker in detail view

# 9. Verify PRD import handler exists
grep -q "Message::ImportPrd" descartes-gui/src/main.rs && echo "PRD import handler exists"
# Expected: PRD import handler exists
```

**Verification Gate**: All commands must succeed before proceeding to Phase 6.

---

## Phase 6: Transcripts View

### Overview
Implement transcript browser with list, search, detail viewer, and replay functionality.

### Changes Required:

#### 6.1 Add Transcript Messages

**File**: `descartes-gui/src/main.rs`
**Changes**: Add transcript-related messages

```rust
enum Message {
    // ... existing ...

    // Transcripts
    LoadTranscripts,
    TranscriptsLoaded(Result<Vec<TranscriptInfo>, String>),
    SelectTranscript(String),
    TranscriptLoaded(Result<String, String>),
    TranscriptSearchChanged(String),
    ReplayTranscript(String),
}
```

#### 6.2 Implement Transcripts View

**File**: `descartes-gui/src/views/transcripts.rs`
**Changes**: Full transcript browser UI

```rust
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column,
};
use iced::{Element, Length};

use crate::state::{AppState, TranscriptInfo};
use crate::Message;

pub fn view(state: &AppState) -> Element<Message> {
    // Search/filter
    let search_bar = row![
        text_input("Search transcripts...", &state.transcript_search)
            .on_input(Message::TranscriptSearchChanged)
            .width(Length::Fill),
        button("Refresh").on_press(Message::LoadTranscripts),
    ]
    .spacing(10);

    // Transcript list
    let filtered: Vec<_> = state.transcripts.iter()
        .filter(|t| {
            state.transcript_search.is_empty() ||
            t.id.contains(&state.transcript_search) ||
            t.task_id.as_ref().map(|id| id.contains(&state.transcript_search)).unwrap_or(false) ||
            t.category.contains(&state.transcript_search)
        })
        .collect();

    let mut list = Column::new().spacing(5);
    for transcript in filtered {
        let is_selected = state.selected_transcript.as_ref() == Some(&transcript.id);

        list = list.push(
            button(
                row![
                    text(&transcript.timestamp).size(12).width(Length::Fixed(180.0)),
                    text(&transcript.category).size(12).width(Length::Fixed(100.0)),
                    text(transcript.task_id.as_deref().unwrap_or("-")).size(12),
                ]
                .spacing(10)
            )
            .on_press(Message::SelectTranscript(transcript.id.clone()))
            .style(if is_selected { button::primary } else { button::secondary })
            .width(Length::Fill)
        );
    }

    let list_panel = container(
        column![
            search_bar,
            scrollable(list).height(Length::Fill),
        ]
        .spacing(10)
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill)
    .padding(10);

    // Content panel
    let content_panel = if let Some(content) = &state.transcript_content {
        container(
            column![
                row![
                    text("Transcript").size(18),
                    if let Some(id) = &state.selected_transcript {
                        button("Replay").on_press(Message::ReplayTranscript(id.clone()))
                    } else {
                        button("Replay")
                    },
                ]
                .spacing(10),
                scrollable(
                    container(text(content))
                        .padding(10)
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(0.1, 0.1, 0.12))),
                            ..Default::default()
                        })
                )
                .height(Length::Fill),
            ]
            .spacing(10)
        )
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .padding(10)
        .into()
    } else {
        container(
            text("Select a transcript to view").style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            })
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::FillPortion(3))
        .into()
    };

    row![list_panel, content_panel]
        .spacing(10)
        .into()
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify TranscriptInfo struct exists
grep -q "pub struct TranscriptInfo" descartes-gui/src/state.rs && echo "TranscriptInfo defined"
# Expected: TranscriptInfo defined

# 5. Verify transcript messages exist
grep -cE "^\s*(LoadTranscripts|TranscriptsLoaded|SelectTranscript|TranscriptLoaded|TranscriptSearchChanged|ReplayTranscript)" descartes-gui/src/main.rs
# Expected: 6

# 6. Verify transcripts view has search and list
grep -q "search_bar" descartes-gui/src/views/transcripts.rs && grep -q "list_panel" descartes-gui/src/views/transcripts.rs && echo "Transcripts view has search and list"
# Expected: Transcripts view has search and list

# 7. Verify content panel displays transcript
grep -q "content_panel" descartes-gui/src/views/transcripts.rs && grep -q "transcript_content" descartes-gui/src/views/transcripts.rs && echo "Content panel implemented"
# Expected: Content panel implemented

# 8. Verify replay button exists
grep -q "ReplayTranscript" descartes-gui/src/views/transcripts.rs && echo "Replay button implemented"
# Expected: Replay button implemented
```

**Verification Gate**: All commands must succeed before proceeding to Phase 7.

---

## Phase 7: Polish & Enhancement

### Overview
Add keyboard shortcuts, command palette, and UI polish.

### Changes Required:

#### 7.1 Add Keyboard Shortcuts

**File**: `descartes-gui/src/main.rs`
**Changes**: Handle keyboard events

```rust
use iced::keyboard;

fn subscription(&self) -> Subscription<Message> {
    let keyboard_sub = keyboard::on_key_press(|key, modifiers| {
        match (key, modifiers.command()) {
            // Cmd/Ctrl + number for view switching
            (keyboard::Key::Character("1"), true) => Some(Message::SwitchView(ViewMode::Waves)),
            (keyboard::Key::Character("2"), true) => Some(Message::SwitchView(ViewMode::Agents)),
            (keyboard::Key::Character("3"), true) => Some(Message::SwitchView(ViewMode::Output)),
            (keyboard::Key::Character("4"), true) => Some(Message::SwitchView(ViewMode::Settings)),
            (keyboard::Key::Character("5"), true) => Some(Message::SwitchView(ViewMode::Skills)),
            (keyboard::Key::Character("6"), true) => Some(Message::SwitchView(ViewMode::Scud)),
            (keyboard::Key::Character("7"), true) => Some(Message::SwitchView(ViewMode::Transcripts)),

            // Cmd/Ctrl + R for refresh
            (keyboard::Key::Character("r"), true) => Some(Message::LoadWaves),

            // Cmd/Ctrl + S for save settings
            (keyboard::Key::Character("s"), true) => Some(Message::SaveSettings),

            // Escape to cancel/dismiss
            (keyboard::Key::Escape, _) => Some(Message::DismissError),

            // Space to pause/resume
            (keyboard::Key::Space, _) => {
                // Context-dependent
                None
            }

            _ => None,
        }
    });

    let time_sub = if self.state.swarm_running {
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    Subscription::batch([keyboard_sub, time_sub])
}
```

#### 7.2 Add Status Bar

**File**: `descartes-gui/src/main.rs`
**Changes**: Add status bar at bottom

```rust
fn view(&self) -> Element<Message> {
    let header = self.view_header();
    let content = // ... existing view dispatch
    let status_bar = self.view_status_bar();

    let main_column = if let Some(ref error) = self.error {
        let error_banner = self.view_error_banner(error);
        column![error_banner, header, content, status_bar]
    } else {
        column![header, content, status_bar]
    };

    container(main_column)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

fn view_status_bar(&self) -> Element<Message> {
    let status = match self.state.agent_status {
        AgentStatus::Idle => "Ready",
        AgentStatus::Running => "Running...",
        AgentStatus::Paused => "Paused",
    };

    let task_info = self.state.current_task.as_ref()
        .map(|t| format!("Task: {}", t))
        .unwrap_or_default();

    let tag_info = self.state.active_tag.as_ref()
        .map(|t| format!("Tag: {}", t))
        .unwrap_or_else(|| "No tag selected".to_string());

    let shortcuts = "Cmd+1-7: Views | Cmd+R: Refresh | Cmd+S: Save";

    container(
        row![
            text(status).size(12),
            text(task_info).size(12),
            text(tag_info).size(12),
            text(shortcuts).size(10).style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            }),
        ]
        .spacing(20)
    )
    .padding(5)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.1, 0.1, 0.1))),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
```

#### 7.3 Add Unsaved Changes Indicator

**File**: `descartes-gui/src/main.rs`
**Changes**: Show indicator when settings are dirty

```rust
fn view_header(&self) -> Element<Message> {
    let nav_buttons = // ... existing

    let mut indicators = row![].spacing(10);

    if self.state.config_dirty {
        indicators = indicators.push(
            text("Unsaved changes").size(12).style(|_| text::Style {
                color: Some(iced::Color::from_rgb(1.0, 0.7, 0.3)),
            })
        );
    }

    if self.state.swarm_running {
        indicators = indicators.push(
            text(format!("Wave {}/{}",
                self.state.current_wave + 1,
                self.state.waves.len()
            )).size(12).style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.4, 0.8, 0.4)),
            })
        );
    }

    row![nav_buttons, indicators]
        .spacing(20)
        .align_y(Alignment::Center)
        .into()
}
```

### Success Criteria:

All verification is automated. Run these commands to verify the phase is complete:

```bash
# 1. Compile check
cargo build -p descartes-gui

# 2. Run tests
cargo test -p descartes-gui

# 3. Clippy check
cargo clippy -p descartes-gui -- -D warnings

# 4. Verify keyboard import
grep -q "use iced::keyboard" descartes-gui/src/main.rs && echo "Keyboard module imported"
# Expected: Keyboard module imported

# 5. Verify keyboard shortcut handlers exist (Cmd+1 through Cmd+7)
grep -E 'Character\("[1-7]"\)' descartes-gui/src/main.rs | wc -l
# Expected: 7

# 6. Verify view_status_bar function exists
grep -q "fn view_status_bar" descartes-gui/src/main.rs && echo "Status bar implemented"
# Expected: Status bar implemented

# 7. Verify config_dirty indicator in header
grep -A 10 "fn view_header" descartes-gui/src/main.rs | grep -q "config_dirty" && echo "Unsaved indicator implemented"
# Expected: Unsaved indicator implemented

# 8. Verify subscription includes keyboard handling
grep -A 5 "fn subscription" descartes-gui/src/main.rs | grep -q "keyboard" && echo "Keyboard subscription active"
# Expected: Keyboard subscription active

# 9. Final integration check - run the GUI binary (non-interactive)
cargo build -p descartes-gui --release && echo "Release build successful"
# Expected: Release build successful
```

**Phase 7 Complete**: All automated verification passes indicate the GUI implementation is complete.

---

## Testing Strategy

### Unit Tests:
- State transitions for each Message type
- View rendering with various state combinations
- Form validation logic

### Integration Tests:
- Config loading/saving round-trip
- SCUD operations (list, status change)
- Skill loading and execution
- Transcript loading

### End-to-End Verification Script:

Run this comprehensive verification after all phases are complete:

```bash
#!/bin/bash
# Full implementation verification script

set -e
echo "=== GUI Full Feature Verification ==="

# Build checks
echo "1. Building all packages..."
cargo build -p descartes-gui -p descartes

echo "2. Running workspace tests..."
cargo test --workspace

echo "3. Running clippy..."
cargo clippy -p descartes-gui -p descartes -- -D warnings

# Code structure verification
echo "4. Verifying view modules..."
VIEWS=$(ls descartes-gui/src/views/*.rs 2>/dev/null | wc -l)
[ "$VIEWS" -ge 7 ] && echo "   View modules: OK ($VIEWS files)" || exit 1

echo "5. Verifying ViewMode variants..."
VARIANTS=$(grep -E "^\s*(Waves|Agents|Output|Settings|Skills|Scud|Transcripts)," descartes-gui/src/main.rs | wc -l)
[ "$VARIANTS" -eq 7 ] && echo "   ViewMode variants: OK (7)" || exit 1

echo "6. Verifying state structs..."
grep -q "pub struct AppState" descartes-gui/src/state.rs && echo "   AppState: OK"
grep -q "pub struct SettingsState" descartes-gui/src/state.rs && echo "   SettingsState: OK"
grep -q "pub struct SkillInfo" descartes-gui/src/state.rs && echo "   SkillInfo: OK"
grep -q "pub struct TaskDetail" descartes-gui/src/state.rs && echo "   TaskDetail: OK"
grep -q "pub struct TranscriptInfo" descartes-gui/src/state.rs && echo "   TranscriptInfo: OK"

echo "7. Verifying executor module..."
test -f descartes-gui/src/executor.rs && echo "   executor.rs: OK"
grep -q "pub struct GuiExecutor" descartes-gui/src/executor.rs && echo "   GuiExecutor: OK"

echo "8. Verifying keyboard shortcuts..."
SHORTCUTS=$(grep -E 'Character\("[1-7]"\)' descartes-gui/src/main.rs | wc -l)
[ "$SHORTCUTS" -eq 7 ] && echo "   Keyboard shortcuts: OK (7)" || exit 1

echo "9. Verifying TODO removed..."
! grep -q "// TODO: Actually spawn the agent" descartes-gui/src/main.rs && echo "   StartTask TODO: Removed"

echo "10. Building release binary..."
cargo build -p descartes-gui --release && echo "    Release build: OK"

echo ""
echo "=== All verifications passed! ==="
```

---

## Performance Considerations

1. **Lazy Loading**: Only load transcript content when selected, not entire list
2. **Pagination**: If task list grows large, implement virtual scrolling
3. **Debouncing**: Debounce text input handlers to reduce update frequency
4. **Async Loading**: All data loads are async to keep UI responsive

---

## References

- GUI research: `thoughts/shared/research/2026-01-15-gui-feature-coverage.md`
- Feature research: `thoughts/shared/research/2026-01-15-review-guidance-planning-workflows.md`
- Current GUI: `descartes-gui/src/main.rs`
- CLI commands: `descartes/src/main.rs`
- Interactive session: `descartes/src/interactive/session.rs`
- Skills system: `descartes/src/interactive/skills.rs`
