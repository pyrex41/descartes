/// Event client for subscribing to daemon events via WebSocket
///
/// This module provides a client for connecting to the daemon's event stream
/// and receiving real-time events with automatic reconnection support.

use crate::errors::{DaemonError, DaemonResult};
use crate::events::{DescartesEvent, EventFilter};
use crate::event_stream::{ClientMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message as WsMessage, Error as WsError},
};
use tracing::{debug, error, info, warn};

/// Configuration for event client
#[derive(Debug, Clone)]
pub struct EventClientConfig {
    /// WebSocket URL (e.g., "ws://127.0.0.1:8080/events")
    pub url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Reconnection attempts (-1 for infinite)
    pub max_reconnect_attempts: i32,
    /// Delay between reconnection attempts
    pub reconnect_delay: Duration,
    /// Event filter
    pub filter: Option<EventFilter>,
}

impl Default for EventClientConfig {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:8080/events".to_string(),
            connect_timeout: Duration::from_secs(10),
            max_reconnect_attempts: -1, // Infinite reconnects
            reconnect_delay: Duration::from_secs(5),
            filter: None,
        }
    }
}

/// Event client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventClientState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Event client for subscribing to daemon events
pub struct EventClient {
    config: EventClientConfig,
    state: Arc<RwLock<EventClientState>>,
    event_tx: mpsc::UnboundedSender<DescartesEvent>,
    subscription_id: Arc<RwLock<Option<String>>>,
}

impl EventClient {
    /// Create a new event client
    pub fn new(config: EventClientConfig) -> (Self, mpsc::UnboundedReceiver<DescartesEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let client = Self {
            config,
            state: Arc::new(RwLock::new(EventClientState::Disconnected)),
            event_tx,
            subscription_id: Arc::new(RwLock::new(None)),
        };

        (client, event_rx)
    }

    /// Create a new event client with default configuration
    pub fn default() -> (Self, mpsc::UnboundedReceiver<DescartesEvent>) {
        Self::new(EventClientConfig::default())
    }

    /// Get current client state
    pub async fn state(&self) -> EventClientState {
        *self.state.read().await
    }

    /// Get subscription ID if subscribed
    pub async fn subscription_id(&self) -> Option<String> {
        self.subscription_id.read().await.clone()
    }

    /// Connect to the event stream and start receiving events
    pub async fn connect(&self) -> DaemonResult<()> {
        let mut reconnect_count = 0;

        loop {
            // Update state to connecting
            *self.state.write().await = EventClientState::Connecting;

            match self.try_connect().await {
                Ok(_) => {
                    info!("Event client connected successfully");
                    reconnect_count = 0;
                }
                Err(e) => {
                    error!("Failed to connect to event stream: {}", e);

                    // Check if we should retry
                    if self.config.max_reconnect_attempts >= 0
                        && reconnect_count >= self.config.max_reconnect_attempts
                    {
                        error!(
                            "Maximum reconnection attempts ({}) exceeded",
                            self.config.max_reconnect_attempts
                        );
                        *self.state.write().await = EventClientState::Failed;
                        return Err(e);
                    }

                    reconnect_count += 1;
                    *self.state.write().await = EventClientState::Reconnecting;

                    warn!(
                        "Reconnecting in {:?} (attempt {})...",
                        self.config.reconnect_delay, reconnect_count
                    );
                    tokio::time::sleep(self.config.reconnect_delay).await;
                }
            }
        }
    }

    /// Try to connect once (internal method)
    async fn try_connect(&self) -> DaemonResult<()> {
        info!("Connecting to event stream at {}", self.config.url);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&self.config.url)
            .await
            .map_err(|e| DaemonError::ConnectionError(e.to_string()))?;

        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        *self.state.write().await = EventClientState::Connected;

        // Subscribe to events
        let subscribe_msg = ClientMessage::Subscribe {
            filter: self.config.filter.clone(),
        };
        let json = serde_json::to_string(&subscribe_msg)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?;

        ws_sink
            .send(WsMessage::Text(json))
            .await
            .map_err(|e| DaemonError::ConnectionError(e.to_string()))?;

        info!("Subscribed to events");

        // Message processing loop
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    debug!("Received message: {}", text);

                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(server_msg) => {
                            match server_msg {
                                ServerMessage::Event(event) => {
                                    // Forward event to application
                                    if let Err(e) = self.event_tx.send(event) {
                                        error!("Failed to forward event: {}", e);
                                        break;
                                    }
                                }
                                ServerMessage::SubscriptionConfirmed { subscription_id } => {
                                    info!("Subscription confirmed: {}", subscription_id);
                                    *self.subscription_id.write().await = Some(subscription_id);
                                }
                                ServerMessage::SubscriptionUpdated { subscription_id } => {
                                    info!("Subscription updated: {}", subscription_id);
                                }
                                ServerMessage::Ping { timestamp } => {
                                    debug!("Received ping at {:?}", timestamp);
                                    // Send pong
                                    let pong = ClientMessage::Pong { timestamp };
                                    let json = serde_json::to_string(&pong).unwrap();
                                    if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                                        error!("Failed to send pong: {}", e);
                                        break;
                                    }
                                }
                                ServerMessage::Error { code, message } => {
                                    error!("Server error {}: {}", code, message);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse server message: {}", e);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    info!("Server closed WebSocket connection");
                    break;
                }
                Ok(WsMessage::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Connection closed or error occurred
        *self.state.write().await = EventClientState::Disconnected;
        *self.subscription_id.write().await = None;

        Ok(())
    }

    /// Update the event filter
    pub async fn update_filter(&self, filter: EventFilter) -> DaemonResult<()> {
        // This would require maintaining a WebSocket connection handle
        // For now, just update the config for next connection
        // TODO: Implement live filter updates
        warn!("Live filter updates not yet implemented");
        Ok(())
    }

    /// Disconnect from the event stream
    pub async fn disconnect(&self) {
        *self.state.write().await = EventClientState::Disconnected;
        info!("Event client disconnected");
    }
}

/// Builder for EventClient
pub struct EventClientBuilder {
    config: EventClientConfig,
}

impl EventClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: EventClientConfig::default(),
        }
    }

    /// Set the WebSocket URL
    pub fn url(mut self, url: String) -> Self {
        self.config.url = url;
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set maximum reconnection attempts
    pub fn max_reconnect_attempts(mut self, max: i32) -> Self {
        self.config.max_reconnect_attempts = max;
        self
    }

    /// Set reconnection delay
    pub fn reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.reconnect_delay = delay;
        self
    }

    /// Set event filter
    pub fn filter(mut self, filter: EventFilter) -> Self {
        self.config.filter = Some(filter);
        self
    }

    /// Build the event client
    pub fn build(self) -> (EventClient, mpsc::UnboundedReceiver<DescartesEvent>) {
        EventClient::new(self.config)
    }
}

impl Default for EventClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = EventClientConfig::default();
        assert_eq!(config.url, "ws://127.0.0.1:8080/events");
        assert_eq!(config.max_reconnect_attempts, -1);
    }

    #[test]
    fn test_builder() {
        let (client, _rx) = EventClientBuilder::new()
            .url("ws://localhost:9090/events".to_string())
            .max_reconnect_attempts(5)
            .reconnect_delay(Duration::from_secs(10))
            .build();

        assert_eq!(client.config.url, "ws://localhost:9090/events");
        assert_eq!(client.config.max_reconnect_attempts, 5);
        assert_eq!(client.config.reconnect_delay, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_initial_state() {
        let (client, _rx) = EventClient::default();
        assert_eq!(client.state().await, EventClientState::Disconnected);
        assert_eq!(client.subscription_id().await, None);
    }
}
