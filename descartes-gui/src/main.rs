//! Descartes GUI - Desktop interface for AI agent orchestration
//!
//! A minimal Iced GUI that wraps v2's simple architecture:
//! - Direct library calls to RalphExecutor
//! - SCUD storage for task/wave management
//! - AgentRegistry for status display

use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use tokio::sync::mpsc;

// Use descartes scud module
use descartes::{scud, Config};

mod state;
mod theme;
mod views;

use state::{AgentStatus, AppState, TaskInfo};

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("descartes_gui=debug,descartes=info")
        .init();

    iced::application(DescartesGui::new, DescartesGui::update, DescartesGui::view)
        .subscription(DescartesGui::subscription)
        .theme(DescartesGui::theme)
        .title("Descartes")
        .run()
}

/// Main application state
struct DescartesGui {
    /// Current view mode
    view: ViewMode,
    /// Application state
    state: AppState,
    /// Control channel for agent commands
    control_tx: Option<mpsc::Sender<AgentCommand>>,
    /// Error message to display
    error: Option<String>,
}

/// View modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    Waves,
    Agents,
    Output,
}

/// Application messages
#[derive(Debug, Clone)]
enum Message {
    // Navigation
    SwitchView(ViewMode),

    // Task management
    LoadWaves,
    WavesLoaded(Result<Vec<Vec<TaskInfo>>, String>),

    // Agent management
    StartAgent(String), // task_id
    AgentOutput(String),
    AgentComplete(Result<(), String>),

    // Control
    PauseAgent,
    ResumeAgent,
    CancelAgent,

    // UI
    DismissError,
    Tick,
}

/// Commands to send to the agent runner
#[derive(Debug, Clone)]
enum AgentCommand {
    Pause,
    Resume,
    Cancel,
}

impl DescartesGui {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                view: ViewMode::Waves,
                state: AppState::default(),
                control_tx: None,
                error: None,
            },
            Task::perform(async { () }, |_| Message::LoadWaves),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchView(view) => {
                self.view = view;
                Task::none()
            }

            Message::LoadWaves => Task::perform(load_waves_from_scud(), Message::WavesLoaded),

            Message::WavesLoaded(result) => {
                match result {
                    Ok(waves) => {
                        self.state.waves = waves;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load waves: {}", e));
                    }
                }
                Task::none()
            }

            Message::StartAgent(task_id) => {
                self.state.agent_status = AgentStatus::Running;
                self.state.current_task = Some(task_id.clone());
                self.state.output_buffer.clear();

                // TODO: Actually spawn the agent via RalphExecutor
                self.state
                    .output_buffer
                    .push_str(&format!("Starting agent for task {}...\n", task_id));

                Task::none()
            }

            Message::AgentOutput(text) => {
                self.state.output_buffer.push_str(&text);
                Task::none()
            }

            Message::AgentComplete(result) => {
                self.state.agent_status = AgentStatus::Idle;
                match result {
                    Ok(()) => {
                        self.state.output_buffer.push_str("\n[Agent completed]\n");
                    }
                    Err(e) => {
                        self.state
                            .output_buffer
                            .push_str(&format!("\n[Agent error: {}]\n", e));
                    }
                }
                Task::none()
            }

            Message::PauseAgent => {
                if let Some(ref tx) = self.control_tx {
                    let tx = tx.clone();
                    return Task::perform(
                        async move {
                            let _ = tx.send(AgentCommand::Pause).await;
                        },
                        |_| Message::Tick,
                    );
                }
                self.state.agent_status = AgentStatus::Paused;
                Task::none()
            }

            Message::ResumeAgent => {
                if let Some(ref tx) = self.control_tx {
                    let tx = tx.clone();
                    return Task::perform(
                        async move {
                            let _ = tx.send(AgentCommand::Resume).await;
                        },
                        |_| Message::Tick,
                    );
                }
                self.state.agent_status = AgentStatus::Running;
                Task::none()
            }

            Message::CancelAgent => {
                if let Some(ref tx) = self.control_tx {
                    let tx = tx.clone();
                    return Task::perform(
                        async move {
                            let _ = tx.send(AgentCommand::Cancel).await;
                        },
                        |_| Message::Tick,
                    );
                }
                self.state.agent_status = AgentStatus::Idle;
                Task::none()
            }

            Message::DismissError => {
                self.error = None;
                Task::none()
            }

            Message::Tick => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let main_column: Element<Message> = if let Some(ref error) = self.error {
            // Show error banner at top
            let error_banner = container(
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
            });

            let header = self.view_header();
            let content = match self.view {
                ViewMode::Waves => self.view_waves(),
                ViewMode::Agents => self.view_agents(),
                ViewMode::Output => self.view_output(),
            };

            column![error_banner, header, content].spacing(10).into()
        } else {
            let header = self.view_header();
            let content = match self.view {
                ViewMode::Waves => self.view_waves(),
                ViewMode::Agents => self.view_agents(),
                ViewMode::Output => self.view_output(),
            };

            column![header, content].spacing(10).into()
        };

        container(main_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    fn view_header(&self) -> Element<Message> {
        let nav_buttons = row![
            button(text("Waves"))
                .on_press(Message::SwitchView(ViewMode::Waves))
                .style(if self.view == ViewMode::Waves {
                    button::primary
                } else {
                    button::secondary
                }),
            button(text("Agents"))
                .on_press(Message::SwitchView(ViewMode::Agents))
                .style(if self.view == ViewMode::Agents {
                    button::primary
                } else {
                    button::secondary
                }),
            button(text("Output"))
                .on_press(Message::SwitchView(ViewMode::Output))
                .style(if self.view == ViewMode::Output {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(10);

        let status = text(format!("Status: {:?}", self.state.agent_status));

        row![nav_buttons, status]
            .spacing(20)
            .align_y(Alignment::Center)
            .into()
    }

    fn view_waves(&self) -> Element<Message> {
        let mut waves_column = Column::new().spacing(15);

        if self.state.waves.is_empty() {
            waves_column = waves_column.push(text("No tasks loaded. Run `scud list` to check tasks."));
        } else {
            for (wave_idx, wave) in self.state.waves.iter().enumerate() {
                let wave_header = text(format!("Wave {}", wave_idx + 1))
                    .size(18);

                let mut task_column = Column::new().spacing(5);
                for task in wave {
                    let task_row = row![
                        text(&task.id).width(Length::Fixed(80.0)),
                        text(&task.title).width(Length::Fill),
                        text(&task.status).width(Length::Fixed(100.0)),
                        button("Start").on_press(Message::StartAgent(task.id.clone())),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center);

                    task_column = task_column.push(task_row);
                }

                waves_column = waves_column.push(wave_header).push(task_column);
            }
        }

        let refresh_button = button("Refresh").on_press(Message::LoadWaves);

        column![refresh_button, scrollable(waves_column).height(Length::Fill)]
            .spacing(10)
            .into()
    }

    fn view_agents(&self) -> Element<Message> {
        let status_text = match self.state.agent_status {
            AgentStatus::Idle => "No agent running",
            AgentStatus::Running => "Agent is running...",
            AgentStatus::Paused => "Agent is paused",
        };

        let mut controls = row![].spacing(10);

        match self.state.agent_status {
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

        let current_task = if let Some(ref task_id) = self.state.current_task {
            text(format!("Current task: {}", task_id))
        } else {
            text("No task selected")
        };

        column![text(status_text).size(18), current_task, controls]
            .spacing(15)
            .into()
    }

    fn view_output(&self) -> Element<Message> {
        let output_text = if self.state.output_buffer.is_empty() {
            text("No output yet. Start an agent to see output here.")
        } else {
            text(&self.state.output_buffer)
        };

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
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

/// Load waves from SCUD storage
async fn load_waves_from_scud() -> Result<Vec<Vec<TaskInfo>>, String> {
    // Load descartes config (defaults work fine for just SCUD access)
    let config = Config::load(None).map_err(|e| e.to_string())?;

    // Get all tasks from active group
    let tasks = scud::list_tasks(&config).map_err(|e| e.to_string())?;

    // Get wave IDs
    let wave_ids = scud::waves(&config).map_err(|e| e.to_string())?;

    // Convert to TaskInfo, matching IDs from waves to tasks
    let waves: Vec<Vec<TaskInfo>> = wave_ids
        .into_iter()
        .map(|wave| {
            wave.into_iter()
                .filter_map(|id| {
                    tasks.iter().find(|t| t.id == id).map(|t| TaskInfo {
                        id: t.id.clone(),
                        title: t.title.clone(),
                        status: format!("{:?}", t.status),
                    })
                })
                .collect()
        })
        .collect();

    Ok(waves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    /// Create a test instance without the Task
    fn test_app() -> DescartesGui {
        DescartesGui {
            view: ViewMode::Waves,
            state: AppState::default(),
            control_tx: None,
            error: None,
        }
    }

    /// Headless UI test using iced_test simulator
    #[test]
    fn test_ui_navigation_clicks() {
        let mut app = test_app();

        // Render the view and create a simulator
        let mut ui = simulator(app.view());

        // Click on "Agents" button
        let _ = ui.click("Agents");

        // Process the messages generated by the click
        for message in ui.into_messages() {
            let _ = app.update(message);
        }

        // Verify the view switched to Agents
        assert_eq!(app.view, ViewMode::Agents);

        // Render again with updated state
        let mut ui = simulator(app.view());

        // Click on "Output" button
        let _ = ui.click("Output");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }

        assert_eq!(app.view, ViewMode::Output);

        // Click back to "Waves"
        let mut ui = simulator(app.view());
        let _ = ui.click("Waves");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }

        assert_eq!(app.view, ViewMode::Waves);
    }

    /// Test clicking Refresh button in waves view
    #[test]
    fn test_ui_refresh_click() {
        let mut app = test_app();

        let mut ui = simulator(app.view());

        // Click the Refresh button
        let click_result = ui.click("Refresh");
        assert!(click_result.is_ok(), "Refresh button should be clickable");

        // Collect messages - should have LoadWaves
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(!messages.is_empty(), "Click should generate a message");

        // The message should be LoadWaves
        for message in messages {
            match message {
                Message::LoadWaves => {
                    // Expected - this triggers async wave loading
                }
                _ => panic!("Expected LoadWaves message, got {:?}", message),
            }
        }
    }

    #[test]
    fn test_switch_view() {
        let mut app = test_app();
        assert_eq!(app.view, ViewMode::Waves);

        let _ = app.update(Message::SwitchView(ViewMode::Agents));
        assert_eq!(app.view, ViewMode::Agents);

        let _ = app.update(Message::SwitchView(ViewMode::Output));
        assert_eq!(app.view, ViewMode::Output);
    }

    #[test]
    fn test_waves_loaded_success() {
        let mut app = test_app();

        let waves = vec![
            vec![TaskInfo {
                id: "1".into(),
                title: "First task".into(),
                status: "Pending".into(),
            }],
            vec![TaskInfo {
                id: "2".into(),
                title: "Second task".into(),
                status: "Pending".into(),
            }],
        ];

        let _ = app.update(Message::WavesLoaded(Ok(waves)));
        assert_eq!(app.state.waves.len(), 2);
        assert!(app.error.is_none());
    }

    #[test]
    fn test_waves_loaded_error() {
        let mut app = test_app();

        let _ = app.update(Message::WavesLoaded(Err("Connection failed".into())));
        assert!(app.error.is_some());
        assert!(app.error.as_ref().unwrap().contains("Connection failed"));
    }

    #[test]
    fn test_dismiss_error() {
        let mut app = test_app();
        app.error = Some("Test error".into());

        let _ = app.update(Message::DismissError);
        assert!(app.error.is_none());
    }

    #[test]
    fn test_start_agent() {
        let mut app = test_app();

        let _ = app.update(Message::StartAgent("task-1".into()));
        assert_eq!(app.state.agent_status, AgentStatus::Running);
        assert_eq!(app.state.current_task, Some("task-1".into()));
        assert!(!app.state.output_buffer.is_empty());
    }

    #[test]
    fn test_pause_resume_agent() {
        let mut app = test_app();
        app.state.agent_status = AgentStatus::Running;

        // Without control_tx, it just updates state directly
        let _ = app.update(Message::PauseAgent);
        assert_eq!(app.state.agent_status, AgentStatus::Paused);

        let _ = app.update(Message::ResumeAgent);
        assert_eq!(app.state.agent_status, AgentStatus::Running);
    }

    #[test]
    fn test_cancel_agent() {
        let mut app = test_app();
        app.state.agent_status = AgentStatus::Running;
        app.state.current_task = Some("task-1".into());

        let _ = app.update(Message::CancelAgent);
        assert_eq!(app.state.agent_status, AgentStatus::Idle);
    }

    #[test]
    fn test_initial_state() {
        let (app, _) = DescartesGui::new();
        assert_eq!(app.view, ViewMode::Waves);
        assert_eq!(app.state.agent_status, AgentStatus::Idle);
        assert!(app.state.current_task.is_none());
        assert!(app.error.is_none());
    }

    // =============================================================
    // Additional UI Interaction Tests
    // =============================================================

    /// Test clicking Pause/Resume buttons in Agents view when agent is running
    #[test]
    fn test_ui_agent_control_buttons() {
        let mut app = test_app();

        // Start an agent first
        let _ = app.update(Message::StartAgent("task-1".into()));
        assert_eq!(app.state.agent_status, AgentStatus::Running);

        // Switch to Agents view
        let _ = app.update(Message::SwitchView(ViewMode::Agents));

        // Render and find the Pause button
        let mut ui = simulator(app.view());
        let pause_result = ui.click("Pause");
        assert!(pause_result.is_ok(), "Pause button should exist when agent is running");

        // Process messages
        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Paused);

        // Now Resume button should appear
        let mut ui = simulator(app.view());
        let resume_result = ui.click("Resume");
        assert!(resume_result.is_ok(), "Resume button should exist when agent is paused");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Running);
    }

    /// Test clicking Cancel button stops the agent
    #[test]
    fn test_ui_cancel_agent() {
        let mut app = test_app();

        // Start an agent
        let _ = app.update(Message::StartAgent("task-1".into()));
        let _ = app.update(Message::SwitchView(ViewMode::Agents));

        let mut ui = simulator(app.view());
        let cancel_result = ui.click("Cancel");
        assert!(cancel_result.is_ok(), "Cancel button should exist");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Idle);
    }

    /// Test error banner dismiss interaction
    #[test]
    fn test_ui_error_banner_dismiss() {
        let mut app = test_app();
        app.error = Some("Test error message".to_string());

        let mut ui = simulator(app.view());

        // Error banner should have Dismiss button
        let dismiss_result = ui.click("Dismiss");
        assert!(dismiss_result.is_ok(), "Dismiss button should exist in error banner");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }
        assert!(app.error.is_none(), "Error should be dismissed");
    }

    /// Test clicking Start button on a task row
    #[test]
    fn test_ui_start_task_from_waves() {
        let mut app = test_app();

        // Load some tasks
        let waves = vec![vec![
            TaskInfo {
                id: "1".into(),
                title: "First task".into(),
                status: "Pending".into(),
            },
            TaskInfo {
                id: "2".into(),
                title: "Second task".into(),
                status: "Pending".into(),
            },
        ]];
        let _ = app.update(Message::WavesLoaded(Ok(waves)));

        // Render waves view
        let mut ui = simulator(app.view());

        // Click Start button (there are multiple, clicking finds first)
        let start_result = ui.click("Start");
        assert!(start_result.is_ok(), "Start button should exist");

        for message in ui.into_messages() {
            let _ = app.update(message);
        }

        // Agent should be started
        assert_eq!(app.state.agent_status, AgentStatus::Running);
        assert!(app.state.current_task.is_some());
    }

    /// Test status display updates correctly
    #[test]
    fn test_ui_status_display() {
        let mut app = test_app();

        // Check initial status shows Idle
        {
            let mut ui = simulator(app.view());
            let status_find = ui.find("Status: Idle");
            assert!(status_find.is_ok(), "Should show Status: Idle initially");
        }

        // Start agent and check status updates
        let _ = app.update(Message::StartAgent("task-1".into()));
        let mut ui = simulator(app.view());
        let status_find = ui.find("Status: Running");
        assert!(status_find.is_ok(), "Should show Status: Running when agent runs");
    }

    /// Test Output view displays agent output
    #[test]
    fn test_ui_output_view_content() {
        let mut app = test_app();

        // Add some output
        let _ = app.update(Message::StartAgent("task-1".into()));
        let _ = app.update(Message::AgentOutput("Line 1\n".into()));
        let _ = app.update(Message::AgentOutput("Line 2\n".into()));

        // Verify the model state directly (more reliable than UI text search for long content)
        assert!(app.state.output_buffer.contains("Line 1"), "Output buffer should contain Line 1");
        assert!(app.state.output_buffer.contains("Line 2"), "Output buffer should contain Line 2");

        // Switch to output view and verify it renders without error
        let _ = app.update(Message::SwitchView(ViewMode::Output));
        let ui = simulator(app.view());
        // Just verify it renders - the output is in a scrollable container
        drop(ui);
    }

    // =============================================================
    // Full Loop Headless Test
    // =============================================================

    /// Simulates a complete workflow: load tasks -> start agent -> pause -> resume -> complete
    #[test]
    fn test_full_workflow_headless() {
        let mut app = test_app();

        // Step 1: Simulate loading waves (mimics async WavesLoaded result)
        let waves = vec![
            vec![TaskInfo {
                id: "1".into(),
                title: "Setup environment".into(),
                status: "Pending".into(),
            }],
            vec![TaskInfo {
                id: "2".into(),
                title: "Build core module".into(),
                status: "Pending".into(),
            }],
        ];
        let _ = app.update(Message::WavesLoaded(Ok(waves)));
        assert_eq!(app.state.waves.len(), 2, "Should have 2 waves loaded");

        // Step 2: Click Start on first task via UI
        let mut ui = simulator(app.view());
        let _ = ui.click("Start");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Running);
        assert_eq!(app.state.current_task, Some("1".into()));

        // Step 3: Navigate to Agents view
        let mut ui = simulator(app.view());
        let _ = ui.click("Agents");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert_eq!(app.view, ViewMode::Agents);

        // Step 4: Pause the agent
        let mut ui = simulator(app.view());
        let _ = ui.click("Pause");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Paused);

        // Step 5: Resume the agent
        let mut ui = simulator(app.view());
        let _ = ui.click("Resume");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert_eq!(app.state.agent_status, AgentStatus::Running);

        // Step 6: Simulate agent output arriving
        let _ = app.update(Message::AgentOutput("Processing task 1...\n".into()));
        let _ = app.update(Message::AgentOutput("Done!\n".into()));

        // Step 7: Agent completes
        let _ = app.update(Message::AgentComplete(Ok(())));
        assert_eq!(app.state.agent_status, AgentStatus::Idle);

        // Step 8: Navigate to Output view to verify output
        let mut ui = simulator(app.view());
        let _ = ui.click("Output");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert_eq!(app.view, ViewMode::Output);

        // Verify output buffer contains expected content (model assertion)
        assert!(
            app.state.output_buffer.contains("Processing task 1"),
            "Output should show task processing"
        );
        assert!(
            app.state.output_buffer.contains("[Agent completed]"),
            "Output should show completion"
        );
    }

    /// Test workflow with an error condition
    #[test]
    fn test_error_workflow_headless() {
        let mut app = test_app();

        // Step 1: Waves load fails
        let _ = app.update(Message::WavesLoaded(Err("Network timeout".into())));
        assert!(app.error.is_some());

        // Step 2: Verify error is set in model (UI text search may not work for styled text)
        assert!(
            app.error.as_ref().unwrap().contains("Network timeout"),
            "Error should contain the message"
        );

        // Step 3: Dismiss error via UI - find and click Dismiss button
        let mut ui = simulator(app.view());
        let dismiss_result = ui.click("Dismiss");
        assert!(dismiss_result.is_ok(), "Dismiss button should be present in error banner");
        for msg in ui.into_messages() {
            let _ = app.update(msg);
        }
        assert!(app.error.is_none(), "Error should be dismissed");

        // Step 4: Retry loading (click Refresh)
        let mut ui = simulator(app.view());
        let _ = ui.click("Refresh");
        let messages: Vec<_> = ui.into_messages().collect();
        assert!(
            messages.iter().any(|m| matches!(m, Message::LoadWaves)),
            "Refresh should trigger LoadWaves"
        );
    }

    /// Test agent failure workflow
    #[test]
    fn test_agent_failure_workflow() {
        let mut app = test_app();

        // Load tasks and start agent
        let waves = vec![vec![TaskInfo {
            id: "1".into(),
            title: "Failing task".into(),
            status: "Pending".into(),
        }]];
        let _ = app.update(Message::WavesLoaded(Ok(waves)));
        let _ = app.update(Message::StartAgent("1".into()));

        // Agent encounters an error
        let _ = app.update(Message::AgentComplete(Err("Build failed with exit code 1".into())));

        assert_eq!(app.state.agent_status, AgentStatus::Idle);
        assert!(app.state.output_buffer.contains("Agent error"));
        assert!(app.state.output_buffer.contains("Build failed"));
    }

    // =============================================================
    // Snapshot Test (Visual Regression)
    // =============================================================

    /// Basic snapshot test - captures UI rendering
    #[test]
    fn test_snapshot_waves_view() {
        let mut app = test_app();

        // Setup known state
        let waves = vec![
            vec![TaskInfo {
                id: "1".into(),
                title: "Task A".into(),
                status: "Pending".into(),
            }],
            vec![TaskInfo {
                id: "2".into(),
                title: "Task B".into(),
                status: "Done".into(),
            }],
        ];
        let _ = app.update(Message::WavesLoaded(Ok(waves)));

        let mut ui = simulator(app.view());
        let theme = Theme::Dark;

        // Take snapshot
        let snapshot_result = ui.snapshot(&theme);
        assert!(snapshot_result.is_ok(), "Should be able to take snapshot");

        // Note: For actual visual regression testing, use:
        // let snapshot = snapshot_result.unwrap();
        // assert!(snapshot.matches_hash("tests/snapshots/waves_view.sha256").unwrap());
    }

    /// Snapshot of error state
    #[test]
    fn test_snapshot_error_state() {
        let mut app = test_app();
        app.error = Some("Connection refused".into());

        let mut ui = simulator(app.view());
        let theme = Theme::Dark;

        let snapshot_result = ui.snapshot(&theme);
        assert!(snapshot_result.is_ok(), "Should snapshot error state");
    }
}
