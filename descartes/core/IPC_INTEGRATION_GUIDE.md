# IPC/Message Bus System - Integration Guide

## Overview

The IPC (Inter-Process Communication) Message Bus System enables seamless communication between multiple agents in the Descartes orchestration platform. It provides reliable, asynchronous messaging with support for pub/sub, request/response, and direct messaging patterns.

## Key Features

- **Multiple Transport Types**: Unix sockets, shared memory, in-memory channels
- **Pub/Sub Messaging**: Publish messages to topics with multiple subscribers
- **Request/Response Pattern**: Correlation tracking for request-response cycles
- **Message Routing**: Intelligent routing with configurable rules
- **Backpressure Handling**: Prevent system overload with configurable backpressure
- **Dead Letter Queue**: Capture and monitor failed messages
- **Message History**: Track all messages for debugging and auditing
- **Reliability Features**: TTL, retries, acknowledgments, and correlation IDs
- **Performance Monitoring**: Built-in statistics and metrics

## Core Components

### IpcMessage
The fundamental unit of communication containing:
- **id**: Unique message identifier
- **msg_type**: Type of message (Direct, Publish, Request, Response, etc.)
- **sender**: Source agent identifier
- **recipient**: Optional destination agent
- **topic**: Optional pub/sub topic
- **payload**: JSON-encoded message data
- **priority**: Message priority (0-100)
- **timestamp**: When the message was created
- **correlation_id**: For tracking related messages
- **requires_ack**: Whether acknowledgment is needed
- **ttl_secs**: Message time-to-live in seconds

### MessageBus
The central hub for all message routing and delivery:
```rust
let config = MessageBusConfig {
    max_message_size: 10 * 1024 * 1024,
    request_timeout: Duration::from_secs(30),
    backpressure: BackpressureConfig::default(),
    dlq_max_size: 1000,
    enable_history: true,
    max_history_size: 10000,
};

let bus = Arc::new(MessageBus::new(config));
```

### MessageRouter
Routes messages to appropriate handlers based on rules:
```rust
let router = bus.router();

// Register a handler
router.register_handler("handler1".to_string(), handler_instance)?;

// Add routing rule
let rule = RoutingRule {
    id: "rule1".to_string(),
    msg_type_filter: Some(MessageType::PublishMessage),
    topic_filter: Some("events".to_string()),
    handler: "handler1".to_string(),
    priority: 10,
    enabled: true,
    ..Default::default()
};
router.add_rule(rule).await?;
```

### Transport Layer
Multiple transport implementations:
- **UnixSocketTransport**: Local IPC via Unix domain sockets
- **MemoryTransport**: In-memory channels for testing/local processes

```rust
// Unix socket transport
let transport = UnixSocketTransport::new(
    PathBuf::from("/tmp/ipc.sock"),
    true // is_server
);
transport.listen().await?;

// Memory transport (for testing)
let (tx_transport, rx_transport) = MemoryTransport::new_pair();
```

### BackpressureController
Prevents system overload by managing message flow:
```rust
let config = BackpressureConfig {
    max_pending: 10000,
    wait_duration: Duration::from_millis(100),
    timeout: Duration::from_secs(30),
};

let backpressure = BackpressureController::new(config);

backpressure.check().await?; // Check before sending
backpressure.increment();     // Track outgoing
backpressure.decrement();     // Track delivered
```

### DeadLetterQueue
Captures and stores failed messages for analysis:
```rust
let dlq = bus.dlq();

// Messages are automatically added on failure
let failed_msgs = dlq.get_all().await;
dlq.clear().await;
let size = dlq.size().await;
```

### RequestResponseTracker
Manages request-response correlation:
```rust
let tracker = RequestResponseTracker::new(Duration::from_secs(30));

let request_id = tracker.register_request(request_msg);
// ... wait for response ...
if let Some(request) = tracker.mark_responded(&request_id) {
    // Process response
}
```

## Usage Patterns

### 1. Direct Messaging
Send a message directly to a specific agent:

```rust
let msg = IpcMessage::new(
    MessageType::DirectMessage,
    "agent1".to_string(),
    serde_json::json!({"command": "start"}),
)
.with_recipient("agent2".to_string())
.with_priority(75);

let msg_id = bus.send(msg).await?;
```

### 2. Publish/Subscribe
Publish messages to topics with multiple subscribers:

```rust
// Subscribe
bus.subscribe("events".to_string(), "agent1".to_string())?;
bus.subscribe("events".to_string(), "agent2".to_string())?;

// Publish
let msg = IpcMessage::new(
    MessageType::PublishMessage,
    "publisher".to_string(),
    serde_json::json!({"event": "data_ready"}),
)
.with_topic("events".to_string())
.require_ack();

bus.send(msg).await?;
```

### 3. Request/Response
Request-response pattern with correlation tracking:

```rust
// Send request
let request = IpcMessage::new(
    MessageType::Request,
    "client".to_string(),
    serde_json::json!({"method": "get_status"}),
)
.with_recipient("service".to_string())
.with_request_id("req-123".to_string());

bus.send(request).await?;

// Response
let response = IpcMessage::new(
    MessageType::Response,
    "service".to_string(),
    serde_json::json!({"status": "ok"}),
)
.with_request_id("req-123".to_string());

bus.send(response).await?;
```

### 4. Priority-Based Messaging
Send critical messages with high priority:

```rust
let alert = IpcMessage::new(
    MessageType::Error,
    "monitor".to_string(),
    serde_json::json!({"alert": "critical"}),
)
.with_priority(100) // Highest priority
.with_correlation_id("alert-batch-1".to_string());

bus.send(alert).await?;
```

### 5. Message with TTL (Time-To-Live)
Messages that expire after a certain time:

```rust
let msg = IpcMessage::new(
    MessageType::PublishMessage,
    "agent".to_string(),
    serde_json::json!({"data": "time_sensitive"}),
)
.with_topic("notifications".to_string())
.with_ttl(300); // Expires in 5 minutes

bus.send(msg).await?;
```

## Custom Message Handlers

Implement the `MessageHandler` trait to create custom message processors:

```rust
struct MyCustomHandler;

#[async_trait::async_trait]
impl MessageHandler for MyCustomHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        println!("Processing message: {}", message.id);
        // Custom logic here
        Ok(())
    }

    fn name(&self) -> &str {
        "my-handler"
    }
}

// Register with router
let handler = Arc::new(MyCustomHandler);
router.register_handler("my-handler".to_string(), handler)?;
```

## Monitoring and Statistics

Access real-time statistics about message flow:

```rust
let stats = bus.get_stats().await;
println!("Total messages: {}", stats.total_messages);
println!("Failed messages: {}", stats.total_failed);
println!("Pending messages: {}", stats.pending_messages);
println!("Per-topic stats: {:?}", stats.per_topic);
println!("Per-sender stats: {:?}", stats.per_sender);

// Message history
let history = bus.get_history().await;
for msg in history {
    println!("Message {}: {}", msg.id, msg.msg_type);
}

// Dead letter queue
let dlq = bus.dlq();
let failed = dlq.get_all().await;
for (msg, reason) in failed {
    println!("Failed message {}: {}", msg.id, reason);
}
```

## Integration with Agent Runners

Connect the message bus to your agent runners for coordinated communication:

```rust
use descartes_core::{AgentRunner, MessageBus};

pub struct CoordinatedAgentRunner {
    runner: Box<dyn AgentRunner>,
    bus: Arc<MessageBus>,
}

impl CoordinatedAgentRunner {
    pub async fn notify_agents(&self, msg: IpcMessage) -> Result<String, String> {
        self.bus.send(msg).await
    }

    pub async fn request_from_agent(
        &self,
        request: IpcMessage,
    ) -> Result<String, String> {
        self.bus.send(request).await
    }
}
```

## Error Handling

The system provides comprehensive error handling:

```rust
match bus.send(message).await {
    Ok(msg_id) => println!("Sent: {}", msg_id),
    Err(e) => {
        match e.as_str() {
            "Message exceeds maximum size" => { /* Handle */ },
            "Message has expired" => { /* Handle */ },
            "Backpressure timeout" => { /* Handle */ },
            _ => { /* Generic error */ },
        }
    }
}
```

## Performance Considerations

1. **Message Size**: Keep messages under the configured limit (default 10MB)
2. **Backpressure**: Configure based on your expected message volume
3. **History**: Limit history size to prevent memory issues
4. **Routing Rules**: Order rules by priority and expected frequency
5. **Topic Subscriptions**: Monitor subscriber count per topic

## Testing

Use MemoryTransport for testing without Unix sockets:

```rust
#[tokio::test]
async fn test_message_flow() {
    let (tx_transport, rx_transport) = MemoryTransport::new_pair();
    let bus = MessageBus::new(MessageBusConfig::default());

    let msg = IpcMessage::new(
        MessageType::DirectMessage,
        "test".to_string(),
        serde_json::json!({}),
    );

    let result = bus.send(msg).await;
    assert!(result.is_ok());
}
```

## Best Practices

1. **Always set correlation IDs** for related messages
2. **Use appropriate priority levels** to ensure critical messages are processed
3. **Implement proper error handling** for message failures
4. **Monitor dead letter queue** regularly for failed messages
5. **Set reasonable TTLs** for time-sensitive messages
6. **Use topics effectively** to organize pub/sub subscriptions
7. **Implement custom handlers** for domain-specific logic
8. **Avoid large message payloads** to reduce latency
9. **Use ack requirement** only when ordering matters
10. **Review statistics** regularly for performance tuning

## Example: Multi-Agent Workflow

```rust
// Agent 1: Data Publisher
let data_msg = IpcMessage::new(
    MessageType::PublishMessage,
    "data_agent".to_string(),
    serde_json::json!({"market_data": {...}}),
)
.with_topic("market_data".to_string())
.with_correlation_id("day_123".to_string());
bus.send(data_msg).await?;

// Agent 2: Analyzer (subscribed to market_data)
bus.subscribe("market_data".to_string(), "analyzer".to_string())?;
// Handler processes messages as they arrive

// Agent 3: Trader (requests analysis)
let request = IpcMessage::new(
    MessageType::Request,
    "trader".to_string(),
    serde_json::json!({"pair": "ETH/USDC"}),
)
.with_recipient("analyzer".to_string())
.with_request_id("req_456".to_string())
.with_correlation_id("day_123".to_string());
bus.send(request).await?;

// Analyzer responds
let response = IpcMessage::new(
    MessageType::Response,
    "analyzer".to_string(),
    serde_json::json!({"recommendation": "BUY"}),
)
.with_request_id("req_456".to_string())
.with_correlation_id("day_123".to_string());
bus.send(response).await?;
```

## Troubleshooting

### Messages Not Received
- Check subscriber list: `bus.get_subscribers(topic)`
- Verify topic name matches exactly
- Check message TTL hasn't expired
- Review routing rules are enabled

### High Failure Rate
- Check DLQ: `bus.dlq().get_all().await`
- Monitor backpressure: `bus.backpressure().pending_count()`
- Review message size limits
- Check handler implementations for errors

### Performance Issues
- Reduce history size if memory usage is high
- Increase backpressure thresholds for high volume
- Check if message payloads are too large
- Profile routing rule matching overhead

## API Reference

See `ipc.rs` for complete API documentation.

Key types:
- `IpcMessage`: Core message structure
- `MessageBus`: Central routing hub
- `MessageType`: Enum of supported message types
- `MessageRouter`: Routes messages to handlers
- `BackpressureController`: Manages backpressure
- `DeadLetterQueue`: Failed message queue
- `RequestResponseTracker`: Tracks request/response cycles

## Future Enhancements

- Network transport for remote agents
- Message encryption for sensitive data
- Advanced filtering and querying
- Message compression for large payloads
- Distributed consensus for ordering guarantees
- Integration with distributed tracing systems
