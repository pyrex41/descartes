/// Time Travel UI Components
///
/// This module provides a comprehensive time travel debugging interface for agent history,
/// including:
/// - Timeline slider for navigating through agent history
/// - Event visualization with icons and markers
/// - Playback controls for automatic replay
/// - Event details display
/// - Git commit integration
use chrono::{DateTime, Utc};
use descartes_core::{AgentHistoryEvent, HistoryEventType, HistorySnapshot};
use iced::widget::canvas::{self, Canvas, Cursor, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, row, text, Column, Row, Space};
use iced::{
    alignment::{Horizontal, Vertical},
    border, mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme,
};
use std::collections::HashMap;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// State for the time travel UI
#[derive(Debug, Clone)]
pub struct TimeTravelState {
    /// All events in the agent's history
    pub events: Vec<AgentHistoryEvent>,

    /// Available snapshots for quick navigation
    pub snapshots: Vec<HistorySnapshot>,

    /// Currently selected event index
    pub selected_index: Option<usize>,

    /// Playback state
    pub playback: PlaybackState,

    /// Timeline view settings
    pub timeline_settings: TimelineSettings,

    /// Whether data is currently loading
    pub loading: bool,

    /// Selected agent ID
    pub agent_id: Option<String>,

    /// Zoom level for timeline (events per screen width)
    pub zoom_level: f32,

    /// Scroll offset for timeline (starting event index)
    pub scroll_offset: usize,
}

impl Default for TimeTravelState {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            snapshots: Vec::new(),
            selected_index: None,
            playback: PlaybackState::default(),
            timeline_settings: TimelineSettings::default(),
            loading: false,
            agent_id: None,
            zoom_level: 1.0,
            scroll_offset: 0,
        }
    }
}

impl TimeTravelState {
    /// Get the currently selected event
    pub fn selected_event(&self) -> Option<&AgentHistoryEvent> {
        self.selected_index.and_then(|idx| self.events.get(idx))
    }

    /// Get timestamp of selected event
    pub fn selected_timestamp(&self) -> Option<i64> {
        self.selected_event().map(|e| e.timestamp)
    }

    /// Get the time range of all events
    pub fn time_range(&self) -> Option<(i64, i64)> {
        if self.events.is_empty() {
            return None;
        }

        let mut min = i64::MAX;
        let mut max = i64::MIN;

        for event in &self.events {
            min = min.min(event.timestamp);
            max = max.max(event.timestamp);
        }

        Some((min, max))
    }

    /// Get visible events based on zoom and scroll
    pub fn visible_events(&self) -> &[AgentHistoryEvent] {
        let events_per_screen = (100.0 / self.zoom_level) as usize;
        let end = (self.scroll_offset + events_per_screen).min(self.events.len());
        &self.events[self.scroll_offset..end]
    }

    /// Move to the next event
    pub fn next_event(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx + 1 < self.events.len() {
                self.selected_index = Some(idx + 1);
            }
        } else if !self.events.is_empty() {
            self.selected_index = Some(0);
        }
    }

    /// Move to the previous event
    pub fn prev_event(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx > 0 {
                self.selected_index = Some(idx - 1);
            }
        }
    }

    /// Jump to a specific event by index
    pub fn jump_to_event(&mut self, index: usize) {
        if index < self.events.len() {
            self.selected_index = Some(index);

            // Adjust scroll to keep selected event visible
            let events_per_screen = (100.0 / self.zoom_level) as usize;
            if index < self.scroll_offset {
                self.scroll_offset = index;
            } else if index >= self.scroll_offset + events_per_screen {
                self.scroll_offset = index.saturating_sub(events_per_screen / 2);
            }
        }
    }

    /// Jump to a snapshot
    pub fn jump_to_snapshot(&mut self, snapshot_id: &uuid::Uuid) {
        if let Some(snapshot) = self
            .snapshots
            .iter()
            .find(|s| s.snapshot_id == *snapshot_id)
        {
            // Find the event closest to this snapshot's timestamp
            if let Some((idx, _)) = self
                .events
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| (e.timestamp - snapshot.timestamp).abs())
            {
                self.jump_to_event(idx);
            }
        }
    }
}

/// Playback state for automatic replay
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackState {
    /// Whether playback is active
    pub playing: bool,

    /// Playback speed multiplier (1.0 = real-time, 2.0 = 2x speed)
    pub speed: f32,

    /// Whether to loop when reaching the end
    pub loop_enabled: bool,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            speed: 1.0,
            loop_enabled: false,
        }
    }
}

/// Settings for timeline visualization
#[derive(Debug, Clone)]
pub struct TimelineSettings {
    /// Show event type icons
    pub show_icons: bool,

    /// Show git commit markers
    pub show_git_commits: bool,

    /// Show timestamp labels
    pub show_timestamps: bool,

    /// Show event tooltips on hover
    pub show_tooltips: bool,

    /// Height of the timeline in pixels
    pub height: f32,

    /// Marker sizes
    pub marker_size: f32,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            show_icons: true,
            show_git_commits: true,
            show_timestamps: true,
            show_tooltips: true,
            height: 100.0,
            marker_size: 8.0,
        }
    }
}

// ============================================================================
// MESSAGES
// ============================================================================

/// Messages for time travel UI interactions
#[derive(Debug, Clone)]
pub enum TimeTravelMessage {
    /// Load events for an agent
    LoadHistory(String),

    /// Events loaded successfully
    HistoryLoaded(Vec<AgentHistoryEvent>, Vec<HistorySnapshot>),

    /// Select an event by index
    SelectEvent(usize),

    /// Select an event by timestamp
    SelectTimestamp(i64),

    /// Navigate to previous event
    PrevEvent,

    /// Navigate to next event
    NextEvent,

    /// Jump to a specific snapshot
    JumpToSnapshot(uuid::Uuid),

    /// Toggle playback
    TogglePlayback,

    /// Set playback speed
    SetPlaybackSpeed(f32),

    /// Toggle loop mode
    ToggleLoop,

    /// Playback tick (called periodically during playback)
    PlaybackTick,

    /// Zoom in on timeline
    ZoomIn,

    /// Zoom out on timeline
    ZoomOut,

    /// Scroll timeline
    ScrollTimeline(i32),

    /// Timeline slider interaction
    TimelineSliderChanged(f32),

    /// Mouse moved over timeline
    TimelineHover(Option<usize>),
}

// ============================================================================
// UI COMPONENTS
// ============================================================================

/// Create the complete time travel UI view
pub fn view(state: &TimeTravelState) -> Element<TimeTravelMessage> {
    if state.loading {
        return container(text("Loading agent history..."))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into();
    }

    if state.events.is_empty() {
        return container(
            column![
                text("No history available").size(20),
                Space::with_height(10),
                text("Select an agent to view its history").size(14),
            ]
            .align_x(Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into();
    }

    let timeline = view_timeline(state);
    let controls = view_playback_controls(state);
    let details = view_event_details(state);
    let stats = view_statistics(state);

    column![
        // Top section: Statistics and controls
        row![stats, Space::with_width(20), controls]
            .spacing(10)
            .align_y(Vertical::Center),
        Space::with_height(20),
        // Middle section: Timeline
        timeline,
        Space::with_height(20),
        // Bottom section: Event details
        details,
    ]
    .spacing(10)
    .padding(20)
    .into()
}

/// View timeline with events
fn view_timeline(state: &TimeTravelState) -> Element<TimeTravelMessage> {
    let timeline_canvas = Canvas::new(TimelineCanvas {
        state: state.clone(),
    })
    .width(Length::Fill)
    .height(state.timeline_settings.height);

    let zoom_controls = row![
        button(text("-").size(16))
            .on_press(TimeTravelMessage::ZoomOut)
            .padding(5),
        text(format!("{}x", state.zoom_level)).size(14),
        button(text("+").size(16))
            .on_press(TimeTravelMessage::ZoomIn)
            .padding(5),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    column![
        row![
            text("Timeline").size(18),
            Space::with_width(Length::Fill),
            zoom_controls,
        ]
        .align_y(Vertical::Center),
        Space::with_height(10),
        timeline_canvas,
    ]
    .into()
}

/// View playback controls
fn view_playback_controls(state: &TimeTravelState) -> Element<TimeTravelMessage> {
    let prev_btn = button(text("◀").size(16))
        .on_press(TimeTravelMessage::PrevEvent)
        .padding(8);

    let play_pause_icon = if state.playback.playing { "⏸" } else { "▶" };
    let play_pause_btn = button(text(play_pause_icon).size(16))
        .on_press(TimeTravelMessage::TogglePlayback)
        .padding(8);

    let next_btn = button(text("▶▶").size(16))
        .on_press(TimeTravelMessage::NextEvent)
        .padding(8);

    let speed_buttons = row![
        button(text("0.5x").size(12))
            .on_press(TimeTravelMessage::SetPlaybackSpeed(0.5))
            .padding(5),
        button(text("1x").size(12))
            .on_press(TimeTravelMessage::SetPlaybackSpeed(1.0))
            .padding(5),
        button(text("2x").size(12))
            .on_press(TimeTravelMessage::SetPlaybackSpeed(2.0))
            .padding(5),
        button(text("5x").size(12))
            .on_press(TimeTravelMessage::SetPlaybackSpeed(5.0))
            .padding(5),
    ]
    .spacing(5);

    let loop_btn = button(
        text(if state.playback.loop_enabled {
            "Loop: On"
        } else {
            "Loop: Off"
        })
        .size(12),
    )
    .on_press(TimeTravelMessage::ToggleLoop)
    .padding(5);

    container(
        column![
            text("Playback Controls").size(14),
            Space::with_height(10),
            row![prev_btn, play_pause_btn, next_btn,].spacing(10),
            Space::with_height(10),
            row![text("Speed:").size(12), Space::with_width(5), speed_buttons,]
                .align_y(Vertical::Center),
            Space::with_height(5),
            loop_btn,
        ]
        .align_x(Horizontal::Center),
    )
    .padding(15)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: border::rounded(8),
        ..Default::default()
    })
    .into()
}

/// View event details for the selected event
fn view_event_details(state: &TimeTravelState) -> Element<TimeTravelMessage> {
    if let Some(event) = state.selected_event() {
        let timestamp = DateTime::from_timestamp(event.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let event_type_color = event_type_color(&event.event_type);
        let event_type_icon = event_type_icon(&event.event_type);

        let event_data_str =
            serde_json::to_string_pretty(&event.event_data).unwrap_or_else(|_| "{}".to_string());

        let tags_display = if event.tags.is_empty() {
            text("No tags").size(12)
        } else {
            text(event.tags.join(", ")).size(12)
        };

        let git_commit_display = if let Some(ref commit) = event.git_commit_hash {
            column![
                text("Git Commit:").size(12),
                text(commit).size(12).style(Color::from_rgb8(100, 200, 255)),
            ]
            .spacing(2)
        } else {
            column![text("No git commit").size(12)]
        };

        container(
            column![
                row![
                    text(event_type_icon).size(24).style(event_type_color),
                    Space::with_width(10),
                    column![
                        text(format!("{:?}", event.event_type)).size(18),
                        text(timestamp).size(12),
                    ]
                    .spacing(2),
                ]
                .align_y(Vertical::Center),
                Space::with_height(15),
                text("Event ID:").size(12),
                text(event.event_id.to_string()).size(11),
                Space::with_height(10),
                text("Agent ID:").size(12),
                text(&event.agent_id).size(11),
                Space::with_height(10),
                text("Tags:").size(12),
                tags_display,
                Space::with_height(10),
                git_commit_display,
                Space::with_height(15),
                text("Event Data:").size(14),
                Space::with_height(5),
                container(text(event_data_str).size(11))
                    .padding(10)
                    .style(|theme: &Theme| container::Style {
                        background: Some(theme.palette().background.into()),
                        border: border::rounded(4),
                        ..Default::default()
                    }),
            ]
            .spacing(5),
        )
        .padding(15)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: border::rounded(8),
            ..Default::default()
        })
        .into()
    } else {
        container(
            text("Select an event to view details")
                .size(14)
                .align_x(Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(20)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: border::rounded(8),
            ..Default::default()
        })
        .into()
    }
}

/// View statistics about the history
fn view_statistics(state: &TimeTravelState) -> Element<TimeTravelMessage> {
    let total_events = state.events.len();
    let selected_index = state
        .selected_index
        .map(|i| format!("{}/{}", i + 1, total_events))
        .unwrap_or_else(|| "None".to_string());

    let (start_time, end_time) = if let Some((start, end)) = state.time_range() {
        let start_dt = DateTime::from_timestamp(start, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let end_dt = DateTime::from_timestamp(end, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        (start_dt, end_dt)
    } else {
        ("N/A".to_string(), "N/A".to_string())
    };

    let duration = if let Some((start, end)) = state.time_range() {
        let duration_secs = end - start;
        if duration_secs < 60 {
            format!("{}s", duration_secs)
        } else if duration_secs < 3600 {
            format!("{}m", duration_secs / 60)
        } else if duration_secs < 86400 {
            format!("{}h", duration_secs / 3600)
        } else {
            format!("{}d", duration_secs / 86400)
        }
    } else {
        "N/A".to_string()
    };

    // Count events by type
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for event in &state.events {
        *type_counts
            .entry(format!("{:?}", event.event_type))
            .or_insert(0) += 1;
    }

    let type_counts_display: Vec<Element<TimeTravelMessage>> = type_counts
        .iter()
        .map(|(event_type, count)| {
            row![
                text(event_type).size(11),
                Space::with_width(5),
                text(format!("({})", count)).size(11),
            ]
            .spacing(5)
            .into()
        })
        .collect();

    container(
        column![
            text("History Statistics").size(14),
            Space::with_height(10),
            text(format!("Total Events: {}", total_events)).size(12),
            text(format!("Selected: {}", selected_index)).size(12),
            text(format!("Duration: {}", duration)).size(12),
            text(format!("Snapshots: {}", state.snapshots.len())).size(12),
            Space::with_height(10),
            text("Event Types:").size(12),
            Column::with_children(type_counts_display).spacing(2),
            Space::with_height(10),
            text(format!("Start: {}", start_time)).size(10),
            text(format!("End: {}", end_time)).size(10),
        ]
        .spacing(5),
    )
    .padding(15)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: border::rounded(8),
        ..Default::default()
    })
    .into()
}

// ============================================================================
// TIMELINE CANVAS
// ============================================================================

/// Custom canvas for rendering the timeline
struct TimelineCanvas {
    state: TimeTravelState,
}

impl<Message> canvas::Program<Message> for TimelineCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &<Self as canvas::Program<Message>>::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        if self.state.events.is_empty() {
            return vec![frame.into_geometry()];
        }

        // Draw background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(30, 30, 40));

        // Draw timeline axis
        let y_center = bounds.height / 2.0;
        let timeline_path = Path::line(
            Point::new(20.0, y_center),
            Point::new(bounds.width - 20.0, y_center),
        );
        frame.stroke(
            &timeline_path,
            Stroke::default()
                .with_width(2.0)
                .with_color(Color::from_rgb8(100, 100, 120)),
        );

        // Calculate positions for visible events
        let visible_events = self.state.visible_events();
        if visible_events.is_empty() {
            return vec![frame.into_geometry()];
        }

        let time_range = self.state.time_range().unwrap();
        let time_span = (time_range.1 - time_range.0) as f32;
        let usable_width = bounds.width - 40.0;

        for (local_idx, event) in visible_events.iter().enumerate() {
            let global_idx = self.state.scroll_offset + local_idx;

            // Calculate x position based on timestamp
            let time_offset = (event.timestamp - time_range.0) as f32;
            let x = 20.0 + (time_offset / time_span) * usable_width;

            // Determine if this event is selected
            let is_selected = Some(global_idx) == self.state.selected_index;

            // Draw event marker
            let marker_size = if is_selected {
                self.state.timeline_settings.marker_size * 1.5
            } else {
                self.state.timeline_settings.marker_size
            };

            let marker_color = if is_selected {
                Color::from_rgb8(255, 200, 50)
            } else {
                event_type_color(&event.event_type)
            };

            // Draw circle for event
            frame.fill(
                &Path::circle(Point::new(x, y_center), marker_size),
                marker_color,
            );

            // Draw git commit marker if present
            if self.state.timeline_settings.show_git_commits && event.git_commit_hash.is_some() {
                frame.fill(
                    &Path::rectangle(Point::new(x - 2.0, y_center - 15.0), Size::new(4.0, 10.0)),
                    Color::from_rgb8(100, 200, 255),
                );
            }

            // Draw event type icon (simplified)
            if self.state.timeline_settings.show_icons {
                let icon = event_type_icon(&event.event_type);
                frame.fill_text(canvas::Text {
                    content: icon.to_string(),
                    position: Point::new(x, y_center - 25.0),
                    size: 12.0.into(),
                    color: marker_color,
                    ..Default::default()
                });
            }

            // Draw timestamp label for selected event
            if is_selected && self.state.timeline_settings.show_timestamps {
                if let Some(dt) = DateTime::from_timestamp(event.timestamp, 0) {
                    let time_str = dt.format("%H:%M:%S").to_string();
                    frame.fill_text(canvas::Text {
                        content: time_str,
                        position: Point::new(x - 25.0, y_center + 20.0),
                        size: 10.0.into(),
                        color: Color::from_rgb8(200, 200, 200),
                        ..Default::default()
                    });
                }
            }
        }

        // Draw snapshot markers
        for snapshot in &self.state.snapshots {
            if let Some((start, _)) = self.state.time_range() {
                let time_offset = (snapshot.timestamp - start) as f32;
                let x = 20.0 + (time_offset / time_span) * usable_width;

                // Draw snapshot marker (triangle)
                frame.fill(
                    &Path::circle(Point::new(x, 15.0), 6.0),
                    Color::from_rgb8(50, 255, 150),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get color for event type
fn event_type_color(event_type: &HistoryEventType) -> Color {
    match event_type {
        HistoryEventType::Thought => Color::from_rgb8(150, 150, 255),
        HistoryEventType::Action => Color::from_rgb8(100, 255, 100),
        HistoryEventType::ToolUse => Color::from_rgb8(255, 200, 100),
        HistoryEventType::StateChange => Color::from_rgb8(255, 150, 255),
        HistoryEventType::Communication => Color::from_rgb8(100, 200, 255),
        HistoryEventType::Decision => Color::from_rgb8(255, 255, 100),
        HistoryEventType::Error => Color::from_rgb8(255, 100, 100),
        HistoryEventType::System => Color::from_rgb8(150, 150, 150),
    }
}

/// Get icon character for event type
fn event_type_icon(event_type: &HistoryEventType) -> &'static str {
    match event_type {
        HistoryEventType::Thought => "💭",
        HistoryEventType::Action => "⚡",
        HistoryEventType::ToolUse => "🔧",
        HistoryEventType::StateChange => "🔄",
        HistoryEventType::Communication => "💬",
        HistoryEventType::Decision => "🎯",
        HistoryEventType::Error => "❌",
        HistoryEventType::System => "⚙",
    }
}

// ============================================================================
// UPDATE LOGIC
// ============================================================================

/// Update time travel state based on messages
pub fn update(state: &mut TimeTravelState, message: TimeTravelMessage) {
    match message {
        TimeTravelMessage::LoadHistory(agent_id) => {
            state.loading = true;
            state.agent_id = Some(agent_id);
            // In a real implementation, this would trigger an async RPC call
        }

        TimeTravelMessage::HistoryLoaded(events, snapshots) => {
            state.events = events;
            state.snapshots = snapshots;
            state.loading = false;
            state.selected_index = None;
            state.scroll_offset = 0;
        }

        TimeTravelMessage::SelectEvent(index) => {
            state.jump_to_event(index);
        }

        TimeTravelMessage::SelectTimestamp(timestamp) => {
            // Find event closest to this timestamp
            if let Some((idx, _)) = state
                .events
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| (e.timestamp - timestamp).abs())
            {
                state.jump_to_event(idx);
            }
        }

        TimeTravelMessage::PrevEvent => {
            state.prev_event();
        }

        TimeTravelMessage::NextEvent => {
            state.next_event();
        }

        TimeTravelMessage::JumpToSnapshot(snapshot_id) => {
            state.jump_to_snapshot(&snapshot_id);
        }

        TimeTravelMessage::TogglePlayback => {
            state.playback.playing = !state.playback.playing;
        }

        TimeTravelMessage::SetPlaybackSpeed(speed) => {
            state.playback.speed = speed;
        }

        TimeTravelMessage::ToggleLoop => {
            state.playback.loop_enabled = !state.playback.loop_enabled;
        }

        TimeTravelMessage::PlaybackTick => {
            if state.playback.playing {
                state.next_event();

                // Handle looping
                if state.playback.loop_enabled {
                    if let Some(idx) = state.selected_index {
                        if idx >= state.events.len() - 1 {
                            state.selected_index = Some(0);
                        }
                    }
                } else {
                    // Stop at the end
                    if let Some(idx) = state.selected_index {
                        if idx >= state.events.len() - 1 {
                            state.playback.playing = false;
                        }
                    }
                }
            }
        }

        TimeTravelMessage::ZoomIn => {
            state.zoom_level = (state.zoom_level * 1.5).min(10.0);
        }

        TimeTravelMessage::ZoomOut => {
            state.zoom_level = (state.zoom_level / 1.5).max(0.1);
        }

        TimeTravelMessage::ScrollTimeline(delta) => {
            if delta > 0 {
                state.scroll_offset = state.scroll_offset.saturating_add(delta as usize);
            } else {
                state.scroll_offset = state.scroll_offset.saturating_sub((-delta) as usize);
            }

            // Clamp to valid range
            let max_offset = state.events.len().saturating_sub(10);
            state.scroll_offset = state.scroll_offset.min(max_offset);
        }

        TimeTravelMessage::TimelineSliderChanged(value) => {
            // Map slider value (0.0-1.0) to event index
            let index = (value * (state.events.len() - 1) as f32) as usize;
            state.jump_to_event(index);
        }

        TimeTravelMessage::TimelineHover(_) => {
            // Could be used to show tooltips
        }
    }
}
