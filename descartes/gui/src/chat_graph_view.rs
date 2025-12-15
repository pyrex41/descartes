//! Chat Graph View - Canvas-based conversation visualization
//!
//! Renders chat conversations as an interactive tree graph using Iced Canvas.

use crate::chat_graph_state::{
    ChatGraphMessage, ChatGraphNode, ChatGraphNodeType, ChatGraphState, NODE_RADIUS, NODE_WIDTH,
};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::mouse::{self, Cursor};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use uuid::Uuid;

/// Interaction state for the canvas
#[derive(Debug, Clone, Default)]
pub struct InteractionState {
    pub panning: bool,
    pub pan_start: Point,
}

/// Canvas program for rendering the chat graph
pub struct ChatGraphCanvas {
    state: ChatGraphState,
}

impl ChatGraphCanvas {
    pub fn new(state: ChatGraphState) -> Self {
        Self { state }
    }
}

impl canvas::Program<ChatGraphMessage> for ChatGraphCanvas {
    type State = InteractionState;

    fn update(
        &self,
        interaction: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (canvas::event::Status, Option<ChatGraphMessage>) {
        let cursor_position = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor_position {
                    // Check if clicking on a node
                    if let Some(node_id) = find_node_at_position(&self.state, pos) {
                        // Check if clicking expand button on subagent
                        if let Some(node) = self.state.nodes.get(&node_id) {
                            if node.node_type == ChatGraphNodeType::SubagentRoot {
                                // Check if within expand button bounds
                                let btn_bounds = expand_button_bounds(node, &self.state);
                                if pos.x >= btn_bounds.x
                                    && pos.x <= btn_bounds.x + btn_bounds.width
                                    && pos.y >= btn_bounds.y
                                    && pos.y <= btn_bounds.y + btn_bounds.height
                                {
                                    return (
                                        canvas::event::Status::Captured,
                                        Some(ChatGraphMessage::ToggleSubagent(node_id)),
                                    );
                                }
                            }
                        }
                        return (
                            canvas::event::Status::Captured,
                            Some(ChatGraphMessage::SelectNode(Some(node_id))),
                        );
                    } else {
                        // Clicking on empty space - deselect and start pan
                        interaction.panning = true;
                        interaction.pan_start = pos;
                        return (
                            canvas::event::Status::Captured,
                            Some(ChatGraphMessage::SelectNode(None)),
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
                            Some(ChatGraphMessage::Pan(delta)),
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
                        Some(ChatGraphMessage::ZoomToPoint(pos, new_zoom)),
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

        // Draw edges first (behind nodes)
        for root_id in &self.state.root_nodes {
            self.draw_edges_recursive(&mut frame, *root_id);
        }

        // Draw nodes on top
        for root_id in &self.state.root_nodes {
            self.draw_node_recursive(&mut frame, *root_id);
        }

        vec![frame.into_geometry()]
    }
}

impl ChatGraphCanvas {
    /// Draw a node and its visible children recursively
    fn draw_node_recursive(&self, frame: &mut Frame, node_id: Uuid) {
        if let Some(node) = self.state.nodes.get(&node_id) {
            self.draw_node(frame, node);

            // Draw children if expanded
            if node.expanded {
                for child_id in &node.children {
                    self.draw_node_recursive(frame, *child_id);
                }
            }
        }
    }

    /// Draw a single node
    fn draw_node(&self, frame: &mut Frame, node: &ChatGraphNode) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        // Transform to screen coordinates
        let x = node.x * zoom + offset.x;
        let y = node.y * zoom + offset.y;
        let width = NODE_WIDTH * zoom;
        let height = node.height() * zoom;
        let radius = NODE_RADIUS * zoom;

        // Get colors based on node type and state
        let (fill_color, border_color) = self.node_colors(node);
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
            let font_size = (12.0 * zoom).clamp(8.0, 18.0);
            let icon_x = x + 10.0 * zoom;
            let label_x = x + 28.0 * zoom;
            let label_y = y + height / 2.0;

            // Node type icon
            let (icon, icon_color) = match node.node_type {
                ChatGraphNodeType::User => ("U", colors::PRIMARY),
                ChatGraphNodeType::Assistant => ("A", colors::SUCCESS),
                ChatGraphNodeType::Tool => ("T", colors::WARNING),
                ChatGraphNodeType::SubagentRoot => ("S", colors::INFO),
                ChatGraphNodeType::SubagentMessage => ("s", colors::INFO),
            };

            frame.fill_text(Text {
                content: icon.to_string(),
                position: Point::new(icon_x, label_y),
                color: icon_color,
                size: font_size.into(),
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            frame.fill_text(Text {
                content: node.label.clone(),
                position: Point::new(label_x, label_y),
                color: colors::TEXT_PRIMARY,
                size: font_size.into(),
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Default::default()
            });
        }

        // Draw live indicator (pulsing dot)
        if node.is_live && zoom > 0.3 {
            let indicator_x = x + width - 12.0 * zoom;
            let indicator_y = y + 12.0 * zoom;
            let indicator_radius = 4.0 * zoom;

            frame.fill(
                &Path::circle(Point::new(indicator_x, indicator_y), indicator_radius),
                colors::WARNING, // Yellow for live
            );
        }

        // Draw expand/collapse indicator for subagent roots
        if node.node_type == ChatGraphNodeType::SubagentRoot && !node.children.is_empty() {
            let btn_x = x + 8.0 * zoom;
            let btn_y = y + height / 2.0 - 8.0 * zoom;
            let btn_size = 16.0 * zoom;

            let btn_rect = Path::rounded_rectangle(
                Point::new(btn_x, btn_y),
                Size::new(btn_size, btn_size),
                (3.0 * zoom).into(),
            );

            frame.fill(&btn_rect, colors::SURFACE_HOVER);
            frame.stroke(
                &btn_rect,
                Stroke::default()
                    .with_color(colors::BORDER)
                    .with_width(1.0),
            );

            // +/- icon
            if zoom > 0.3 {
                let icon = if node.expanded { "-" } else { "+" };
                frame.fill_text(Text {
                    content: icon.to_string(),
                    position: Point::new(btn_x + btn_size / 2.0, btn_y + btn_size / 2.0),
                    color: colors::TEXT_MUTED,
                    size: (12.0 * zoom).into(),
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    vertical_alignment: iced::alignment::Vertical::Center,
                    ..Default::default()
                });
            }
        }
    }

    /// Get fill and border colors for a node
    fn node_colors(&self, node: &ChatGraphNode) -> (Color, Color) {
        match node.node_type {
            ChatGraphNodeType::User => (colors::PRIMARY_DIM, colors::PRIMARY),
            ChatGraphNodeType::Assistant => (colors::SURFACE, colors::BORDER),
            ChatGraphNodeType::Tool => (colors::SURFACE, colors::TEXT_MUTED),
            ChatGraphNodeType::SubagentRoot => (colors::SURFACE, colors::INFO),
            ChatGraphNodeType::SubagentMessage => {
                (colors::SURFACE, Color::from_rgb(0.4, 0.3, 0.6))
            }
        }
    }

    /// Draw edges from a node to its children
    fn draw_edges_recursive(&self, frame: &mut Frame, node_id: Uuid) {
        if let Some(node) = self.state.nodes.get(&node_id) {
            if node.expanded {
                for child_id in &node.children {
                    self.draw_edge(frame, node, *child_id);
                    self.draw_edges_recursive(frame, *child_id);
                }
            }
        }
    }

    /// Draw a bezier curve edge from parent to child
    fn draw_edge(&self, frame: &mut Frame, parent: &ChatGraphNode, child_id: Uuid) {
        if let Some(child) = self.state.nodes.get(&child_id) {
            let zoom = self.state.zoom;
            let offset = self.state.offset;

            // Start from bottom center of parent
            let start_x = (parent.x + NODE_WIDTH / 2.0) * zoom + offset.x;
            let start_y = (parent.y + parent.height()) * zoom + offset.y;

            // End at top center of child
            let end_x = (child.x + NODE_WIDTH / 2.0) * zoom + offset.x;
            let end_y = child.y * zoom + offset.y;

            // Edge color based on child type
            let edge_color = match child.node_type {
                ChatGraphNodeType::Tool => colors::TEXT_MUTED,
                ChatGraphNodeType::SubagentRoot | ChatGraphNodeType::SubagentMessage => {
                    colors::INFO
                }
                _ => colors::BORDER,
            };

            // Draw curved line using quadratic bezier approximation
            // Iced Path doesn't have bezier, so use line segments
            let mid_y = (start_y + end_y) / 2.0;

            let edge_path = Path::new(|builder| {
                builder.move_to(Point::new(start_x, start_y));
                builder.line_to(Point::new(start_x, mid_y));
                builder.line_to(Point::new(end_x, mid_y));
                builder.line_to(Point::new(end_x, end_y));
            });

            let edge_width = if child.is_live { 2.0 } else { 1.5 };

            frame.stroke(
                &edge_path,
                Stroke::default()
                    .with_color(edge_color)
                    .with_width(edge_width),
            );
        }
    }
}

/// Coordinate transformation: screen to world
#[allow(dead_code)]
pub fn screen_to_world(point: Point, state: &ChatGraphState) -> Point {
    Point::new(
        (point.x - state.offset.x) / state.zoom,
        (point.y - state.offset.y) / state.zoom,
    )
}

/// Coordinate transformation: world to screen
pub fn world_to_screen(point: Point, state: &ChatGraphState) -> Point {
    Point::new(
        point.x * state.zoom + state.offset.x,
        point.y * state.zoom + state.offset.y,
    )
}

/// Check if a point is inside a node
pub fn point_in_node(point: Point, node: &ChatGraphNode, state: &ChatGraphState) -> bool {
    let screen_pos = world_to_screen(Point::new(node.x, node.y), state);
    let width = NODE_WIDTH * state.zoom;
    let height = node.height() * state.zoom;

    point.x >= screen_pos.x
        && point.x <= screen_pos.x + width
        && point.y >= screen_pos.y
        && point.y <= screen_pos.y + height
}

/// Find the node at a given screen position
pub fn find_node_at_position(state: &ChatGraphState, position: Point) -> Option<Uuid> {
    // Check in reverse order (top-most first)
    for node in state.nodes.values() {
        if point_in_node(position, node, state) {
            return Some(node.id);
        }
    }
    None
}

/// Get the bounding box of the expand button for a subagent node
pub fn expand_button_bounds(node: &ChatGraphNode, state: &ChatGraphState) -> Rectangle {
    let zoom = state.zoom;
    let screen_pos = world_to_screen(Point::new(node.x, node.y), state);
    let height = node.height() * zoom;

    let btn_x = screen_pos.x + 8.0 * zoom;
    let btn_y = screen_pos.y + height / 2.0 - 8.0 * zoom;
    let btn_size = 16.0 * zoom;

    Rectangle::new(Point::new(btn_x, btn_y), Size::new(btn_size, btn_size))
}

/// Render the complete chat graph view
#[allow(dead_code)]
pub fn view(state: &ChatGraphState) -> Element<'static, ChatGraphMessage> {
    if !state.show_graph_view {
        // Return empty if not showing graph view
        return Space::with_height(0).into();
    }

    // Toggle button to switch back to linear view
    let toggle_btn = view_toggle_button(state.show_graph_view);

    let header = row![Space::with_width(Length::Fill), toggle_btn,].padding([8, 16]);

    let canvas_widget = canvas::Canvas::new(ChatGraphCanvas::new(state.clone()))
        .width(Length::Fill)
        .height(Length::Fill);

    // Wrap in container
    let graph_container = container(canvas_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(container_styles::panel);

    // Node detail popup (if selected)
    if let Some(node_id) = state.selected_node {
        if let Some(node) = state.get_node(node_id) {
            // Stack popup over graph with header
            return column![header, graph_container, view_node_detail(node),].into();
        }
    }

    column![header, graph_container,].into()
}

/// Render node detail popup
fn view_node_detail(node: &ChatGraphNode) -> Element<'static, ChatGraphMessage> {
    let type_label = match node.node_type {
        ChatGraphNodeType::User => "User Message",
        ChatGraphNodeType::Assistant => "Assistant",
        ChatGraphNodeType::Tool => "Tool Call",
        ChatGraphNodeType::SubagentRoot => "Subagent",
        ChatGraphNodeType::SubagentMessage => "Subagent Message",
    };

    let type_color = match node.node_type {
        ChatGraphNodeType::User => colors::PRIMARY,
        ChatGraphNodeType::Assistant => colors::SUCCESS,
        ChatGraphNodeType::Tool => colors::WARNING,
        ChatGraphNodeType::SubagentRoot | ChatGraphNodeType::SubagentMessage => colors::INFO,
    };

    let content = if let Some(ref tool) = node.tool_call {
        format!(
            "Tool: {}\n\nInput:\n{}\n\nOutput:\n{}",
            tool.name,
            tool.arguments,
            tool.output.as_deref().unwrap_or("(pending)")
        )
    } else if let Some(ref subagent) = node.subagent {
        format!(
            "Task: {}\nRole: {}\nStatus: {:?}",
            subagent.description, subagent.role, subagent.status
        )
    } else {
        node.content.clone().unwrap_or_default()
    };

    let close_btn = button(text("x").size(18))
        .on_press(ChatGraphMessage::SelectNode(None))
        .padding([4, 8])
        .style(button_styles::icon);

    let popup_content = column![
        row![
            container(text(type_label).size(10).color(type_color))
                .padding([3, 8])
                .style(container_styles::badge_success),
            Space::with_width(Length::Fill),
            close_btn,
        ]
        .align_y(Vertical::Center),
        Space::with_height(12),
        scrollable(
            text(content)
                .size(13)
                .font(fonts::MONO)
                .color(colors::TEXT_PRIMARY)
        )
        .height(Length::Fill),
    ]
    .spacing(0)
    .padding(16);

    container(popup_content)
        .width(400)
        .height(300)
        .style(container_styles::panel)
        .into()
}

/// Render the toggle button for switching between graph and linear view
pub fn view_toggle_button(show_graph: bool) -> Element<'static, ChatGraphMessage> {
    let icon = if show_graph { "List" } else { "Graph" };

    button(text(icon).size(12))
        .on_press(ChatGraphMessage::ToggleView)
        .padding([4, 8])
        .style(if show_graph {
            button_styles::nav_active
        } else {
            button_styles::secondary
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_graph_state::ChatGraphNode;

    #[test]
    fn test_point_in_node() {
        let mut state = ChatGraphState::new();
        let mut node = ChatGraphNode::user("Hello".to_string());
        node.x = 100.0;
        node.y = 100.0;
        state.add_node(node.clone());

        // Point inside node
        assert!(point_in_node(
            Point::new(150.0, 120.0),
            &node,
            &state
        ));

        // Point outside node
        assert!(!point_in_node(
            Point::new(50.0, 50.0),
            &node,
            &state
        ));
    }

    #[test]
    fn test_coordinate_transforms() {
        let mut state = ChatGraphState::new();
        state.zoom = 2.0;
        state.offset = Vector::new(50.0, 100.0);

        let world = Point::new(10.0, 20.0);
        let screen = world_to_screen(world, &state);

        assert_eq!(screen.x, 10.0 * 2.0 + 50.0); // 70
        assert_eq!(screen.y, 20.0 * 2.0 + 100.0); // 140
    }
}
