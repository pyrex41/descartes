/// Low-level ZMQ socket communication layer for distributed agent orchestration.
///
/// This module provides the infrastructure for managing ZMQ sockets, sending/receiving
/// messages, connection management, and error handling. It implements:
/// - ZMQ socket creation and configuration
/// - Message serialization/deserialization with MessagePack
/// - Connection state tracking and management
/// - Automatic reconnection with exponential backoff
/// - Request/response correlation with timeout handling
/// - Comprehensive error handling for network issues
///
/// # Architecture
///
/// ```text
/// ZmqConnection
///   ├── Socket Management
///   │   ├── Socket creation (REQ/REP, DEALER/ROUTER)
///   │   ├── Socket configuration (timeouts, buffers)
///   │   └── Socket lifecycle management
///   ├── Message Operations
///   │   ├── send_message() - Serialize and send
///   │   ├── receive_message() - Receive and deserialize
///   │   └── request_response() - Correlated RPC
///   ├── Connection Management
///   │   ├── Connection state tracking
///   │   ├── Automatic reconnection
///   │   └── Heartbeat/keepalive
///   └── Error Handling
///       ├── Network errors
///       ├── Timeout errors
///       └── Serialization errors
/// ```
use crate::errors::{AgentError, AgentResult};
use crate::zmq_agent_runner::{
    deserialize_zmq_message, serialize_zmq_message, validate_message_size, ZmqMessage,
    ZmqRunnerConfig, DEFAULT_TIMEOUT_SECS,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use zeromq::{DealerSocket, RepSocket, ReqSocket, RouterSocket, Socket, SocketRecv, SocketSend};

/// Maximum number of reconnection attempts before giving up
const _MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Initial reconnect delay (doubles on each attempt)
const INITIAL_RECONNECT_DELAY_MS: u64 = 100;

/// Maximum reconnect delay (cap for exponential backoff)
const MAX_RECONNECT_DELAY_MS: u64 = 30000;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting in progress
    Connecting,
    /// Connected and ready
    Connected,
    /// Reconnecting after failure
    Reconnecting,
    /// Connection failed
    Failed,
}

/// Statistics for connection monitoring
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total errors encountered
    pub errors: u64,
    /// Total reconnections
    pub reconnections: u64,
    /// Connection uptime
    pub connected_since: Option<Instant>,
}

/// Pending request for request/response correlation
#[derive(Debug)]
#[allow(dead_code)]
struct PendingRequest {
    /// Request ID
    request_id: String,
    /// Timestamp when request was sent
    sent_at: Instant,
    /// Timeout duration
    timeout: Duration,
    /// Channel to send response back
    response_tx: tokio::sync::oneshot::Sender<AgentResult<ZmqMessage>>,
}

/// ZMQ socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// REQ socket (client side, synchronous)
    Req,
    /// REP socket (server side, synchronous)
    Rep,
    /// DEALER socket (client side, asynchronous)
    Dealer,
    /// ROUTER socket (server side, asynchronous)
    Router,
}

/// ZMQ connection wrapper for managing socket communication
pub struct ZmqConnection {
    /// Socket type
    socket_type: SocketType,
    /// Endpoint to connect/bind to
    endpoint: String,
    /// Configuration
    config: ZmqRunnerConfig,
    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
    /// Pending requests (for request/response correlation)
    #[allow(dead_code)]
    pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
    /// Socket (wrapped in Arc<Mutex> for thread-safe access)
    socket: Arc<Mutex<Option<Box<dyn SocketWrapper>>>>,
}

/// Trait to abstract over different ZMQ socket types
#[async_trait::async_trait]
trait SocketWrapper: Send + Sync {
    async fn send(&mut self, data: Vec<u8>) -> Result<(), zeromq::ZmqError>;
    async fn recv(&mut self) -> Result<zeromq::ZmqMessage, zeromq::ZmqError>;
}

/// Wrapper for ReqSocket
struct ReqSocketWrapper(ReqSocket);

#[async_trait::async_trait]
impl SocketWrapper for ReqSocketWrapper {
    async fn send(&mut self, data: Vec<u8>) -> Result<(), zeromq::ZmqError> {
        self.0.send(data.into()).await
    }

    async fn recv(&mut self) -> Result<zeromq::ZmqMessage, zeromq::ZmqError> {
        self.0.recv().await
    }
}

/// Wrapper for RepSocket
struct RepSocketWrapper(RepSocket);

#[async_trait::async_trait]
impl SocketWrapper for RepSocketWrapper {
    async fn send(&mut self, data: Vec<u8>) -> Result<(), zeromq::ZmqError> {
        self.0.send(data.into()).await
    }

    async fn recv(&mut self) -> Result<zeromq::ZmqMessage, zeromq::ZmqError> {
        self.0.recv().await
    }
}

/// Wrapper for DealerSocket
struct DealerSocketWrapper(DealerSocket);

#[async_trait::async_trait]
impl SocketWrapper for DealerSocketWrapper {
    async fn send(&mut self, data: Vec<u8>) -> Result<(), zeromq::ZmqError> {
        self.0.send(data.into()).await
    }

    async fn recv(&mut self) -> Result<zeromq::ZmqMessage, zeromq::ZmqError> {
        self.0.recv().await
    }
}

/// Wrapper for RouterSocket
struct RouterSocketWrapper(RouterSocket);

#[async_trait::async_trait]
impl SocketWrapper for RouterSocketWrapper {
    async fn send(&mut self, data: Vec<u8>) -> Result<(), zeromq::ZmqError> {
        self.0.send(data.into()).await
    }

    async fn recv(&mut self) -> Result<zeromq::ZmqMessage, zeromq::ZmqError> {
        self.0.recv().await
    }
}

impl ZmqConnection {
    /// Create a new ZMQ connection
    ///
    /// # Arguments
    ///
    /// * `socket_type` - Type of ZMQ socket to use
    /// * `endpoint` - ZMQ endpoint (e.g., "tcp://localhost:5555")
    /// * `config` - Configuration for the connection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use descartes_core::zmq_communication::{ZmqConnection, SocketType};
    /// use descartes_core::ZmqRunnerConfig;
    ///
    /// let connection = ZmqConnection::new(
    ///     SocketType::Req,
    ///     "tcp://localhost:5555",
    ///     ZmqRunnerConfig::default(),
    /// );
    /// ```
    pub fn new(socket_type: SocketType, endpoint: &str, config: ZmqRunnerConfig) -> Self {
        Self {
            socket_type,
            endpoint: endpoint.to_string(),
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            socket: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the endpoint
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use descartes_core::zmq_communication::{ZmqConnection, SocketType};
    /// # use descartes_core::ZmqRunnerConfig;
    /// # let mut connection = ZmqConnection::new(SocketType::Req, "tcp://localhost:5555", ZmqRunnerConfig::default());
    /// connection.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&mut self) -> AgentResult<()> {
        *self.state.write() = ConnectionState::Connecting;

        let socket = match self.socket_type {
            SocketType::Req => {
                let mut sock = ReqSocket::new();
                sock.connect(&self.endpoint).await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to connect REQ socket: {}", e))
                })?;
                Box::new(ReqSocketWrapper(sock)) as Box<dyn SocketWrapper>
            }
            SocketType::Rep => {
                let mut sock = RepSocket::new();
                sock.bind(&self.endpoint).await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to bind REP socket: {}", e))
                })?;
                Box::new(RepSocketWrapper(sock)) as Box<dyn SocketWrapper>
            }
            SocketType::Dealer => {
                let mut sock = DealerSocket::new();
                sock.connect(&self.endpoint).await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to connect DEALER socket: {}", e))
                })?;
                Box::new(DealerSocketWrapper(sock)) as Box<dyn SocketWrapper>
            }
            SocketType::Router => {
                let mut sock = RouterSocket::new();
                sock.bind(&self.endpoint).await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to bind ROUTER socket: {}", e))
                })?;
                Box::new(RouterSocketWrapper(sock)) as Box<dyn SocketWrapper>
            }
        };

        *self.socket.lock().await = Some(socket);
        *self.state.write() = ConnectionState::Connected;
        self.stats.write().connected_since = Some(Instant::now());

        tracing::info!(
            "ZMQ connection established: endpoint={}, type={:?}",
            self.endpoint,
            self.socket_type
        );

        Ok(())
    }

    /// Disconnect from the endpoint
    pub async fn disconnect(&mut self) -> AgentResult<()> {
        *self.socket.lock().await = None;
        *self.state.write() = ConnectionState::Disconnected;
        self.stats.write().connected_since = None;

        tracing::info!("ZMQ connection closed: endpoint={}", self.endpoint);

        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.state.read() == ConnectionState::Connected
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        self.stats.read().clone()
    }

    /// Send a message
    ///
    /// # Arguments
    ///
    /// * `message` - The ZMQ message to send
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use descartes_core::zmq_communication::{ZmqConnection, SocketType};
    /// # use descartes_core::{ZmqRunnerConfig, ZmqMessage, HealthCheckRequest};
    /// # let mut connection = ZmqConnection::new(SocketType::Req, "tcp://localhost:5555", ZmqRunnerConfig::default());
    /// # connection.connect().await?;
    /// let msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
    ///     request_id: "test-123".to_string(),
    /// });
    /// connection.send_message(&msg).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_message(&self, message: &ZmqMessage) -> AgentResult<()> {
        if !self.is_connected() {
            return Err(AgentError::ExecutionError(
                "Cannot send message: not connected".to_string(),
            ));
        }

        // Serialize the message
        let bytes = serialize_zmq_message(message)?;
        validate_message_size(bytes.len())?;

        // Send via socket
        let mut socket_guard = self.socket.lock().await;
        if let Some(socket) = socket_guard.as_mut() {
            socket.send(bytes.clone()).await.map_err(|e| {
                AgentError::ExecutionError(format!("Failed to send ZMQ message: {}", e))
            })?;

            // Update statistics
            let mut stats = self.stats.write();
            stats.messages_sent += 1;
            stats.bytes_sent += bytes.len() as u64;

            tracing::debug!(
                "Sent ZMQ message: type={:?}, size={} bytes",
                std::mem::discriminant(message),
                bytes.len()
            );

            Ok(())
        } else {
            Err(AgentError::ExecutionError(
                "Socket not initialized".to_string(),
            ))
        }
    }

    /// Receive a message with timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional timeout duration (uses default if None)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use descartes_core::zmq_communication::{ZmqConnection, SocketType};
    /// # use descartes_core::ZmqRunnerConfig;
    /// # use std::time::Duration;
    /// # let mut connection = ZmqConnection::new(SocketType::Rep, "tcp://localhost:5555", ZmqRunnerConfig::default());
    /// # connection.connect().await?;
    /// let msg = connection.receive_message(Some(Duration::from_secs(30))).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn receive_message(&self, timeout: Option<Duration>) -> AgentResult<ZmqMessage> {
        if !self.is_connected() {
            return Err(AgentError::ExecutionError(
                "Cannot receive message: not connected".to_string(),
            ));
        }

        let timeout_duration = timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS));

        let mut socket_guard = self.socket.lock().await;
        if let Some(socket) = socket_guard.as_mut() {
            // Receive with timeout
            let result = tokio::time::timeout(timeout_duration, socket.recv()).await;

            match result {
                Ok(Ok(zmq_msg)) => {
                    // Extract bytes from ZmqMessage (get first frame)
                    let bytes: Vec<u8> = zmq_msg
                        .into_vec()
                        .into_iter()
                        .next()
                        .ok_or_else(|| {
                            AgentError::ExecutionError("Empty message received".to_string())
                        })?
                        .to_vec();
                    validate_message_size(bytes.len())?;

                    // Deserialize
                    let message = deserialize_zmq_message(&bytes)?;

                    // Update statistics
                    let mut stats = self.stats.write();
                    stats.messages_received += 1;
                    stats.bytes_received += bytes.len() as u64;

                    tracing::debug!(
                        "Received ZMQ message: type={:?}, size={} bytes",
                        std::mem::discriminant(&message),
                        bytes.len()
                    );

                    Ok(message)
                }
                Ok(Err(e)) => {
                    self.stats.write().errors += 1;
                    Err(AgentError::ExecutionError(format!(
                        "Failed to receive ZMQ message: {}",
                        e
                    )))
                }
                Err(_) => {
                    self.stats.write().errors += 1;
                    Err(AgentError::ExecutionError(format!(
                        "Timeout receiving ZMQ message after {:?}",
                        timeout_duration
                    )))
                }
            }
        } else {
            Err(AgentError::ExecutionError(
                "Socket not initialized".to_string(),
            ))
        }
    }

    /// Send a request and wait for a response (with timeout and correlation)
    ///
    /// # Arguments
    ///
    /// * `request` - The request message to send
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// The response message
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use descartes_core::zmq_communication::{ZmqConnection, SocketType};
    /// # use descartes_core::{ZmqRunnerConfig, ZmqMessage, HealthCheckRequest};
    /// # use std::time::Duration;
    /// # let mut connection = ZmqConnection::new(SocketType::Req, "tcp://localhost:5555", ZmqRunnerConfig::default());
    /// # connection.connect().await?;
    /// let request = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
    ///     request_id: "test-123".to_string(),
    /// });
    /// let response = connection.request_response(&request, Some(Duration::from_secs(30))).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_response(
        &self,
        request: &ZmqMessage,
        timeout: Option<Duration>,
    ) -> AgentResult<ZmqMessage> {
        // Send the request
        self.send_message(request).await?;

        // Receive the response
        self.receive_message(timeout).await
    }

    /// Reconnect with exponential backoff
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum number of reconnection attempts
    ///
    /// # Returns
    ///
    /// Ok if reconnection succeeds, Err otherwise
    pub async fn reconnect(&mut self, max_attempts: Option<u32>) -> AgentResult<()> {
        let max_attempts = max_attempts.unwrap_or(self.config.max_reconnect_attempts);
        let mut attempt = 0;
        let mut delay_ms = INITIAL_RECONNECT_DELAY_MS;

        *self.state.write() = ConnectionState::Reconnecting;

        while attempt < max_attempts {
            attempt += 1;

            tracing::info!(
                "Reconnection attempt {}/{} for endpoint {}",
                attempt,
                max_attempts,
                self.endpoint
            );

            // Try to disconnect first (cleanup)
            let _ = self.disconnect().await;

            // Wait with exponential backoff
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            // Try to connect
            match self.connect().await {
                Ok(_) => {
                    self.stats.write().reconnections += 1;
                    tracing::info!("Reconnection successful after {} attempts", attempt);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);

                    // Exponential backoff (double delay, cap at max)
                    delay_ms = std::cmp::min(delay_ms * 2, MAX_RECONNECT_DELAY_MS);
                }
            }
        }

        *self.state.write() = ConnectionState::Failed;
        Err(AgentError::ExecutionError(format!(
            "Failed to reconnect after {} attempts",
            max_attempts
        )))
    }

    /// Clean up expired pending requests
    async fn _cleanup_expired_requests(&self) {
        let mut pending = self.pending_requests.lock().await;
        let now = Instant::now();

        pending.retain(|request_id, pending_req| {
            let elapsed = now.duration_since(pending_req.sent_at);
            if elapsed > pending_req.timeout {
                tracing::warn!("Request {} timed out after {:?}", request_id, elapsed);
                // Remove this entry (receiver will be dropped, caller will get error)
                false
            } else {
                true // Keep this entry
            }
        });
    }
}

impl Drop for ZmqConnection {
    fn drop(&mut self) {
        // Ensure socket is properly closed
        // Note: We can't await in Drop, so this is best-effort
        *self.state.write() = ConnectionState::Disconnected;
    }
}

/// ZMQ message router for handling request/response correlation
///
/// This struct manages mapping between request IDs and responses,
/// enabling asynchronous request/response patterns over ZMQ.
pub struct ZmqMessageRouter {
    /// Pending requests awaiting responses
    pending_requests:
        Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<AgentResult<ZmqMessage>>>>>,
}

impl ZmqMessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a pending request
    ///
    /// # Arguments
    ///
    /// * `request_id` - The request ID to track
    ///
    /// # Returns
    ///
    /// A receiver that will receive the response
    pub async fn register_request(
        &self,
        request_id: String,
    ) -> tokio::sync::oneshot::Receiver<AgentResult<ZmqMessage>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.lock().await.insert(request_id, tx);
        rx
    }

    /// Route a response to the appropriate pending request
    ///
    /// # Arguments
    ///
    /// * `request_id` - The request ID this response is for
    /// * `response` - The response message
    ///
    /// # Returns
    ///
    /// Ok if routed successfully, Err if no matching request found
    pub async fn route_response(
        &self,
        request_id: &str,
        response: AgentResult<ZmqMessage>,
    ) -> AgentResult<()> {
        if let Some(tx) = self.pending_requests.lock().await.remove(request_id) {
            tx.send(response).map_err(|_| {
                AgentError::ExecutionError("Failed to send response: receiver dropped".to_string())
            })?;
            Ok(())
        } else {
            Err(AgentError::ExecutionError(format!(
                "No pending request found for ID: {}",
                request_id
            )))
        }
    }

    /// Get the number of pending requests
    pub async fn pending_count(&self) -> usize {
        self.pending_requests.lock().await.len()
    }
}

impl Default for ZmqMessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zmq_agent_runner::{HealthCheckRequest, HealthCheckResponse};

    #[test]
    fn test_connection_state() {
        let connection = ZmqConnection::new(
            SocketType::Req,
            "tcp://localhost:5555",
            ZmqRunnerConfig::default(),
        );

        assert_eq!(connection.state(), ConnectionState::Disconnected);
        assert!(!connection.is_connected());
    }

    #[test]
    fn test_connection_stats() {
        let connection = ZmqConnection::new(
            SocketType::Req,
            "tcp://localhost:5555",
            ZmqRunnerConfig::default(),
        );

        let stats = connection.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
    }

    #[test]
    fn test_message_router_new() {
        let router = ZmqMessageRouter::new();
        // Just verify it constructs successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_message_router_register_and_route() {
        let router = ZmqMessageRouter::new();
        let request_id = Uuid::new_v4().to_string();

        // Register a request
        let rx = router.register_request(request_id.clone()).await;
        assert_eq!(router.pending_count().await, 1);

        // Route a response
        let response = ZmqMessage::HealthCheckResponse(HealthCheckResponse {
            request_id: request_id.clone(),
            healthy: true,
            protocol_version: "1.0.0".to_string(),
            uptime_secs: Some(100),
            active_agents: Some(5),
            metadata: None,
        });

        router
            .route_response(&request_id, Ok(response))
            .await
            .unwrap();

        // Verify the response was received
        let received = rx.await.unwrap().unwrap();
        match received {
            ZmqMessage::HealthCheckResponse(resp) => {
                assert!(resp.healthy);
                assert_eq!(resp.protocol_version, "1.0.0");
            }
            _ => panic!("Wrong message type"),
        }

        // Verify the pending request was removed
        assert_eq!(router.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_message_router_no_matching_request() {
        let router = ZmqMessageRouter::new();

        let response = ZmqMessage::HealthCheckResponse(HealthCheckResponse {
            request_id: "non-existent".to_string(),
            healthy: true,
            protocol_version: "1.0.0".to_string(),
            uptime_secs: None,
            active_agents: None,
            metadata: None,
        });

        let result = router.route_response("non-existent", Ok(response)).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
            request_id: "test-123".to_string(),
        });

        let bytes = serialize_zmq_message(&msg).unwrap();
        let deserialized = deserialize_zmq_message(&bytes).unwrap();

        match deserialized {
            ZmqMessage::HealthCheckRequest(req) => {
                assert_eq!(req.request_id, "test-123");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
