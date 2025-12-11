//! ZMQ SUB client for receiving stream chunks from daemon
//!
//! Subscribes to chat session topics on the daemon's ZMQ PUB socket
//! and streams chunks to the GUI for real-time display.

use descartes_core::StreamChunk;
use futures::SinkExt;
use tokio::sync::mpsc;
use uuid::Uuid;
use zeromq::{Socket, SocketRecv, SubSocket};

/// Subscribe to a chat session's stream
///
/// Connects to the daemon's ZMQ PUB socket and subscribes to the topic
/// for the given session ID. Returns an unbounded receiver that yields
/// StreamChunks as they arrive.
///
/// # Arguments
/// * `pub_endpoint` - The ZMQ PUB endpoint (e.g., "tcp://127.0.0.1:19480")
/// * `session_id` - The session UUID to subscribe to
///
/// # Returns
/// A receiver that yields StreamChunks, or an error if connection fails
pub async fn subscribe_to_session(
    pub_endpoint: &str,
    session_id: Uuid,
) -> Result<mpsc::UnboundedReceiver<StreamChunk>, String> {
    let topic = format!("chat/{}", session_id);

    let mut socket = SubSocket::new();
    socket
        .connect(pub_endpoint)
        .await
        .map_err(|e| format!("Failed to connect to ZMQ PUB socket: {}", e))?;

    socket
        .subscribe(&topic)
        .await
        .map_err(|e| format!("Failed to subscribe to topic '{}': {}", topic, e))?;

    tracing::info!(
        "Subscribed to ZMQ topic '{}' at {}",
        topic,
        pub_endpoint
    );

    let (tx, rx) = mpsc::unbounded_channel();

    // Spawn task to receive messages and forward to channel
    tokio::spawn(async move {
        loop {
            match socket.recv().await {
                Ok(msg) => {
                    // ZMQ message format from daemon: "topic payload" in a single frame
                    // The topic is "chat/{session_id}" and payload is JSON
                    let frames: Vec<_> = msg.into_vec();

                    let payload_bytes = if frames.len() >= 2 {
                        // Multipart format: [topic, payload]
                        Some(frames[1].as_ref())
                    } else if frames.len() == 1 {
                        // Single frame format: "topic payload" - split by first space
                        let data = frames[0].as_ref();
                        if let Some(space_pos) = data.iter().position(|&b| b == b' ') {
                            Some(&data[space_pos + 1..])
                        } else {
                            tracing::warn!("Single-frame message has no space separator");
                            None
                        }
                    } else {
                        tracing::warn!("Received empty ZMQ message");
                        None
                    };

                    if let Some(payload) = payload_bytes {
                        match serde_json::from_slice::<StreamChunk>(payload) {
                            Ok(chunk) => {
                                let is_complete = matches!(chunk, StreamChunk::Complete { .. });
                                if tx.send(chunk).is_err() {
                                    tracing::debug!("Subscriber channel closed");
                                    break;
                                }
                                if is_complete {
                                    tracing::debug!("Session completed, closing subscriber");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse stream chunk: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("ZMQ recv error: {}", e);
                    // Send error chunk before breaking
                    let _ = tx.send(StreamChunk::Error {
                        message: format!("Connection error: {}", e),
                    });
                    break;
                }
            }
        }

        tracing::info!("ZMQ subscriber task ended");
    });

    Ok(rx)
}

/// Create an Iced subscription for a chat session
///
/// This creates a subscription that connects to the daemon's ZMQ PUB socket
/// and yields ChatMessage variants for each stream chunk received.
pub fn chat_subscription<M: Clone + Send + 'static>(
    endpoint: String,
    session_id: Uuid,
    on_chunk: impl Fn(StreamChunk) -> M + Send + Sync + 'static,
    on_error: impl Fn(String) -> M + Send + Sync + 'static,
) -> iced::Subscription<M> {
    iced::Subscription::run_with_id(
        format!("zmq_chat_{}", session_id),
        iced::stream::channel(100, move |mut output| {
            let endpoint = endpoint.clone();
            let on_chunk = std::sync::Arc::new(on_chunk);
            let on_error = std::sync::Arc::new(on_error);

            async move {
                match subscribe_to_session(&endpoint, session_id).await {
                    Ok(mut rx) => {
                        while let Some(chunk) = rx.recv().await {
                            let msg = on_chunk(chunk);
                            if output.send(msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let msg = on_error(e);
                        let _ = output.send(msg).await;
                    }
                }

                // Keep subscription alive (will be cancelled when GUI unsubscribes)
                futures::future::pending::<()>().await
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_format() {
        let session_id = Uuid::new_v4();
        let topic = format!("chat/{}", session_id);
        assert!(topic.starts_with("chat/"));
        assert_eq!(topic.len(), 5 + 36); // "chat/" + UUID
    }
}
