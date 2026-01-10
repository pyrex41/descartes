//! History Graph View state management
//!
//! Provides data structures for visualizing agent history as a graph.
//! Shows causality relationships, agent spawning, and time-travel debugging.

use descartes_core::agent_history::{AgentHistoryEvent, HistoryEventType, HistorySnapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Types of nodes in the history graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistoryNodeType {
    /// Agent lifecycle start
    AgentStart,
    /// Internal reasoning/thought
    Thought,
    /// Action performed
    Action,
    /// External tool usage
    ToolUse,
    /// State machine transition
    StateChange,
    /// Message sent or received
    Communication,
    /// Choice/decision point
    Decision,
    /// Failure or error
    Error,
    /// System/lifecycle event
    System,
    /// Agent completed/finished
    AgentComplete,
}

impl From<&HistoryEventType> for HistoryNodeType {
    fn from(event_type: &HistoryEventType) -> Self {
        match event_type {
            HistoryEventType::Thought => HistoryNodeType::Thought,
            HistoryEventType::Action => HistoryNodeType::Action,
            HistoryEventType::ToolUse => HistoryNodeType::ToolUse,
            HistoryEventType::StateChange => HistoryNodeType::StateChange,
            HistoryEventType::Communication => HistoryNodeType::Communication,
            HistoryEventType::Decision => HistoryNodeType::Decision,
            HistoryEventType::Error => HistoryNodeType::Error,
            HistoryEventType::System => HistoryNodeType::System,
        }
    }
}

/// A node in the history graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryGraphNode {
    /// Unique identifier (from event_id)
    pub id: Uuid,
    /// Type of node
    pub node_type: HistoryNodeType,
    /// Which agent this belongs to
    pub agent_id: String,
    /// Computed X position (set by layout)
    pub x: f32,
    /// Computed Y position (set by layout)
    pub y: f32,
    /// Short label for display
    pub label: String,
    /// Full event data for detail view
    pub event: Option<AgentHistoryEvent>,
    /// Child node IDs (causal successors)
    pub children: Vec<Uuid>,
    /// Parent node ID (causal predecessor)
    pub parent: Option<Uuid>,
    /// If this is a spawn event, ID of the spawned agent's root node
    pub spawned_agent: Option<String>,
    /// Timestamp for ordering and time-travel
    pub timestamp: i64,
    /// Git commit hash at this point (for body state)
    pub git_commit: Option<String>,
    /// Whether this node's branch is expanded
    pub expanded: bool,
    /// Session ID for grouping
    pub session_id: Option<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl HistoryGraphNode {
    /// Create a node from an AgentHistoryEvent
    pub fn from_event(event: &AgentHistoryEvent) -> Self {
        let label = Self::extract_label(event);
        let node_type = HistoryNodeType::from(&event.event_type);

        // Check if this is a spawn event
        let spawned_agent = if event.event_type == HistoryEventType::Action {
            event.event_data.get("spawned_agent_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        Self {
            id: event.event_id,
            node_type,
            agent_id: event.agent_id.clone(),
            x: 0.0,
            y: 0.0,
            label,
            event: Some(event.clone()),
            children: Vec::new(),
            parent: event.parent_event_id,
            spawned_agent,
            timestamp: event.timestamp,
            git_commit: event.git_commit_hash.clone(),
            expanded: true,
            session_id: event.session_id.clone(),
            tags: event.tags.clone(),
        }
    }

    /// Create a synthetic agent start node
    pub fn agent_start(agent_id: &str, timestamp: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_type: HistoryNodeType::AgentStart,
            agent_id: agent_id.to_string(),
            x: 0.0,
            y: 0.0,
            label: format!("Agent: {}", agent_id),
            event: None,
            children: Vec::new(),
            parent: None,
            spawned_agent: None,
            timestamp,
            git_commit: None,
            expanded: true,
            session_id: None,
            tags: Vec::new(),
        }
    }

    /// Extract a short label from an event
    fn extract_label(event: &AgentHistoryEvent) -> String {
        // Try to extract a meaningful label from event_data
        let label = event.event_data.get("summary")
            .or_else(|| event.event_data.get("action"))
            .or_else(|| event.event_data.get("tool_name"))
            .or_else(|| event.event_data.get("message"))
            .or_else(|| event.event_data.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{:?}", event.event_type));

        truncate_label(&label, 40)
    }

    /// Get the height of this node type
    pub fn height(&self) -> f32 {
        match self.node_type {
            HistoryNodeType::AgentStart | HistoryNodeType::AgentComplete => NODE_HEIGHT + 10.0,
            HistoryNodeType::Error => NODE_HEIGHT + 5.0,
            _ => NODE_HEIGHT,
        }
    }
}

/// Truncate a string for label display
fn truncate_label(s: &str, max_len: usize) -> String {
    let s = s.lines().next().unwrap_or(s);
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

// Layout constants
pub const NODE_WIDTH: f32 = 220.0;
pub const NODE_HEIGHT: f32 = 50.0;
pub const HORIZONTAL_GAP: f32 = 60.0;
pub const VERTICAL_GAP: f32 = 25.0;
pub const AGENT_INDENT: f32 = 80.0;
pub const PADDING: f32 = 40.0;
pub const NODE_RADIUS: f32 = 6.0;

/// Filter options for the history graph
#[derive(Debug, Clone, Default)]
pub struct HistoryFilter {
    /// Filter by agent IDs (empty = show all)
    pub agent_ids: HashSet<String>,
    /// Filter by event types (empty = show all)
    pub event_types: HashSet<HistoryNodeType>,
    /// Only show events after this timestamp
    pub after_timestamp: Option<i64>,
    /// Only show events before this timestamp
    pub before_timestamp: Option<i64>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Search text in labels
    pub search_text: Option<String>,
}

impl HistoryFilter {
    /// Check if a node passes the filter
    pub fn matches(&self, node: &HistoryGraphNode) -> bool {
        // Agent filter
        if !self.agent_ids.is_empty() && !self.agent_ids.contains(&node.agent_id) {
            return false;
        }

        // Event type filter
        if !self.event_types.is_empty() && !self.event_types.contains(&node.node_type) {
            return false;
        }

        // Time range filters
        if let Some(after) = self.after_timestamp {
            if node.timestamp < after {
                return false;
            }
        }
        if let Some(before) = self.before_timestamp {
            if node.timestamp > before {
                return false;
            }
        }

        // Session filter
        if let Some(ref session) = self.session_id {
            if node.session_id.as_ref() != Some(session) {
                return false;
            }
        }

        // Search text filter
        if let Some(ref search) = self.search_text {
            if !node.label.to_lowercase().contains(&search.to_lowercase()) {
                return false;
            }
        }

        true
    }
}

/// State for the history graph view
#[derive(Debug, Clone)]
pub struct HistoryGraphState {
    /// All nodes in the graph (keyed by ID)
    pub nodes: HashMap<Uuid, HistoryGraphNode>,
    /// Root nodes for each agent (agent_id -> root node id)
    pub agent_roots: HashMap<String, Uuid>,
    /// Order of agents (for consistent Y positioning)
    pub agent_order: Vec<String>,
    /// Currently selected node
    pub selected_node: Option<Uuid>,
    /// Current timeline position (for time-travel scrubbing)
    pub timeline_position: i64,
    /// Minimum timestamp in the history
    pub min_timestamp: i64,
    /// Maximum timestamp in the history
    pub max_timestamp: i64,
    /// Set of collapsed agent branches
    pub collapsed_agents: HashSet<String>,
    /// Active filter
    pub filter: HistoryFilter,
    /// Canvas pan offset
    pub offset: iced::Vector,
    /// Canvas zoom level
    pub zoom: f32,
    /// Computed graph dimensions
    pub graph_width: f32,
    pub graph_height: f32,
    /// Available snapshots for quick restore
    pub snapshots: Vec<HistorySnapshot>,
    /// Whether the graph is in "live" mode (auto-scrolling to latest)
    pub live_mode: bool,
}

impl Default for HistoryGraphState {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryGraphState {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            agent_roots: HashMap::new(),
            agent_order: Vec::new(),
            selected_node: None,
            timeline_position: 0,
            min_timestamp: 0,
            max_timestamp: 0,
            collapsed_agents: HashSet::new(),
            filter: HistoryFilter::default(),
            offset: iced::Vector::new(0.0, 0.0),
            zoom: 1.0,
            graph_width: 800.0,
            graph_height: 600.0,
            snapshots: Vec::new(),
            live_mode: true,
        }
    }

    /// Load history from events and snapshots
    pub fn load_history(&mut self, events: Vec<AgentHistoryEvent>, snapshots: Vec<HistorySnapshot>) {
        self.clear();
        self.snapshots = snapshots;

        if events.is_empty() {
            return;
        }

        // Find time range
        self.min_timestamp = events.iter().map(|e| e.timestamp).min().unwrap_or(0);
        self.max_timestamp = events.iter().map(|e| e.timestamp).max().unwrap_or(0);
        self.timeline_position = self.max_timestamp;

        // Group events by agent and build nodes
        let mut agent_first_event: HashMap<String, i64> = HashMap::new();

        for event in &events {
            // Track first event per agent
            agent_first_event
                .entry(event.agent_id.clone())
                .and_modify(|ts| *ts = (*ts).min(event.timestamp))
                .or_insert(event.timestamp);

            // Create node
            let node = HistoryGraphNode::from_event(event);
            self.nodes.insert(node.id, node);
        }

        // Create agent root nodes and establish order
        let mut agents: Vec<_> = agent_first_event.into_iter().collect();
        agents.sort_by_key(|(_, ts)| *ts);

        for (agent_id, first_ts) in agents {
            // Create synthetic agent start node
            let root_node = HistoryGraphNode::agent_start(&agent_id, first_ts - 1);
            let root_id = root_node.id;
            self.nodes.insert(root_id, root_node);
            self.agent_roots.insert(agent_id.clone(), root_id);
            self.agent_order.push(agent_id);
        }

        // Link nodes to their parents and agent roots
        self.rebuild_links();
    }

    /// Rebuild parent-child links based on parent_event_id
    fn rebuild_links(&mut self) {
        // Collect parent-child relationships
        let relationships: Vec<_> = self.nodes.values()
            .filter_map(|node| {
                node.parent.map(|parent_id| (parent_id, node.id))
            })
            .collect();

        // Apply relationships
        for (parent_id, child_id) in relationships {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                if !parent.children.contains(&child_id) {
                    parent.children.push(child_id);
                }
            }
        }

        // Link orphan nodes to their agent roots
        let orphans: Vec<_> = self.nodes.values()
            .filter(|node| node.parent.is_none() && node.node_type != HistoryNodeType::AgentStart)
            .map(|node| (node.agent_id.clone(), node.id))
            .collect();

        for (agent_id, node_id) in orphans {
            if let Some(&root_id) = self.agent_roots.get(&agent_id) {
                if let Some(root) = self.nodes.get_mut(&root_id) {
                    if !root.children.contains(&node_id) {
                        root.children.push(node_id);
                    }
                }
                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.parent = Some(root_id);
                }
            }
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, id: Uuid) -> Option<&HistoryGraphNode> {
        self.nodes.get(&id)
    }

    /// Get visible nodes (respecting filter and timeline position)
    pub fn visible_nodes(&self) -> Vec<&HistoryGraphNode> {
        self.nodes.values()
            .filter(|node| {
                // Check timeline position
                node.timestamp <= self.timeline_position &&
                // Check if agent is collapsed
                !self.collapsed_agents.contains(&node.agent_id) &&
                // Check filter
                self.filter.matches(node)
            })
            .collect()
    }

    /// Toggle agent branch collapse
    pub fn toggle_agent(&mut self, agent_id: &str) {
        if self.collapsed_agents.contains(agent_id) {
            self.collapsed_agents.remove(agent_id);
        } else {
            self.collapsed_agents.insert(agent_id.to_string());
        }
    }

    /// Get all unique agent IDs
    pub fn agents(&self) -> &[String] {
        &self.agent_order
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.agent_roots.clear();
        self.agent_order.clear();
        self.selected_node = None;
        self.timeline_position = 0;
        self.min_timestamp = 0;
        self.max_timestamp = 0;
        self.snapshots.clear();
    }

    /// Check if history is loaded
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get agent count
    pub fn agent_count(&self) -> usize {
        self.agent_order.len()
    }
}

/// Messages for history graph operations
#[derive(Debug, Clone)]
pub enum HistoryGraphMessage {
    /// Select a node (for detail view)
    SelectNode(Option<Uuid>),
    /// Toggle agent branch visibility
    ToggleAgent(String),
    /// Set timeline position (for time-travel)
    SetTimelinePosition(i64),
    /// Step forward in time
    StepForward,
    /// Step backward in time
    StepBackward,
    /// Jump to start
    JumpToStart,
    /// Jump to end (latest)
    JumpToEnd,
    /// Toggle live mode
    ToggleLiveMode,
    /// Pan the canvas
    Pan(iced::Vector),
    /// Zoom the canvas
    Zoom(f32),
    /// Zoom to point (for scroll wheel)
    ZoomToPoint(iced::Point, f32),
    /// Update filter
    SetFilter(HistoryFilter),
    /// Clear filter
    ClearFilter,
    /// Load history data
    LoadHistory(Vec<AgentHistoryEvent>, Vec<HistorySnapshot>),
    /// Restore to a snapshot
    RestoreSnapshot(Uuid),
    /// Reset view (zoom and pan)
    ResetView,
}

/// Update history graph state
pub fn update(state: &mut HistoryGraphState, message: HistoryGraphMessage) {
    match message {
        HistoryGraphMessage::SelectNode(node_id) => {
            state.selected_node = node_id;
        }
        HistoryGraphMessage::ToggleAgent(agent_id) => {
            state.toggle_agent(&agent_id);
        }
        HistoryGraphMessage::SetTimelinePosition(timestamp) => {
            state.timeline_position = timestamp.clamp(state.min_timestamp, state.max_timestamp);
            state.live_mode = false;
        }
        HistoryGraphMessage::StepForward => {
            // Find next event after current position
            let next = state.nodes.values()
                .filter(|n| n.timestamp > state.timeline_position)
                .map(|n| n.timestamp)
                .min();
            if let Some(ts) = next {
                state.timeline_position = ts;
            }
            state.live_mode = false;
        }
        HistoryGraphMessage::StepBackward => {
            // Find previous event before current position
            let prev = state.nodes.values()
                .filter(|n| n.timestamp < state.timeline_position)
                .map(|n| n.timestamp)
                .max();
            if let Some(ts) = prev {
                state.timeline_position = ts;
            }
            state.live_mode = false;
        }
        HistoryGraphMessage::JumpToStart => {
            state.timeline_position = state.min_timestamp;
            state.live_mode = false;
        }
        HistoryGraphMessage::JumpToEnd => {
            state.timeline_position = state.max_timestamp;
            state.live_mode = true;
        }
        HistoryGraphMessage::ToggleLiveMode => {
            state.live_mode = !state.live_mode;
            if state.live_mode {
                state.timeline_position = state.max_timestamp;
            }
        }
        HistoryGraphMessage::Pan(delta) => {
            state.offset = iced::Vector::new(state.offset.x + delta.x, state.offset.y + delta.y);
        }
        HistoryGraphMessage::Zoom(zoom) => {
            state.zoom = zoom.clamp(0.1, 5.0);
        }
        HistoryGraphMessage::ZoomToPoint(position, new_zoom) => {
            let old_zoom = state.zoom;
            state.zoom = new_zoom.clamp(0.1, 5.0);

            // Adjust offset to keep point stationary
            let world_x = (position.x - state.offset.x) / old_zoom;
            let world_y = (position.y - state.offset.y) / old_zoom;

            state.offset = iced::Vector::new(
                position.x - world_x * state.zoom,
                position.y - world_y * state.zoom,
            );
        }
        HistoryGraphMessage::SetFilter(filter) => {
            state.filter = filter;
        }
        HistoryGraphMessage::ClearFilter => {
            state.filter = HistoryFilter::default();
        }
        HistoryGraphMessage::LoadHistory(events, snapshots) => {
            state.load_history(events, snapshots);
        }
        HistoryGraphMessage::RestoreSnapshot(snapshot_id) => {
            // Find the snapshot and set timeline to its timestamp
            if let Some(snapshot) = state.snapshots.iter().find(|s| s.snapshot_id == snapshot_id) {
                state.timeline_position = snapshot.timestamp;
                state.live_mode = false;
            }
        }
        HistoryGraphMessage::ResetView => {
            state.zoom = 1.0;
            state.offset = iced::Vector::new(0.0, 0.0);
        }
    }
}
