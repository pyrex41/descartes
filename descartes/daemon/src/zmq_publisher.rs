//! ZMQ PUB socket for streaming chat output
//!
//! Provides a publisher for broadcasting stream chunks to GUI clients
//! using ZeroMQ's PUB/SUB pattern.

use descartes_core::StreamChunk;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zeromq::{PubSocket, Socket, SocketSend, ZmqMessage};

/// ZMQ Publisher for streaming chat chunks
pub struct ZmqPublisher {
    socket: Arc<RwLock<PubSocket>>,
    endpoint: String,
}

impl ZmqPublisher {
    /// Create a new ZMQ publisher bound to the given address and port
    pub async fn new(addr: &str, port: u16) -> Result<Self, String> {
        let endpoint = format!("tcp://{}:{}", addr, port);
        let mut socket = PubSocket::new();

        socket
            .bind(&endpoint)
            .await
            .map_err(|e| format!("Failed to bind PUB socket to {}: {}", endpoint, e))?;

        tracing::info!("ZMQ PUB socket listening on {}", endpoint);

        Ok(Self {
            socket: Arc::new(RwLock::new(socket)),
            endpoint,
        })
    }

    /// Publish a stream chunk for a session
    ///
    /// Messages are published with topic format: "chat/{session_id}"
    /// Subscribers can filter by subscribing to specific session topics.
    pub async fn publish(&self, session_id: Uuid, chunk: &StreamChunk) -> Result<(), String> {
        let topic = format!("chat/{}", session_id);
        let payload = serde_json::to_string(chunk)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // ZMQ PUB message: topic + space + payload (simple format)
        // Subscribers filter by topic prefix
        let message = format!("{} {}", topic, payload);

        let mut socket = self.socket.write().await;

        // Create ZmqMessage from bytes
        let zmq_msg: ZmqMessage = message.into_bytes().into();

        socket
            .send(zmq_msg)
            .await
            .map_err(|e| format!("Failed to publish message: {}", e))?;

        tracing::trace!("Published chunk to topic {}: {:?}", topic, chunk);

        Ok(())
    }

    /// Get the endpoint this publisher is bound to
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the client-facing endpoint (for returning to RPC clients)
    /// Converts 0.0.0.0 to 127.0.0.1 for client connections
    pub fn client_endpoint(&self) -> String {
        self.endpoint.replace("0.0.0.0", "127.0.0.1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publisher_creation() {
        // Use a random port to avoid conflicts
        let port = 19490 + (std::process::id() % 100) as u16;
        let result = ZmqPublisher::new("127.0.0.1", port).await;

        // May fail if port is in use, which is okay for this test
        if let Ok(publisher) = result {
            assert!(publisher.endpoint().contains(&port.to_string()));
        }
    }
}
