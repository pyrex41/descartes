use iced::widget::{column, container, text, row, button, Space};
use iced::{Element, Length, Theme, Size, window, Event};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, Key};

mod time_travel;
use time_travel::{TimeTravelState, TimeTravelMessage};

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("descartes_gui=debug,info")
        .init();

    tracing::info!("Starting Descartes GUI");

    iced::application("Descartes", DescartesGui::update, DescartesGui::view)
        .subscription(DescartesGui::subscription)
        .window(window::Settings {
            size: Size::new(1200.0, 800.0),
            position: window::Position::Centered,
            min_size: Some(Size::new(800.0, 600.0)),
            ..Default::default()
        })
        .theme(|_| Theme::TokyoNight)
        .run_with(|| (DescartesGui::new(), iced::Task::none()))
}

/// Main application state
#[derive(Debug, Clone)]
struct DescartesGui {
    /// Current view/tab
    current_view: ViewMode,
    /// Connection status to daemon
    daemon_connected: bool,
    /// Time travel debugger state
    time_travel_state: TimeTravelState,
}

/// Different views/modes in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Dashboard,
    TaskBoard,
    SwarmMonitor,
    Debugger,
}

/// Messages that drive the application
#[derive(Debug, Clone)]
enum Message {
    /// Switch to a different view
    SwitchView(ViewMode),
    /// Connect to daemon
    ConnectDaemon,
    /// Disconnect from daemon
    DisconnectDaemon,
    /// Time travel debugger message
    TimeTravel(TimeTravelMessage),
    /// Load sample history data for demo
    LoadSampleHistory,
}

impl DescartesGui {
    /// Create a new application instance
    fn new() -> Self {
        Self {
            current_view: ViewMode::Dashboard,
            daemon_connected: false,
            time_travel_state: TimeTravelState::default(),
        }
    }
}

impl DescartesGui {
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SwitchView(view) => {
                tracing::debug!("Switching view to: {:?}", view);
                self.current_view = view;
            }
            Message::ConnectDaemon => {
                tracing::info!("Connecting to daemon");
                self.daemon_connected = true;
            }
            Message::DisconnectDaemon => {
                tracing::info!("Disconnecting from daemon");
                self.daemon_connected = false;
            }
            Message::TimeTravel(tt_msg) => {
                time_travel::update(&mut self.time_travel_state, tt_msg);
            }
            Message::LoadSampleHistory => {
                tracing::info!("Loading sample history data");
                self.load_sample_history();
            }
        }
        iced::Task::none()
    }

    /// Load sample history data for demonstration
    fn load_sample_history(&mut self) {
        use descartes_core::{AgentHistoryEvent, HistoryEventType, HistorySnapshot};
        use chrono::Utc;

        let base_time = Utc::now().timestamp();
        let agent_id = "demo-agent-123".to_string();

        // Create sample events
        let mut events = Vec::new();

        // Event 1: System startup
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::System,
                serde_json::json!({"event": "agent_started", "version": "1.0.0"}),
            )
            .with_tags(vec!["startup".to_string()])
            .with_git_commit("abc123def456".to_string()),
        );

        // Event 2: Initial thought
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Thought,
                serde_json::json!({"content": "Analyzing task requirements", "confidence": 0.85}),
            )
            .with_tags(vec!["planning".to_string()]),
        );

        // Event 3: Tool usage
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::ToolUse,
                serde_json::json!({"tool": "grep", "pattern": "TODO", "matches": 15}),
            )
            .with_tags(vec!["search".to_string()]),
        );

        // Event 4: State change
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::StateChange,
                serde_json::json!({"from": "idle", "to": "working"}),
            )
            .with_tags(vec!["state_machine".to_string()]),
        );

        // Event 5: Action
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Action,
                serde_json::json!({"action": "create_file", "path": "/tmp/output.txt"}),
            )
            .with_tags(vec!["file_operation".to_string()])
            .with_git_commit("def456ghi789".to_string()),
        );

        // Event 6: Communication
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Communication,
                serde_json::json!({"type": "user_message", "content": "How is progress?"}),
            )
            .with_tags(vec!["user_interaction".to_string()]),
        );

        // Event 7: Decision
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Decision,
                serde_json::json!({"choice": "use_parallel_execution", "reasoning": "Better performance"}),
            )
            .with_tags(vec!["optimization".to_string()]),
        );

        // Event 8: Error
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Error,
                serde_json::json!({"error": "FileNotFound", "path": "/missing/file.txt"}),
            )
            .with_tags(vec!["error".to_string()]),
        );

        // Event 9: Another thought
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Thought,
                serde_json::json!({"content": "Need to handle edge case", "confidence": 0.92}),
            )
            .with_tags(vec!["problem_solving".to_string()]),
        );

        // Event 10: Final action
        events.push(
            AgentHistoryEvent::new(
                agent_id.clone(),
                HistoryEventType::Action,
                serde_json::json!({"action": "complete_task", "status": "success"}),
            )
            .with_tags(vec!["completion".to_string()])
            .with_git_commit("ghi789jkl012".to_string()),
        );

        // Adjust timestamps to be sequential
        for (i, event) in events.iter_mut().enumerate() {
            event.timestamp = base_time + (i as i64 * 60); // Events 1 minute apart
        }

        // Create a couple of snapshots
        let snapshots = vec![
            HistorySnapshot::new(
                agent_id.clone(),
                events[0..3].to_vec(),
                Some("abc123def456".to_string()),
            )
            .with_description("Initial planning phase".to_string()),
            HistorySnapshot::new(
                agent_id.clone(),
                events[0..7].to_vec(),
                Some("def456ghi789".to_string()),
            )
            .with_description("Main execution checkpoint".to_string()),
        ];

        // Update the time travel state
        time_travel::update(
            &mut self.time_travel_state,
            TimeTravelMessage::HistoryLoaded(events, snapshots),
        );
    }

    fn view(&self) -> Element<Message> {
        let header = self.view_header();
        let nav = self.view_navigation();
        let content = self.view_content();

        let main_layout = column![
            header,
            row![
                nav,
                content,
            ]
            .spacing(0)
        ]
        .spacing(0);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Handle subscriptions (keyboard events, timers, etc.)
    fn subscription(&self) -> iced::Subscription<Message> {
        // Listen for keyboard events
        iced::event::listen_with(|event, _status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) = event
            {
                // Only handle keyboard shortcuts in Debugger view
                match key {
                    // Arrow keys for navigation
                    Key::Named(keyboard::key::Named::ArrowLeft) => {
                        return Some(Message::TimeTravel(TimeTravelMessage::PrevEvent));
                    }
                    Key::Named(keyboard::key::Named::ArrowRight) => {
                        return Some(Message::TimeTravel(TimeTravelMessage::NextEvent));
                    }
                    // Space bar for play/pause
                    Key::Character(ref c) if c == " " => {
                        return Some(Message::TimeTravel(TimeTravelMessage::TogglePlayback));
                    }
                    // +/- for zoom
                    Key::Character(ref c) if c == "+" || c == "=" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::ZoomIn));
                    }
                    Key::Character(ref c) if c == "-" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::ZoomOut));
                    }
                    // Number keys for speed control
                    Key::Character(ref c) if c == "1" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(0.5)));
                    }
                    Key::Character(ref c) if c == "2" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(1.0)));
                    }
                    Key::Character(ref c) if c == "3" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(2.0)));
                    }
                    Key::Character(ref c) if c == "4" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(5.0)));
                    }
                    // L for loop toggle
                    Key::Character(ref c) if c == "l" && !modifiers.shift() => {
                        return Some(Message::TimeTravel(TimeTravelMessage::ToggleLoop));
                    }
                    _ => {}
                }
            }
            None
        })
    }

    /// Render the header bar
    fn view_header(&self) -> Element<Message> {
        let title = text("Descartes")
            .size(24)
            .width(Length::Shrink);

        let status_text = if self.daemon_connected {
            "Connected"
        } else {
            "Disconnected"
        };

        let status_indicator = text(format!("Daemon: {}", status_text))
            .size(14)
            .width(Length::Shrink);

        let connect_button = if self.daemon_connected {
            button(text("Disconnect"))
                .on_press(Message::DisconnectDaemon)
        } else {
            button(text("Connect"))
                .on_press(Message::ConnectDaemon)
        };

        let header_row = row![
            title,
            Space::with_width(Length::Fill),
            status_indicator,
            Space::with_width(20),
            connect_button,
        ]
        .spacing(10)
        .align_y(Vertical::Center);

        container(header_row)
            .width(Length::Fill)
            .padding(15)
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 0.0,
                        color: iced::Color::TRANSPARENT,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    /// Render the navigation sidebar
    fn view_navigation(&self) -> Element<Message> {
        let nav_items = vec![
            (ViewMode::Dashboard, "Dashboard"),
            (ViewMode::TaskBoard, "Task Board"),
            (ViewMode::SwarmMonitor, "Swarm Monitor"),
            (ViewMode::Debugger, "Debugger"),
        ];

        let buttons: Vec<Element<Message>> = nav_items
            .into_iter()
            .map(|(view, label)| {
                let is_active = self.current_view == view;
                let btn = button(
                    text(label)
                        .size(16)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                )
                .width(Length::Fill)
                .padding(15)
                .on_press(Message::SwitchView(view));

                if is_active {
                    container(btn)
                        .style(|theme: &Theme| {
                            container::Style {
                                background: Some(theme.palette().primary.into()),
                                ..Default::default()
                            }
                        })
                        .into()
                } else {
                    btn.into()
                }
            })
            .collect();

        let nav_column = column(buttons)
            .spacing(5)
            .padding(10);

        container(nav_column)
            .width(200)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 1.0,
                        color: theme.palette().text.scale_alpha(0.2),
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    /// Render the main content area
    fn view_content(&self) -> Element<Message> {
        let content = match self.current_view {
            ViewMode::Dashboard => self.view_dashboard(),
            ViewMode::TaskBoard => self.view_task_board(),
            ViewMode::SwarmMonitor => self.view_swarm_monitor(),
            ViewMode::Debugger => self.view_debugger(),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    /// Dashboard view
    fn view_dashboard(&self) -> Element<Message> {
        let title = text("Dashboard")
            .size(32)
            .width(Length::Fill);

        let welcome = text("Welcome to Descartes!")
            .size(18)
            .width(Length::Fill);

        let description = text(
            "This is the Descartes GUI - a native interface for managing your AI agent workflows.\n\n\
             Phase 3.1: Initial Application Setup Complete\n\n\
             Features coming soon:\n\
             - Real-time task monitoring\n\
             - Agent swarm visualization\n\
             - Interactive debugger with time-travel\n\
             - Visual DAG editor"
        )
        .size(14)
        .width(Length::Fill);

        column![
            title,
            Space::with_height(20),
            welcome,
            Space::with_height(20),
            description,
        ]
        .spacing(10)
        .into()
    }

    /// Task Board view (placeholder)
    fn view_task_board(&self) -> Element<Message> {
        let title = text("Task Board")
            .size(32)
            .width(Length::Fill);

        let placeholder = text("Task Board view will display active tasks in a Kanban layout.")
            .size(16)
            .width(Length::Fill);

        column![
            title,
            Space::with_height(20),
            placeholder,
        ]
        .spacing(10)
        .into()
    }

    /// Swarm Monitor view (placeholder)
    fn view_swarm_monitor(&self) -> Element<Message> {
        let title = text("Swarm Monitor")
            .size(32)
            .width(Length::Fill);

        let placeholder = text("Swarm Monitor will visualize active agents and their status.")
            .size(16)
            .width(Length::Fill);

        column![
            title,
            Space::with_height(20),
            placeholder,
        ]
        .spacing(10)
        .into()
    }

    /// Debugger view with time travel UI
    fn view_debugger(&self) -> Element<Message> {
        let title = text("Time Travel Debugger")
            .size(32)
            .width(Length::Fill);

        // Add a button to load sample history if no events are loaded
        let load_sample_btn = if self.time_travel_state.events.is_empty() {
            column![
                Space::with_height(10),
                button(text("Load Sample History"))
                    .on_press(Message::LoadSampleHistory)
                    .padding(10),
                Space::with_height(10),
            ]
        } else {
            column![]
        };

        column![
            title,
            load_sample_btn,
            // Map time travel messages to main messages
            time_travel::view(&self.time_travel_state).map(Message::TimeTravel),
        ]
        .spacing(10)
        .into()
    }
}
