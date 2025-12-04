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

mod dag_canvas_interactions;
mod dag_editor;
mod event_handler;
mod file_tree_view;
mod knowledge_graph_panel;
mod rpc_client;
mod session_selector;
mod session_state;
mod task_board;
mod theme;
mod time_travel;

use theme::{colors, container_styles, button_styles, humanlayer_theme};

use chrono::Utc;
use dag_editor::{DAGEditorMessage, DAGEditorState};
use descartes_core::{Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_daemon::DescartesEvent;
use event_handler::EventHandler;
use file_tree_view::{FileTreeMessage, FileTreeState};
use knowledge_graph_panel::{KnowledgeGraphMessage, KnowledgeGraphPanelState};
use rpc_client::GuiRpcClient;
use session_state::{SessionMessage, SessionState};
use task_board::{KanbanBoard, TaskBoardMessage, TaskBoardState};
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
        .run_with(|| (DescartesGui::new(), iced::Task::none()))
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
    /// Task board state
    task_board_state: TaskBoardState,
    /// DAG editor state
    dag_editor_state: DAGEditorState,
    /// File tree view state
    file_tree_state: FileTreeState,
    /// Knowledge graph panel state
    knowledge_graph_panel_state: KnowledgeGraphPanelState,
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
    TaskBoard,
    SwarmMonitor,
    Debugger,
    DagEditor,
    ContextBrowser,
    FileBrowser,
    KnowledgeGraph,
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
    /// Task board message
    TaskBoard(TaskBoardMessage),
    /// DAG editor message
    DAGEditor(DAGEditorMessage),
    /// File tree view message
    FileTree(FileTreeMessage),
    /// Knowledge graph panel message
    KnowledgeGraph(KnowledgeGraphMessage),
    /// Load sample history data for demo
    LoadSampleHistory,
    /// Load sample tasks for demo
    LoadSampleTasks,
    /// Load sample DAG for demo
    LoadSampleDAG,
    /// Load sample file tree for demo
    LoadSampleFileTree,
    /// Load sample knowledge graph for demo
    LoadSampleKnowledgeGraph,
    /// Generate knowledge graph from file tree
    GenerateKnowledgeGraph,
    /// Clear status message
    ClearStatus,
    /// Show error message
    ShowError(String),
}

impl DescartesGui {
    /// Create a new application instance
    fn new() -> Self {
        Self {
            current_view: ViewMode::Dashboard,
            daemon_connected: false,
            connection_error: None,
            session_state: SessionState::default(),
            time_travel_state: TimeTravelState::default(),
            task_board_state: TaskBoardState::default(),
            dag_editor_state: DAGEditorState::default(),
            file_tree_state: FileTreeState::default(),
            knowledge_graph_panel_state: KnowledgeGraphPanelState::default(),
            rpc_client: None,
            event_handler: None,
            recent_events: Vec::new(),
            status_message: Some(
                "Welcome to Descartes GUI! Select a session or connect to the daemon.".to_string(),
            ),
        }
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

                // Create RPC client
                match GuiRpcClient::default() {
                    Ok(client) => {
                        let client = Arc::new(client);
                        self.rpc_client = Some(Arc::clone(&client));

                        // Create event handler
                        let event_handler = EventHandler::default();
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

                // Update status
                self.status_message = Some(format!("Received event: {:?}", event));
                iced::Task::none()
            }
            Message::Session(msg) => {
                session_state::update(&mut self.session_state, msg);
                iced::Task::none()
            }
            Message::TimeTravel(tt_msg) => {
                time_travel::update(&mut self.time_travel_state, tt_msg);
                iced::Task::none()
            }
            Message::TaskBoard(msg) => {
                task_board::update(&mut self.task_board_state, msg);
                iced::Task::none()
            }
            Message::DAGEditor(msg) => {
                dag_editor::update(&mut self.dag_editor_state, msg);
                iced::Task::none()
            }
            Message::FileTree(msg) => {
                file_tree_view::update(&mut self.file_tree_state, msg);
                iced::Task::none()
            }
            Message::KnowledgeGraph(msg) => {
                knowledge_graph_panel::update(&mut self.knowledge_graph_panel_state, msg);
                iced::Task::none()
            }
            Message::LoadSampleHistory => {
                tracing::info!("Loading sample history data");
                self.load_sample_history();
                iced::Task::none()
            }
            Message::LoadSampleTasks => {
                tracing::info!("Loading sample tasks data");
                self.load_sample_tasks();
                iced::Task::none()
            }
            Message::LoadSampleDAG => {
                tracing::info!("Loading sample DAG data");
                self.load_sample_dag();
                iced::Task::none()
            }
            Message::LoadSampleFileTree => {
                tracing::info!("Loading sample file tree data");
                self.load_sample_file_tree();
                iced::Task::none()
            }
            Message::LoadSampleKnowledgeGraph => {
                tracing::info!("Loading sample knowledge graph data");
                self.load_sample_knowledge_graph();
                iced::Task::none()
            }
            Message::GenerateKnowledgeGraph => {
                tracing::info!("Generating knowledge graph from file tree");
                self.generate_knowledge_graph_from_file_tree();
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
        }
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

        // Update the time travel state
        time_travel::update(
            &mut self.time_travel_state,
            TimeTravelMessage::HistoryLoaded(events, snapshots),
        );
    }

    /// Load sample tasks for demonstration
    fn load_sample_tasks(&mut self) {
        let now = Utc::now().timestamp();

        // Create sample tasks with various states
        let mut todo_tasks = vec![];
        let mut in_progress_tasks = vec![];
        let mut done_tasks = vec![];
        let mut blocked_tasks = vec![];

        // Todo tasks
        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Implement user authentication".to_string(),
            description: Some("Add JWT-based authentication to the API endpoints".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 86400,
            updated_at: now - 3600,
            metadata: None,
        });

        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Write unit tests for parser".to_string(),
            description: Some("Cover edge cases in the expression parser".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 7200,
            updated_at: now - 7200,
            metadata: None,
        });

        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Update documentation".to_string(),
            description: Some("Refresh API documentation with new endpoints".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: now - 3600,
            updated_at: now - 3600,
            metadata: None,
        });

        // In Progress tasks
        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Refactor database layer".to_string(),
            description: Some(
                "Migrate from SQLite to PostgreSQL for better performance".to_string(),
            ),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Epic,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 172800,
            updated_at: now - 1800,
            metadata: None,
        });

        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Design new UI mockups".to_string(),
            description: Some("Create Figma designs for the dashboard redesign".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("charlie".to_string()),
            dependencies: vec![],
            created_at: now - 86400,
            updated_at: now - 600,
            metadata: None,
        });

        // Done tasks
        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Setup CI/CD pipeline".to_string(),
            description: Some(
                "Configure GitHub Actions for automated testing and deployment".to_string(),
            ),
            status: TaskStatus::Done,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 259200,
            updated_at: now - 86400,
            metadata: None,
        });

        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Fix login bug".to_string(),
            description: Some("Resolved issue with session timeout handling".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 172800,
            updated_at: now - 43200,
            metadata: None,
        });

        // Blocked tasks
        let dep_task_id = Uuid::new_v4();
        blocked_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Deploy to production".to_string(),
            description: Some("Waiting for security audit to complete".to_string()),
            status: TaskStatus::Blocked,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![dep_task_id],
            created_at: now - 43200,
            updated_at: now - 3600,
            metadata: None,
        });

        blocked_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Optimize image loading".to_string(),
            description: Some("Blocked on infrastructure team to set up CDN".to_string()),
            status: TaskStatus::Blocked,
            priority: TaskPriority::Low,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("charlie".to_string()),
            dependencies: vec![dep_task_id],
            created_at: now - 86400,
            updated_at: now - 7200,
            metadata: None,
        });

        // Update the task board state
        let board = KanbanBoard {
            todo: todo_tasks,
            in_progress: in_progress_tasks,
            done: done_tasks,
            blocked: blocked_tasks,
        };

        task_board::update(
            &mut self.task_board_state,
            TaskBoardMessage::TasksLoaded(board),
        );
    }

    /// Load sample DAG for demonstration
    fn load_sample_dag(&mut self) {
        use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, DAG};

        let mut dag = DAG::new("Sample Workflow");
        dag.description =
            Some("A sample workflow demonstrating DAG editor capabilities".to_string());

        // Create nodes in a workflow pattern
        // Layer 1: Start node
        let start_node = DAGNode::new_auto("Start")
            .with_position(400.0, 50.0)
            .with_description("Workflow entry point")
            .with_tag("entry");
        let start_id = start_node.node_id;
        dag.add_node(start_node).unwrap();

        // Layer 2: Initial tasks
        let task1 = DAGNode::new_auto("Load Data")
            .with_position(200.0, 180.0)
            .with_description("Load input data from sources")
            .with_tag("io");
        let task1_id = task1.node_id;
        dag.add_node(task1).unwrap();

        let task2 = DAGNode::new_auto("Initialize Config")
            .with_position(400.0, 180.0)
            .with_description("Load and validate configuration")
            .with_tag("config");
        let task2_id = task2.node_id;
        dag.add_node(task2).unwrap();

        let task3 = DAGNode::new_auto("Setup Resources")
            .with_position(600.0, 180.0)
            .with_description("Allocate necessary resources")
            .with_tag("setup");
        let task3_id = task3.node_id;
        dag.add_node(task3).unwrap();

        // Layer 3: Processing tasks
        let process1 = DAGNode::new_auto("Validate Data")
            .with_position(200.0, 310.0)
            .with_description("Validate input data integrity")
            .with_tag("validation");
        let process1_id = process1.node_id;
        dag.add_node(process1).unwrap();

        let process2 = DAGNode::new_auto("Transform Data")
            .with_position(400.0, 310.0)
            .with_description("Apply data transformations")
            .with_tag("processing");
        let process2_id = process2.node_id;
        dag.add_node(process2).unwrap();

        let process3 = DAGNode::new_auto("Generate Reports")
            .with_position(600.0, 310.0)
            .with_description("Create analysis reports")
            .with_tag("reporting");
        let process3_id = process3.node_id;
        dag.add_node(process3).unwrap();

        // Layer 4: Aggregation
        let aggregate = DAGNode::new_auto("Aggregate Results")
            .with_position(400.0, 440.0)
            .with_description("Combine all results")
            .with_tag("aggregation");
        let aggregate_id = aggregate.node_id;
        dag.add_node(aggregate).unwrap();

        // Layer 5: End node
        let end_node = DAGNode::new_auto("Complete")
            .with_position(400.0, 570.0)
            .with_description("Workflow completion")
            .with_tag("exit");
        let end_id = end_node.node_id;
        dag.add_node(end_node).unwrap();

        // Create edges (dependencies)
        // Start to Layer 2
        dag.add_edge(DAGEdge::dependency(start_id, task1_id))
            .unwrap();
        dag.add_edge(DAGEdge::dependency(start_id, task2_id))
            .unwrap();
        dag.add_edge(DAGEdge::dependency(start_id, task3_id))
            .unwrap();

        // Layer 2 to Layer 3
        dag.add_edge(DAGEdge::dependency(task1_id, process1_id))
            .unwrap();
        dag.add_edge(DAGEdge::new(task1_id, process2_id, EdgeType::DataFlow))
            .unwrap();
        dag.add_edge(DAGEdge::dependency(task2_id, process2_id))
            .unwrap();
        dag.add_edge(DAGEdge::soft_dependency(task3_id, process3_id))
            .unwrap();

        // Layer 3 to Layer 4
        dag.add_edge(DAGEdge::dependency(process1_id, aggregate_id))
            .unwrap();
        dag.add_edge(DAGEdge::new(process2_id, aggregate_id, EdgeType::DataFlow))
            .unwrap();
        dag.add_edge(DAGEdge::dependency(process3_id, aggregate_id))
            .unwrap();

        // Layer 4 to End
        dag.add_edge(DAGEdge::new(aggregate_id, end_id, EdgeType::Trigger))
            .unwrap();

        // Update the DAG editor state
        dag_editor::update(&mut self.dag_editor_state, DAGEditorMessage::LoadDAG(dag));
    }

    fn view(&self) -> Element<Message> {
        let header = self.view_header();
        let nav = self.view_navigation();
        let content = self.view_content();

        let main_layout = column![header, row![nav, content,].spacing(0)].spacing(0);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Handle subscriptions (keyboard events, timers, etc.)
    fn subscription(&self) -> iced::Subscription<Message> {
        // Keyboard event subscription
        let keyboard_sub = iced::event::listen_with(|event, _status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
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
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(
                            0.5,
                        )));
                    }
                    Key::Character(ref c) if c == "2" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(
                            1.0,
                        )));
                    }
                    Key::Character(ref c) if c == "3" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(
                            2.0,
                        )));
                    }
                    Key::Character(ref c) if c == "4" => {
                        return Some(Message::TimeTravel(TimeTravelMessage::SetPlaybackSpeed(
                            5.0,
                        )));
                    }
                    // L for loop toggle
                    Key::Character(ref c) if c == "l" && !modifiers.shift() => {
                        return Some(Message::TimeTravel(TimeTravelMessage::ToggleLoop));
                    }
                    _ => {}
                }
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

        iced::Subscription::batch(vec![keyboard_sub, event_sub])
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
        // Navigation items with icons
        let nav_items = vec![
            (ViewMode::Sessions, "\u{2726}", "Sessions"),  // ✦
            (ViewMode::Dashboard, "\u{2302}", "Dashboard"),  // ⌂
            (ViewMode::TaskBoard, "\u{2630}", "Tasks"),  // ☰
            (ViewMode::SwarmMonitor, "\u{25CE}", "Swarm"),  // ◎
            (ViewMode::Debugger, "\u{23F1}", "Debugger"),  // ⏱
            (ViewMode::DagEditor, "\u{25C7}", "Workflows"),  // ◇
            (ViewMode::ContextBrowser, "\u{25C8}", "Context"),  // ◈
            (ViewMode::FileBrowser, "\u{25A4}", "Files"),  // ▤
            (ViewMode::KnowledgeGraph, "\u{25C9}", "Graph"),  // ◉
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
            ViewMode::TaskBoard => self.view_task_board(),
            ViewMode::SwarmMonitor => self.view_swarm_monitor(),
            ViewMode::Debugger => self.view_debugger(),
            ViewMode::DagEditor => self.view_dag_editor(),
            ViewMode::ContextBrowser => self.view_context_browser(),
            ViewMode::FileBrowser => self.view_file_browser(),
            ViewMode::KnowledgeGraph => self.view_knowledge_graph(),
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

    /// Dashboard view
    fn view_dashboard(&self) -> Element<Message> {
        let title = text("Dashboard")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Welcome to Descartes Agent Orchestration")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Stats cards row
        let task_count = self.task_board_state.kanban_board.todo.len() +
            self.task_board_state.kanban_board.in_progress.len() +
            self.task_board_state.kanban_board.done.len() +
            self.task_board_state.kanban_board.blocked.len();

        let stats_cards = row![
            self.view_stat_card("Agents".to_string(), "0".to_string(), "◎".to_string(), colors::PRIMARY),
            Space::with_width(12),
            self.view_stat_card("Tasks".to_string(), format!("{}", task_count), "☰".to_string(), colors::INFO),
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
                    button(text("Load Sample Tasks").size(12))
                        .on_press(Message::LoadSampleTasks)
                        .padding([8, 12])
                        .style(button_styles::secondary),
                    Space::with_width(8),
                    button(text("Load Sample DAG").size(12))
                        .on_press(Message::LoadSampleDAG)
                        .padding([8, 12])
                        .style(button_styles::secondary),
                    Space::with_width(8),
                    button(text("Load History").size(12))
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

    /// Task Board view
    fn view_task_board(&self) -> Element<Message> {
        // Check if tasks are loaded
        let total_tasks = self.task_board_state.kanban_board.todo.len()
            + self.task_board_state.kanban_board.in_progress.len()
            + self.task_board_state.kanban_board.done.len()
            + self.task_board_state.kanban_board.blocked.len();

        if total_tasks == 0 {
            let title = text("Task Board")
                .size(28)
                .color(colors::TEXT_PRIMARY);

            let description = text("No tasks loaded. Load sample tasks to see the Task Board in action.")
                .size(14)
                .color(colors::TEXT_SECONDARY);

            let load_sample_btn = button(text("Load Sample Tasks").size(13))
                .on_press(Message::LoadSampleTasks)
                .padding([10, 16])
                .style(button_styles::primary);

            column![
                title,
                Space::with_height(8),
                description,
                Space::with_height(24),
                load_sample_btn,
            ]
            .spacing(0)
            .into()
        } else {
            // Map task board messages to main messages
            task_board::view(&self.task_board_state).map(Message::TaskBoard)
        }
    }

    /// Swarm Monitor view (placeholder)
    fn view_swarm_monitor(&self) -> Element<Message> {
        let title = text("Swarm Monitor")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Visualize active agents and their status")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Placeholder content
        let placeholder_card = container(
            column![
                text("◎").size(32).color(colors::TEXT_MUTED),
                Space::with_height(12),
                text("No active agents").size(16).color(colors::TEXT_PRIMARY),
                Space::with_height(4),
                text("Spawn an agent to see it here").size(12).color(colors::TEXT_MUTED),
            ]
            .align_x(Horizontal::Center)
        )
        .padding(40)
        .width(Length::Fill)
        .style(container_styles::panel)
        .align_x(Horizontal::Center);

        column![
            title,
            Space::with_height(4),
            subtitle,
            Space::with_height(24),
            placeholder_card,
        ]
        .spacing(0)
        .into()
    }

    /// Debugger view with time travel UI
    fn view_debugger(&self) -> Element<Message> {
        let title = text("Time Travel Debugger")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Step through agent execution history")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        // Add a button to load sample history if no events are loaded
        let load_sample_btn = if self.time_travel_state.events.is_empty() {
            column![
                Space::with_height(16),
                button(text("Load Sample History").size(13))
                    .on_press(Message::LoadSampleHistory)
                    .padding([10, 16])
                    .style(button_styles::primary),
                Space::with_height(16),
            ]
        } else {
            column![]
        };

        column![
            title,
            Space::with_height(4),
            subtitle,
            load_sample_btn,
            // Map time travel messages to main messages
            time_travel::view(&self.time_travel_state).map(Message::TimeTravel),
        ]
        .spacing(0)
        .into()
    }

    /// DAG Editor view
    fn view_dag_editor(&self) -> Element<Message> {
        // Check if DAG is empty
        if self.dag_editor_state.dag.nodes.is_empty() {
            let title = text("Workflow Editor")
                .size(28)
                .color(colors::TEXT_PRIMARY);

            let subtitle = text("Design and visualize agent workflows as directed acyclic graphs")
                .size(14)
                .color(colors::TEXT_SECONDARY);

            let load_sample_btn = button(text("Load Sample Workflow").size(13))
                .on_press(Message::LoadSampleDAG)
                .padding([10, 16])
                .style(button_styles::primary);

            column![
                title,
                Space::with_height(4),
                subtitle,
                Space::with_height(24),
                load_sample_btn,
            ]
            .spacing(0)
            .into()
        } else {
            // Map DAG editor messages to main messages
            dag_editor::view(&self.dag_editor_state).map(Message::DAGEditor)
        }
    }

    /// Context Browser view (placeholder)
    fn view_context_browser(&self) -> Element<Message> {
        let title = text("Context Browser")
            .size(28)
            .color(colors::TEXT_PRIMARY);

        let subtitle = text("Browse and manage agent execution context")
            .size(14)
            .color(colors::TEXT_SECONDARY);

        let features = vec![
            ("◈", "View current agent state"),
            ("◇", "Browse variable bindings"),
            ("▤", "Inspect memory contents"),
            ("⌕", "Search through context history"),
            ("↗", "Export context snapshots"),
        ];

        let feature_list: Vec<Element<Message>> = features
            .into_iter()
            .map(|(icon, text_str)| {
                row![
                    text(icon).size(14).color(colors::TEXT_MUTED),
                    Space::with_width(12),
                    text(text_str).size(13).color(colors::TEXT_SECONDARY),
                ]
                .align_y(Vertical::Center)
                .into()
            })
            .collect();

        let coming_soon = container(
            column![
                text("Coming Soon").size(12).color(colors::WARNING),
                Space::with_height(16),
                column(feature_list).spacing(12),
            ]
        )
        .padding(20)
        .style(container_styles::panel);

        column![
            title,
            Space::with_height(4),
            subtitle,
            Space::with_height(24),
            coming_soon,
        ]
        .spacing(0)
        .into()
    }

    /// File Browser view
    fn view_file_browser(&self) -> Element<Message> {
        // Check if file tree is loaded
        if self.file_tree_state.tree.is_none() {
            let title = text("File Browser")
                .size(28)
                .color(colors::TEXT_PRIMARY);

            let subtitle = text("Browse and navigate the project file structure")
                .size(14)
                .color(colors::TEXT_SECONDARY);

            let load_sample_btn = button(text("Load Sample File Tree").size(13))
                .on_press(Message::LoadSampleFileTree)
                .padding([10, 16])
                .style(button_styles::primary);

            column![
                title,
                Space::with_height(4),
                subtitle,
                Space::with_height(24),
                load_sample_btn,
            ]
            .spacing(0)
            .into()
        } else {
            // Map file tree messages to main messages
            file_tree_view::view(&self.file_tree_state).map(Message::FileTree)
        }
    }

    /// Load sample file tree for demonstration
    #[cfg(feature = "agent-runner")]
    fn load_sample_file_tree(&mut self) {
        use descartes_agent_runner::file_tree_builder::FileTreeBuilder;
        use std::path::PathBuf;

        // Get the current project directory
        let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        tracing::info!("Loading file tree from: {:?}", project_dir);

        // Create file tree builder
        let mut builder = FileTreeBuilder::new();

        // Scan the directory
        match builder.scan_directory(&project_dir) {
            Ok(tree) => {
                tracing::info!(
                    "File tree loaded: {} files, {} directories",
                    tree.file_count,
                    tree.directory_count
                );

                // Add some sample knowledge links to demonstrate the feature
                // In a real application, these would come from actual code analysis
                self.add_sample_knowledge_links_to_tree(&tree);

                // Update the file tree state
                file_tree_view::update(
                    &mut self.file_tree_state,
                    FileTreeMessage::TreeLoaded(tree),
                );

                self.status_message = Some("File tree loaded successfully!".to_string());
            }
            Err(e) => {
                tracing::error!("Failed to load file tree: {}", e);
                self.status_message = Some(format!("Failed to load file tree: {}", e));
            }
        }
    }

    /// Load sample file tree for demonstration (stub without agent-runner feature)
    #[cfg(not(feature = "agent-runner"))]
    fn load_sample_file_tree(&mut self) {
        self.status_message =
            Some("File tree feature requires the 'agent-runner' feature to be enabled".to_string());
        tracing::warn!("File tree loading not available without agent-runner feature");
    }

    /// Add sample knowledge links to the tree for demonstration
    #[cfg(feature = "agent-runner")]
    fn add_sample_knowledge_links_to_tree(
        &mut self,
        _tree: &descartes_agent_runner::knowledge_graph::FileTree,
    ) {
        // This is a demonstration function that adds sample knowledge links
        // In a real application, these would come from actual code analysis

        // For now, we'll add deterministic knowledge links to Rust files
        // to show how the badges appear in the UI
        if let Some(tree_state) = &mut self.file_tree_state.tree {
            let mut counter = 0u32;
            for (_, node) in tree_state.nodes.iter_mut() {
                // Add knowledge links to some Rust files
                if let Some(lang) = &node.metadata.language {
                    if matches!(lang, descartes_agent_runner::types::Language::Rust) {
                        // Deterministically add 0-5 knowledge links based on counter
                        let link_count = (counter % 6) as usize;
                        for i in 0..link_count {
                            node.add_knowledge_link(format!("demo-link-{}-{}", counter, i));
                        }
                        counter = counter.wrapping_add(1);
                    }
                }
            }
        }
    }

    /// Knowledge Graph view
    fn view_knowledge_graph(&self) -> Element<Message> {
        // Check if knowledge graph is loaded
        if self.knowledge_graph_panel_state.graph.is_none() {
            let title = text("Knowledge Graph")
                .size(24)
                .color(colors::TEXT_PRIMARY);

            let description = text(
                "No knowledge graph loaded. Generate one from the file tree or load a sample.",
            )
            .size(14)
            .color(colors::TEXT_SECONDARY);

            let steps = column![
                text("Steps:").size(14).color(colors::TEXT_PRIMARY),
                text("1. Go to File Browser and load a file tree").size(13).color(colors::TEXT_SECONDARY),
                text("2. Come back here and click 'Generate from File Tree'").size(13).color(colors::TEXT_SECONDARY),
                text("Or click 'Load Sample' to see a demo knowledge graph.").size(13).color(colors::TEXT_MUTED),
            ]
            .spacing(6);

            let load_sample_btn = button(text("Load Sample").size(13))
                .on_press(Message::LoadSampleKnowledgeGraph)
                .padding([8, 16])
                .style(button_styles::primary);

            let generate_btn = button(text("Generate from File Tree").size(13))
                .on_press(Message::GenerateKnowledgeGraph)
                .padding([8, 16])
                .style(button_styles::secondary);

            let content = column![
                title,
                Space::with_height(12),
                description,
                Space::with_height(16),
                steps,
                Space::with_height(20),
                row![load_sample_btn, generate_btn].spacing(12),
            ]
            .spacing(8)
            .padding(24);

            container(content)
                .style(container_styles::panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            // Map knowledge graph messages to main messages
            knowledge_graph_panel::view(&self.knowledge_graph_panel_state)
                .map(Message::KnowledgeGraph)
        }
    }

    /// Load sample knowledge graph for demonstration
    #[cfg(feature = "agent-runner")]
    fn load_sample_knowledge_graph(&mut self) {
        use descartes_agent_runner::knowledge_graph::{
            FileReference, KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType,
            RelationshipType,
        };
        use std::path::PathBuf;

        let mut graph = KnowledgeGraph::new();

        // Create sample nodes representing a small codebase
        // Module: main
        let mut main_module = KnowledgeNode::new(
            KnowledgeNodeType::Module,
            "main".to_string(),
            "main".to_string(),
        );
        main_module.description = Some("Main application module".to_string());
        main_module.add_file_reference(FileReference {
            file_node_id: "main-file".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line_range: (1, 50),
            column_range: None,
            is_definition: true,
        });
        let main_module_id = graph.add_node(main_module);

        // Function: main
        let mut main_fn = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "main".to_string(),
            "main::main".to_string(),
        );
        main_fn.description = Some("Application entry point".to_string());
        main_fn.signature = Some("fn main()".to_string());
        main_fn.parent_id = Some(main_module_id.clone());
        main_fn.add_file_reference(FileReference {
            file_node_id: "main-file".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line_range: (10, 15),
            column_range: Some((0, 10)),
            is_definition: true,
        });
        let main_fn_id = graph.add_node(main_fn);

        // Function: initialize
        let mut init_fn = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "initialize".to_string(),
            "main::initialize".to_string(),
        );
        init_fn.description = Some("Initialize application state".to_string());
        init_fn.signature = Some("fn initialize() -> AppState".to_string());
        init_fn.return_type = Some("AppState".to_string());
        init_fn.parent_id = Some(main_module_id.clone());
        init_fn.add_file_reference(FileReference {
            file_node_id: "main-file".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line_range: (20, 30),
            column_range: Some((0, 15)),
            is_definition: true,
        });
        let init_fn_id = graph.add_node(init_fn);

        // Class: AppState
        let mut app_state = KnowledgeNode::new(
            KnowledgeNodeType::Class,
            "AppState".to_string(),
            "app::AppState".to_string(),
        );
        app_state.description = Some("Application state container".to_string());
        app_state.add_file_reference(FileReference {
            file_node_id: "app-file".to_string(),
            file_path: PathBuf::from("src/app.rs"),
            line_range: (5, 20),
            column_range: None,
            is_definition: true,
        });
        let app_state_id = graph.add_node(app_state);

        // Method: new
        let mut new_method = KnowledgeNode::new(
            KnowledgeNodeType::Method,
            "new".to_string(),
            "app::AppState::new".to_string(),
        );
        new_method.description = Some("Create new AppState instance".to_string());
        new_method.signature = Some("fn new() -> Self".to_string());
        new_method.return_type = Some("Self".to_string());
        new_method.parent_id = Some(app_state_id.clone());
        new_method.add_file_reference(FileReference {
            file_node_id: "app-file".to_string(),
            file_path: PathBuf::from("src/app.rs"),
            line_range: (22, 27),
            column_range: Some((4, 15)),
            is_definition: true,
        });
        let new_method_id = graph.add_node(new_method);

        // Module: utils
        let mut utils_module = KnowledgeNode::new(
            KnowledgeNodeType::Module,
            "utils".to_string(),
            "utils".to_string(),
        );
        utils_module.description = Some("Utility functions".to_string());
        utils_module.add_file_reference(FileReference {
            file_node_id: "utils-file".to_string(),
            file_path: PathBuf::from("src/utils.rs"),
            line_range: (1, 100),
            column_range: None,
            is_definition: true,
        });
        let utils_module_id = graph.add_node(utils_module);

        // Function: helper
        let mut helper_fn = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "helper".to_string(),
            "utils::helper".to_string(),
        );
        helper_fn.description = Some("Helper utility function".to_string());
        helper_fn.signature = Some("fn helper(data: &str) -> String".to_string());
        helper_fn.parameters = vec!["data: &str".to_string()];
        helper_fn.return_type = Some("String".to_string());
        helper_fn.parent_id = Some(utils_module_id.clone());
        helper_fn.add_file_reference(FileReference {
            file_node_id: "utils-file".to_string(),
            file_path: PathBuf::from("src/utils.rs"),
            line_range: (10, 15),
            column_range: Some((0, 10)),
            is_definition: true,
        });
        let helper_fn_id = graph.add_node(helper_fn);

        // Create relationships
        graph.add_edge(KnowledgeEdge::new(
            main_fn_id.clone(),
            init_fn_id.clone(),
            RelationshipType::Calls,
        ));

        graph.add_edge(KnowledgeEdge::new(
            init_fn_id.clone(),
            new_method_id.clone(),
            RelationshipType::Calls,
        ));

        graph.add_edge(KnowledgeEdge::new(
            init_fn_id.clone(),
            app_state_id.clone(),
            RelationshipType::Uses,
        ));

        graph.add_edge(KnowledgeEdge::new(
            main_fn_id.clone(),
            helper_fn_id.clone(),
            RelationshipType::Calls,
        ));

        graph.add_edge(KnowledgeEdge::new(
            new_method_id.clone(),
            app_state_id.clone(),
            RelationshipType::DefinedIn,
        ));

        tracing::info!(
            "Sample knowledge graph loaded: {} nodes, {} edges",
            graph.nodes.len(),
            graph.edges.len()
        );

        // Update the knowledge graph panel state
        knowledge_graph_panel::update(
            &mut self.knowledge_graph_panel_state,
            KnowledgeGraphMessage::GraphLoaded(graph),
        );

        self.status_message = Some("Sample knowledge graph loaded successfully!".to_string());
    }

    /// Load sample knowledge graph for demonstration (stub without agent-runner feature)
    #[cfg(not(feature = "agent-runner"))]
    fn load_sample_knowledge_graph(&mut self) {
        self.status_message = Some(
            "Knowledge graph feature requires the 'agent-runner' feature to be enabled".to_string(),
        );
        tracing::warn!("Knowledge graph loading not available without agent-runner feature");
    }

    /// Generate knowledge graph from the current file tree
    #[cfg(feature = "agent-runner")]
    fn generate_knowledge_graph_from_file_tree(&mut self) {
        if self.file_tree_state.tree.is_none() {
            self.status_message = Some("No file tree loaded. Load a file tree first.".to_string());
            return;
        }

        use descartes_agent_runner::knowledge_graph_overlay::KnowledgeGraphOverlay;

        let mut file_tree = self.file_tree_state.tree.clone().unwrap();

        tracing::info!("Generating knowledge graph from file tree");

        match KnowledgeGraphOverlay::new() {
            Ok(mut overlay) => {
                match overlay.generate_and_link(&mut file_tree) {
                    Ok(knowledge_graph) => {
                        tracing::info!(
                            "Knowledge graph generated: {} nodes, {} edges",
                            knowledge_graph.nodes.len(),
                            knowledge_graph.edges.len()
                        );

                        // Update file tree state with links
                        self.file_tree_state.tree = Some(file_tree);

                        // Update knowledge graph panel
                        knowledge_graph_panel::update(
                            &mut self.knowledge_graph_panel_state,
                            KnowledgeGraphMessage::GraphLoaded(knowledge_graph),
                        );

                        self.status_message = Some(format!(
                            "Knowledge graph generated successfully! {} entities extracted.",
                            self.knowledge_graph_panel_state
                                .graph
                                .as_ref()
                                .unwrap()
                                .nodes
                                .len()
                        ));
                    }
                    Err(e) => {
                        tracing::error!("Failed to generate knowledge graph: {}", e);
                        self.status_message =
                            Some(format!("Failed to generate knowledge graph: {}", e));
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create knowledge graph overlay: {}", e);
                self.status_message = Some(format!("Failed to create overlay: {}", e));
            }
        }
    }

    /// Generate knowledge graph from the current file tree (stub without agent-runner feature)
    #[cfg(not(feature = "agent-runner"))]
    fn generate_knowledge_graph_from_file_tree(&mut self) {
        self.status_message = Some(
            "Knowledge graph generation requires the 'agent-runner' feature to be enabled"
                .to_string(),
        );
        tracing::warn!("Knowledge graph generation not available without agent-runner feature");
    }
}
