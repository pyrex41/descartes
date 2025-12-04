/// Event handler integration for Iced GUI
///
/// This module provides an Iced-compatible event handler that manages
/// WebSocket subscriptions to the daemon's event stream and converts
/// daemon events into Iced messages.
use descartes_daemon::{
    DescartesEvent, EventClient, EventClientConfig, EventClientState,
    EventFilter,
};
use futures::SinkExt;
use iced::Task;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Event handler for managing daemon event subscriptions in Iced
pub struct EventHandler {
    /// Event client for WebSocket connection
    client: Option<Arc<EventClient>>,
    /// Event receiver channel
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<DescartesEvent>>>>,
    /// Configuration
    config: EventClientConfig,
    /// Connection state
    state: Arc<RwLock<EventClientState>>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(url: String) -> Self {
        let config = EventClientConfig {
            url,
            ..Default::default()
        };

        Self {
            client: None,
            event_rx: Arc::new(RwLock::new(None)),
            config,
            state: Arc::new(RwLock::new(EventClientState::Disconnected)),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new("ws://127.0.0.1:8080/events".to_string())
    }

    /// Create with custom filter
    pub fn with_filter(url: String, filter: EventFilter) -> Self {
        let config = EventClientConfig {
            url,
            filter: Some(filter),
            ..Default::default()
        };

        Self {
            client: None,
            event_rx: Arc::new(RwLock::new(None)),
            config,
            state: Arc::new(RwLock::new(EventClientState::Disconnected)),
        }
    }

    /// Get current connection state
    pub async fn state(&self) -> EventClientState {
        *self.state.read().await
    }

    /// Connect to the daemon event stream
    ///
    /// Returns an Iced Task that connects and starts listening for events
    pub fn connect(&mut self) -> Task<()> {
        let (client, rx) = EventClient::new(self.config.clone());
        let client = Arc::new(client);

        self.client = Some(Arc::clone(&client));
        *self.event_rx.blocking_write() = Some(rx);

        let state = Arc::clone(&self.state);

        // Spawn background task to connect
        Task::future(async move {
            *state.write().await = EventClientState::Connecting;

            if let Err(e) = client.connect().await {
                error!("Failed to connect to event stream: {}", e);
                *state.write().await = EventClientState::Failed;
            } else {
                *state.write().await = EventClientState::Connected;
            }
        })
    }

    /// Subscribe to events and create an Iced subscription
    ///
    /// This creates a continuous stream that yields Iced messages when events arrive
    pub fn subscription<Message, F>(&self, f: F) -> iced::Subscription<Message>
    where
        Message: 'static + Send + Clone,
        F: Fn(DescartesEvent) -> Message + Send + Sync + 'static + Clone,
    {
        let event_rx = Arc::clone(&self.event_rx);
        let state = Arc::clone(&self.state);

        iced::Subscription::run_with_id(
            "event_stream",
            iced::stream::channel(100, move |mut output| {
                let event_rx = Arc::clone(&event_rx);
                let state = Arc::clone(&state);
                let f = f.clone();

                async move {
                    loop {
                        // Wait for event receiver to be available
                        let mut rx_guard = event_rx.write().await;
                        if let Some(rx) = rx_guard.as_mut() {
                            // Wait for next event
                            match rx.recv().await {
                                Some(event) => {
                                    debug!("Received event: {:?}", event);
                                    let msg = f(event);
                                    if output.send(msg).await.is_err() {
                                        error!("Failed to send event to output channel");
                                        break;
                                    }
                                }
                                None => {
                                    warn!("Event channel closed");
                                    *state.write().await = EventClientState::Disconnected;
                                    break;
                                }
                            }
                        } else {
                            // Wait for connection
                            drop(rx_guard);
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }

                    std::future::pending::<()>().await;
                    unreachable!()
                }
            }),
        )
    }

    /// Disconnect from the event stream
    pub async fn disconnect(&mut self) {
        if let Some(client) = &self.client {
            client.disconnect().await;
        }
        self.client = None;
        *self.event_rx.write().await = None;
        *self.state.write().await = EventClientState::Disconnected;
        info!("Event handler disconnected");
    }
}

/// Event handler statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventHandlerStats {
    pub total_events_received: u64,
    pub events_by_type: std::collections::HashMap<String, u64>,
    pub connection_uptime: Option<chrono::Duration>,
    pub last_event_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Event handler builder for more complex configurations
pub struct EventHandlerBuilder {
    url: String,
    filter: Option<EventFilter>,
}

impl EventHandlerBuilder {
    pub fn new() -> Self {
        Self {
            url: "ws://127.0.0.1:8080/events".to_string(),
            filter: None,
        }
    }

    pub fn url(mut self, url: String) -> Self {
        self.url = url;
        self
    }

    pub fn filter(mut self, filter: EventFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn build(self) -> EventHandler {
        if let Some(filter) = self.filter {
            EventHandler::with_filter(self.url, filter)
        } else {
            EventHandler::new(self.url)
        }
    }
}

impl Default for EventHandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_creation() {
        let handler = EventHandler::default();
        assert!(handler.client.is_none());
    }

    #[test]
    fn test_event_handler_builder() {
        let handler = EventHandlerBuilder::new()
            .url("ws://localhost:9090/events".to_string())
            .build();

        assert_eq!(handler.config.url, "ws://localhost:9090/events");
    }

    #[tokio::test]
    async fn test_initial_state() {
        let handler = EventHandler::default();
        assert_eq!(handler.state().await, EventClientState::Disconnected);
    }
}
