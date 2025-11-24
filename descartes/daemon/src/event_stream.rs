/// WebSocket event streaming for real-time event delivery to clients
///
/// This module provides WebSocket endpoint for streaming events from the event bus
/// to connected clients with filtering and reconnection support.

use crate::errors::{DaemonError, DaemonResult};
use crate::events::{DescartesEvent, EventBus, EventFilter};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Message as WsMessage, Error as WsError},
};
use tracing::{debug, error, info, warn};

/// Message sent from server to client over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Event from the event bus
    Event(DescartesEvent),
    /// Subscription confirmed
    SubscriptionConfirmed {
        subscription_id: String,
    },
    /// Subscription updated
    SubscriptionUpdated {
        subscription_id: String,
    },
    /// Heartbeat/ping
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Error message
    Error {
        code: String,
        message: String,
    },
}

/// Message sent from client to server over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Subscribe to events with filter
    Subscribe {
        filter: Option<EventFilter>,
    },
    /// Update subscription filter
    UpdateFilter {
        filter: EventFilter,
    },
    /// Unsubscribe
    Unsubscribe,
    /// Pong response to ping
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Handle a WebSocket connection for event streaming
pub async fn handle_event_stream(
    stream: tokio::net::TcpStream,
    event_bus: Arc<EventBus>,
) -> DaemonResult<()> {
    info!("New WebSocket connection for event streaming");

    // Accept WebSocket connection
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Failed to accept WebSocket connection: {}", e);
            return Err(DaemonError::ConnectionError(e.to_string()));
        }
    };

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Initially no subscription
    let mut subscription_id: Option<String> = None;
    let mut event_receiver: Option<broadcast::Receiver<DescartesEvent>> = None;
    let mut filter: Option<EventFilter> = None;

    // Heartbeat interval
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            // Handle incoming WebSocket messages from client
            Some(msg) = ws_stream.next() => {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                match handle_client_message(
                                    client_msg,
                                    &event_bus,
                                    &mut subscription_id,
                                    &mut event_receiver,
                                    &mut filter,
                                ).await {
                                    Ok(response) => {
                                        if let Some(resp) = response {
                                            let json = serde_json::to_string(&resp).unwrap();
                                            if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                                                error!("Failed to send response: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = ServerMessage::Error {
                                            code: "HANDLER_ERROR".to_string(),
                                            message: e.to_string(),
                                        };
                                        let json = serde_json::to_string(&error_msg).unwrap();
                                        if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                                            error!("Failed to send error: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid client message: {}", e);
                                let error_msg = ServerMessage::Error {
                                    code: "INVALID_MESSAGE".to_string(),
                                    message: format!("Failed to parse message: {}", e),
                                };
                                let json = serde_json::to_string(&error_msg).unwrap();
                                if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                                    error!("Failed to send error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("Client closed WebSocket connection");
                        break;
                    }
                    Ok(WsMessage::Ping(data)) => {
                        // Respond to ping
                        if let Err(e) = ws_sink.send(WsMessage::Pong(data)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
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

            // Handle incoming events from event bus
            Some(event) = async {
                if let Some(ref mut rx) = event_receiver {
                    rx.recv().await.ok()
                } else {
                    std::future::pending().await
                }
            } => {
                // Apply filter if present
                if let Some(ref f) = filter {
                    if !f.matches(&event) {
                        continue;
                    }
                }

                let server_msg = ServerMessage::Event(event);
                let json = serde_json::to_string(&server_msg).unwrap();

                if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                    error!("Failed to send event: {}", e);
                    break;
                }
            }

            // Send heartbeat
            _ = heartbeat_interval.tick() => {
                let ping_msg = ServerMessage::Ping {
                    timestamp: chrono::Utc::now(),
                };
                let json = serde_json::to_string(&ping_msg).unwrap();

                if let Err(e) = ws_sink.send(WsMessage::Text(json)).await {
                    error!("Failed to send heartbeat: {}", e);
                    break;
                }
            }
        }
    }

    // Cleanup subscription
    if let Some(sub_id) = subscription_id {
        event_bus.unsubscribe(&sub_id).await;
        info!("Unsubscribed from events: {}", sub_id);
    }

    info!("WebSocket connection closed");
    Ok(())
}

/// Handle a client message and return optional server response
async fn handle_client_message(
    message: ClientMessage,
    event_bus: &Arc<EventBus>,
    subscription_id: &mut Option<String>,
    event_receiver: &mut Option<broadcast::Receiver<DescartesEvent>>,
    filter: &mut Option<EventFilter>,
) -> DaemonResult<Option<ServerMessage>> {
    match message {
        ClientMessage::Subscribe { filter: new_filter } => {
            // Unsubscribe from previous subscription if exists
            if let Some(sub_id) = subscription_id.take() {
                event_bus.unsubscribe(&sub_id).await;
            }

            // Subscribe with new filter
            let (sub_id, rx) = event_bus.subscribe(new_filter.clone()).await;
            *subscription_id = Some(sub_id.clone());
            *event_receiver = Some(rx);
            *filter = new_filter;

            info!("Client subscribed to events: {}", sub_id);

            Ok(Some(ServerMessage::SubscriptionConfirmed {
                subscription_id: sub_id,
            }))
        }

        ClientMessage::UpdateFilter { filter: new_filter } => {
            *filter = Some(new_filter);

            if let Some(ref sub_id) = subscription_id {
                info!("Updated filter for subscription: {}", sub_id);
                Ok(Some(ServerMessage::SubscriptionUpdated {
                    subscription_id: sub_id.clone(),
                }))
            } else {
                Err(DaemonError::Other("No active subscription to update".to_string()))
            }
        }

        ClientMessage::Unsubscribe => {
            if let Some(sub_id) = subscription_id.take() {
                event_bus.unsubscribe(&sub_id).await;
                *event_receiver = None;
                *filter = None;
                info!("Client unsubscribed: {}", sub_id);
            }
            Ok(None)
        }

        ClientMessage::Pong { .. } => {
            // Just acknowledge, no response needed
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::SubscriptionConfirmed {
            subscription_id: "test-123".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("SubscriptionConfirmed"));
        assert!(json.contains("test-123"));
    }

    #[test]
    fn test_client_message_deserialization() {
        let json = r#"{"type":"Subscribe","payload":{"filter":null}}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe { filter } => {
                assert!(filter.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_error_message() {
        let msg = ServerMessage::Error {
            code: "TEST_ERROR".to_string(),
            message: "Test error message".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("TEST_ERROR"));
    }
}
