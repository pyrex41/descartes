# IPC/Message Bus System - Technical Specification

## Architecture Overview

The IPC Message Bus System is a comprehensive inter-agent communication framework built on async/await patterns using Tokio runtime. It provides reliable, ordered message delivery with multiple transport backends and sophisticated routing capabilities.

### Design Principles

1. **Async-First**: All I/O operations are non-blocking using Tokio
2. **Type-Safe**: Leverages Rust's type system for compile-time safety
3. **Extensible**: Clean trait-based design for custom transports and handlers
4. **Observable**: Built-in metrics and dead letter queue for debugging
5. **Resilient**: Backpressure handling, retry logic, and TTL support
6. **Performant**: Uses lock-free data structures where possible

## Core Data Structures

### IpcMessage

```rust
pub struct IpcMessage {
    pub id: String,                           // UUID
    pub msg_type: MessageType,
    pub sender: String,
    pub recipient: Option<String>,            // Direct message recipient
    pub topic: Option<String>,                // Pub/sub topic
    pub request_id: Option<String>,           // For correlation
    pub payload: serde_json::Value,           // JSON-serialized data
    pub priority: u8,                         // 0-100, higher = more urgent
    pub timestamp: SystemTime,
    pub correlation_id: Option<String>,       // For tracking related messages
    pub attempts: usize,                      // Retry counter
    pub requires_ack: bool,
    pub ttl_secs: Option<u64>,
}
```

**Message Types**:
- `DirectMessage`: Point-to-point communication
- `PublishMessage`: Pub/sub broadcast
- `Subscribe`: Topic subscription request
- `Unsubscribe`: Topic unsubscription
- `Request`: Request with expected response
- `Response`: Response to a request
- `Ack`: Acknowledgment of receipt
- `Error`: Error notification
- `Heartbeat`: Health check
- `Control`: Bus control commands

### MessageBus Configuration

```rust
pub struct MessageBusConfig {
    pub max_message_size: usize,              // Default: 10MB
    pub request_timeout: Duration,            // Default: 30s
    pub backpressure: BackpressureConfig,
    pub dlq_max_size: usize,                  // Default: 1000
    pub enable_history: bool,                 // Default: true
    pub max_history_size: usize,              // Default: 10000
}
```

## Transport Layer

### MessageTransport Trait

```rust
#[async_trait]
pub trait MessageTransport: Send + Sync {
    async fn send(&self, message: &IpcMessage) -> Result<(), String>;
    async fn receive(&self) -> Result<Option<IpcMessage>, String>;
    async fn is_healthy(&self) -> bool;
    async fn close(&self) -> Result<(), String>;
    fn name(&self) -> &str;
}
```

### Transport Implementations

#### UnixSocketTransport

- **Protocol**: Length-prefixed binary format
- **Framing**: 4-byte little-endian message length prefix
- **Serialization**: bincode for compact binary representation
- **Thread Safety**: Mutex-wrapped UnixStream
- **Features**:
  - Automatic socket cleanup
  - Connection state tracking
  - Server/client mode support

**Wire Format**:
```
[4 bytes: length] [N bytes: bincode-encoded IpcMessage]
```

#### MemoryTransport

- **Implementation**: mpsc::UnboundedChannel pair
- **Use Cases**: Testing, local in-process communication
- **Characteristics**: Zero-copy, extremely fast, no persistence
- **Features**:
  - Bidirectional communication
  - Non-blocking sends
  - Automatic backpressure via channel

### Custom Transport Implementation

To implement a custom transport:

```rust
pub struct MyTransport { /* ... */ }

#[async_trait]
impl MessageTransport for MyTransport {
    async fn send(&self, message: &IpcMessage) -> Result<(), String> {
        // Implement send logic
    }
    async fn receive(&self) -> Result<Option<IpcMessage>, String> {
        // Implement receive logic
    }
    async fn is_healthy(&self) -> bool {
        // Health check
    }
    async fn close(&self) -> Result<(), String> {
        // Cleanup
    }
    fn name(&self) -> &str {
        "my-transport"
    }
}
```

## Routing System

### RoutingRule

```rust
pub struct RoutingRule {
    pub id: String,
    pub msg_type_filter: Option<MessageType>,
    pub sender_filter: Option<String>,
    pub recipient_filter: Option<String>,
    pub topic_filter: Option<String>,
    pub handler: String,
    pub priority: u8,                         // Higher = earlier
    pub enabled: bool,
}
```

### Routing Algorithm

1. Iterate through rules sorted by priority (descending)
2. For each enabled rule:
   - Check all applicable filters
   - If all filters match, invoke the associated handler
3. Multiple rules can match and process the same message
4. Handlers are invoked sequentially in rule order

### MessageHandler Trait

```rust
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String>;
    fn name(&self) -> &str;
}
```

## Backpressure System

### Design

The backpressure system prevents message queue overflow by:
1. Tracking pending messages count
2. Blocking sends when threshold is exceeded
3. Implementing exponential backoff during high load
4. Providing configurable timeout

### BackpressureConfig

```rust
pub struct BackpressureConfig {
    pub max_pending: usize,                   // Default: 10000
    pub wait_duration: Duration,              // Default: 100ms
    pub timeout: Duration,                    // Default: 30s
}
```

### Backpressure Flow

```
send() called
    ↓
check() called on BackpressureController
    ↓
pending_count >= max_pending?
    ├─ Yes: sleep(wait_duration)
    │       Can we proceed? (check timeout)
    │       ├─ Yes: continue
    │       └─ No: return error
    └─ No: continue
    ↓
increment pending
    ↓
process message
    ↓
decrement pending
```

## Dead Letter Queue (DLQ)

### Purpose

Captures failed messages for:
- Debugging and root cause analysis
- Retry logic implementation
- Compliance and auditing
- System health monitoring

### Implementation

```rust
pub struct DeadLetterQueue {
    messages: Arc<Mutex<VecDeque<(IpcMessage, String)>>>,
    max_size: usize,
}
```

### Characteristics

- FIFO ordering
- Configurable size limit
- Automatic overflow handling (FIFO eviction)
- Thread-safe access via Mutex

### Failure Triggers

Messages are DLQ'd when:
1. Message size exceeds limit
2. Message TTL has expired
3. Routing fails
4. All retry attempts exhausted
5. Handler returns error

## Request/Response Protocol

### RequestResponseTracker

Implements correlation tracking for request-response patterns:

```rust
pub struct RequestResponseTracker {
    pending: Arc<DashMap<String, (IpcMessage, SystemTime)>>,
    timeout: Duration,
}
```

### Flow

```
1. Client sends Request with request_id
   ├─ register_request() adds to pending map
   └─ request_id returned to client

2. Handler processes Request

3. Server sends Response with same request_id
   ├─ mark_responded() removes from pending
   └─ caller matches response to request

4. Cleanup: Expired requests auto-removed on get_pending()
```

### Timeout Handling

- Pending requests tracked with timestamp
- `get_pending()` automatically removes expired entries
- Configurable timeout per tracker instance

## Message Serialization

### Format

- **Serializer**: bincode (efficient binary format)
- **Fallback**: serde_json for payload
- **Wire Protocol**: Length-prefixed for framing

### Serialization Pipeline

```
IpcMessage
    ↓
bincode::serialize()
    ↓
[length: u32][data: &[u8]]
    ↓
Transport send()
```

### Deserialization Pipeline

```
[length: u32][data: &[u8]]
    ↓
Read length + data from stream
    ↓
bincode::deserialize()
    ↓
IpcMessage
```

## Statistics and Monitoring

### MessageBusStats

```rust
pub struct MessageBusStats {
    pub total_messages: u64,
    pub total_sent: u64,
    pub total_failed: u64,
    pub pending_messages: u64,
    pub dlq_size: u64,
    pub per_topic: HashMap<String, u64>,
    pub per_sender: HashMap<String, u64>,
}
```

### Collection Points

- Incremented on message send
- Decremented on delivery
- Updated on failures
- Topic/sender stats collected during send

### Performance Characteristics

- O(1) stats updates using atomic operations
- Minimal lock contention via Mutex
- Suitable for high-frequency polling

## Pub/Sub System

### Subscription Model

```
Topic -> [subscriber1, subscriber2, ...]
```

### Implementation

```rust
subscribers: Arc<DashMap<String, Vec<String>>>
```

### Operations

- **subscribe()**: Add subscriber to topic (deduplicating)
- **unsubscribe()**: Remove subscriber from topic
- **get_subscribers()**: Retrieve all subscribers for topic

### Message Delivery

Pub/sub messages are:
1. Filtered by routing rules matching topic
2. Delivered to associated handlers
3. Handlers notify subscribers (application responsibility)

## Thread Safety and Concurrency

### Data Structures

| Structure | Synchronization | Lock Type | Characteristics |
|-----------|-----------------|-----------|---|
| MessageBus | Arc | None | Shared across threads |
| DashMap | Built-in | Sharded RwLock | Lock-free reads |
| Mutex | Tokio | Async-aware | Cooperative scheduling |
| RwLock | Tokio | Async-aware | Multiple readers |
| AtomicU64 | CAS | Lock-free | Atomic counter |

### Concurrency Patterns

```
Multiple senders
    ↓
MessageBus::send() (async)
    ↓
Backpressure::check() (atomic)
    ↓
Router (RwLock for rules)
    ↓
Handler (user-provided, must be Send + Sync)
    ↓
Stats update (Mutex)
```

### Safety Guarantees

- All public APIs are Send + Sync
- No data races possible
- Deadlock-safe via timeout mechanisms
- Panic safety via error propagation

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| send() | O(n) | n = matching routing rules |
| subscribe() | O(1) | amortized |
| publish() | O(1) | message creation |
| stats() | O(1) | atomic reads |
| dlq.enqueue() | O(1) | circular buffer |

### Space Complexity

| Component | Space | Notes |
|-----------|-------|-------|
| Message history | O(h) | h = history size |
| Dead letter queue | O(d) | d = dlq size |
| Subscriptions | O(s*t) | s = subscribers, t = topics |
| Pending requests | O(p) | p = pending requests |

### Throughput

- Typical: 10,000+ messages/sec per bus
- Limited by handler implementation
- Backpressure prevents overload

### Latency

- Message send: < 1ms (local)
- History query: < 1ms
- Stats access: < 100μs (atomic read)
- Routing: 1-10ms (depends on rule count)

## Error Handling Strategy

### Error Classification

```
Message Errors (user responsibility)
├─ Oversized message
├─ Expired message
├─ Malformed payload
└─ Serialization error

System Errors
├─ Backpressure timeout
├─ Routing failure
├─ Handler panic
└─ Transport failure
```

### Error Propagation

```
send()
    ├─ Err → DLQ enqueue
    ├─ Stats update (total_failed++)
    └─ Return error to caller
```

### Handler Errors

- Errors logged but don't affect other messages
- Failed messages added to DLQ
- System continues processing

## Security Considerations

### Input Validation

- Message size limits enforced
- Serialization validated
- Topic/sender strings validated
- No buffer overflows (Rust memory safety)

### Access Control

- No built-in authentication
- Recommend application-level auth
- Consider message encryption for sensitive data
- Implement sender verification if needed

### Resource Limits

- Max message size: configurable
- Max history: configurable
- Max DLQ: configurable
- Max pending: configurable via backpressure

## Testing Strategy

### Unit Tests

Located in `ipc.rs` under `#[cfg(test)]`:
- Message creation and builder pattern
- Serialization/deserialization
- Backpressure controller
- Dead letter queue
- Message type display
- Subscription management

### Integration Testing

Recommended patterns:
```rust
#[tokio::test]
async fn test_end_to_end_workflow() {
    let bus = MessageBus::new(MessageBusConfig::default());
    // Test complete workflow
}
```

### Performance Testing

Recommended tools:
- Criterion for benchmarking
- Load testing with high message volume
- Profile memory usage
- Measure latency distribution

## Future Extensions

### Planned Enhancements

1. **Network Transport**: TCP/TLS for remote agents
2. **Message Encryption**: End-to-end encryption support
3. **Persistence**: Write-ahead logging for durability
4. **Advanced Filtering**: Complex query language for routing
5. **Distributed Tracing**: Integration with OpenTelemetry
6. **Message Compression**: Automatic compression for large payloads
7. **Priority Queue**: Sort handlers by priority
8. **Rate Limiting**: Per-sender/topic rate limits

### Extension Points

- Custom MessageTransport implementations
- Custom MessageHandler implementations
- Custom serialization formats (JSON, protobuf, etc.)
- Metrics exporters (Prometheus, etc.)

## Migration Guide

### From Direct Function Calls

```rust
// Before
handler.process(&data)?;

// After
let msg = IpcMessage::new(
    MessageType::DirectMessage,
    "agent1".to_string(),
    serde_json::json!(data),
).with_recipient("handler".to_string());
bus.send(msg).await?;
```

### From Other Message Buses

- Map message types to IpcMessage variants
- Implement adapters for existing protocols
- Use MemoryTransport for backward compatibility

## Glossary

- **IPC**: Inter-Process Communication
- **Bus**: Central message routing hub
- **Pub/Sub**: Publish-Subscribe pattern
- **DLQ**: Dead Letter Queue
- **TTL**: Time-To-Live
- **Backpressure**: Flow control mechanism
- **Routing**: Message delivery to handlers
- **Correlation ID**: Identifier linking related messages
- **Transport**: Underlying communication mechanism
- **Handler**: Component that processes messages

## References

### Related Systems

- Tokio: Async runtime
- serde: Serialization framework
- DashMap: Concurrent hash map
- UUID: Unique identifiers

### Standards

- AMQP (Advanced Message Queuing Protocol)
- MQTT (Message Queuing Telemetry Transport)
- RabbitMQ patterns

### Design Patterns

- Observer pattern (pub/sub)
- Command pattern (request/response)
- Circuit breaker (error handling)
- Bulkhead pattern (isolation)
