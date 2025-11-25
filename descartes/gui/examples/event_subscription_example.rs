/// Example: Event Subscription with Iced GUI
///
/// This example demonstrates how to use the EventHandler to subscribe to
/// daemon events and display them in an Iced application.
///
/// Usage:
///   1. Start the daemon: cargo run --bin descartes-daemon
///   2. Run this example: cargo run --example event_subscription_example
use descartes_daemon::DescartesEvent;
use descartes_gui::EventHandler;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length, Subscription, Task, Theme};

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("event_subscription_example=debug,info")
        .init();

    iced::application("Event Subscription Example", App::update, App::view)
        .theme(|_| Theme::TokyoNight)
        .subscription(App::subscription)
        .run_with(|| (App::new(), Task::none()))
}

/// Application state
struct App {
    /// Event handler
    event_handler: EventHandler,
    /// Whether we're connected
    connected: bool,
    /// Received events
    events: Vec<EventDisplay>,
    /// Max events to display
    max_events: usize,
}

/// Display representation of an event
#[derive(Debug, Clone)]
struct EventDisplay {
    timestamp: String,
    event_type: String,
    summary: String,
    details: String,
}

impl EventDisplay {
    fn from_event(event: &DescartesEvent) -> Self {
        match event {
            DescartesEvent::AgentEvent(e) => Self {
                timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                event_type: format!("Agent: {:?}", e.event_type),
                summary: format!("Agent {} - {:?}", e.agent_id, e.event_type),
                details: format!(
                    "{}",
                    serde_json::to_string_pretty(&e.data).unwrap_or_default()
                ),
            },
            DescartesEvent::TaskEvent(e) => Self {
                timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                event_type: format!("Task: {:?}", e.event_type),
                summary: format!("Task {} - {:?}", e.task_id, e.event_type),
                details: format!(
                    "{}",
                    serde_json::to_string_pretty(&e.data).unwrap_or_default()
                ),
            },
            DescartesEvent::WorkflowEvent(e) => Self {
                timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                event_type: format!("Workflow: {:?}", e.event_type),
                summary: format!("Workflow {} - {:?}", e.workflow_id, e.event_type),
                details: format!(
                    "{}",
                    serde_json::to_string_pretty(&e.data).unwrap_or_default()
                ),
            },
            DescartesEvent::SystemEvent(e) => Self {
                timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                event_type: format!("System: {:?}", e.event_type),
                summary: format!("System - {:?}", e.event_type),
                details: format!(
                    "{}",
                    serde_json::to_string_pretty(&e.data).unwrap_or_default()
                ),
            },
            DescartesEvent::StateEvent(e) => Self {
                timestamp: e.timestamp.format("%H:%M:%S").to_string(),
                event_type: format!("State: {:?}", e.event_type),
                summary: format!("State {} - {:?}", e.key, e.event_type),
                details: format!(
                    "{}",
                    serde_json::to_string_pretty(&e.data).unwrap_or_default()
                ),
            },
        }
    }
}

/// Messages for the application
#[derive(Debug, Clone)]
enum Message {
    /// Connect to event stream
    Connect,
    /// Disconnect from event stream
    Disconnect,
    /// Event received from daemon
    EventReceived(DescartesEvent),
    /// Clear event history
    ClearEvents,
}

impl App {
    fn new() -> Self {
        Self {
            event_handler: EventHandler::new("ws://127.0.0.1:8080/events".to_string()),
            connected: false,
            events: Vec::new(),
            max_events: 100,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Connect => {
                tracing::info!("Connecting to event stream...");
                self.connected = true;
                self.event_handler.connect()
            }
            Message::Disconnect => {
                tracing::info!("Disconnecting from event stream...");
                self.connected = false;
                let mut handler = std::mem::replace(
                    &mut self.event_handler,
                    EventHandler::new("ws://127.0.0.1:8080/events".to_string()),
                );
                Task::future(async move {
                    handler.disconnect().await;
                })
            }
            Message::EventReceived(event) => {
                tracing::debug!("Event received: {:?}", event);

                let display = EventDisplay::from_event(&event);
                self.events.insert(0, display);

                // Keep only max_events
                if self.events.len() > self.max_events {
                    self.events.truncate(self.max_events);
                }

                Task::none()
            }
            Message::ClearEvents => {
                self.events.clear();
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let status = if self.connected {
            text("Connected").style(|theme: &Theme| text::Style {
                color: Some(iced::Color::from_rgb(0.0, 1.0, 0.0)),
            })
        } else {
            text("Disconnected").style(|theme: &Theme| text::Style {
                color: Some(iced::Color::from_rgb(1.0, 0.0, 0.0)),
            })
        };

        let connect_btn = if self.connected {
            button(text("Disconnect")).on_press(Message::Disconnect)
        } else {
            button(text("Connect")).on_press(Message::Connect)
        };

        let clear_btn = button(text("Clear Events"))
            .on_press(Message::ClearEvents)
            .padding(10);

        let header = row![
            text("Event Stream Monitor").size(24),
            Space::with_width(Length::Fill),
            status,
            Space::with_width(20),
            connect_btn,
            Space::with_width(10),
            clear_btn,
        ]
        .spacing(10)
        .padding(20);

        let event_count = text(format!("Events: {}", self.events.len())).size(14);

        let events_view = if self.events.is_empty() {
            column![text("No events received yet").size(16)]
                .padding(20)
                .spacing(10)
        } else {
            let event_list: Vec<Element<Message>> = self
                .events
                .iter()
                .map(|event| {
                    let event_row = column![
                        row![
                            text(&event.timestamp).size(12),
                            Space::with_width(20),
                            text(&event.event_type)
                                .size(14)
                                .style(|theme: &Theme| text::Style {
                                    color: Some(theme.palette().primary),
                                }),
                        ]
                        .spacing(10),
                        text(&event.summary).size(14),
                        text(&event.details)
                            .size(12)
                            .style(|theme: &Theme| text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.7)),
                            }),
                    ]
                    .spacing(5)
                    .padding(10);

                    container(event_row)
                        .width(Length::Fill)
                        .padding(5)
                        .style(|theme: &Theme| container::Style {
                            background: Some(theme.palette().background.into()),
                            border: iced::Border {
                                width: 1.0,
                                color: theme.palette().text.scale_alpha(0.2),
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        })
                        .into()
                })
                .collect();

            column(event_list).spacing(10).padding(20)
        };

        let scrollable_events = scrollable(events_view);

        let main_view = column![header, event_count.padding(20), scrollable_events]
            .spacing(0)
            .padding(0);

        container(main_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.connected {
            self.event_handler.subscription(Message::EventReceived)
        } else {
            Subscription::none()
        }
    }
}
