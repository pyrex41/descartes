//! Swarm Monitor UI View - Phase 3.5.4
//!
//! This module implements a comprehensive UI for visualizing active agents, their statuses,
//! and real-time 'Thinking' states in an intuitive layout.
//!
//! # Features
//!
//! - **Agent Cards**: Visual cards for each agent showing status, progress, and thoughts
//! - **Status Indicators**: Color-coded badges for different agent states
//! - **Real-time Updates**: Dynamic updates from JSON stream parser
//! - **Statistics Panel**: Aggregated metrics and swarm health
//! - **Filtering & Grouping**: Filter by status, group by type, search by name
//! - **Thinking Visualization**: Animated "thinking bubble" for agents in Thinking state
//! - **Progress Bars**: Visual progress indicators for running agents
//! - **Error Display**: Clear error messages for failed agents
//! - **Timeline View**: Status transition history

use descartes_core::{
    AgentProgress, AgentRuntimeState, RuntimeAgentError, RuntimeAgentStatus, StatusTransition,
};
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{alignment, Color, Element, Length, Theme};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Target frames per second for animations
pub const TARGET_FPS: f32 = 60.0;

/// Frame time budget in milliseconds (16.67ms for 60 FPS)
pub const FRAME_TIME_BUDGET_MS: f32 = 16.67;

/// Maximum number of frame times to track for performance monitoring
pub const MAX_FRAME_TIME_SAMPLES: usize = 100;

// ============================================================================
// CONNECTION STATUS
// ============================================================================

/// Connection status for live event streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl ConnectionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Error => "Error",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ConnectionStatus::Disconnected => Color::from_rgb(0.5, 0.5, 0.5),
            ConnectionStatus::Connecting => Color::from_rgb(0.9, 0.7, 0.3),
            ConnectionStatus::Connected => Color::from_rgb(0.3, 0.8, 0.3),
            ConnectionStatus::Error => Color::from_rgb(0.9, 0.3, 0.3),
        }
    }
}

// ============================================================================
// SWARM MONITOR STATE
// ============================================================================

/// Main state for the Swarm Monitor view
#[derive(Debug, Clone)]
pub struct SwarmMonitorState {
    /// All tracked agents
    pub agents: HashMap<Uuid, AgentRuntimeState>,

    /// Current filter settings
    pub filter: AgentFilter,

    /// Current grouping mode
    pub grouping: GroupingMode,

    /// Search query
    pub search_query: String,

    /// Selected agent for detailed view
    pub selected_agent: Option<Uuid>,

    /// Animation state for thinking indicators (0.0 to 1.0)
    pub animation_phase: f32,

    /// Sort mode
    pub sort_mode: SortMode,

    /// Live update settings
    pub live_updates_enabled: bool,

    /// WebSocket streaming enabled
    pub websocket_enabled: bool,

    /// Performance tracking
    pub last_update: Instant,
    pub update_count: u64,
    pub fps: f32,

    /// Animation performance tracking
    pub frame_times: Vec<f32>,
    pub max_frame_time: f32,

    /// Connection status
    pub connection_status: ConnectionStatus,

    /// Pending attach request (agent_id waiting for credentials)
    pub pending_attach: Option<Uuid>,

    /// Last received attach credentials (agent_id, token, url)
    pub last_attach_credentials: Option<(Uuid, String, String)>,
}

impl Default for SwarmMonitorState {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            filter: AgentFilter::All,
            grouping: GroupingMode::None,
            search_query: String::new(),
            selected_agent: None,
            animation_phase: 0.0,
            sort_mode: SortMode::ByName,
            live_updates_enabled: true,
            websocket_enabled: false,
            last_update: Instant::now(),
            update_count: 0,
            fps: 0.0,
            frame_times: Vec::with_capacity(MAX_FRAME_TIME_SAMPLES),
            max_frame_time: 0.0,
            connection_status: ConnectionStatus::Disconnected,
            pending_attach: None,
            last_attach_credentials: None,
        }
    }
}

impl SwarmMonitorState {
    /// Create a new swarm monitor state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update or add an agent
    pub fn update_agent(&mut self, agent: AgentRuntimeState) {
        self.agents.insert(agent.agent_id, agent);
    }

    /// Remove an agent
    pub fn remove_agent(&mut self, agent_id: &Uuid) {
        self.agents.remove(agent_id);
    }

    /// Get filtered and sorted agents
    pub fn filtered_agents(&self) -> Vec<&AgentRuntimeState> {
        let mut agents: Vec<&AgentRuntimeState> = self
            .agents
            .values()
            .filter(|agent| self.filter.matches(agent))
            .filter(|agent| {
                if self.search_query.is_empty() {
                    true
                } else {
                    let query = self.search_query.to_lowercase();
                    agent.name.to_lowercase().contains(&query)
                        || agent.task.to_lowercase().contains(&query)
                        || agent.agent_id.to_string().contains(&query)
                }
            })
            .collect();

        // Sort agents
        match self.sort_mode {
            SortMode::ByName => agents.sort_by(|a, b| a.name.cmp(&b.name)),
            SortMode::ByStatus => agents.sort_by_key(|a| a.status as u8),
            SortMode::ByCreatedAt => agents.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortMode::ByUpdatedAt => agents.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        }

        agents
    }

    /// Get grouped agents
    pub fn grouped_agents(&self) -> Vec<(String, Vec<&AgentRuntimeState>)> {
        let filtered = self.filtered_agents();

        match self.grouping {
            GroupingMode::None => {
                vec![("All Agents".to_string(), filtered)]
            }
            GroupingMode::ByStatus => {
                let mut groups: HashMap<RuntimeAgentStatus, Vec<&AgentRuntimeState>> =
                    HashMap::new();
                for agent in filtered {
                    groups
                        .entry(agent.status)
                        .or_default()
                        .push(agent);
                }

                let mut result: Vec<(String, Vec<&AgentRuntimeState>)> = groups
                    .into_iter()
                    .map(|(status, agents)| (status.to_string(), agents))
                    .collect();

                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
            GroupingMode::ByModel => {
                let mut groups: HashMap<String, Vec<&AgentRuntimeState>> = HashMap::new();
                for agent in filtered {
                    groups
                        .entry(agent.model_backend.clone())
                        .or_default()
                        .push(agent);
                }

                let mut result: Vec<(String, Vec<&AgentRuntimeState>)> =
                    groups.into_iter().collect();

                result.sort_by(|a, b| a.0.cmp(&b.0));
                result
            }
        }
    }

    /// Compute current statistics
    pub fn compute_statistics(&self) -> SwarmStatistics {
        let agents: Vec<&AgentRuntimeState> = self.agents.values().collect();

        let mut status_counts = HashMap::new();
        let mut total_active = 0;
        let mut total_completed = 0;
        let mut total_failed = 0;
        let mut execution_times = Vec::new();

        for agent in &agents {
            *status_counts.entry(agent.status).or_insert(0) += 1;

            if agent.is_active() {
                total_active += 1;
            }

            match agent.status {
                RuntimeAgentStatus::Completed => total_completed += 1,
                RuntimeAgentStatus::Failed => total_failed += 1,
                _ => {}
            }

            if let Some(exec_time) = agent.execution_time() {
                execution_times.push(exec_time.num_seconds() as f64);
            }
        }

        let avg_execution_time = if !execution_times.is_empty() {
            Some(execution_times.iter().sum::<f64>() / execution_times.len() as f64)
        } else {
            None
        };

        SwarmStatistics {
            total_agents: agents.len(),
            status_counts,
            total_active,
            total_completed,
            total_failed,
            avg_execution_time,
        }
    }

    /// Increment animation phase (for thinking indicators)
    /// Optimized for 60 FPS: increment by 1/60 = 0.0167 per frame
    pub fn tick_animation(&mut self) {
        let frame_start = Instant::now();

        // Increment animation phase at 60 FPS
        self.animation_phase = (self.animation_phase + 0.0167) % 1.0;

        // Track performance
        self.update_count += 1;
        let frame_time = frame_start.elapsed().as_secs_f32() * 1000.0; // ms

        // Update frame time tracking
        if self.frame_times.len() >= MAX_FRAME_TIME_SAMPLES {
            self.frame_times.remove(0);
        }
        self.frame_times.push(frame_time);

        if frame_time > self.max_frame_time {
            self.max_frame_time = frame_time;
        }

        // Calculate FPS
        let elapsed = self.last_update.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.update_count as f32 / elapsed;
            self.update_count = 0;
            self.last_update = Instant::now();
        }
    }

    /// Get average frame time in milliseconds
    pub fn avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    /// Check if animation performance is acceptable (< 16.67ms per frame for 60 FPS)
    pub fn is_performance_acceptable(&self) -> bool {
        self.avg_frame_time() < FRAME_TIME_BUDGET_MS
    }

    /// Enable live updates
    pub fn enable_live_updates(&mut self) {
        self.live_updates_enabled = true;
    }

    /// Disable live updates
    pub fn disable_live_updates(&mut self) {
        self.live_updates_enabled = false;
    }

    /// Toggle live updates
    pub fn toggle_live_updates(&mut self) {
        self.live_updates_enabled = !self.live_updates_enabled;
    }

    /// Enable WebSocket streaming
    pub fn enable_websocket(&mut self) {
        self.websocket_enabled = true;
    }

    /// Disable WebSocket streaming
    pub fn disable_websocket(&mut self) {
        self.websocket_enabled = false;
    }

    /// Toggle WebSocket streaming
    pub fn toggle_websocket(&mut self) {
        self.websocket_enabled = !self.websocket_enabled;
    }

    /// Update connection status
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    /// Batch update multiple agents (more efficient than individual updates)
    pub fn update_agents_batch(&mut self, agents: HashMap<Uuid, AgentRuntimeState>) {
        for (agent_id, agent_state) in agents {
            self.agents.insert(agent_id, agent_state);
        }
    }

    /// Update agent from event stream (called on live updates)
    pub fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AgentSpawned { agent } => {
                self.update_agent(agent);
            }
            AgentEvent::AgentStatusChanged { agent_id, status } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent
                        .transition_to(status, Some("Status update from event stream".to_string()))
                        .ok();
                }
            }
            AgentEvent::AgentThoughtUpdate { agent_id, thought } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.update_thought(thought);
                }
            }
            AgentEvent::AgentProgressUpdate { agent_id, progress } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.update_progress(progress);
                }
            }
            AgentEvent::AgentCompleted { agent_id } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent
                        .transition_to(
                            RuntimeAgentStatus::Completed,
                            Some("Agent completed".to_string()),
                        )
                        .ok();
                }
            }
            AgentEvent::AgentFailed { agent_id, error } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.set_error(error);
                    agent
                        .transition_to(RuntimeAgentStatus::Failed, Some("Agent failed".to_string()))
                        .ok();
                }
            }
            AgentEvent::AgentTerminated { agent_id } => {
                self.remove_agent(&agent_id);
            }
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            fps: self.fps,
            avg_frame_time_ms: self.avg_frame_time(),
            max_frame_time_ms: self.max_frame_time,
            total_agents: self.agents.len(),
            active_agents: self.agents.values().filter(|a| a.is_active()).count(),
            is_acceptable: self.is_performance_acceptable(),
        }
    }
}

// ============================================================================
// AGENT EVENTS
// ============================================================================

/// Agent events for live updates
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AgentEvent {
    AgentSpawned {
        agent: AgentRuntimeState,
    },
    AgentStatusChanged {
        agent_id: Uuid,
        status: RuntimeAgentStatus,
    },
    AgentThoughtUpdate {
        agent_id: Uuid,
        thought: String,
    },
    AgentProgressUpdate {
        agent_id: Uuid,
        progress: AgentProgress,
    },
    AgentCompleted {
        agent_id: Uuid,
    },
    AgentFailed {
        agent_id: Uuid,
        error: RuntimeAgentError,
    },
    AgentTerminated {
        agent_id: Uuid,
    },
}

// ============================================================================
// PERFORMANCE STATISTICS
// ============================================================================

/// Performance statistics for the swarm monitor
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Current frames per second
    pub fps: f32,

    /// Average frame time in milliseconds
    pub avg_frame_time_ms: f32,

    /// Maximum frame time in milliseconds
    pub max_frame_time_ms: f32,

    /// Total number of agents being tracked
    pub total_agents: usize,

    /// Number of active agents
    pub active_agents: usize,

    /// Whether performance is acceptable (meeting 60 FPS target)
    pub is_acceptable: bool,
}

// ============================================================================
// SWARM STATISTICS
// ============================================================================

/// Aggregated statistics for the swarm
#[derive(Debug, Clone)]
pub struct SwarmStatistics {
    pub total_agents: usize,
    pub status_counts: HashMap<RuntimeAgentStatus, usize>,
    pub total_active: usize,
    pub total_completed: usize,
    pub total_failed: usize,
    pub avg_execution_time: Option<f64>,
}

// ============================================================================
// FILTER & GROUPING
// ============================================================================

/// Filter options for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentFilter {
    All,
    Active, // Running, Thinking, Initializing
    Idle,
    Running,
    Thinking,
    Paused,
    Completed,
    Failed,
    Terminated,
}

impl AgentFilter {
    pub fn matches(&self, agent: &AgentRuntimeState) -> bool {
        match self {
            AgentFilter::All => true,
            AgentFilter::Active => agent.is_active(),
            AgentFilter::Idle => agent.status == RuntimeAgentStatus::Idle,
            AgentFilter::Running => agent.status == RuntimeAgentStatus::Running,
            AgentFilter::Thinking => agent.status == RuntimeAgentStatus::Thinking,
            AgentFilter::Paused => agent.status == RuntimeAgentStatus::Paused,
            AgentFilter::Completed => agent.status == RuntimeAgentStatus::Completed,
            AgentFilter::Failed => agent.status == RuntimeAgentStatus::Failed,
            AgentFilter::Terminated => agent.status == RuntimeAgentStatus::Terminated,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgentFilter::All => "All",
            AgentFilter::Active => "Active",
            AgentFilter::Idle => "Idle",
            AgentFilter::Running => "Running",
            AgentFilter::Thinking => "Thinking",
            AgentFilter::Paused => "Paused",
            AgentFilter::Completed => "Completed",
            AgentFilter::Failed => "Failed",
            AgentFilter::Terminated => "Terminated",
        }
    }
}

/// Grouping modes for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingMode {
    None,
    ByStatus,
    ByModel,
}

impl GroupingMode {
    pub fn label(&self) -> &'static str {
        match self {
            GroupingMode::None => "No Grouping",
            GroupingMode::ByStatus => "By Status",
            GroupingMode::ByModel => "By Model",
        }
    }
}

/// Sort modes for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    ByName,
    ByStatus,
    ByCreatedAt,
    ByUpdatedAt,
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            SortMode::ByName => "Name",
            SortMode::ByStatus => "Status",
            SortMode::ByCreatedAt => "Created",
            SortMode::ByUpdatedAt => "Updated",
        }
    }
}

// ============================================================================
// MESSAGES
// ============================================================================

/// Messages for the swarm monitor
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum SwarmMonitorMessage {
    /// Change filter
    SetFilter(AgentFilter),

    /// Change grouping
    SetGrouping(GroupingMode),

    /// Change sort mode
    SetSortMode(SortMode),

    /// Update search query
    SearchQueryChanged(String),

    /// Select agent for detailed view
    SelectAgent(Uuid),

    /// Deselect agent
    DeselectAgent,

    /// Refresh agent data (triggered by external events)
    RefreshAgents(HashMap<Uuid, AgentRuntimeState>),

    /// Animation tick (60 FPS)
    AnimationTick,

    /// Toggle live updates
    ToggleLiveUpdates,

    /// Toggle WebSocket streaming
    ToggleWebSocket,

    /// Connection status changed
    ConnectionStatusChanged(ConnectionStatus),

    /// Agent event received (from live stream)
    AgentEventReceived(AgentEvent),

    /// Batch agent update (more efficient for large updates)
    BatchAgentUpdate(Vec<AgentRuntimeState>),

    /// Pause an agent
    PauseAgent(Uuid),

    /// Resume a paused agent
    ResumeAgent(Uuid),

    /// Request attach credentials for an agent
    AttachToAgent(Uuid),

    /// Pause operation completed
    PauseResult(Uuid, Result<(), String>),

    /// Resume operation completed
    ResumeResult(Uuid, Result<(), String>),

    /// Attach operation completed (with token and URL)
    AttachResult(Uuid, Result<(String, String), String>),
}

// ============================================================================
// UPDATE FUNCTION
// ============================================================================

/// Update the swarm monitor state
pub fn update(state: &mut SwarmMonitorState, message: SwarmMonitorMessage) {
    match message {
        SwarmMonitorMessage::SetFilter(filter) => {
            state.filter = filter;
        }
        SwarmMonitorMessage::SetGrouping(grouping) => {
            state.grouping = grouping;
        }
        SwarmMonitorMessage::SetSortMode(sort_mode) => {
            state.sort_mode = sort_mode;
        }
        SwarmMonitorMessage::SearchQueryChanged(query) => {
            state.search_query = query;
        }
        SwarmMonitorMessage::SelectAgent(agent_id) => {
            state.selected_agent = Some(agent_id);
        }
        SwarmMonitorMessage::DeselectAgent => {
            state.selected_agent = None;
        }
        SwarmMonitorMessage::RefreshAgents(agents) => {
            state.agents = agents;
        }
        SwarmMonitorMessage::AnimationTick => {
            state.tick_animation();
        }
        SwarmMonitorMessage::ToggleLiveUpdates => {
            state.toggle_live_updates();
        }
        SwarmMonitorMessage::ToggleWebSocket => {
            state.toggle_websocket();
        }
        SwarmMonitorMessage::ConnectionStatusChanged(status) => {
            state.set_connection_status(status);
        }
        SwarmMonitorMessage::AgentEventReceived(event) => {
            if state.live_updates_enabled {
                state.handle_agent_event(event);
            }
        }
        SwarmMonitorMessage::BatchAgentUpdate(agents) => {
            let agent_map: HashMap<Uuid, AgentRuntimeState> = agents
                .into_iter()
                .map(|agent| (agent.agent_id, agent))
                .collect();
            state.update_agents_batch(agent_map);
        }
        SwarmMonitorMessage::PauseAgent(agent_id) => {
            // Log the pause request - actual RPC call would be triggered by main app
            tracing::info!("Pause requested for agent: {}", agent_id);
            // Mark agent as transitioning (optional local state update)
            if let Some(agent) = state.agents.get_mut(&agent_id) {
                agent.status = RuntimeAgentStatus::Paused;
            }
        }
        SwarmMonitorMessage::ResumeAgent(agent_id) => {
            tracing::info!("Resume requested for agent: {}", agent_id);
            if let Some(agent) = state.agents.get_mut(&agent_id) {
                agent.status = RuntimeAgentStatus::Running;
            }
        }
        SwarmMonitorMessage::AttachToAgent(agent_id) => {
            tracing::info!("Attach requested for agent: {}", agent_id);
            // This would trigger a modal or copy credentials to clipboard
            state.pending_attach = Some(agent_id);
        }
        SwarmMonitorMessage::PauseResult(agent_id, result) => {
            match result {
                Ok(()) => {
                    tracing::info!("Agent {} paused successfully", agent_id);
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.status = RuntimeAgentStatus::Paused;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to pause agent {}: {}", agent_id, e);
                }
            }
        }
        SwarmMonitorMessage::ResumeResult(agent_id, result) => {
            match result {
                Ok(()) => {
                    tracing::info!("Agent {} resumed successfully", agent_id);
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.status = RuntimeAgentStatus::Running;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to resume agent {}: {}", agent_id, e);
                }
            }
        }
        SwarmMonitorMessage::AttachResult(agent_id, result) => {
            match result {
                Ok((token, url)) => {
                    tracing::info!("Attach credentials for agent {}: token={}, url={}", agent_id, token, url);
                    state.last_attach_credentials = Some((agent_id, token, url));
                    state.pending_attach = None;
                }
                Err(e) => {
                    tracing::error!("Failed to get attach credentials for agent {}: {}", agent_id, e);
                    state.pending_attach = None;
                }
            }
        }
    }
}

// ============================================================================
// SUBSCRIPTIONS
// ============================================================================

/// Create a subscription for animations (60 FPS)
pub fn subscription() -> iced::Subscription<SwarmMonitorMessage> {
    use iced::time;
    use std::time::Duration;

    // Target 60 FPS: 1000ms / 60 = ~16.67ms per frame
    time::every(Duration::from_millis(16)).map(|_| SwarmMonitorMessage::AnimationTick)
}

// ============================================================================
// VIEW FUNCTIONS
// ============================================================================

/// Render the swarm monitor view
pub fn view(state: &SwarmMonitorState) -> Element<SwarmMonitorMessage> {
    let title = text("Swarm Monitor").size(32).width(Length::Shrink);

    // Statistics panel
    let stats_panel = view_statistics_panel(state);

    // Live updates and performance panel
    let live_panel = view_live_control_panel(state);

    // Control panel (filters, search, grouping)
    let control_panel = view_control_panel(state);

    // Agent grid or list
    let agent_view = if let Some(agent_id) = state.selected_agent {
        if let Some(agent) = state.agents.get(&agent_id) {
            view_agent_detail(agent, state)
        } else {
            view_agent_grid(state)
        }
    } else {
        view_agent_grid(state)
    };

    let content = column![
        title,
        Space::with_height(20),
        stats_panel,
        Space::with_height(10),
        live_panel,
        Space::with_height(20),
        control_panel,
        Space::with_height(20),
        agent_view,
    ]
    .spacing(10)
    .padding(20);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the live updates and performance control panel
fn view_live_control_panel(state: &SwarmMonitorState) -> Element<SwarmMonitorMessage> {
    let perf_stats = state.get_performance_stats();

    // Live updates toggle
    let live_updates_label = if state.live_updates_enabled {
        "Live Updates: ON"
    } else {
        "Live Updates: OFF"
    };
    let live_updates_btn = button(text(live_updates_label).size(12))
        .padding(8)
        .on_press(SwarmMonitorMessage::ToggleLiveUpdates);

    let live_updates_box = if state.live_updates_enabled {
        container(live_updates_btn).style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgb(0.3, 0.8, 0.3).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgb(0.5, 1.0, 0.5),
                radius: 4.0.into(),
            },
            ..Default::default()
        })
    } else {
        container(live_updates_btn)
    };

    // WebSocket toggle
    let websocket_label = if state.websocket_enabled {
        "WebSocket: ON"
    } else {
        "WebSocket: OFF"
    };
    let websocket_btn = button(text(websocket_label).size(12))
        .padding(8)
        .on_press(SwarmMonitorMessage::ToggleWebSocket);

    let websocket_box = if state.websocket_enabled {
        container(websocket_btn).style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgb(0.3, 0.7, 0.9).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgb(0.5, 0.9, 1.0),
                radius: 4.0.into(),
            },
            ..Default::default()
        })
    } else {
        container(websocket_btn)
    };

    // Connection status
    let status_color = state.connection_status.color();
    let status_badge = container(text(state.connection_status.label()).size(11))
        .padding(6)
        .style(move |_theme: &Theme| container::Style {
            background: Some(status_color.scale_alpha(0.3).into()),
            border: iced::Border {
                width: 1.0,
                color: status_color,
                radius: 4.0.into(),
            },
            text_color: Some(status_color),
            ..Default::default()
        });

    // Performance stats
    let fps_text =
        text(format!("FPS: {:.1}", perf_stats.fps))
            .size(12)
            .color(if perf_stats.is_acceptable {
                Color::from_rgb(0.3, 0.8, 0.3)
            } else {
                Color::from_rgb(0.9, 0.5, 0.3)
            });

    let frame_time_text = text(format!(
        "Frame: {:.2}ms / {:.2}ms",
        perf_stats.avg_frame_time_ms, perf_stats.max_frame_time_ms
    ))
    .size(11)
    .color(Color::from_rgb(0.7, 0.7, 0.8));

    let agent_count_text = text(format!(
        "Agents: {} ({} active)",
        perf_stats.total_agents, perf_stats.active_agents
    ))
    .size(11)
    .color(Color::from_rgb(0.7, 0.7, 0.8));

    let perf_col = column![fps_text, frame_time_text, agent_count_text]
        .spacing(3)
        .align_x(alignment::Horizontal::Right);

    let controls_row = row![
        live_updates_box,
        Space::with_width(10),
        websocket_box,
        Space::with_width(10),
        status_badge,
        Space::with_width(Length::Fill),
        perf_col,
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    container(controls_row)
        .padding(12)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.15, 0.15, 0.25, 0.6).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgba(0.3, 0.3, 0.4, 0.5),
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render the statistics panel
fn view_statistics_panel(state: &SwarmMonitorState) -> Element<SwarmMonitorMessage> {
    let stats = state.compute_statistics();

    let total = stat_box(
        "Total",
        stats.total_agents.to_string(),
        Color::from_rgb(0.4, 0.4, 0.9),
    );
    let active = stat_box(
        "Active",
        stats.total_active.to_string(),
        Color::from_rgb(0.3, 0.8, 0.3),
    );
    let completed = stat_box(
        "Completed",
        stats.total_completed.to_string(),
        Color::from_rgb(0.5, 0.5, 0.5),
    );
    let failed = stat_box(
        "Failed",
        stats.total_failed.to_string(),
        Color::from_rgb(0.9, 0.3, 0.3),
    );

    let avg_time = if let Some(avg) = stats.avg_execution_time {
        format!("{:.1}s", avg)
    } else {
        "N/A".to_string()
    };
    let avg_time_box = stat_box("Avg Time", avg_time, Color::from_rgb(0.7, 0.5, 0.9));

    let stats_row = row![total, active, completed, failed, avg_time_box]
        .spacing(15)
        .width(Length::Fill);

    container(stats_row)
        .padding(15)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.2, 0.2, 0.3, 0.5).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgba(0.4, 0.4, 0.6, 0.3),
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Create a stat box widget
fn stat_box(
    label: impl Into<String>,
    value: impl Into<String>,
    color: Color,
) -> Element<'static, SwarmMonitorMessage> {
    let label_text = text(label.into()).size(12);
    let value_text = text(value.into()).size(24).color(color);

    let content = column![label_text, value_text]
        .spacing(5)
        .align_x(alignment::Horizontal::Center);

    container(content)
        .padding(10)
        .width(Length::Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.15, 0.15, 0.25, 0.8).into()),
            border: iced::Border {
                width: 1.0,
                color: color.scale_alpha(0.3),
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render the control panel (filters, search, grouping)
fn view_control_panel(state: &SwarmMonitorState) -> Element<SwarmMonitorMessage> {
    // Filter buttons
    let filters = vec![
        AgentFilter::All,
        AgentFilter::Active,
        AgentFilter::Running,
        AgentFilter::Thinking,
        AgentFilter::Paused,
        AgentFilter::Completed,
        AgentFilter::Failed,
    ];

    let filter_buttons: Vec<Element<SwarmMonitorMessage>> = filters
        .into_iter()
        .map(|filter| {
            let is_active = state.filter == filter;
            let btn = button(text(filter.label()).size(12))
                .padding(8)
                .on_press(SwarmMonitorMessage::SetFilter(filter));

            if is_active {
                container(btn)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Color::from_rgb(0.3, 0.5, 0.9).into()),
                        border: iced::Border {
                            width: 1.0,
                            color: Color::from_rgb(0.5, 0.7, 1.0),
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                btn.into()
            }
        })
        .collect();

    let filter_row = row(filter_buttons).spacing(8).width(Length::Shrink);

    // Search input
    let search_input = text_input("Search agents...", &state.search_query)
        .on_input(SwarmMonitorMessage::SearchQueryChanged)
        .padding(10)
        .width(250);

    // Grouping buttons
    let grouping_modes = vec![
        GroupingMode::None,
        GroupingMode::ByStatus,
        GroupingMode::ByModel,
    ];

    let grouping_label = text("Group:").size(14);

    let grouping_buttons: Vec<Element<SwarmMonitorMessage>> = grouping_modes
        .into_iter()
        .map(|mode| {
            let is_active = state.grouping == mode;
            let btn = button(text(mode.label()).size(12))
                .padding(8)
                .on_press(SwarmMonitorMessage::SetGrouping(mode));

            if is_active {
                container(btn)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Color::from_rgb(0.3, 0.7, 0.5).into()),
                        border: iced::Border {
                            width: 1.0,
                            color: Color::from_rgb(0.5, 0.9, 0.7),
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                btn.into()
            }
        })
        .collect();

    let grouping_row = row![grouping_label]
        .push(Space::with_width(10))
        .extend(grouping_buttons)
        .spacing(8)
        .align_y(alignment::Vertical::Center);

    // Sort buttons
    let sort_modes = vec![SortMode::ByName, SortMode::ByStatus, SortMode::ByUpdatedAt];

    let sort_label = text("Sort:").size(14);

    let sort_buttons: Vec<Element<SwarmMonitorMessage>> = sort_modes
        .into_iter()
        .map(|mode| {
            let is_active = state.sort_mode == mode;
            let btn = button(text(mode.label()).size(12))
                .padding(8)
                .on_press(SwarmMonitorMessage::SetSortMode(mode));

            if is_active {
                container(btn)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Color::from_rgb(0.7, 0.5, 0.9).into()),
                        border: iced::Border {
                            width: 1.0,
                            color: Color::from_rgb(0.9, 0.7, 1.0),
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                btn.into()
            }
        })
        .collect();

    let sort_row = row![sort_label]
        .push(Space::with_width(10))
        .extend(sort_buttons)
        .spacing(8)
        .align_y(alignment::Vertical::Center);

    let controls = column![
        row![filter_row, Space::with_width(Length::Fill), search_input]
            .spacing(20)
            .align_y(alignment::Vertical::Center),
        Space::with_height(10),
        row![grouping_row, Space::with_width(20), sort_row]
            .spacing(20)
            .align_y(alignment::Vertical::Center),
    ]
    .spacing(10);

    container(controls)
        .padding(15)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.2, 0.2, 0.3, 0.3).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgba(0.4, 0.4, 0.6, 0.3),
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render the agent grid
fn view_agent_grid(state: &SwarmMonitorState) -> Element<SwarmMonitorMessage> {
    let grouped_agents = state.grouped_agents();

    if grouped_agents.is_empty() || grouped_agents.iter().all(|(_, agents)| agents.is_empty()) {
        return container(
            text("No agents found")
                .size(18)
                .color(Color::from_rgb(0.6, 0.6, 0.6)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into();
    }

    let mut content = column![].spacing(20);

    for (group_name, agents) in grouped_agents {
        if agents.is_empty() {
            continue;
        }

        // Group header
        let header = text(group_name)
            .size(20)
            .color(Color::from_rgb(0.8, 0.8, 0.9));

        content = content.push(header);

        // Agent cards in a grid (3 columns)
        let mut current_row: Vec<Element<SwarmMonitorMessage>> = Vec::new();
        let mut agent_elements = Vec::new();

        for (idx, agent) in agents.iter().enumerate() {
            let card = view_agent_card(agent, state);
            current_row.push(card);

            if current_row.len() == 3 || idx == agents.len() - 1 {
                // Add spacing to make all rows the same width
                while current_row.len() < 3 {
                    current_row.push(Space::with_width(Length::FillPortion(1)).into());
                }

                let grid_row = row(std::mem::take(&mut current_row))
                    .spacing(15)
                    .width(Length::Fill);

                agent_elements.push(grid_row.into());
            }
        }

        let grid = column(agent_elements).spacing(15);
        content = content.push(grid);
    }

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render control buttons for agent actions (pause/resume/attach)
fn view_agent_control_buttons(agent: &AgentRuntimeState) -> Element<'static, SwarmMonitorMessage> {
    let agent_id = agent.agent_id;

    let mut buttons_row = row![].spacing(6);

    // Pause button (only show for Running/Thinking states)
    if matches!(
        agent.status,
        RuntimeAgentStatus::Running | RuntimeAgentStatus::Thinking
    ) {
        let pause_btn = button(text("⏸ Pause").size(11))
            .padding([4, 8])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::from_rgb(0.8, 0.6, 0.2).into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb(0.9, 0.7, 0.3),
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .on_press(SwarmMonitorMessage::PauseAgent(agent_id));
        buttons_row = buttons_row.push(pause_btn);
    }

    // Resume button (only show for Paused state)
    if agent.status == RuntimeAgentStatus::Paused {
        let resume_btn = button(text("▶ Resume").size(11))
            .padding([4, 8])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::from_rgb(0.2, 0.7, 0.4).into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb(0.3, 0.8, 0.5),
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .on_press(SwarmMonitorMessage::ResumeAgent(agent_id));
        buttons_row = buttons_row.push(resume_btn);
    }

    // Attach button (show for active or paused agents)
    let is_active = matches!(
        agent.status,
        RuntimeAgentStatus::Running
            | RuntimeAgentStatus::Thinking
            | RuntimeAgentStatus::Paused
    );
    if is_active {
        let attach_btn = button(text("🔗 Attach").size(11))
            .padding([4, 8])
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::from_rgb(0.3, 0.5, 0.8).into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb(0.4, 0.6, 0.9),
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .on_press(SwarmMonitorMessage::AttachToAgent(agent_id));
        buttons_row = buttons_row.push(attach_btn);
    }

    container(buttons_row)
        .width(Length::Fill)
        .into()
}

/// Render an individual agent card
fn view_agent_card(
    agent: &AgentRuntimeState,
    state: &SwarmMonitorState,
) -> Element<'static, SwarmMonitorMessage> {
    let status_color = get_status_color(agent.status);
    let status_badge = container(text(agent.status.to_string().to_uppercase()).size(10))
        .padding(4)
        .style(move |_theme: &Theme| container::Style {
            background: Some(status_color.into()),
            border: iced::Border {
                width: 1.0,
                color: status_color.scale_alpha(0.5),
                radius: 4.0.into(),
            },
            text_color: Some(Color::WHITE),
            ..Default::default()
        });

    // Agent name and ID
    let name_text = text(agent.name.clone()).size(16).color(Color::WHITE);
    let agent_id_str = agent.agent_id.to_string();
    let id_text = text(format!(
        "ID: {}",
        &agent_id_str[..8.min(agent_id_str.len())]
    ))
    .size(10)
    .color(Color::from_rgb(0.6, 0.6, 0.7));

    let header = row![name_text, Space::with_width(Length::Fill), status_badge]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

    let mut card_content = column![header, id_text].spacing(8);

    // Task
    let task_text = text(agent.task.clone())
        .size(12)
        .color(Color::from_rgb(0.8, 0.8, 0.85));
    card_content = card_content.push(Space::with_height(5)).push(task_text);

    // Control buttons (pause/resume/attach)
    let control_buttons = view_agent_control_buttons(agent);
    card_content = card_content.push(Space::with_height(8)).push(control_buttons);

    // Thinking state with animated bubble
    if agent.status == RuntimeAgentStatus::Thinking {
        if let Some(thought) = &agent.current_thought {
            let thinking_icon = text("💭").size(16);
            let thought_text = text(thought.clone())
                .size(12)
                .color(Color::from_rgb(0.5, 0.8, 1.0));

            let thinking_row = row![thinking_icon, thought_text]
                .spacing(8)
                .align_y(alignment::Vertical::Center);

            let animation_phase = state.animation_phase;
            let thinking_container = container(thinking_row)
                .padding(8)
                .width(Length::Fill)
                .style(move |_theme: &Theme| {
                    // Animated pulsing effect
                    let alpha = 0.3 + (animation_phase * 0.4);
                    container::Style {
                        background: Some(Color::from_rgba(0.3, 0.6, 0.9, alpha).into()),
                        border: iced::Border {
                            width: 1.0,
                            color: Color::from_rgba(0.5, 0.8, 1.0, alpha + 0.2),
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                });

            card_content = card_content
                .push(Space::with_height(8))
                .push(thinking_container);
        }
    }

    // Progress bar
    if let Some(progress) = &agent.progress {
        let progress_value = progress.percentage / 100.0;
        let progress_bar_widget = progress_bar(0.0..=1.0, progress_value);

        let progress_label = text(format!("{:.1}%", progress.percentage))
            .size(10)
            .color(Color::from_rgb(0.7, 0.7, 0.8));

        card_content = card_content
            .push(Space::with_height(8))
            .push(progress_bar_widget)
            .push(Space::with_height(2))
            .push(progress_label);
    }

    // Error display
    if let Some(error) = &agent.error {
        let error_icon = text("⚠️").size(14);
        let error_text = text(error.message.clone())
            .size(11)
            .color(Color::from_rgb(1.0, 0.3, 0.3));

        let error_row = row![error_icon, error_text]
            .spacing(6)
            .align_y(alignment::Vertical::Center);

        let error_container =
            container(error_row)
                .padding(6)
                .width(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Color::from_rgba(0.9, 0.2, 0.2, 0.2).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: Color::from_rgba(0.9, 0.3, 0.3, 0.5),
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                });

        card_content = card_content
            .push(Space::with_height(8))
            .push(error_container);
    }

    // Timestamps
    let time_info = if let Some(exec_time) = agent.execution_time() {
        format!("Running for {:.1}s", exec_time.num_seconds())
    } else {
        format!("Created {}", format_relative_time(&agent.created_at))
    };

    let time_text = text(time_info)
        .size(10)
        .color(Color::from_rgb(0.5, 0.5, 0.6));

    card_content = card_content.push(Space::with_height(8)).push(time_text);

    // Make card clickable
    let agent_id = agent.agent_id;
    let card_button = button(card_content)
        .width(Length::FillPortion(1))
        .padding(15)
        .on_press(SwarmMonitorMessage::SelectAgent(agent_id));

    container(card_button)
        .width(Length::FillPortion(1))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.2, 0.2, 0.3, 0.8).into()),
            border: iced::Border {
                width: 2.0,
                color: status_color.scale_alpha(0.4),
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render detailed view for a single agent
fn view_agent_detail(
    agent: &AgentRuntimeState,
    _state: &SwarmMonitorState,
) -> Element<'static, SwarmMonitorMessage> {
    let back_button = button(text("← Back to Grid").size(14))
        .padding(10)
        .on_press(SwarmMonitorMessage::DeselectAgent);

    let status_color = get_status_color(agent.status);

    // Header
    let name_text = text(agent.name.clone()).size(24).color(Color::WHITE);
    let status_badge = container(text(agent.status.to_string().to_uppercase()).size(12))
        .padding(8)
        .style(move |_theme: &Theme| container::Style {
            background: Some(status_color.into()),
            border: iced::Border {
                width: 1.0,
                color: status_color.scale_alpha(0.5),
                radius: 4.0.into(),
            },
            text_color: Some(Color::WHITE),
            ..Default::default()
        });

    let header = row![name_text, Space::with_width(Length::Fill), status_badge]
        .spacing(20)
        .align_y(alignment::Vertical::Center);

    // Details section
    let mut details = column![]
        .spacing(10)
        .push(detail_row("Agent ID", agent.agent_id.to_string()))
        .push(detail_row("Task", agent.task.clone()))
        .push(detail_row("Model", agent.model_backend.clone()))
        .push(detail_row(
            "Created",
            agent.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ))
        .push(detail_row(
            "Updated",
            agent.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));

    if let Some(started) = agent.started_at {
        details = details.push(detail_row(
            "Started",
            started.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
    }

    if let Some(completed) = agent.completed_at {
        details = details.push(detail_row(
            "Completed",
            completed.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
    }

    if let Some(exec_time) = agent.execution_time() {
        details = details.push(detail_row(
            "Execution Time",
            format!("{:.2}s", exec_time.num_seconds()),
        ));
    }

    if let Some(pid) = agent.pid {
        details = details.push(detail_row("PID", pid.to_string()));
    }

    // Current thought
    if let Some(thought) = &agent.current_thought {
        let thought_header = text("Current Thought:")
            .size(16)
            .color(Color::from_rgb(0.8, 0.8, 0.9));
        let thought_content = text(thought.clone())
            .size(14)
            .color(Color::from_rgb(0.6, 0.8, 1.0));

        let thought_box =
            container(column![thought_header, Space::with_height(8), thought_content].spacing(5))
                .padding(15)
                .width(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Color::from_rgba(0.2, 0.4, 0.6, 0.3).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: Color::from_rgba(0.4, 0.6, 0.8, 0.5),
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                });

        details = details.push(Space::with_height(10)).push(thought_box);
    }

    // Progress
    if let Some(progress) = &agent.progress {
        let progress_header = text("Progress:")
            .size(16)
            .color(Color::from_rgb(0.8, 0.8, 0.9));
        let progress_value = progress.percentage / 100.0;
        let progress_bar_widget = progress_bar(0.0..=1.0, progress_value);
        let progress_label = text(format!("{:.1}%", progress.percentage)).size(14);

        let mut progress_col = column![
            progress_header,
            Space::with_height(8),
            progress_bar_widget,
            progress_label
        ]
        .spacing(5);

        if let (Some(current), Some(total)) = (progress.current_step, progress.total_steps) {
            progress_col =
                progress_col.push(text(format!("Step {} of {}", current, total)).size(12));
        }

        if let Some(msg) = &progress.message {
            progress_col = progress_col.push(
                text(msg.clone())
                    .size(12)
                    .color(Color::from_rgb(0.7, 0.7, 0.8)),
            );
        }

        details = details.push(Space::with_height(10)).push(progress_col);
    }

    // Error
    if let Some(error) = &agent.error {
        let error_header = text("Error:")
            .size(16)
            .color(Color::from_rgb(1.0, 0.4, 0.4));
        let error_code = text(format!("Code: {}", error.code)).size(12);
        let error_message = text(error.message.clone())
            .size(14)
            .color(Color::from_rgb(1.0, 0.5, 0.5));

        let mut error_col = column![
            error_header,
            Space::with_height(8),
            error_code,
            error_message
        ]
        .spacing(5);

        if let Some(details_str) = &error.details {
            error_col = error_col.push(Space::with_height(5)).push(
                text(details_str.clone())
                    .size(11)
                    .color(Color::from_rgb(0.8, 0.4, 0.4)),
            );
        }

        let error_box =
            container(error_col)
                .padding(15)
                .width(Length::Fill)
                .style(|_theme: &Theme| container::Style {
                    background: Some(Color::from_rgba(0.6, 0.2, 0.2, 0.3).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: Color::from_rgba(0.9, 0.3, 0.3, 0.5),
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                });

        details = details.push(Space::with_height(10)).push(error_box);
    }

    // Timeline
    let timeline_header = text("Status Timeline:")
        .size(16)
        .color(Color::from_rgb(0.8, 0.8, 0.9));
    let timeline = view_timeline(&agent.timeline);

    let content = column![
        back_button,
        Space::with_height(20),
        header,
        Space::with_height(20),
        details,
        Space::with_height(20),
        timeline_header,
        Space::with_height(10),
        timeline,
    ]
    .spacing(5);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Helper to create a detail row
fn detail_row(
    label: impl Into<String>,
    value: impl Into<String>,
) -> Element<'static, SwarmMonitorMessage> {
    let label_text = text(label.into())
        .size(12)
        .color(Color::from_rgb(0.6, 0.6, 0.7));
    let value_text = text(value.into())
        .size(14)
        .color(Color::from_rgb(0.9, 0.9, 0.95));

    row![container(label_text).width(150), value_text,]
        .spacing(20)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// Render agent timeline
fn view_timeline(timeline: &[StatusTransition]) -> Element<'static, SwarmMonitorMessage> {
    let mut timeline_col = column![].spacing(10);

    for transition in timeline.iter().rev() {
        let from_text = if let Some(from) = transition.from {
            from.to_string()
        } else {
            "—".to_string()
        };

        let to_color = get_status_color(transition.to);

        let time_text = text(transition.timestamp.format("%H:%M:%S").to_string())
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.6));

        let transition_text = text(format!("{} → {}", from_text, transition.to))
            .size(12)
            .color(to_color);

        let reason_text = if let Some(reason) = &transition.reason {
            text(reason.clone())
                .size(11)
                .color(Color::from_rgb(0.6, 0.6, 0.7))
        } else {
            text("").size(11)
        };

        let timeline_item = row![
            time_text,
            Space::with_width(20),
            transition_text,
            Space::with_width(20),
            reason_text,
        ]
        .spacing(5)
        .align_y(alignment::Vertical::Center);

        timeline_col = timeline_col.push(timeline_item);
    }

    container(timeline_col)
        .padding(15)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.15, 0.15, 0.25, 0.5).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgba(0.3, 0.3, 0.4, 0.5),
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get color for agent status
fn get_status_color(status: RuntimeAgentStatus) -> Color {
    match status {
        RuntimeAgentStatus::Idle => Color::from_rgb(0.5, 0.5, 0.5),
        RuntimeAgentStatus::Initializing => Color::from_rgb(0.5, 0.7, 0.9),
        RuntimeAgentStatus::Running => Color::from_rgb(0.3, 0.8, 0.3),
        RuntimeAgentStatus::Thinking => Color::from_rgb(0.5, 0.7, 1.0),
        RuntimeAgentStatus::Paused => Color::from_rgb(0.9, 0.7, 0.3),
        RuntimeAgentStatus::Completed => Color::from_rgb(0.4, 0.6, 0.4),
        RuntimeAgentStatus::Failed => Color::from_rgb(0.9, 0.3, 0.3),
        RuntimeAgentStatus::Terminated => Color::from_rgb(0.6, 0.3, 0.3),
    }
}

/// Format relative time (e.g., "5m ago")
fn format_relative_time(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now - *timestamp;

    let seconds = duration.num_seconds();

    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}d ago", seconds / 86400)
    }
}
