#![allow(mismatched_lifetime_syntaxes)]
#![allow(dead_code)]

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, Key};
use iced::widget::{button, column, container, row, text, Space};
use iced::{window, Element, Event, Font, Length, Size};

/// JetBrains Mono font - Regular weight (embedded at compile time)
const JETBRAINS_MONO_REGULAR: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
/// JetBrains Mono font - Medium weight
const JETBRAINS_MONO_MEDIUM: &[u8] = include_bytes!("../fonts/JetBrainsMono-Medium.ttf");
/// JetBrains Mono font - Bold weight
const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");
use std::sync::Arc;

mod chat_graph_layout;
mod chat_graph_state;
mod chat_graph_view;
mod chat_state;
mod chat_view;
mod event_handler;
mod lisp_debugger;
mod rpc_client;
mod session_selector;
mod session_state;
mod theme;
mod time_travel;
mod zmq_subscriber;
mod history_graph_state;
mod history_graph_layout;
mod history_graph_view;

use theme::{colors, container_styles, button_styles, humanlayer_theme};
use history_graph_state::{HistoryGraphMessage, HistoryGraphState};

use chrono::Utc;
use descartes_daemon::DescartesEvent;
use event_handler::EventHandler;
use lisp_debugger::{LispDebuggerMessage, LispDebuggerState};
use rpc_client::GuiRpcClient;
use session_state::{SessionMessage, SessionState};
use time_travel::{TimeTravelMessage, TimeTravelState};
use uuid::Uuid;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("descartes_gui=debug,info")
        .init();

    tracing::info!("Starting Descartes GUI");

    iced::application("Descartes", DescartesGui::update, DescartesGui::view)
        .subscription(DescartesGui::subscription)
        .window(window::Settings {
            size: Size::new(1400.0, 900.0),
            position: window::Position::Centered,
            min_size: Some(Size::new(900.0, 600.0)),
            ..Default::default()
        })
        .theme(|_| humanlayer_theme())
        // Load JetBrains Mono font family
        .font(JETBRAINS_MONO_REGULAR)
        .font(JETBRAINS_MONO_MEDIUM)
        .font(JETBRAINS_MONO_BOLD)
        // Set JetBrains Mono as the default font
        .default_font(Font::with_name("JetBrains Mono"))
        .run_with(DescartesGui::new)
}

/// Main application state
struct DescartesGui {
    /// Current view/tab
    current_view: ViewMode,
    /// Connection status to daemon
    daemon_connected: bool,
    /// Connection error message
    connection_error: Option<String>,
    /// Session/workspace state
    session_state: SessionState,
    /// Time travel debugger state
    time_travel_state: TimeTravelState,
    /// History graph debugger state
    history_graph_state: HistoryGraphState,
    /// Chat interface state
    chat_state: chat_state::ChatState,
    /// Chat graph view state
    chat_graph_state: chat_graph_state::ChatGraphState,
    /// Lisp debugger state
    lisp_debugger_state: LispDebuggerState,
    /// RPC client (wrapped in Arc for cloning)
    rpc_client: Option<Arc<GuiRpcClient>>,
    /// Event handler
    event_handler: Option<Arc<tokio::sync::RwLock<EventHandler>>>,
    /// Recent events received
    recent_events: Vec<DescartesEvent>,
    /// Status message
    status_message: Option<String>,
}

/// Different views/modes in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Sessions,
    Dashboard,
    Chat,
    SwarmMonitor,
    Debugger,
}

/// Messages that drive the application
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum Message {
    /// Switch to a different view
    SwitchView(ViewMode),
    /// Connect to daemon
    ConnectDaemon,
    /// Connection result
    ConnectionResult(Result<(), String>),
    /// Disconnect from daemon
    DisconnectDaemon,
    /// Daemon event received
    DaemonEvent(DescartesEvent),
    /// Session management message
    Session(SessionMessage),
    /// Time travel debugger message
    TimeTravel(TimeTravelMessage),
    /// History graph debugger message
    HistoryGraph(HistoryGraphMessage),
    /// Chat interface message
    Chat(chat_state::ChatMessage),
    /// Chat graph view message
    ChatGraph(chat_graph_state::ChatGraphMessage),
    /// Lisp debugger message
    LispDebugger(LispDebuggerMessage),
    /// Load sample history data for demo
    LoadSampleHistory,
    /// Clear status message
    ClearStatus,
    /// Show error message
    ShowError(String),
    /// Global keyboard event (routed based on current view)
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    /// Focus the chat input
    FocusChatInput,
}

impl DescartesGui {
    /// Create a new application instance with startup task
    fn new() -> (Self, iced::Task<Message>) {
        let app = Self {
            current_view: ViewMode::Dashboard,
            daemon_connected: false,
            connection_error: None,
            session_state: SessionState::default(),
            time_travel_state: TimeTravelState::default(),
            history_graph_state: HistoryGraphState::new(),
            chat_state: chat_state::ChatState::new(),
            chat_graph_state: chat_graph_state::ChatGraphState::new(),
            lisp_debugger_state: LispDebuggerState::new(),
            rpc_client: None,
            event_handler: None,
            recent_events: Vec::new(),
            status_message: Some(
                "Starting up... connecting to daemon.".to_string(),
            ),
        };

        // Auto-start daemon and connect on startup
        let startup_task = iced::Task::perform(
            async {
                // Ensure daemon is running (starts if needed)
                descartes_core::ensure_daemon_running().await
                    .map_err(|e| format!("Failed to start daemon: {}", e))?;
                Ok::<(), String>(())
            },
            |result| match result {
                Ok(()) => Message::ConnectDaemon,
                Err(e) => Message::ShowError(e),
            },
        );

        (app, startup_task)
    }
}

impl DescartesGui {
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::SwitchView(view) => {
                tracing::debug!("Switching view to: {:?}", view);
                self.current_view = view;
                iced::Task::none()
            }
            Message::ConnectDaemon => {
                tracing::info!("Connecting to daemon");
                self.status_message = Some("Connecting to daemon...".to_string());
                self.connection_error = None;

                // Use the global daemon endpoint
                let endpoint = descartes_core::daemon_http_endpoint();
                tracing::info!("Using daemon endpoint: {}", endpoint);

                // Create RPC client
                match GuiRpcClient::new(&endpoint) {
                    Ok(client) => {
                        let client = Arc::new(client);
                        self.rpc_client = Some(Arc::clone(&client));

                        // Create event handler
                        let event_handler = EventHandler::with_defaults();
                        self.event_handler =
                            Some(Arc::new(tokio::sync::RwLock::new(event_handler)));

                        // Perform async connection
                        iced::Task::perform(
                            async move {
                                match client.connect().await {
                                    Ok(_) => Ok(()),
                                    Err(e) => Err(e.to_string()),
                                }
                            },
                            Message::ConnectionResult,
                        )
                    }
                    Err(e) => {
                        self.connection_error = Some(format!("Failed to create RPC client: {}", e));
                        self.status_message = None;
                        iced::Task::none()
                    }
                }
            }
            Message::ConnectionResult(result) => {
                match result {
                    Ok(_) => {
                        tracing::info!("Successfully connected to daemon");
                        self.daemon_connected = true;
                        self.connection_error = None;
                        self.status_message = Some("Connected to daemon successfully!".to_string());
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to daemon: {}", e);
                        self.daemon_connected = false;
                        self.connection_error = Some(e);
                        self.status_message = None;
                        self.rpc_client = None;
                        self.event_handler = None;
                    }
                }
                iced::Task::none()
            }
            Message::DisconnectDaemon => {
                tracing::info!("Disconnecting from daemon");
                self.daemon_connected = false;
                self.rpc_client = None;
                self.event_handler = None;
                self.status_message = Some("Disconnected from daemon".to_string());
                iced::Task::none()
            }
            Message::DaemonEvent(event) => {
                tracing::debug!("Received daemon event: {:?}", event);
                self.recent_events.push(event.clone());

                // Keep only the last 100 events
                if self.recent_events.len() > 100 {
                    self.recent_events.remove(0);
                }

                // Check for DebuggerPaused events (Lisp debugger)
                if let DescartesEvent::AgentEvent(ref agent_event) = event {
                    use descartes_daemon::events::AgentEventType;
                    if agent_event.event_type == AgentEventType::DebuggerPaused {
                        // Parse the debugger event and show the panel
                        if let Some(msg) = lisp_debugger::parse_debugger_event(
                            agent_event.agent_id.clone(),
                            &agent_event.data,
                        ) {
                            lisp_debugger::update(&mut self.lisp_debugger_state, msg);
                            self.status_message = Some("Lisp debugger activated".to_string());
                            return iced::Task::none();
                        }
                    }
                }

                // Update status
                self.status_message = Some(format!("Received event: {:?}", event));
                iced::Task::none()
            }
            Message::Session(msg) => {
                use session_state::SessionMessage;

                // Handle async operations before updating state
                let task = match &msg {
                    SessionMessage::SelectSession(_id) => {
                        // Session selection is now instant - daemon is global
                        // Just update state, no daemon spawning needed
                        iced::Task::none()
                    }
                    SessionMessage::CreateSession => {
                        // Get the session details from state
                        let name = self.session_state.new_session_name.clone();
                        let path = self.session_state.new_session_path.clone();

                        if name.is_empty() || path.is_empty() {
                            // Don't create if fields are empty
                            return iced::Task::none();
                        }

                        // Create session asynchronously
                        iced::Task::perform(
                            create_session(name, path),
                            |result| match result {
                                Ok(session) => Message::Session(SessionMessage::SessionCreated(session)),
                                Err(e) => Message::Session(SessionMessage::DaemonError(e)),
                            },
                        )
                    }
                    SessionMessage::FocusPathInput => {
                        // Focus the path input field
                        iced::widget::text_input::focus(iced::widget::text_input::Id::new("session-path"))
                    }
                    _ => iced::Task::none(),
                };

                session_state::update(&mut self.session_state, msg);
                task
            }
            Message::TimeTravel(tt_msg) => {
                time_travel::update(&mut self.time_travel_state, tt_msg);
                iced::Task::none()
            }
            Message::HistoryGraph(hg_msg) => {
                history_graph_state::update(&mut self.history_graph_state, hg_msg);
                iced::Task::none()
            }
            Message::Chat(msg) => {
                use chat_state::ChatMessage as ChatMsg;

                let task = match &msg {
                    ChatMsg::SubmitPrompt => {
                        // Get prompt before updating state
                        let prompt = self.chat_state.prompt_input.clone();
                        if prompt.trim().is_empty() {
                            return iced::Task::none();
                        }

                        // Get working directory from active session or use current dir
                        let working_dir = self
                            .session_state
                            .active_session()
                            .map(|s| s.path.display().to_string())
                            .or_else(|| self.chat_state.working_directory.clone())
                            .unwrap_or_else(|| ".".to_string());

                        // Update chat state with working directory if from session
                        if self.chat_state.working_directory.is_none() {
                            if let Some(session) = self.session_state.active_session() {
                                self.chat_state.working_directory = Some(session.path.display().to_string());
                            }
                        }

                        // If we have a daemon connection and no active session, create one via RPC
                        // Use the two-phase approach: create session, subscribe, then send prompt
                        if self.daemon_connected && self.chat_state.daemon_session_id.is_none() {
                            if let Some(ref client) = self.rpc_client {
                                let client = client.clone();
                                let wd = working_dir.clone();
                                let pending_prompt = prompt.clone();
                                return iced::Task::perform(
                                    async move {
                                        create_daemon_chat_session(client, wd).await
                                    },
                                    move |result| match result {
                                        Ok((session_id, pub_endpoint)) => {
                                            // Return SessionCreated with the pending prompt
                                            // This will trigger subscription, then SendPendingPrompt
                                            Message::Chat(ChatMsg::SessionCreated {
                                                session_id,
                                                pub_endpoint,
                                                pending_prompt: pending_prompt.clone(),
                                            })
                                        }
                                        Err(e) => Message::Chat(ChatMsg::Error(e)),
                                    },
                                );
                            }
                        }

                        // If we have an active daemon session, send prompt via RPC
                        if let Some(session_id) = self.chat_state.daemon_session_id {
                            if let Some(ref client) = self.rpc_client {
                                let client = client.clone();
                                return iced::Task::perform(
                                    async move {
                                        send_daemon_prompt(client, session_id, prompt).await
                                    },
                                    |result| match result {
                                        Ok(()) => Message::Chat(ChatMsg::PromptSent),
                                        Err(e) => Message::Chat(ChatMsg::Error(e)),
                                    },
                                );
                            }
                        }

                        // Fallback: Spawn async task to run Claude Code directly (legacy mode)
                        iced::Task::perform(
                            run_claude_code(prompt, working_dir),
                            |result| match result {
                                Ok(response) => Message::Chat(ChatMsg::ResponseComplete(response)),
                                Err(e) => Message::Chat(ChatMsg::Error(e)),
                            },
                        )
                    }
                    ChatMsg::UpgradeToAgent => {
                        // Upgrade to agent mode via RPC
                        if let Some(session_id) = self.chat_state.daemon_session_id {
                            if let Some(ref client) = self.rpc_client {
                                let client = client.clone();
                                return iced::Task::perform(
                                    async move {
                                        upgrade_to_agent(client, session_id).await
                                    },
                                    |result| match result {
                                        Ok(()) => Message::Chat(ChatMsg::UpgradedToAgent),
                                        Err(e) => Message::Chat(ChatMsg::Error(e)),
                                    },
                                );
                            }
                        }
                        iced::Task::none()
                    }
                    ChatMsg::SessionCreated { session_id, pub_endpoint, pending_prompt } => {
                        // Session created - store info (triggers subscription) and return delayed task
                        // to send the pending prompt after subscription has time to establish
                        tracing::info!(
                            "Session {} created, will send prompt after subscription ready",
                            session_id
                        );

                        // Update state first (this enables the ZMQ subscription)
                        chat_state::update(&mut self.chat_state, ChatMsg::SessionCreated {
                            session_id: *session_id,
                            pub_endpoint: pub_endpoint.clone(),
                            pending_prompt: pending_prompt.clone(),
                        });

                        // Return a delayed task that triggers SendPendingPrompt
                        // Small delay (100ms) to allow subscription to establish
                        return iced::Task::perform(
                            async {
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            },
                            |_| Message::Chat(ChatMsg::SendPendingPrompt),
                        );
                    }
                    ChatMsg::SendPendingPrompt => {
                        // Send the pending prompt to start the CLI
                        if let Some(pending) = self.chat_state.pending_prompt.take() {
                            if let Some(session_id) = self.chat_state.daemon_session_id {
                                if let Some(ref client) = self.rpc_client {
                                    let client = client.clone();
                                    tracing::info!(
                                        "Sending pending prompt to session {}: {}",
                                        session_id,
                                        pending.chars().take(50).collect::<String>()
                                    );
                                    return iced::Task::perform(
                                        async move {
                                            send_daemon_prompt(client, session_id, pending).await
                                        },
                                        |result| match result {
                                            Ok(()) => Message::Chat(ChatMsg::PromptSent),
                                            Err(e) => Message::Chat(ChatMsg::Error(e)),
                                        },
                                    );
                                }
                            }
                        }
                        iced::Task::none()
                    }
                    _ => iced::Task::none(),
                };

                chat_state::update(&mut self.chat_state, msg);

                // Rebuild graph if visible
                if self.chat_graph_state.show_graph_view {
                    self.rebuild_chat_graph();
                }

                task
            }
            Message::ChatGraph(msg) => {
                // Rebuild graph when toggling view on or explicitly requested
                let should_rebuild = match &msg {
                    chat_graph_state::ChatGraphMessage::ToggleView => {
                        !self.chat_graph_state.show_graph_view // Will be toggled to true
                    }
                    chat_graph_state::ChatGraphMessage::RebuildGraph => true,
                    _ => false,
                };

                chat_graph_state::update(&mut self.chat_graph_state, msg);

                if should_rebuild {
                    self.rebuild_chat_graph();
                }

                iced::Task::none()
            }
            Message::LispDebugger(msg) => {
                // Handle Lisp debugger messages
                if let Some((agent_id, restart_index)) = lisp_debugger::update(&mut self.lisp_debugger_state, msg) {
                    // Need to invoke the restart via RPC
                    tracing::info!("Invoking Lisp restart {} for agent {}", restart_index, agent_id);

                    if let Some(ref client) = self.rpc_client {
                        let client = Arc::clone(client);
                        return iced::Task::perform(
                            async move {
                                // Call the daemon to invoke the restart
                                // The daemon will forward this to the Swank client
                                match client.invoke_swank_restart(&agent_id, restart_index).await {
                                    Ok(_) => Ok(()),
                                    Err(e) => Err(e.to_string()),
                                }
                            },
                            |result| match result {
                                Ok(()) => Message::LispDebugger(LispDebuggerMessage::RestartComplete),
                                Err(e) => Message::LispDebugger(LispDebuggerMessage::RestartError(e)),
                            },
                        );
                    } else {
                        // No RPC client - just dismiss the debugger
                        self.lisp_debugger_state.set_error("No daemon connection".to_string());
                    }
                }
                iced::Task::none()
            }
            Message::LoadSampleHistory => {
                tracing::info!("Loading sample history data");
                self.load_sample_history();
                iced::Task::none()
            }
            Message::ClearStatus => {
                self.status_message = None;
                iced::Task::none()
            }
            Message::ShowError(error) => {
                self.status_message = Some(format!("Error: {}", error));
                iced::Task::none()
            }
            Message::KeyPressed(key, modifiers) => {
                self.handle_keyboard(key, modifiers)
            }
            Message::FocusChatInput => {
                iced::widget::text_input::focus(iced::widget::text_input::Id::new("chat-prompt"))
            }
        }
    }

    /// Centralized keyboard handler with view-specific routing
    fn handle_keyboard(&mut self, key: keyboard::Key, modifiers: keyboard::Modifiers) -> iced::Task<Message> {
        use keyboard::key::Named;

        // === GLOBAL SHORTCUTS (work in all views) ===
        //
        // Vim-like keybindings:
        //   Navigation: j/k (down/up), h/l (left/right in time travel)
        //   Jump: g (start), G (end)
        //   Views: 1-6 (switch views, no modifier needed)
        //   Actions: i (insert/focus input), o (open/new), r (refresh)
        //   Search: / (filter/search)
        //   Cancel: Escape, q (quit modal)

        // Plain number keys 1-6: View switching (vim-like, no modifier)
        if !modifiers.control() && !modifiers.shift() && !modifiers.alt() {
            match &key {
                Key::Character(c) if c == "1" => {
                    self.current_view = ViewMode::Sessions;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "2" => {
                    self.current_view = ViewMode::Dashboard;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "3" => {
                    self.current_view = ViewMode::Chat;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "4" => {
                    self.current_view = ViewMode::SwarmMonitor;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "5" => {
                    self.current_view = ViewMode::Debugger;
                    return iced::Task::none();
                }
                _ => {}
            }
        }

        // Ctrl+1-6: Also support Ctrl+number for view switching (fallback)
        if modifiers.control() && !modifiers.shift() && !modifiers.alt() {
            match &key {
                Key::Character(c) if c == "1" => {
                    self.current_view = ViewMode::Sessions;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "2" => {
                    self.current_view = ViewMode::Dashboard;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "3" => {
                    self.current_view = ViewMode::Chat;
                    return iced::widget::text_input::focus(
                        iced::widget::text_input::Id::new("chat-prompt")
                    );
                }
                Key::Character(c) if c == "4" => {
                    self.current_view = ViewMode::SwarmMonitor;
                    return iced::Task::none();
                }
                Key::Character(c) if c == "5" => {
                    self.current_view = ViewMode::Debugger;
                    return iced::Task::none();
                }
                _ => {}
            }
        }

        // Tab / Shift+Tab: Focus navigation between inputs
        if matches!(key, Key::Named(Named::Tab)) {
            if modifiers.shift() {
                return iced::widget::focus_previous();
            } else {
                return iced::widget::focus_next();
            }
        }

        // Escape or q: Close modals, cancel operations, or clear errors
        if matches!(key, Key::Named(Named::Escape)) || matches!(&key, Key::Character(c) if c == "q" && !modifiers.control()) {
            // First check if Lisp debugger is active
            if self.lisp_debugger_state.is_active() {
                lisp_debugger::update(&mut self.lisp_debugger_state, LispDebuggerMessage::Dismiss);
                return iced::Task::none();
            }

            // Check if session creation dialog is open
            if self.session_state.show_create_dialog {
                self.session_state.show_create_dialog = false;
                return iced::Task::none();
            }

            // Clear any error messages
            if self.connection_error.is_some() {
                self.connection_error = None;
                return iced::Task::none();
            }

            // View-specific escape handling
            match self.current_view {
                ViewMode::Chat => {
                    // Clear chat error if present
                    if self.chat_state.error.is_some() {
                        chat_state::update(&mut self.chat_state, chat_state::ChatMessage::ClearError);
                        return iced::Task::none();
                    }
                }
                _ => {}
            }
        }

        // r or F5: Refresh (view-specific) - vim-like 'r' for refresh
        if matches!(key, Key::Named(Named::F5)) || matches!(&key, Key::Character(c) if c == "r" && !modifiers.control() && !modifiers.shift() && !modifiers.alt()) {
            match self.current_view {
                ViewMode::Sessions => {
                    session_state::update(&mut self.session_state, SessionMessage::RefreshSessions);
                    return iced::Task::none();
                }
                ViewMode::Dashboard => {
                    // Reconnect to daemon
                    return iced::Task::done(Message::ConnectDaemon);
                }
                _ => {}
            }
        }

        // === VIEW-SPECIFIC SHORTCUTS ===
        match self.current_view {
            ViewMode::Debugger => {
                // History Graph shortcuts (only active in Debugger view)
                // Vim-like: h/l for step, g/G for start/end
                match &key {
                    // h or Left Arrow: Step backward in history
                    Key::Named(Named::ArrowLeft) => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::StepBackward);
                    }
                    Key::Character(c) if c == "h" && !modifiers.control() && !modifiers.alt() => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::StepBackward);
                    }
                    // l or Right Arrow: Step forward in history
                    Key::Named(Named::ArrowRight) => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::StepForward);
                    }
                    Key::Character(c) if c == "l" && !modifiers.shift() && !modifiers.control() && !modifiers.alt() => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::StepForward);
                    }
                    // g or Home: Jump to start
                    Key::Named(Named::Home) => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::JumpToStart);
                    }
                    Key::Character(c) if c == "g" && !modifiers.shift() && !modifiers.control() => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::JumpToStart);
                    }
                    // G or End: Jump to end
                    Key::Named(Named::End) => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::JumpToEnd);
                    }
                    Key::Character(c) if c == "G" && modifiers.shift() && !modifiers.control() => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::JumpToEnd);
                    }
                    // +/= for zoom in, - for zoom out
                    Key::Character(c) if c == "+" || c == "=" => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::Zoom(1.1));
                    }
                    Key::Character(c) if c == "-" => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::Zoom(0.9));
                    }
                    // r: Reset zoom and pan
                    Key::Character(c) if c == "r" && !modifiers.control() && !modifiers.shift() => {
                        history_graph_state::update(&mut self.history_graph_state, HistoryGraphMessage::ResetView);
                    }
                    _ => {}
                }
            }

            ViewMode::Chat => {
                // Chat shortcuts (vim-like)
                match &key {
                    // i: Enter insert mode (focus chat input) - vim-like
                    Key::Character(c) if c == "i" && !modifiers.control() && !modifiers.shift() && !modifiers.alt() => {
                        return iced::widget::text_input::focus(
                            iced::widget::text_input::Id::new("chat-prompt")
                        );
                    }
                    // a: Append mode (also focus input) - vim-like
                    Key::Character(c) if c == "a" && !modifiers.control() && !modifiers.shift() && !modifiers.alt() => {
                        return iced::widget::text_input::focus(
                            iced::widget::text_input::Id::new("chat-prompt")
                        );
                    }
                    // Ctrl+L: Clear conversation
                    Key::Character(c) if c == "l" && modifiers.control() => {
                        chat_state::update(&mut self.chat_state, chat_state::ChatMessage::ClearConversation);
                    }
                    // j/k: Scroll through messages (if implemented)
                    // TODO: Add message scrolling with j/k
                    _ => {}
                }
            }

            ViewMode::Sessions => {
                // Session list navigation (vim-like)
                match &key {
                    // k or Up Arrow: Previous session (vim: up motion)
                    Key::Named(Named::ArrowUp) => {
                        session_state::update(&mut self.session_state, SessionMessage::SelectPrevious);
                    }
                    Key::Character(c) if c == "k" && !modifiers.control() && !modifiers.alt() => {
                        session_state::update(&mut self.session_state, SessionMessage::SelectPrevious);
                    }
                    // j or Down Arrow: Next session (vim: down motion)
                    Key::Named(Named::ArrowDown) => {
                        session_state::update(&mut self.session_state, SessionMessage::SelectNext);
                    }
                    Key::Character(c) if c == "j" && !modifiers.control() && !modifiers.alt() => {
                        session_state::update(&mut self.session_state, SessionMessage::SelectNext);
                    }
                    // Enter: Activate selected session
                    Key::Named(Named::Enter) => {
                        if let Some(id) = self.session_state.selected_session_id {
                            session_state::update(&mut self.session_state, SessionMessage::SelectSession(id));
                        }
                    }
                    // o: Open/create new session (vim: open line below)
                    Key::Character(c) if c == "o" && !modifiers.control() && !modifiers.shift() && !modifiers.alt() => {
                        self.session_state.show_create_dialog = true;
                    }
                    // Ctrl+N: Also support Ctrl+N for new session (fallback)
                    Key::Character(c) if c == "n" && modifiers.control() => {
                        self.session_state.show_create_dialog = true;
                    }
                    // /: Filter/search sessions (vim: search)
                    Key::Character(c) if c == "/" && !modifiers.control() => {
                        // Focus the filter input if we have one
                        return iced::widget::text_input::focus(
                            iced::widget::text_input::Id::new("session-filter")
                        );
                    }
                    // g: Jump to first session (vim: gg)
                    Key::Character(c) if c == "g" && !modifiers.shift() && !modifiers.control() => {
                        let visible = self.session_state.visible_sessions();
                        if let Some(first) = visible.first() {
                            self.session_state.selected_session_id = Some(first.id);
                        }
                    }
                    // G: Jump to last session (vim: G)
                    Key::Character(c) if c == "G" && modifiers.shift() && !modifiers.control() => {
                        let visible = self.session_state.visible_sessions();
                        if let Some(last) = visible.last() {
                            self.session_state.selected_session_id = Some(last.id);
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        iced::Task::none()
    }

    /// Rebuild the chat graph from current chat state
    fn rebuild_chat_graph(&mut self) {
        use chat_graph_state::ChatGraphNode;

        self.chat_graph_state.clear();

        let mut last_node_id: Option<uuid::Uuid> = None;

        for msg in &self.chat_state.messages {
            let mut node = match msg.role {
                chat_state::ChatRole::User => ChatGraphNode::user(msg.content.clone()),
                chat_state::ChatRole::Assistant => ChatGraphNode::assistant(msg.content.clone()),
                chat_state::ChatRole::System => continue, // Skip system messages
            };

            // Link to previous node as parent
            if let Some(parent_id) = last_node_id {
                node.parent = Some(parent_id);
            }

            let node_id = node.id;
            self.chat_graph_state.add_node(node);
            last_node_id = Some(node_id);
        }

        // Compute layout
        chat_graph_layout::compute_layout(&mut self.chat_graph_state);
    }

    /// Load sample history data for demonstration
    fn load_sample_history(&mut self) {
        use descartes_core::{AgentHistoryEvent, HistoryEventType, HistorySnapshot};

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

        // Update the time travel state (kept for backwards compatibility)
        time_travel::update(
            &mut self.time_travel_state,
            TimeTravelMessage::HistoryLoaded(events.clone(), snapshots.clone()),
        );

        // Also update the history graph state
        history_graph_state::update(
            &mut self.history_graph_state,
            HistoryGraphMessage::LoadHistory(events, snapshots),
        );
    }

    fn view(&self) -> Element<Message> {
        let header = self.view_header();
        let nav = self.view_navigation();
        let content = self.view_content();

        let main_layout = column![header, row![nav, content,].spacing(0)].spacing(0);

        let main_container: Element<Message> = container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // If the Lisp debugger is visible, overlay it on top of the main content
        if self.lisp_debugger_state.visible {
            let debugger_overlay = lisp_debugger::view(&self.lisp_debugger_state)
                .map(Message::LispDebugger);

            // Use a stack to overlay the debugger on top of the main content
            iced::widget::stack![main_container, debugger_overlay].into()
        } else {
            main_container
        }
    }

    /// Handle subscriptions (keyboard events, timers, etc.)
    fn subscription(&self) -> iced::Subscription<Message> {
        // Keyboard event subscription - route all key presses to centralized handler
        let keyboard_sub = iced::event::listen_with(|event, status, _window| {
            // Only handle if the event wasn't captured by a widget (e.g., text input)
            if status == iced::event::Status::Captured {
                return None;
            }

            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                // Route all keyboard events to centralized handler
                return Some(Message::KeyPressed(key, modifiers));
            }
            None
        });

        // Event stream subscription (when connected)
        let event_sub = if self.daemon_connected && self.event_handler.is_some() {
            // Create event subscription using the event handler
            let event_handler_arc = self.event_handler.as_ref().unwrap().clone();

            iced::Subscription::run_with_id(
                "daemon_events",
                iced::stream::channel(100, move |_output| {
                    let _event_handler_arc = event_handler_arc.clone();
                    async move {
                        // This is a simplified subscription - in a real implementation,
                        // we would properly integrate with the EventHandler's subscription system
                        tracing::info!("Event subscription active");

                        // Keep the subscription alive
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                }),
            )
        } else {
            iced::Subscription::none()
        };

        // ZMQ subscription for chat streaming
        let zmq_sub = if let (Some(endpoint), Some(session_id)) = (
            &self.chat_state.pub_endpoint,
            self.chat_state.daemon_session_id,
        ) {
            zmq_subscriber::chat_subscription(
                endpoint.clone(),
                session_id,
                |chunk| Message::Chat(chat_state::ChatMessage::StreamChunk(chunk)),
                |e| Message::Chat(chat_state::ChatMessage::Error(e)),
            )
        } else {
            iced::Subscription::none()
        };

        iced::Subscription::batch(vec![keyboard_sub, event_sub, zmq_sub])
    }

    /// Render the header bar
    fn view_header(&self) -> Element<Message> {
        // Logo/brand with subtle styling
        let logo = text("◆").size(20).color(colors::PRIMARY);
        let title = text("DESCARTES").size(18).color(colors::TEXT_PRIMARY);
        let subtitle = text("Agent Orchestration").size(12).color(colors::TEXT_MUTED);

        let brand = row![
            logo,
            Space::with_width(8),
            column![title, subtitle].spacing(0),
        ]
        .align_y(Vertical::Center);

        // Status indicator with modern pill design
        let (status_color, _status_bg, status_text) = if self.daemon_connected {
            (colors::SUCCESS, colors::SUCCESS_DIM, "Connected")
        } else {
            (colors::ERROR, colors::ERROR_DIM, "Disconnected")
        };

        let status_indicator = container(
            row![
                text("●").size(10).color(status_color),
                Space::with_width(6),
                text(status_text).size(12).color(colors::TEXT_PRIMARY),
            ]
            .align_y(Vertical::Center)
        )
        .padding([4, 12])
        .style(container_styles::card);

        // Connect/Disconnect button with modern styling
        let connect_button = if self.daemon_connected {
            button(text("Disconnect").size(13))
                .on_press(Message::DisconnectDaemon)
                .padding([8, 16])
                .style(button_styles::secondary)
        } else {
            button(text("Connect").size(13))
                .on_press(Message::ConnectDaemon)
                .padding([8, 16])
                .style(button_styles::primary)
        };

        // Status message area
        let message_display = if let Some(ref error) = self.connection_error {
            text(format!("⚠ {}", error))
                .size(12)
                .color(colors::ERROR)
        } else if let Some(ref msg) = self.status_message {
            text(msg)
                .size(12)
                .color(colors::TEXT_SECONDARY)
        } else {
            text("")
        };

        let header_content = column![
            row![
                brand,
                Space::with_width(Length::Fill),
                status_indicator,
                Space::with_width(12),
                connect_button,
            ]
            .spacing(10)
            .align_y(Vertical::Center),
            if self.connection_error.is_some() || self.status_message.is_some() {
                container(message_display).padding(8)
            } else {
                container(text(""))
            },
        ]
        .spacing(0);

        container(header_content)
            .width(Length::Fill)
            .padding([16, 20])
            .style(container_styles::header)
            .into()
    }

    /// Render the navigation sidebar
    fn view_navigation(&self) -> Element<Message> {
        // ============================================================
        // ACTIVE VIEWS:
        // - Sessions: Session management (create, select, delete)
        // - Dashboard: Overview with stats and recent events
        // - Chat: Agent conversation interface
        // - Agents (SwarmMonitor): Live agent monitoring
        // - Debugger: History Graph for visualizing agent execution
        //
        // REMOVED (2026-01 Simplification):
        // - TaskBoard: Kanban-style task view (code deleted)
        // - DagEditor: Visual workflow design (code deleted)
        // ============================================================

        // Navigation items with icons
        let nav_items = vec![
            (ViewMode::Sessions, "\u{25C6}", "Sessions"),    // ◆
            (ViewMode::Dashboard, "\u{2302}", "Dashboard"),  // ⌂
            (ViewMode::Chat, "\u{2709}", "Chat"),            // ✉
            (ViewMode::SwarmMonitor, "\u{25CE}", "Agents"),  // ◎
            (ViewMode::Debugger, "\u{23F1}", "Debugger"),    // ⏱
        ];

        let buttons: Vec<Element<Message>> = nav_items
            .into_iter()
            .map(|(view, icon, label)| {
                let is_active = self.current_view == view;

                let text_color = if is_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                };

                let icon_color = if is_active {
                    colors::PRIMARY
                } else {
                    colors::TEXT_MUTED
                };

                let content = row![
                    text(icon).size(16).color(icon_color),
                    Space::with_width(10),
                    text(label).size(14).color(text_color),
                ]
                .align_y(Vertical::Center);

                let btn = button(content)
                    .width(Length::Fill)
                    .padding([12, 16])
                    .on_press(Message::SwitchView(view))
                    .style(if is_active {
                        button_styles::nav_active
                    } else {
                        button_styles::nav
                    });

                container(btn)
                    .width(Length::Fill)
                    .into()
            })
            .collect();

        // Section header
        let section_header = text("NAVIGATION")
            .size(10)
            .color(colors::TEXT_MUTED);

        let nav_column = column![
            container(section_header).padding([8, 16]),
            column(buttons).spacing(2),
        ]
        .spacing(0)
        .padding([16, 8]);

        container(nav_column)
            .width(180)
            .height(Length::Fill)
            .style(container_styles::sidebar)
            .into()
    }

    /// Render the main content area
    fn view_content(&self) -> Element<Message> {
        let content = match self.current_view {
            ViewMode::Sessions => self.view_sessions(),
            ViewMode::Dashboard => self.view_dashboard(),
            ViewMode::Chat => self.view_chat(),
            ViewMode::SwarmMonitor => self.view_swarm_monitor(),
            ViewMode::Debugger => self.view_debugger(),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }

    /// Sessions view
    fn view_sessions(&self) -> Element<Message> {
        session_selector::view_sessions_panel(&self.session_state)
            .map(Message::Session)
    }

    /// Chat view - full session Claude Code integration
    fn view_chat(&self) -> Element<Message> {
        if self.chat_graph_state.show_graph_view {
            // Show graph view
            chat_graph_view::view(&self.chat_graph_state).map(Message::ChatGraph)
        } else {
            // Show linear view with toggle button in header
            let toggle_btn = chat_graph_view::view_toggle_button(self.chat_graph_state.show_graph_view)
                .map(Message::ChatGraph);

            let chat_content = chat_view::view(&self.chat_state).map(Message::Chat);

            column![
                row![
                    Space::with_width(Length::Fill),
                    toggle_btn,
                ]
                .padding([8, 16]),
                chat_content,
            ]
            .into()
        }
    }

    /// Dashboard view
    fn view_dashboard(&self) -> Element<Message> {
        let title = text("Dashboard")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Welcome to Descartes Agent Orchestration")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Stats cards row
        let stats_cards = row![
            self.view_stat_card("Agents".to_string(), "0".to_string(), "◎".to_string(), colors::PRIMARY),
            Space::with_width(12),
            self.view_stat_card("Sessions".to_string(), format!("{}", self.session_state.sessions.len()), "☰".to_string(), colors::INFO),
            Space::with_width(12),
            self.view_stat_card("Events".to_string(), format!("{}", self.recent_events.len()), "◈".to_string(), colors::SUCCESS),
        ];

        // Connection status card
        let (status_color, status_text, status_desc) = if self.daemon_connected {
            (colors::SUCCESS, "Connected", "Daemon is running and responsive")
        } else {
            (colors::WARNING, "Disconnected", "Click Connect to establish connection")
        };

        let status_card = container(
            column![
                row![
                    text("●").size(12).color(status_color),
                    Space::with_width(8),
                    text("Connection Status").size(14).color(colors::TEXT_PRIMARY),
                ]
                .align_y(Vertical::Center),
                Space::with_height(8),
                text(status_text).size(20).color(status_color),
                Space::with_height(4),
                text(status_desc).size(12).color(colors::TEXT_MUTED),
            ]
            .spacing(0)
        )
        .padding(16)
        .width(Length::Fill)
        .style(container_styles::panel);

        // Recent events section
        let recent_events_section = if !self.recent_events.is_empty() {
            let event_list: Vec<Element<Message>> = self
                .recent_events
                .iter()
                .rev()
                .take(5)
                .map(|event| {
                    let (icon, event_str, color) = match event {
                        DescartesEvent::AgentEvent(e) => {
                            ("◎", format!("Agent {:?}: {}", e.event_type, e.agent_id), colors::PRIMARY)
                        }
                        DescartesEvent::TaskEvent(e) => {
                            ("☰", format!("Task: {}", e.task_id), colors::INFO)
                        }
                        DescartesEvent::WorkflowEvent(e) => {
                            ("◇", format!("Workflow: {}", e.workflow_id), colors::SUCCESS)
                        }
                        DescartesEvent::SystemEvent(e) => {
                            ("⚙", format!("System: {:?}", e.event_type), colors::TEXT_MUTED)
                        }
                        DescartesEvent::StateEvent(e) => {
                            ("◈", format!("State {:?}: {}", e.event_type, e.key), colors::WARNING)
                        }
                    };
                    container(
                        row![
                            text(icon).size(12).color(color),
                            Space::with_width(8),
                            text(event_str).size(12).color(colors::TEXT_SECONDARY),
                        ]
                        .align_y(Vertical::Center)
                    )
                    .padding([6, 0])
                    .into()
                })
                .collect();

            container(
                column![
                    text("Recent Events").size(14).color(colors::TEXT_PRIMARY),
                    Space::with_height(12),
                    column(event_list).spacing(2),
                ]
            )
            .padding(16)
            .width(Length::Fill)
            .style(container_styles::panel)
        } else {
            container(
                column![
                    text("Recent Events").size(14).color(colors::TEXT_PRIMARY),
                    Space::with_height(12),
                    text("No events yet").size(12).color(colors::TEXT_MUTED),
                ]
            )
            .padding(16)
            .width(Length::Fill)
            .style(container_styles::panel)
        };

        // Quick actions
        let quick_actions = container(
            column![
                text("Quick Actions").size(14).color(colors::TEXT_PRIMARY),
                Space::with_height(12),
                row![
                    button(text("Load Sample History").size(12))
                        .on_press(Message::LoadSampleHistory)
                        .padding([8, 12])
                        .style(button_styles::secondary),
                ]
            ]
        )
        .padding(16)
        .width(Length::Fill)
        .style(container_styles::panel);

        column![
            title,
            Space::with_height(4),
            subtitle,
            Space::with_height(24),
            stats_cards,
            Space::with_height(16),
            row![
                container(status_card).width(Length::FillPortion(1)),
                Space::with_width(12),
                container(recent_events_section).width(Length::FillPortion(2)),
            ],
            Space::with_height(16),
            quick_actions,
        ]
        .spacing(0)
        .into()
    }

    /// Helper to create a stat card
    fn view_stat_card(&self, label: String, value: String, icon: String, color: iced::Color) -> Element<Message> {
        container(
            column![
                row![
                    text(icon).size(14).color(color),
                    Space::with_width(8),
                    text(label).size(12).color(colors::TEXT_MUTED),
                ]
                .align_y(Vertical::Center),
                Space::with_height(8),
                text(value).size(28).color(colors::TEXT_PRIMARY),
            ]
        )
        .padding(16)
        .width(Length::Fill)
        .style(container_styles::panel)
        .into()
    }

    /// Swarm Monitor view - shows active session and daemon status
    fn view_swarm_monitor(&self) -> Element<Message> {
        let title = text("Agent Monitor")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Active sessions and daemon status")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Session status card
        let session_card = if let Some(session) = self.session_state.active_session() {
            let (status_icon, status_color, status_text) = match session.status {
                descartes_core::SessionStatus::Active => ("●", colors::SUCCESS, "Active"),
                descartes_core::SessionStatus::Starting => ("◐", colors::WARNING, "Starting"),
                descartes_core::SessionStatus::Stopping => ("◐", colors::WARNING, "Stopping"),
                descartes_core::SessionStatus::Inactive => ("○", colors::TEXT_MUTED, "Inactive"),
                descartes_core::SessionStatus::Error => ("●", colors::ERROR, "Error"),
                descartes_core::SessionStatus::Archived => ("○", colors::TEXT_MUTED, "Archived"),
            };

            // Global daemon info (daemon is now shared across all sessions)
            let daemon_info = if self.daemon_connected {
                column![
                    row![
                        text("Endpoint:").size(12).color(colors::TEXT_MUTED),
                        Space::with_width(8),
                        text(descartes_core::daemon_http_endpoint()).size(12).color(colors::PRIMARY),
                    ],
                    row![
                        text("WebSocket:").size(12).color(colors::TEXT_MUTED),
                        Space::with_width(8),
                        text(descartes_core::daemon_ws_endpoint())
                            .size(12)
                            .color(colors::TEXT_SECONDARY),
                    ],
                    row![
                        text("Status:").size(12).color(colors::TEXT_MUTED),
                        Space::with_width(8),
                        text("Running (global)")
                            .size(12)
                            .color(colors::SUCCESS),
                    ],
                ]
                .spacing(4)
            } else {
                column![
                    text("Daemon not connected").size(12).color(colors::TEXT_MUTED),
                    Space::with_height(4),
                    text("Click 'Connect' in the header to connect").size(11).color(colors::TEXT_MUTED),
                ]
            };

            container(
                column![
                    // Session header
                    row![
                        text("◆").size(20).color(colors::PRIMARY),
                        Space::with_width(12),
                        column![
                            text(&session.name).size(18).color(colors::TEXT_PRIMARY),
                            text(session.path.display().to_string())
                                .size(11)
                                .color(colors::TEXT_MUTED),
                        ],
                        Space::with_width(Length::Fill),
                        row![
                            text(status_icon).size(12).color(status_color),
                            Space::with_width(6),
                            text(status_text).size(12).color(status_color),
                        ],
                    ]
                    .align_y(Vertical::Center),
                    Space::with_height(16),
                    // Daemon info
                    container(daemon_info)
                        .padding(12)
                        .width(Length::Fill)
                        .style(container_styles::card),
                    Space::with_height(16),
                    // Connection status
                    row![
                        text("GUI Connection:").size(12).color(colors::TEXT_MUTED),
                        Space::with_width(8),
                        text(if self.daemon_connected { "●" } else { "○" })
                            .size(12)
                            .color(if self.daemon_connected { colors::SUCCESS } else { colors::TEXT_MUTED }),
                        Space::with_width(4),
                        text(if self.daemon_connected { "Connected" } else { "Disconnected" })
                            .size(12)
                            .color(if self.daemon_connected { colors::SUCCESS } else { colors::TEXT_MUTED }),
                    ],
                ]
            )
            .padding(20)
            .width(Length::Fill)
            .style(container_styles::panel)
        } else {
            // No active session
            container(
                column![
                    text("◎").size(32).color(colors::TEXT_MUTED),
                    Space::with_height(12),
                    text("No active session").size(16).color(colors::TEXT_PRIMARY),
                    Space::with_height(4),
                    text("Select a session from the Sessions view").size(12).color(colors::TEXT_MUTED),
                ]
                .align_x(Horizontal::Center)
            )
            .padding(40)
            .width(Length::Fill)
            .style(container_styles::panel)
            .align_x(Horizontal::Center)
        };

        // Stats row
        let (active, inactive, _archived) = self.session_state.status_counts();
        let stats_row = row![
            self.view_stat_card("Active".to_string(), active.to_string(), "●".to_string(), colors::SUCCESS),
            Space::with_width(12),
            self.view_stat_card("Inactive".to_string(), inactive.to_string(), "○".to_string(), colors::TEXT_MUTED),
            Space::with_width(12),
            self.view_stat_card("Events".to_string(), self.recent_events.len().to_string(), "◈".to_string(), colors::INFO),
        ];

        column![
            title,
            Space::with_height(4),
            subtitle,
            Space::with_height(24),
            stats_row,
            Space::with_height(16),
            session_card,
        ]
        .spacing(0)
        .into()
    }

    /// Debugger view with time travel UI
    fn view_debugger(&self) -> Element<Message> {
        let title = text("Agent History Graph")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Visualize agent execution, causality, and time-travel through history")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Show graph or empty state based on whether we have nodes
        let content: Element<Message> = if self.history_graph_state.nodes.is_empty() {
            // Empty state with load sample button
            column![
                Space::with_height(32),
                history_graph_view::view_empty_state().map(Message::HistoryGraph),
                Space::with_height(16),
                button(text("Load Sample History").size(13))
                    .on_press(Message::LoadSampleHistory)
                    .padding([10, 16])
                    .style(button_styles::primary),
            ]
            .align_x(iced::alignment::Horizontal::Center)
            .into()
        } else {
            // Show the history graph (need mutable access for layout computation)
            // Clone the state for view since view() needs mutable access for layout
            let mut graph_state = self.history_graph_state.clone();
            history_graph_view::view(&mut graph_state).map(Message::HistoryGraph)
        };

        column![
            title,
            Space::with_height(4),
            subtitle,
            Space::with_height(8),
            content,
        ]
        .spacing(0)
        .into()
    }
}

/// Create a new session by initializing the workspace directory
async fn create_session(name: String, path: String) -> Result<descartes_core::Session, String> {
    use std::path::PathBuf;
    use tokio::fs;

    let workspace_path = PathBuf::from(&path);

    // Ensure the workspace directory exists
    if !workspace_path.exists() {
        fs::create_dir_all(&workspace_path)
            .await
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
    }

    // Create .scud directory for session data
    let scud_path = workspace_path.join(".scud");
    if !scud_path.exists() {
        fs::create_dir_all(&scud_path)
            .await
            .map_err(|e| format!("Failed to create .scud directory: {}", e))?;
    }

    // Create sessions subdirectory for transcripts
    let sessions_path = scud_path.join("sessions");
    if !sessions_path.exists() {
        fs::create_dir_all(&sessions_path)
            .await
            .map_err(|e| format!("Failed to create sessions directory: {}", e))?;
    }

    // Create the session object
    let session = descartes_core::Session::new(name, workspace_path);

    // Save session metadata
    let metadata_path = session.metadata_path();
    let metadata_json = serde_json::to_string_pretty(&session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;
    fs::write(&metadata_path, metadata_json)
        .await
        .map_err(|e| format!("Failed to write session metadata: {}", e))?;

    tracing::info!("Created new session '{}' at {}", session.name, session.path.display());

    Ok(session)
}

/// Run Claude Code CLI with a prompt and return the response
///
/// This function wraps the Claude Code CLI to provide full session capabilities,
/// including file editing, bash execution, web search, and multi-turn context.
async fn run_claude_code(prompt: String, working_dir: String) -> Result<String, String> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    tracing::info!(
        "Running Claude Code in directory: {} with prompt: {}",
        working_dir,
        prompt.chars().take(50).collect::<String>()
    );

    // Check if claude is available
    let claude_path = which::which("claude").map_err(|_| {
        "Claude Code CLI not found. Please install it with: npm install -g @anthropic/claude-code".to_string()
    })?;

    tracing::debug!("Found claude at: {:?}", claude_path);

    // Run claude with the prompt in non-interactive mode
    // Using --print flag for single-shot output mode
    let mut cmd = Command::new(claude_path);
    cmd.arg("--print")  // Single-shot mode - outputs response and exits
        .arg(&prompt)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());  // Prevent interactive mode

    let mut child = cmd.spawn().map_err(|e| {
        format!("Failed to spawn Claude Code process: {}", e)
    })?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Read output
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut output = String::new();
    let mut error_output = String::new();

    // Read stdout and stderr concurrently
    loop {
        tokio::select! {
            line = stdout_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if !output.is_empty() {
                            output.push('\n');
                        }
                        output.push_str(&line);
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!("Error reading stdout: {}", e);
                        break;
                    }
                }
            }
            line = stderr_reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if !error_output.is_empty() {
                            error_output.push('\n');
                        }
                        error_output.push_str(&line);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Error reading stderr: {}", e);
                    }
                }
            }
        }
    }

    // Wait for process to complete
    let status = child.wait().await.map_err(|e| {
        format!("Failed to wait for Claude Code process: {}", e)
    })?;

    if !status.success() {
        let error_msg = if !error_output.is_empty() {
            error_output
        } else if !output.is_empty() {
            output
        } else {
            format!("Claude Code exited with status: {}", status)
        };
        return Err(error_msg);
    }

    if output.is_empty() {
        return Err("Claude Code returned empty response".to_string());
    }

    tracing::info!("Claude Code completed successfully ({} chars)", output.len());
    Ok(output)
}

/// Create a chat session via daemon RPC (without starting CLI)
///
/// Calls chat.create RPC method and returns the session ID and PUB endpoint.
/// The caller should then:
/// 1. Set up ZMQ subscription with the session_id
/// 2. Call send_daemon_prompt to start the CLI and streaming
async fn create_daemon_chat_session(
    client: Arc<GuiRpcClient>,
    working_dir: String,
) -> Result<(Uuid, String), String> {
    use serde_json::json;

    tracing::info!(
        "Creating daemon chat session in {} (CLI not yet started)",
        working_dir,
    );

    let response = client
        .client()
        .call(
            "chat.create",
            Some(json!({
                "working_dir": working_dir,
                "enable_thinking": true,
                "thinking_level": "normal",
            })),
        )
        .await
        .map_err(|e| format!("RPC call failed: {}", e))?;

    let session_id: Uuid = response["session_id"]
        .as_str()
        .ok_or("Missing session_id in response")?
        .parse()
        .map_err(|_| "Invalid session_id format")?;

    let pub_endpoint = response["pub_endpoint"]
        .as_str()
        .ok_or("Missing pub_endpoint in response")?
        .to_string();

    tracing::info!(
        "Created daemon chat session {} at {} (ready for subscription)",
        session_id,
        pub_endpoint
    );

    Ok((session_id, pub_endpoint))
}

/// Start a chat session via daemon RPC (legacy - starts CLI immediately)
///
/// Calls chat.start RPC method and returns the session ID and PUB endpoint.
/// Note: This has a race condition - prefer create_daemon_chat_session + send_daemon_prompt
async fn start_daemon_chat_session(
    client: Arc<GuiRpcClient>,
    working_dir: String,
    initial_prompt: String,
) -> Result<(Uuid, String), String> {
    use serde_json::json;

    tracing::info!(
        "Starting daemon chat session in {} with prompt: {}",
        working_dir,
        initial_prompt.chars().take(50).collect::<String>()
    );

    let response = client
        .client()
        .call(
            "chat.start",
            Some(json!({
                "working_dir": working_dir,
                "enable_thinking": true,
                "thinking_level": "normal",
                "initial_prompt": initial_prompt,
            })),
        )
        .await
        .map_err(|e| format!("RPC call failed: {}", e))?;

    let session_id: Uuid = response["session_id"]
        .as_str()
        .ok_or("Missing session_id in response")?
        .parse()
        .map_err(|_| "Invalid session_id format")?;

    let pub_endpoint = response["pub_endpoint"]
        .as_str()
        .ok_or("Missing pub_endpoint in response")?
        .to_string();

    tracing::info!(
        "Started daemon chat session {} at {}",
        session_id,
        pub_endpoint
    );

    Ok((session_id, pub_endpoint))
}

/// Send a prompt to an existing daemon chat session
async fn send_daemon_prompt(
    client: Arc<GuiRpcClient>,
    session_id: Uuid,
    prompt: String,
) -> Result<(), String> {
    use serde_json::json;

    tracing::info!(
        "Sending prompt to session {}: {}",
        session_id,
        prompt.chars().take(50).collect::<String>()
    );

    client
        .client()
        .call(
            "chat.prompt",
            Some(json!({
                "session_id": session_id.to_string(),
                "prompt": prompt,
            })),
        )
        .await
        .map_err(|e| format!("RPC call failed: {}", e))?;

    Ok(())
}

/// Upgrade a chat session to agent mode via daemon RPC
async fn upgrade_to_agent(client: Arc<GuiRpcClient>, session_id: Uuid) -> Result<(), String> {
    use serde_json::json;

    tracing::info!("Upgrading session {} to agent mode", session_id);

    client
        .client()
        .call(
            "chat.upgrade_to_agent",
            Some(json!({
                "session_id": session_id.to_string(),
            })),
        )
        .await
        .map_err(|e| format!("RPC call failed: {}", e))?;

    tracing::info!("Session {} upgraded to agent mode", session_id);

    Ok(())
}
