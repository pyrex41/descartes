# IPC Message Bus - Quick Reference

## Import

```rust
use descartes_core::{
    IpcMessage, MessageBus, MessageBusConfig, MessageType, MessageHandler,
    RoutingRule, BackpressureConfig,
};
use std::sync::Arc;
```

## Create Message Bus

```rust
let bus = Arc::new(MessageBus::new(MessageBusConfig::default()));
```

## Create Messages

### Direct Message
```rust
let msg = IpcMessage::new(
    MessageType::DirectMessage,
    "sender".to_string(),
    serde_json::json!({"data": "value"}),
)
.with_recipient("receiver".to_string());
```

### Publish Message
```rust
let msg = IpcMessage::new(
    MessageType::PublishMessage,
    "publisher".to_string(),
    serde_json::json!({"event": "triggered"}),
)
.with_topic("events".to_string());
```

### Request
```rust
let msg = IpcMessage::new(
    MessageType::Request,
    "client".to_string(),
    serde_json::json!({"method": "get"}),
)
.with_recipient("service".to_string())
.with_request_id("req-1".to_string());
```

### Response
```rust
let msg = IpcMessage::new(
    MessageType::Response,
    "service".to_string(),
    serde_json::json!({"result": "ok"}),
)
.with_request_id("req-1".to_string());
```

### Error
```rust
let msg = IpcMessage::new(
    MessageType::Error,
    "component".to_string(),
    serde_json::json!({"error": "something failed"}),
)
.with_priority(100);  // High priority
```

## Message Builder Methods

```rust
.with_recipient(String)           // Set direct recipient
.with_topic(String)               // Set pub/sub topic
.with_request_id(String)          // Set request correlation
.with_priority(u8)                // Set priority (0-100)
.with_correlation_id(String)      // Set batch/workflow ID
.with_ttl(u64)                    // Set expiration (seconds)
.require_ack()                    // Require acknowledgment
```

## Send Messages

```rust
// Send and forget
let msg_id = bus.send(message).await?;

// Send request (waits for response)
let response = bus.request(request).await?;
```

## Pub/Sub

```rust
// Subscribe
bus.subscribe("topic".to_string(), "agent1".to_string())?;

// Get subscribers
let subs = bus.get_subscribers("topic");

// Unsubscribe
bus.unsubscribe("topic", "agent1")?;
```

## Custom Handler

```rust
struct MyHandler;

#[async_trait::async_trait]
impl MessageHandler for MyHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        // Process message
        Ok(())
    }

    fn name(&self) -> &str {
        "my-handler"
    }
}

// Register
let router = bus.router();
router.register_handler("my-handler".to_string(), Arc::new(MyHandler))?;
```

## Routing Rules

```rust
let rule = RoutingRule {
    id: "rule1".to_string(),
    msg_type_filter: Some(MessageType::PublishMessage),
    sender_filter: Some("agent1".to_string()),
    recipient_filter: None,
    topic_filter: Some("events".to_string()),
    handler: "handler1".to_string(),
    priority: 10,
    enabled: true,
};

router.add_rule(rule).await?;
router.remove_rule("rule1").await?;
```

## Statistics

```rust
let stats = bus.get_stats().await;
println!("Total: {}", stats.total_messages);
println!("Sent: {}", stats.total_sent);
println!("Failed: {}", stats.total_failed);
println!("Pending: {}", stats.pending_messages);
```

## History

```rust
// Get all messages
let history = bus.get_history().await;

// Get last N messages
for msg in history.iter().rev().take(10) {
    println!("{}: {}", msg.id, msg.sender);
}

// Clear history
bus.clear_history().await;
```

## Dead Letter Queue

```rust
let dlq = bus.dlq();

// Get failed messages
let failed = dlq.get_all().await;
for (msg, reason) in failed {
    println!("Failed {}: {}", msg.id, reason);
}

// Get size
let size = dlq.size().await;

// Clear
dlq.clear().await;
```

## Backpressure

```rust
let bp = bus.backpressure();

// Check pending
let pending = bp.pending_count();

// Manual tracking
bp.increment();
bp.decrement();
```

## Message Priority Levels

```
0:    Low (background)
50:   Normal (default)
75:   High (important)
100:  Critical (urgent)
```

## Message Type Quick Reference

| Type | Use Case |
|------|----------|
| DirectMessage | Point-to-point |
| PublishMessage | Broadcast to topic |
| Request | RPC request |
| Response | RPC response |
| Subscribe | Topic subscription |
| Unsubscribe | Remove subscription |
| Ack | Confirm receipt |
| Error | Error notification |
| Heartbeat | Health check |
| Control | Bus control |

## Error Handling

```rust
match bus.send(msg).await {
    Ok(id) => println!("Sent: {}", id),
    Err(e) => {
        if e.contains("size") {
            // Handle oversized
        } else if e.contains("expired") {
            // Handle expired
        } else if e.contains("backpressure") {
            // Handle backpressure
        }
    }
}
```

## Configuration

```rust
let config = MessageBusConfig {
    max_message_size: 10 * 1024 * 1024,
    request_timeout: Duration::from_secs(30),
    backpressure: BackpressureConfig {
        max_pending: 10000,
        wait_duration: Duration::from_millis(100),
        timeout: Duration::from_secs(30),
    },
    dlq_max_size: 1000,
    enable_history: true,
    max_history_size: 10000,
};
```

## Common Patterns

### Broadcast Event
```rust
bus.subscribe("topic".to_string(), "agent1".to_string())?;
bus.subscribe("topic".to_string(), "agent2".to_string())?;

let msg = IpcMessage::new(MessageType::PublishMessage, "pub".to_string(), data)
    .with_topic("topic".to_string());
bus.send(msg).await?;
```

### RPC Call
```rust
let req = IpcMessage::new(MessageType::Request, "client".to_string(), params)
    .with_recipient("service".to_string())
    .with_request_id("req-123".to_string());
bus.send(req).await?;

let resp = IpcMessage::new(MessageType::Response, "service".to_string(), result)
    .with_request_id("req-123".to_string());
bus.send(resp).await?;
```

### Alert System
```rust
let alert = IpcMessage::new(MessageType::Error, "monitor".to_string(), alert_data)
    .with_priority(100)
    .with_correlation_id("batch-1".to_string());
bus.send(alert).await?;
```

### Batch Processing
```rust
for item in items {
    let msg = IpcMessage::new(MessageType::PublishMessage, "processor".to_string(), item)
        .with_topic("batch".to_string())
        .with_correlation_id("batch-id".to_string());
    bus.send(msg).await?;
}
```

## Testing

```rust
#[tokio::test]
async fn test_message_flow() {
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

## Performance Tips

1. **Keep messages small** - Reduces latency and memory
2. **Use topics wisely** - Minimize subscribers per topic
3. **Set appropriate TTL** - Remove stale messages
4. **Monitor DLQ** - Investigate failures
5. **Tune backpressure** - Balance throughput vs latency
6. **Limit history size** - Prevent memory bloat
7. **Prioritize messages** - Ensure critical messages process first
8. **Batch correlated messages** - Use correlation_id for grouping

## Debugging

```rust
// Check routing rules
let router = bus.router();

// Get subscribers
let subs = bus.get_subscribers("topic");
println!("Subscribers: {:?}", subs);

// Check history
let history = bus.get_history().await;
println!("Total messages: {}", history.len());

// Check DLQ
let dlq = bus.dlq();
let failed = dlq.get_all().await;
println!("Failed: {}", failed.len());

// Check backpressure
let bp = bus.backpressure();
println!("Pending: {}", bp.pending_count());

// Check stats
let stats = bus.get_stats().await;
println!("Stats: {:?}", stats);
```

## Links

- Integration Guide: `IPC_INTEGRATION_GUIDE.md`
- Technical Spec: `IPC_TECHNICAL_SPEC.md`
- Implementation: `IPC_IMPLEMENTATION_SUMMARY.md`
- Full README: `IPC_README.md`
- Examples: `examples/`

## Common Mistakes to Avoid

1. **Forgetting `.await`** on async operations
2. **Not handling errors** from send/receive
3. **Using wrong message type** for use case
4. **Not setting timeout** for request/response
5. **Ignoring DLQ** for failed messages
6. **Overloading topics** with too many subscribers
7. **Large message payloads** causing latency
8. **Not setting correlation IDs** for batches
9. **Infinite loops** in handlers
10. **Panicking in handlers** - use Result instead

## Version

- Implementation: 0.1.0
- Rust Edition: 2021
- Tokio: 1.35+
- Async: Yes (fully async)

## Status

✓ Implementation Complete
✓ Tests Included
✓ Documentation Complete
✓ Examples Provided
✓ Production Ready
