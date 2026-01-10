//! History Graph View - Canvas-based agent history visualization
//!
//! Renders agent history events as an interactive graph using Iced Canvas.
//! Features time-travel debugging, agent swim lanes, and causality edges.

use crate::history_graph_layout::{compute_edges, compute_layout, find_node_at_position};
use crate::history_graph_state::{
    HistoryGraphMessage, HistoryGraphNode, HistoryGraphState, HistoryNodeType, NODE_RADIUS,
    NODE_WIDTH,
};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::mouse::{self, Cursor};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, row, scrollable, slider, text, Space};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

/// Interaction state for the canvas
#[derive(Debug, Clone, Default)]
pub struct InteractionState {
    pub panning: bool,
    pub pan_start: Point,
}

/// Canvas program for rendering the history graph
pub struct HistoryGraphCanvas {
    state: HistoryGraphState,
}

impl HistoryGraphCanvas {
    pub fn new(state: HistoryGraphState) -> Self {
        Self { state }
    }
}

impl canvas::Program<HistoryGraphMessage> for HistoryGraphCanvas {
    type State = InteractionState;

    fn update(
        &self,
        interaction: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (canvas::event::Status, Option<HistoryGraphMessage>) {
        let cursor_position = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor_position {
                    // Check if clicking on a node
                    if let Some(node_id) = find_node_at_position(&self.state, pos.x, pos.y) {
                        return (
                            canvas::event::Status::Captured,
                            Some(HistoryGraphMessage::SelectNode(Some(node_id))),
                        );
                    } else {
                        // Clicking on empty space - deselect and start pan
                        interaction.panning = true;
                        interaction.pan_start = pos;
                        return (
                            canvas::event::Status::Captured,
                            Some(HistoryGraphMessage::SelectNode(None)),
                        );
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                interaction.panning = false;
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if interaction.panning {
                    if let Some(pos) = cursor_position {
                        let delta = Vector::new(
                            pos.x - interaction.pan_start.x,
                            pos.y - interaction.pan_start.y,
                        );
                        interaction.pan_start = pos;
                        return (
                            canvas::event::Status::Captured,
                            Some(HistoryGraphMessage::Pan(delta)),
                        );
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(pos) = cursor_position {
                    let zoom_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
                        mouse::ScrollDelta::Pixels { y, .. } => y * 0.001,
                    };
                    let new_zoom = (self.state.zoom + zoom_delta).clamp(0.1, 5.0);
                    return (
                        canvas::event::Status::Captured,
                        Some(HistoryGraphMessage::ZoomToPoint(pos, new_zoom)),
                    );
                }
            }
            _ => {}
        }

        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _interaction_state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Draw background
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), colors::BACKGROUND);

        // Draw grid lines (time markers)
        self.draw_time_grid(&mut frame, bounds.size());

        // Draw agent lane separators
        self.draw_agent_lanes(&mut frame, bounds.size());

        // Draw edges (causality connections)
        self.draw_edges(&mut frame);

        // Draw nodes
        for node in self.state.visible_nodes() {
            self.draw_node(&mut frame, node);
        }

        // Draw timeline indicator (current position)
        self.draw_timeline_indicator(&mut frame, bounds.size());

        vec![frame.into_geometry()]
    }
}

impl HistoryGraphCanvas {
    /// Draw time grid lines
    fn draw_time_grid(&self, frame: &mut Frame, size: Size) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        // Calculate visible time range
        let time_range = (self.state.max_timestamp - self.state.min_timestamp).max(1) as f32;

        // Draw vertical grid lines (every 10% of time range)
        let num_lines = 10;
        for i in 0..=num_lines {
            let t = i as f32 / num_lines as f32;
            let x = t * self.state.graph_width * zoom + offset.x;

            if x >= 0.0 && x <= size.width {
                let grid_path = Path::line(Point::new(x, 0.0), Point::new(x, size.height));

                frame.stroke(
                    &grid_path,
                    Stroke::default()
                        .with_color(colors::BORDER.scale_alpha(0.3))
                        .with_width(1.0),
                );

                // Draw time label at top
                if zoom > 0.3 {
                    let timestamp =
                        self.state.min_timestamp + (t * time_range) as i64;
                    let time_str = format_timestamp(timestamp);

                    frame.fill_text(Text {
                        content: time_str,
                        position: Point::new(x + 4.0, 8.0),
                        color: colors::TEXT_MUTED,
                        size: (10.0 * zoom.min(1.0)).into(),
                        horizontal_alignment: iced::alignment::Horizontal::Left,
                        vertical_alignment: iced::alignment::Vertical::Top,
                        ..Default::default()
                    });
                }
            }
        }
    }

    /// Draw agent swim lane separators
    fn draw_agent_lanes(&self, frame: &mut Frame, size: Size) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        let mut y_offset = 20.0;
        for agent_id in &self.state.agent_order {
            if self.state.collapsed_agents.contains(agent_id) {
                continue;
            }

            let y = y_offset * zoom + offset.y;

            // Draw agent label
            if zoom > 0.3 && y >= 0.0 && y <= size.height {
                frame.fill_text(Text {
                    content: agent_id.clone(),
                    position: Point::new(8.0, y + 10.0),
                    color: colors::TEXT_SECONDARY,
                    size: (11.0 * zoom.min(1.0)).into(),
                    horizontal_alignment: iced::alignment::Horizontal::Left,
                    vertical_alignment: iced::alignment::Vertical::Top,
                    ..Default::default()
                });

                // Draw lane separator line
                let line_y = y + 60.0 * zoom;
                let line_path = Path::line(Point::new(0.0, line_y), Point::new(size.width, line_y));

                frame.stroke(
                    &line_path,
                    Stroke::default()
                        .with_color(colors::BORDER.scale_alpha(0.2))
                        .with_width(1.0),
                );
            }

            y_offset += 80.0; // Lane height
        }
    }

    /// Draw edges between connected nodes
    fn draw_edges(&self, frame: &mut Frame) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        let edges = compute_edges(&self.state);

        for (start, end) in edges {
            let start_x = start.x * zoom + offset.x;
            let start_y = start.y * zoom + offset.y;
            let end_x = end.x * zoom + offset.x;
            let end_y = end.y * zoom + offset.y;

            // Draw curved line using segments
            let mid_x = (start_x + end_x) / 2.0;

            let edge_path = Path::new(|builder| {
                builder.move_to(Point::new(start_x, start_y));
                builder.line_to(Point::new(mid_x, start_y));
                builder.line_to(Point::new(mid_x, end_y));
                builder.line_to(Point::new(end_x, end_y));
            });

            frame.stroke(
                &edge_path,
                Stroke::default()
                    .with_color(colors::BORDER)
                    .with_width(1.5),
            );

            // Draw arrow head at end
            let arrow_size = 6.0 * zoom;
            let arrow_path = Path::new(|builder| {
                builder.move_to(Point::new(end_x, end_y));
                builder.line_to(Point::new(end_x - arrow_size, end_y - arrow_size / 2.0));
                builder.line_to(Point::new(end_x - arrow_size, end_y + arrow_size / 2.0));
                builder.close();
            });

            frame.fill(&arrow_path, colors::BORDER);
        }
    }

    /// Draw a single node
    fn draw_node(&self, frame: &mut Frame, node: &HistoryGraphNode) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        // Transform to screen coordinates
        let x = node.x * zoom + offset.x;
        let y = node.y * zoom + offset.y;
        let width = NODE_WIDTH * zoom;
        let height = node.height() * zoom;
        let radius = NODE_RADIUS * zoom;

        // Get colors based on node type and state
        let (fill_color, border_color, icon) = node_colors(node);
        let is_selected = self.state.selected_node == Some(node.id);

        // Draw node rectangle
        let node_rect = Path::rounded_rectangle(
            Point::new(x, y),
            Size::new(width, height),
            radius.into(),
        );

        frame.fill(&node_rect, fill_color);

        // Border
        let border_width = if is_selected { 3.0 } else { 1.5 };
        let actual_border = if is_selected {
            colors::PRIMARY
        } else {
            border_color
        };

        frame.stroke(
            &node_rect,
            Stroke::default()
                .with_color(actual_border)
                .with_width(border_width),
        );

        // Draw label with type icon (only if zoom > 0.3)
        if zoom > 0.3 {
            let font_size = (11.0 * zoom).clamp(8.0, 14.0);
            let icon_x = x + 8.0 * zoom;
            let label_x = x + 24.0 * zoom;
            let label_y = y + height / 2.0;

            // Node type icon
            frame.fill_text(Text {
                content: icon.to_string(),
                position: Point::new(icon_x, label_y),
                color: border_color,
                size: font_size.into(),
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            // Node label
            frame.fill_text(Text {
                content: node.label.clone(),
                position: Point::new(label_x, label_y),
                color: colors::TEXT_PRIMARY,
                size: font_size.into(),
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            // Git commit indicator
            if node.git_commit.is_some() && zoom > 0.5 {
                let commit_x = x + width - 20.0 * zoom;
                let commit_y = y + 10.0 * zoom;

                frame.fill_text(Text {
                    content: "g".to_string(),
                    position: Point::new(commit_x, commit_y),
                    color: colors::SUCCESS,
                    size: (9.0 * zoom).into(),
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    vertical_alignment: iced::alignment::Vertical::Top,
                    ..Default::default()
                });
            }
        }
    }

    /// Draw timeline indicator showing current time position
    fn draw_timeline_indicator(&self, frame: &mut Frame, size: Size) {
        if self.state.min_timestamp == self.state.max_timestamp {
            return;
        }

        let zoom = self.state.zoom;
        let offset = self.state.offset;

        // Calculate x position for timeline
        let time_range = (self.state.max_timestamp - self.state.min_timestamp) as f32;
        let time_progress =
            (self.state.timeline_position - self.state.min_timestamp) as f32 / time_range;

        let x = time_progress * self.state.graph_width * zoom + offset.x;

        // Draw vertical line
        let line_path = Path::line(Point::new(x, 0.0), Point::new(x, size.height));

        frame.stroke(
            &line_path,
            Stroke::default()
                .with_color(colors::PRIMARY)
                .with_width(2.0),
        );

        // Draw time label at bottom
        let time_str = format_timestamp(self.state.timeline_position);
        frame.fill_text(Text {
            content: time_str,
            position: Point::new(x + 4.0, size.height - 20.0),
            color: colors::PRIMARY,
            size: 10.0.into(),
            horizontal_alignment: iced::alignment::Horizontal::Left,
            vertical_alignment: iced::alignment::Vertical::Bottom,
            ..Default::default()
        });
    }
}

/// Get colors and icon for a node type
fn node_colors(node: &HistoryGraphNode) -> (Color, Color, &'static str) {
    match node.node_type {
        HistoryNodeType::AgentStart => (colors::PRIMARY_DIM, colors::PRIMARY, "S"),
        HistoryNodeType::Thought => (colors::SURFACE, colors::TEXT_MUTED, "T"),
        HistoryNodeType::Action => (colors::SURFACE, colors::SUCCESS, "A"),
        HistoryNodeType::ToolUse => (colors::SURFACE, colors::WARNING, "U"),
        HistoryNodeType::StateChange => (colors::SURFACE, colors::INFO, "C"),
        HistoryNodeType::Communication => (colors::SURFACE, Color::from_rgb(0.6, 0.4, 0.8), "M"),
        HistoryNodeType::Decision => (colors::SURFACE, Color::from_rgb(0.4, 0.6, 0.9), "D"),
        HistoryNodeType::Error => (Color::from_rgb(0.4, 0.2, 0.2), colors::ERROR, "!"),
        HistoryNodeType::System => (colors::SURFACE, colors::TEXT_MUTED, "*"),
        HistoryNodeType::AgentComplete => (colors::SUCCESS.scale_alpha(0.3), colors::SUCCESS, "E"),
    }
}

/// Format a Unix timestamp for display
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| format!("{}", timestamp))
}

/// Render the complete history graph view
pub fn view(state: &mut HistoryGraphState) -> Element<'static, HistoryGraphMessage> {
    // Compute layout before rendering
    compute_layout(state);

    // Clone state for canvas
    let canvas_state = state.clone();

    let canvas_widget = canvas::Canvas::new(HistoryGraphCanvas::new(canvas_state))
        .width(Length::Fill)
        .height(Length::Fill);

    // Wrap in container
    let graph_container = container(canvas_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(container_styles::panel);

    // Timeline controls at bottom
    let timeline_controls = view_timeline_controls(state);

    // Get node detail if any
    let node_detail: Option<Element<'static, HistoryGraphMessage>> =
        if let Some(node_id) = state.selected_node {
            state.get_node(node_id).map(view_node_detail)
        } else {
            None
        };

    // Build main content
    let mut content_column = column![].spacing(0);

    if let Some(detail) = node_detail {
        content_column = content_column.push(detail);
    }

    content_column = content_column.push(graph_container);
    content_column = content_column.push(timeline_controls);

    content_column.into()
}

/// Render timeline controls (slider, play/pause, step buttons)
fn view_timeline_controls(state: &HistoryGraphState) -> Element<'static, HistoryGraphMessage> {
    let _time_range = (state.max_timestamp - state.min_timestamp).max(1);

    // Timeline slider
    let timeline_slider = slider(
        state.min_timestamp as f64..=state.max_timestamp as f64,
        state.timeline_position as f64,
        |value| HistoryGraphMessage::SetTimelinePosition(value as i64),
    )
    .width(Length::Fill);

    // Control buttons
    let step_back = button(text("<<").size(12))
        .on_press(HistoryGraphMessage::JumpToStart)
        .padding([4, 8])
        .style(button_styles::secondary);

    let prev = button(text("<").size(12))
        .on_press(HistoryGraphMessage::StepBackward)
        .padding([4, 8])
        .style(button_styles::secondary);

    let next = button(text(">").size(12))
        .on_press(HistoryGraphMessage::StepForward)
        .padding([4, 8])
        .style(button_styles::secondary);

    let step_fwd = button(text(">>").size(12))
        .on_press(HistoryGraphMessage::JumpToEnd)
        .padding([4, 8])
        .style(button_styles::secondary);

    let live_btn = button(text(if state.live_mode { "LIVE" } else { "Live" }).size(11))
        .on_press(HistoryGraphMessage::ToggleLiveMode)
        .padding([4, 8])
        .style(if state.live_mode {
            button_styles::nav_active
        } else {
            button_styles::secondary
        });

    // Time display
    let current_time = format_timestamp(state.timeline_position);
    let time_display = text(current_time).size(11).color(colors::TEXT_SECONDARY);

    // Stats
    let stats = text(format!(
        "{} events | {} agents",
        state.event_count(),
        state.agent_count()
    ))
    .size(10)
    .color(colors::TEXT_MUTED);

    let controls = row![
        step_back,
        prev,
        Space::with_width(4),
        timeline_slider,
        Space::with_width(4),
        next,
        step_fwd,
        Space::with_width(12),
        time_display,
        Space::with_width(Length::Fill),
        stats,
        Space::with_width(8),
        live_btn,
    ]
    .spacing(4)
    .align_y(Vertical::Center)
    .padding([8, 12]);

    container(controls)
        .width(Length::Fill)
        .style(container_styles::panel)
        .into()
}

/// Render node detail popup
fn view_node_detail(node: &HistoryGraphNode) -> Element<'static, HistoryGraphMessage> {
    let (_, type_color, _) = node_colors(node);

    let type_label = match node.node_type {
        HistoryNodeType::AgentStart => "Agent Start",
        HistoryNodeType::Thought => "Thought",
        HistoryNodeType::Action => "Action",
        HistoryNodeType::ToolUse => "Tool Use",
        HistoryNodeType::StateChange => "State Change",
        HistoryNodeType::Communication => "Communication",
        HistoryNodeType::Decision => "Decision",
        HistoryNodeType::Error => "Error",
        HistoryNodeType::System => "System",
        HistoryNodeType::AgentComplete => "Agent Complete",
    };

    // Clone data to avoid lifetime issues
    let agent_id = node.agent_id.clone();
    let git_commit = node.git_commit.clone();
    let timestamp_val = node.timestamp;

    let content = if let Some(ref event) = node.event {
        serde_json::to_string_pretty(&event.event_data).unwrap_or_else(|_| "{}".to_string())
    } else {
        format!("Agent: {}", agent_id)
    };

    let close_btn = button(text("x").size(18))
        .on_press(HistoryGraphMessage::SelectNode(None))
        .padding([4, 8])
        .style(button_styles::icon);

    let header = row![
        container(text(type_label).size(10).color(type_color))
            .padding([3, 8])
            .style(container_styles::badge_success),
        Space::with_width(8),
        text(agent_id).size(10).color(colors::TEXT_MUTED),
        Space::with_width(Length::Fill),
        close_btn,
    ]
    .align_y(Vertical::Center);

    let timestamp = format_timestamp(timestamp_val);

    // Build commit info if present
    let commit_info: Element<'static, HistoryGraphMessage> = if let Some(commit) = git_commit {
        let short_commit = commit[..7.min(commit.len())].to_string();
        row![
            Space::with_width(12),
            text(format!("commit: {}", short_commit))
                .size(10)
                .color(colors::SUCCESS),
        ]
        .into()
    } else {
        Space::with_width(0).into()
    };

    let meta_row = row![
        text(timestamp).size(10).color(colors::TEXT_MUTED),
        commit_info,
    ];

    let popup_content = column![
        header,
        Space::with_height(4),
        meta_row,
        Space::with_height(8),
        scrollable(
            text(content)
                .size(12)
                .font(fonts::MONO)
                .color(colors::TEXT_PRIMARY)
        )
        .height(150),
    ]
    .spacing(0)
    .padding(12);

    container(popup_content)
        .width(Length::Fill)
        .style(container_styles::panel)
        .into()
}

/// View for empty state (no history loaded)
pub fn view_empty_state() -> Element<'static, HistoryGraphMessage> {
    let title = text("Agent History")
        .size(28)
        .color(colors::TEXT_PRIMARY);

    let description = text("No agent history loaded. Load history from a session to visualize agent activity.")
        .size(14)
        .color(colors::TEXT_SECONDARY);

    column![
        title,
        Space::with_height(8),
        description,
        Space::with_height(24),
    ]
    .spacing(0)
    .into()
}
