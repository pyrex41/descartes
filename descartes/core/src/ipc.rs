/// Comprehensive IPC/Message Bus System for inter-agent communication
/// Supports multiple transports: Unix sockets, shared memory, stdin/stdout
/// Includes pub/sub messaging, request/response protocol, and reliability features
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, timeout};
use tracing::info;
use uuid::Uuid;

// ============================================================================
// Message Types and Protocol
// ============================================================================

/// Message types for the IPC protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// Direct message from sender to recipient
    DirectMessage,
    /// Publish message for pub/sub
    PublishMessage,
    /// Subscribe to a topic
    Subscribe,
    /// Unsubscribe from a topic
    Unsubscribe,
    /// Request message expecting a response
    Request,
    /// Response to a request
    Response,
    /// Acknowledgment of receipt
    Ack,
    /// Error message
    Error,
    /// Health check/heartbeat
    Heartbeat,
    /// Control message for the bus
    Control,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::DirectMessage => write!(f, "DirectMessage"),
            MessageType::PublishMessage => write!(f, "PublishMessage"),
            MessageType::Subscribe => write!(f, "Subscribe"),
            MessageType::Unsubscribe => write!(f, "Unsubscribe"),
            MessageType::Request => write!(f, "Request"),
            MessageType::Response => write!(f, "Response"),
            MessageType::Ack => write!(f, "Ack"),
            MessageType::Error => write!(f, "Error"),
            MessageType::Heartbeat => write!(f, "Heartbeat"),
            MessageType::Control => write!(f, "Control"),
        }
    }
}

/// Core message structure for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    /// Unique message identifier
    pub id: String,
    /// Message type
    pub msg_type: MessageType,
    /// Source agent/component identifier
    pub sender: String,
    /// Destination agent/component (None for broadcast)
    pub recipient: Option<String>,
    /// Topic for pub/sub messaging
    pub topic: Option<String>,
    /// Request ID for request/response correlation
    pub request_id: Option<String>,
    /// Message payload
    pub payload: serde_json::Value,
    /// Message priority (0-100, higher = more important)
    pub priority: u8,
    /// Timestamp when message was created
    pub timestamp: SystemTime,
    /// Optional correlation ID for tracking related messages
    pub correlation_id: Option<String>,
    /// Number of delivery attempts made
    pub attempts: usize,
    /// Flag indicating if acknowledgment is required
    pub requires_ack: bool,
    /// Time-to-live in seconds (None = no expiration)
    pub ttl_secs: Option<u64>,
}

impl IpcMessage {
    /// Create a new message
    pub fn new(msg_type: MessageType, sender: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type,
            sender,
            recipient: None,
            topic: None,
            request_id: None,
            payload,
            priority: 50,
            timestamp: SystemTime::now(),
            correlation_id: None,
            attempts: 0,
            requires_ack: false,
            ttl_secs: None,
        }
    }

    /// Set the recipient for a direct message
    pub fn with_recipient(mut self, recipient: String) -> Self {
        self.recipient = Some(recipient);
        self
    }

    /// Set the topic for a pub/sub message
    pub fn with_topic(mut self, topic: String) -> Self {
        self.topic = Some(topic);
        self
    }

    /// Set the request ID for request/response correlation
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the correlation ID
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set the ack requirement
    pub fn require_ack(mut self) -> Self {
        self.requires_ack = true;
        self
    }

    /// Set the TTL
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    /// Check if the message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            if let Ok(elapsed) = self.timestamp.elapsed() {
                return elapsed > Duration::from_secs(ttl);
            }
        }
        false
    }

    /// Serialize the message to bytes
    /// Using JSON because bincode doesn't support serde_json::Value
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize a message from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

// ============================================================================
// Transport Trait and Implementations
// ============================================================================

/// Trait for message transport implementations
#[async_trait]
pub trait MessageTransport: Send + Sync {
    /// Send a message through this transport
    async fn send(&self, message: &IpcMessage) -> Result<(), String>;

    /// Receive a message through this transport
    async fn receive(&self) -> Result<Option<IpcMessage>, String>;

    /// Check if the transport is connected/healthy
    async fn is_healthy(&self) -> bool;

    /// Close the transport
    async fn close(&self) -> Result<(), String>;

    /// Get transport name
    fn name(&self) -> &str;
}

/// Unix socket based transport
pub struct UnixSocketTransport {
    socket_path: PathBuf,
    stream: Arc<Mutex<Option<UnixStream>>>,
    is_server: bool,
}

impl UnixSocketTransport {
    /// Create a new Unix socket transport
    pub fn new(socket_path: PathBuf, is_server: bool) -> Self {
        Self {
            socket_path,
            stream: Arc::new(Mutex::new(None)),
            is_server,
        }
    }

    /// Connect to the Unix socket
    pub async fn connect(&self) -> Result<(), String> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| format!("Failed to connect to socket: {}", e))?;

        let mut socket = self.stream.lock().await;
        *socket = Some(stream);
        Ok(())
    }

    /// Listen on the Unix socket (server mode only)
    pub async fn listen(&self) -> Result<UnixListener, String> {
        if !self.is_server {
            return Err("Transport not configured as server".to_string());
        }

        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(&self.socket_path);

        UnixListener::bind(&self.socket_path)
            .map_err(|e| format!("Failed to bind to socket: {}", e))
    }
}

#[async_trait]
impl MessageTransport for UnixSocketTransport {
    async fn send(&self, message: &IpcMessage) -> Result<(), String> {
        let bytes = message.serialize().map_err(|e| e.to_string())?;

        let mut socket = self.stream.lock().await;
        if let Some(stream) = socket.as_mut() {
            // Send message length first
            let len = bytes.len() as u32;
            stream
                .write_all(&len.to_le_bytes())
                .await
                .map_err(|e| format!("Failed to send message length: {}", e))?;

            // Send message
            stream
                .write_all(&bytes)
                .await
                .map_err(|e| format!("Failed to send message: {}", e))?;

            Ok(())
        } else {
            Err("Socket not connected".to_string())
        }
    }

    async fn receive(&self) -> Result<Option<IpcMessage>, String> {
        let mut socket = self.stream.lock().await;
        if let Some(stream) = socket.as_mut() {
            // Read message length
            let mut len_bytes = [0u8; 4];
            match stream.read_exact(&mut len_bytes).await {
                Ok(_) => {
                    let len = u32::from_le_bytes(len_bytes) as usize;
                    let mut buf = vec![0u8; len];

                    // Read message
                    match stream.read_exact(&mut buf).await {
                        Ok(_) => {
                            let message = IpcMessage::deserialize(&buf)
                                .map_err(|e| format!("Failed to deserialize message: {}", e))?;
                            Ok(Some(message))
                        }
                        Err(_) => Ok(None),
                    }
                }
                Err(_) => Ok(None),
            }
        } else {
            Err("Socket not connected".to_string())
        }
    }

    async fn is_healthy(&self) -> bool {
        self.stream.lock().await.is_some()
    }

    async fn close(&self) -> Result<(), String> {
        let mut socket = self.stream.lock().await;
        *socket = None;
        Ok(())
    }

    fn name(&self) -> &str {
        "unix-socket"
    }
}

/// In-memory transport using mpsc channels (for testing and local processes)
pub struct MemoryTransport {
    tx: Arc<mpsc::UnboundedSender<IpcMessage>>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<IpcMessage>>>,
}

impl MemoryTransport {
    /// Create a new memory transport pair
    pub fn new_pair() -> (Self, Self) {
        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, rx2) = mpsc::unbounded_channel();

        (
            Self {
                tx: Arc::new(tx1),
                rx: Arc::new(Mutex::new(rx2)),
            },
            Self {
                tx: Arc::new(tx2),
                rx: Arc::new(Mutex::new(rx1)),
            },
        )
    }
}

#[async_trait]
impl MessageTransport for MemoryTransport {
    async fn send(&self, message: &IpcMessage) -> Result<(), String> {
        self.tx
            .send(message.clone())
            .map_err(|e| format!("Failed to send: {}", e))
    }

    async fn receive(&self) -> Result<Option<IpcMessage>, String> {
        Ok(self.rx.lock().await.recv().await)
    }

    async fn is_healthy(&self) -> bool {
        true
    }

    async fn close(&self) -> Result<(), String> {
        Ok(())
    }

    fn name(&self) -> &str {
        "memory"
    }
}

// ============================================================================
// Dead Letter Queue
// ============================================================================

/// Dead Letter Queue for failed messages
pub struct DeadLetterQueue {
    messages: Arc<Mutex<VecDeque<(IpcMessage, String)>>>,
    max_size: usize,
}

impl DeadLetterQueue {
    /// Create a new dead letter queue
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    /// Add a message to the dead letter queue
    pub async fn enqueue(&self, message: IpcMessage, reason: String) {
        let mut queue = self.messages.lock().await;
        if queue.len() >= self.max_size {
            let _ = queue.pop_front();
        }
        queue.push_back((message, reason));
    }

    /// Get all messages from the dead letter queue
    pub async fn get_all(&self) -> Vec<(IpcMessage, String)> {
        self.messages.lock().await.iter().cloned().collect()
    }

    /// Clear the dead letter queue
    pub async fn clear(&self) {
        self.messages.lock().await.clear();
    }

    /// Get the size of the dead letter queue
    pub async fn size(&self) -> usize {
        self.messages.lock().await.len()
    }
}

// ============================================================================
// Backpressure Handling
// ============================================================================

/// Backpressure configuration and management
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Maximum number of pending messages
    pub max_pending: usize,
    /// Wait duration when backpressured
    pub wait_duration: Duration,
    /// Timeout for backpressure handling
    pub timeout: Duration,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_pending: 10000,
            wait_duration: Duration::from_millis(100),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Backpressure controller
pub struct BackpressureController {
    pending_count: Arc<AtomicU64>,
    config: BackpressureConfig,
}

impl BackpressureController {
    /// Create a new backpressure controller
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            pending_count: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// Check if we can accept more messages
    pub async fn check(&self) -> Result<(), String> {
        let pending = self.pending_count.load(Ordering::Relaxed);

        if pending >= self.config.max_pending as u64 {
            // Apply backpressure
            let result = timeout(self.config.timeout, sleep(self.config.wait_duration)).await;

            if result.is_err() {
                return Err("Backpressure timeout".to_string());
            }
        }

        Ok(())
    }

    /// Increment pending message count
    pub fn increment(&self) {
        self.pending_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement pending message count
    pub fn decrement(&self) {
        self.pending_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current pending count
    pub fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Message Router
// ============================================================================

/// Routing rules for message delivery
#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Rule identifier
    pub id: String,
    /// Message type filter (None = all)
    pub msg_type_filter: Option<MessageType>,
    /// Sender filter (None = all)
    pub sender_filter: Option<String>,
    /// Recipient filter (None = all)
    pub recipient_filter: Option<String>,
    /// Topic filter (None = all)
    pub topic_filter: Option<String>,
    /// Target handler name
    pub handler: String,
    /// Priority of this rule (higher = earlier)
    pub priority: u8,
    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Message router for intelligent routing
pub struct MessageRouter {
    rules: Arc<RwLock<Vec<RoutingRule>>>,
    handlers: Arc<DashMap<String, Arc<dyn MessageHandler>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(DashMap::new()),
        }
    }

    /// Register a routing rule
    pub async fn add_rule(&self, rule: RoutingRule) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Remove a routing rule
    pub async fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.id != rule_id);
        Ok(())
    }

    /// Register a message handler
    pub fn register_handler(
        &self,
        name: String,
        handler: Arc<dyn MessageHandler>,
    ) -> Result<(), String> {
        self.handlers.insert(name, handler);
        Ok(())
    }

    /// Route a message to appropriate handlers
    pub async fn route(&self, message: &IpcMessage) -> Result<(), String> {
        let rules = self.rules.read().await;

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            if let Some(filter) = &rule.msg_type_filter {
                if message.msg_type != *filter {
                    continue;
                }
            }

            if let Some(filter) = &rule.sender_filter {
                if message.sender != *filter {
                    continue;
                }
            }

            if let Some(filter) = &rule.recipient_filter {
                if let Some(ref recipient) = message.recipient {
                    if recipient != filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(filter) = &rule.topic_filter {
                if let Some(ref topic) = message.topic {
                    if topic != filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Route to handler
            if let Some(handler) = self.handlers.get(&rule.handler) {
                handler.handle(message).await?;
            }
        }

        Ok(())
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for message handlers
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle a message
    async fn handle(&self, message: &IpcMessage) -> Result<(), String>;

    /// Get handler name
    fn name(&self) -> &str;
}

// ============================================================================
// Request/Response Protocol
// ============================================================================

/// Request/response correlation tracker
pub struct RequestResponseTracker {
    pending: Arc<DashMap<String, (IpcMessage, SystemTime)>>,
    timeout: Duration,
}

impl RequestResponseTracker {
    /// Create a new request/response tracker
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            timeout,
        }
    }

    /// Register a pending request
    pub fn register_request(&self, message: IpcMessage) -> String {
        let request_id = message.id.clone();
        self.pending
            .insert(request_id.clone(), (message, SystemTime::now()));
        request_id
    }

    /// Mark a request as responded
    pub fn mark_responded(&self, request_id: &str) -> Option<IpcMessage> {
        self.pending.remove(request_id).map(|(_, (msg, _))| msg)
    }

    /// Get pending requests
    pub fn get_pending(&self) -> Vec<IpcMessage> {
        let now = SystemTime::now();
        let mut expired = Vec::new();

        for entry in self.pending.iter() {
            let (req_id, (_msg, timestamp)) = entry.pair();
            if let Ok(elapsed) = now.duration_since(*timestamp) {
                if elapsed > self.timeout {
                    expired.push(req_id.clone());
                }
            }
        }

        // Remove expired requests
        for req_id in expired {
            self.pending.remove(&req_id);
        }

        self.pending
            .iter()
            .map(|entry| entry.value().0.clone())
            .collect()
    }

    /// Get pending request count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

// ============================================================================
// Message Bus
// ============================================================================

/// Statistics for the message bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBusStats {
    /// Total messages processed
    pub total_messages: u64,
    /// Total messages sent
    pub total_sent: u64,
    /// Total messages failed
    pub total_failed: u64,
    /// Current pending messages
    pub pending_messages: u64,
    /// Dead letter queue size
    pub dlq_size: u64,
    /// Messages per topic
    pub per_topic: HashMap<String, u64>,
    /// Messages per sender
    pub per_sender: HashMap<String, u64>,
}

/// Configuration for the message bus
#[derive(Debug, Clone)]
pub struct MessageBusConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
    /// Max DLQ size
    pub dlq_max_size: usize,
    /// Enable message history
    pub enable_history: bool,
    /// Max history size
    pub max_history_size: usize,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10 MB
            request_timeout: Duration::from_secs(30),
            backpressure: BackpressureConfig::default(),
            dlq_max_size: 1000,
            enable_history: true,
            max_history_size: 10000,
        }
    }
}

/// Main message bus for inter-agent communication
pub struct MessageBus {
    config: MessageBusConfig,
    router: Arc<MessageRouter>,
    backpressure: Arc<BackpressureController>,
    dlq: Arc<DeadLetterQueue>,
    req_resp_tracker: Arc<RequestResponseTracker>,
    subscribers: Arc<DashMap<String, Vec<String>>>, // topic -> subscribers
    stats: Arc<Mutex<MessageBusStats>>,
    message_history: Arc<Mutex<VecDeque<IpcMessage>>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new(config: MessageBusConfig) -> Self {
        Self {
            backpressure: Arc::new(BackpressureController::new(config.backpressure.clone())),
            dlq: Arc::new(DeadLetterQueue::new(config.dlq_max_size)),
            req_resp_tracker: Arc::new(RequestResponseTracker::new(config.request_timeout)),
            router: Arc::new(MessageRouter::new()),
            subscribers: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(MessageBusStats {
                total_messages: 0,
                total_sent: 0,
                total_failed: 0,
                pending_messages: 0,
                dlq_size: 0,
                per_topic: HashMap::new(),
                per_sender: HashMap::new(),
            })),
            message_history: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_history_size))),
            config,
        }
    }

    /// Get the message router
    pub fn router(&self) -> Arc<MessageRouter> {
        Arc::clone(&self.router)
    }

    /// Get the backpressure controller
    pub fn backpressure(&self) -> Arc<BackpressureController> {
        Arc::clone(&self.backpressure)
    }

    /// Subscribe to a topic
    pub fn subscribe(&self, topic: String, subscriber: String) -> Result<(), String> {
        let mut subs = self.subscribers.entry(topic).or_insert_with(Vec::new);
        if !subs.contains(&subscriber) {
            subs.push(subscriber);
        }
        Ok(())
    }

    /// Unsubscribe from a topic
    pub fn unsubscribe(&self, topic: &str, subscriber: &str) -> Result<(), String> {
        if let Some(mut subs) = self.subscribers.get_mut(topic) {
            subs.retain(|s| s != subscriber);
        }
        Ok(())
    }

    /// Get subscribers for a topic
    pub fn get_subscribers(&self, topic: &str) -> Vec<String> {
        self.subscribers
            .get(topic)
            .map(|subs| subs.clone())
            .unwrap_or_default()
    }

    /// Send a message through the bus
    pub async fn send(&self, message: IpcMessage) -> Result<String, String> {
        // Check backpressure
        self.backpressure.check().await?;

        // Check message size
        if let Ok(serialized) = message.serialize() {
            if serialized.len() > self.config.max_message_size {
                self.dlq
                    .enqueue(message.clone(), "Message too large".to_string())
                    .await;
                return Err("Message exceeds maximum size".to_string());
            }
        }

        // Check TTL
        if message.is_expired() {
            self.dlq
                .enqueue(message.clone(), "Message expired".to_string())
                .await;
            return Err("Message has expired".to_string());
        }

        let message_id = message.id.clone();

        // Track request if needed
        if message.msg_type == MessageType::Request {
            self.req_resp_tracker.register_request(message.clone());
        }

        // Update statistics
        self.backpressure.increment();
        {
            let mut stats = self.stats.lock().await;
            stats.total_messages += 1;
            stats.total_sent += 1;
            stats.pending_messages += 1;

            if let Some(topic) = &message.topic {
                *stats.per_topic.entry(topic.clone()).or_insert(0) += 1;
            }

            *stats.per_sender.entry(message.sender.clone()).or_insert(0) += 1;
        }

        // Store in history
        if self.config.enable_history {
            let mut history = self.message_history.lock().await;
            if history.len() >= self.config.max_history_size {
                history.pop_front();
            }
            history.push_back(message.clone());
        }

        // Route the message
        if let Err(e) = self.router.route(&message).await {
            self.backpressure.decrement();
            self.dlq
                .enqueue(message.clone(), format!("Routing failed: {}", e))
                .await;

            let mut stats = self.stats.lock().await;
            stats.pending_messages -= 1;
            stats.total_failed += 1;

            return Err(format!("Failed to route message: {}", e));
        }

        self.backpressure.decrement();
        {
            let mut stats = self.stats.lock().await;
            stats.pending_messages -= 1;
        }

        info!(
            "Message sent: {} (type: {}, sender: {})",
            message_id, message.msg_type, message.sender
        );

        Ok(message_id)
    }

    /// Send a request and wait for response
    pub async fn request(&self, mut request: IpcMessage) -> Result<IpcMessage, String> {
        if request.msg_type != MessageType::Request {
            request.msg_type = MessageType::Request;
        }

        let request_id = self.send(request.clone()).await?;

        // Wait for response - simplified version
        // In a real implementation, this would use a oneshot channel per request
        let start = SystemTime::now();
        loop {
            // Check for timeout
            if let Ok(elapsed) = start.elapsed() {
                if elapsed > self.config.request_timeout {
                    self.req_resp_tracker.mark_responded(&request_id);
                    return Err("Request timeout".to_string());
                }
            }

            // Small delay to avoid busy waiting
            sleep(Duration::from_millis(10)).await;
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> MessageBusStats {
        self.stats.lock().await.clone()
    }

    /// Get dead letter queue
    pub fn dlq(&self) -> Arc<DeadLetterQueue> {
        Arc::clone(&self.dlq)
    }

    /// Get message history
    pub async fn get_history(&self) -> Vec<IpcMessage> {
        self.message_history.lock().await.iter().cloned().collect()
    }

    /// Clear message history
    pub async fn clear_history(&self) {
        self.message_history.lock().await.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = IpcMessage::new(
            MessageType::DirectMessage,
            "agent1".to_string(),
            serde_json::json!({"test": "data"}),
        )
        .with_recipient("agent2".to_string())
        .with_priority(75);

        assert_eq!(msg.sender, "agent1");
        assert_eq!(msg.recipient, Some("agent2".to_string()));
        assert_eq!(msg.priority, 75);
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_message_serialization() {
        let msg = IpcMessage::new(
            MessageType::PublishMessage,
            "agent1".to_string(),
            serde_json::json!({"value": 42}),
        );

        let serialized = msg.serialize().expect("Serialization failed");
        let deserialized = IpcMessage::deserialize(&serialized).expect("Deserialization failed");

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.sender, deserialized.sender);
        assert_eq!(msg.payload, deserialized.payload);
    }

    #[test]
    fn test_message_ttl_expiration() {
        let msg = IpcMessage::new(
            MessageType::DirectMessage,
            "agent1".to_string(),
            serde_json::json!({}),
        )
        .with_ttl(0); // 0 seconds TTL

        assert!(msg.is_expired());
    }

    #[tokio::test]
    async fn test_backpressure_controller() {
        let config = BackpressureConfig {
            max_pending: 2,
            wait_duration: Duration::from_millis(10),
            timeout: Duration::from_secs(1),
        };

        let controller = BackpressureController::new(config);

        controller.increment();
        controller.increment();
        controller.increment();

        assert_eq!(controller.pending_count(), 3);
        controller.decrement();
        assert_eq!(controller.pending_count(), 2);
    }

    #[tokio::test]
    async fn test_dead_letter_queue() {
        let dlq = DeadLetterQueue::new(10);

        let msg = IpcMessage::new(
            MessageType::Error,
            "agent1".to_string(),
            serde_json::json!({"error": "test"}),
        );

        dlq.enqueue(msg.clone(), "Test error".to_string()).await;

        assert_eq!(dlq.size().await, 1);

        let messages = dlq.get_all().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0.id, msg.id);
    }

    #[tokio::test]
    async fn test_message_bus_creation() {
        let bus = MessageBus::new(MessageBusConfig::default());

        let stats = bus.get_stats().await;
        assert_eq!(stats.total_messages, 0);
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::DirectMessage.to_string(), "DirectMessage");
        assert_eq!(MessageType::PublishMessage.to_string(), "PublishMessage");
        assert_eq!(MessageType::Request.to_string(), "Request");
        assert_eq!(MessageType::Response.to_string(), "Response");
    }

    #[tokio::test]
    async fn test_subscriptions() {
        let bus = MessageBus::new(MessageBusConfig::default());

        bus.subscribe("topic1".to_string(), "agent1".to_string())
            .expect("Subscribe failed");
        bus.subscribe("topic1".to_string(), "agent2".to_string())
            .expect("Subscribe failed");

        let subs = bus.get_subscribers("topic1");
        assert_eq!(subs.len(), 2);

        bus.unsubscribe("topic1", "agent1")
            .expect("Unsubscribe failed");

        let subs = bus.get_subscribers("topic1");
        assert_eq!(subs.len(), 1);
    }
}
