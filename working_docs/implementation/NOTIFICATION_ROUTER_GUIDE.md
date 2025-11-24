# NotificationRouter Trait - Complete Guide

## Overview

The NotificationRouter is a comprehensive, extensible notification system designed for multi-channel delivery with support for routing, filtering, rate limiting, retries, and templating. It enables consistent, reliable notification delivery across multiple channels (Telegram, Slack, Email, Webhooks) with sophisticated routing logic.

## Architecture

### Core Components

1. **NotificationRouter Trait**: Main interface for routing and delivering notifications
2. **NotificationAdapter Trait**: Interface for channel-specific implementations
3. **Event Types & Severity Levels**: Categorization system for notifications
4. **Routing Rules**: Dynamic rules-based channel selection
5. **Rate Limiting**: Prevents notification spam
6. **Retry Logic**: Exponential backoff for failed deliveries
7. **Message Templating**: Dynamic message formatting

### DefaultNotificationRouter

Production-ready implementation featuring:
- Concurrent adapter management
- In-memory notification history
- Automatic rate limit enforcement
- Exponential backoff retry logic
- Comprehensive statistics tracking
- Duplicate detection

## Types and Enums

### Severity

Notification importance levels:
```rust
pub enum Severity {
    Info,      // Informational messages
    Warning,   // Warning messages
    Error,     // Error messages
    Critical,  // Critical issues requiring immediate attention
}
```

### NotificationChannel

Supported communication channels:
```rust
pub enum NotificationChannel {
    Telegram,  // Telegram bot notifications
    Slack,     // Slack webhook notifications
    Email,     // Email notifications
    Webhook,   // Generic HTTP webhook notifications
}
```

### NotificationEventType

Predefined event categories:
```rust
pub enum NotificationEventType {
    AgentStarted,
    AgentCompleted,
    AgentFailed,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    Alert,
    Metric,
    ArbitrageOpportunity,
    Custom(String),  // Custom event types
}
```

## Key Structures

### NotificationPayload

Complete notification message with metadata:
```rust
pub struct NotificationPayload {
    pub id: String,                           // Unique ID
    pub event_type: NotificationEventType,    // Event category
    pub severity: Severity,                   // Importance level
    pub title: String,                        // Subject/title
    pub message: String,                      // Main content
    pub data: HashMap<String, Value>,         // Structured data
    pub timestamp: SystemTime,                // Creation time
    pub source: String,                       // Originating component
    pub tags: Vec<String>,                    // Categorization tags
}
```

### RoutingRule

Automatic channel selection rules:
```rust
pub struct RoutingRule {
    pub id: String,
    pub event_types: Vec<NotificationEventType>,  // Matching event types
    pub min_severity: Severity,                   // Minimum severity
    pub channels: Vec<NotificationChannel>,       // Target channels
    pub required_tags: Vec<String>,               // Tag filters
    pub enabled: bool,
}
```

### RateLimitConfig

Rate limiting configuration:
```rust
pub struct RateLimitConfig {
    pub max_per_window: usize,                    // Max notifications
    pub window_duration: Duration,                // Time window
    pub deduplication_window: Option<Duration>,   // Duplicate detection
}
```

### RetryConfig

Retry and backoff configuration:
```rust
pub struct RetryConfig {
    pub max_attempts: usize,        // Maximum retry attempts
    pub initial_backoff: Duration,  // Starting backoff
    pub backoff_multiplier: f32,    // Exponential multiplier
    pub max_backoff: Duration,      // Maximum backoff cap
}
```

## NotificationRouter Trait

```rust
#[async_trait]
pub trait NotificationRouter: Send + Sync {
    // Adapter management
    async fn register_adapter(&self, adapter: Arc<dyn NotificationAdapter>) -> Result<(), String>;
    async fn unregister_adapter(&self, channel: NotificationChannel) -> Result<(), String>;

    // Rule management
    async fn add_routing_rule(&self, rule: RoutingRule) -> Result<(), String>;
    async fn remove_routing_rule(&self, rule_id: &str) -> Result<(), String>;
    async fn update_routing_rule(&self, rule: RoutingRule) -> Result<(), String>;

    // Configuration
    async fn set_rate_limit(&self, config: RateLimitConfig) -> Result<(), String>;
    async fn set_retry_config(&self, config: RetryConfig) -> Result<(), String>;

    // Sending
    async fn send_notification(&self, payload: NotificationPayload)
        -> Result<Vec<NotificationSendResult>, String>;
    async fn send_to_channels(&self, payload: NotificationPayload,
        channels: Vec<NotificationChannel>) -> Result<Vec<NotificationSendResult>, String>;

    // History and stats
    async fn get_recent_notifications(&self, since: Duration) -> Vec<NotificationPayload>;
    async fn clear_history(&self) -> Result<(), String>;
    async fn get_statistics(&self) -> Result<NotificationStats, String>;

    // Validation
    async fn check_rate_limit(&self, payload: &NotificationPayload) -> Result<(), String>;
    async fn validate_adapters(&self) -> Result<(), String>;
}
```

## NotificationAdapter Trait

```rust
#[async_trait]
pub trait NotificationAdapter: Send + Sync {
    fn channel(&self) -> NotificationChannel;
    async fn send(&self, payload: &NotificationPayload) -> Result<(), String>;
    fn format_message(&self, template: &str, context: &TemplateContext)
        -> Result<String, String>;
    async fn validate(&self) -> Result<(), String>;
    fn adapter_name(&self) -> &str;
}
```

## Usage Examples

### Initialize the Router

```rust
use descartes_core::DefaultNotificationRouter;

#[tokio::main]
async fn main() {
    let router = DefaultNotificationRouter::new();

    // Register adapters
    let telegram_adapter = Arc::new(TelegramAdapter::new("token"));
    router.register_adapter(telegram_adapter).await.unwrap();

    // Validate all adapters
    router.validate_adapters().await.unwrap();
}
```

### Create a Routing Rule

```rust
use descartes_core::*;

let rule = RoutingRule {
    id: "critical-alerts".to_string(),
    event_types: vec![
        NotificationEventType::AgentFailed,
        NotificationEventType::Alert,
    ],
    min_severity: Severity::Critical,
    channels: vec![
        NotificationChannel::Telegram,
        NotificationChannel::Slack,
    ],
    required_tags: vec!["urgent".to_string()],
    enabled: true,
};

router.add_routing_rule(rule).await?;
```

### Send a Notification

```rust
let payload = NotificationPayload::builder()
    .event_type(NotificationEventType::AgentCompleted)
    .severity(Severity::Info)
    .title("Agent Task Completed".to_string())
    .message("Agent processing completed successfully".to_string())
    .source("agent-runner".to_string())
    .tag("status".to_string())
    .data("duration".to_string(), json!("5s"))
    .build();

let results = router.send_notification(payload).await?;
for result in results {
    println!("Sent to {}: {}", result.channel, result.success);
}
```

### Direct Channel Send (Bypass Routing)

```rust
let results = router.send_to_channels(
    payload,
    vec![NotificationChannel::Slack, NotificationChannel::Email],
).await?;
```

### Configure Rate Limiting

```rust
let rate_limit = RateLimitConfig {
    max_per_window: 100,
    window_duration: Duration::from_secs(60),
    deduplication_window: Some(Duration::from_secs(300)),
};

router.set_rate_limit(rate_limit).await?;
```

### Configure Retries

```rust
let retry_config = RetryConfig {
    max_attempts: 3,
    initial_backoff: Duration::from_secs(1),
    backoff_multiplier: 2.0,
    max_backoff: Duration::from_secs(60),
};

router.set_retry_config(retry_config).await?;
```

### Get Statistics

```rust
let stats = router.get_statistics().await?;
println!("Total sent: {}", stats.total_sent);
println!("Total failed: {}", stats.total_failed);

for (channel, channel_stats) in &stats.per_channel {
    println!("Channel {}: {} successful, {} failed",
        channel, channel_stats.successful, channel_stats.failed);
}
```

### Message Templating

```rust
let context = TemplateContext::new()
    .add_variable("agent_name".to_string(), "AIBot".to_string())
    .add_variable("status".to_string(), "running".to_string());

let template = "Agent {{agent_name}} is {{status}}";
let formatted = context.format(template);
// Result: "Agent AIBot is running"
```

## Implementing Custom Adapters

```rust
use async_trait::async_trait;
use descartes_core::*;

pub struct CustomAdapter {
    endpoint: String,
}

#[async_trait]
impl NotificationAdapter for CustomAdapter {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Webhook
    }

    async fn send(&self, payload: &NotificationPayload) -> Result<(), String> {
        // Send to your endpoint
        let client = reqwest::Client::new();
        client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn format_message(
        &self,
        template: &str,
        context: &TemplateContext,
    ) -> Result<String, String> {
        Ok(context.format(template))
    }

    async fn validate(&self) -> Result<(), String> {
        // Validate configuration
        if self.endpoint.is_empty() {
            return Err("Endpoint not configured".to_string());
        }
        Ok(())
    }

    fn adapter_name(&self) -> &str {
        "CustomAdapter"
    }
}
```

## Best Practices

### 1. Rule Organization
- Group related rules for better maintainability
- Use clear, descriptive rule IDs
- Document each rule's purpose

### 2. Error Handling
- Always handle rate limit errors gracefully
- Log failed deliveries for debugging
- Implement fallback channels

### 3. Performance
- Use appropriate rate limiting settings
- Enable deduplication for high-volume notifications
- Monitor statistics regularly

### 4. Security
- Validate all external inputs in adapters
- Secure sensitive data in templates
- Use environment variables for credentials

### 5. Testing
- Test routing rules with mock data
- Verify rate limiting behavior
- Test retry logic with simulated failures

## Configuration Recommendations

### Development
```rust
RateLimitConfig {
    max_per_window: 1000,
    window_duration: Duration::from_secs(60),
    deduplication_window: Some(Duration::from_secs(60)),
}

RetryConfig {
    max_attempts: 2,
    initial_backoff: Duration::from_millis(500),
    backoff_multiplier: 2.0,
    max_backoff: Duration::from_secs(10),
}
```

### Production
```rust
RateLimitConfig {
    max_per_window: 100,
    window_duration: Duration::from_secs(60),
    deduplication_window: Some(Duration::from_secs(300)),
}

RetryConfig {
    max_attempts: 3,
    initial_backoff: Duration::from_secs(1),
    backoff_multiplier: 2.0,
    max_backoff: Duration::from_secs(60),
}
```

## Testing

The notification system includes comprehensive unit tests:

```bash
# Run all notification tests
cargo test --lib notifications

# Run specific test
cargo test --lib test_notification_payload_builder

# Run with output
cargo test --lib notifications -- --nocapture
```

## Error Handling

Common error scenarios:

```rust
use descartes_core::NotificationError;

match router.send_notification(payload).await {
    Ok(results) => {
        for result in results {
            if result.success {
                println!("Sent successfully");
            } else if let Some(error) = result.error {
                eprintln!("Failed: {}", error);
            }
        }
    }
    Err(e) => {
        eprintln!("Router error: {}", e);
        // Handle router-level errors
    }
}
```

## Performance Considerations

- **Memory Usage**: Notification history stored in memory; clear regularly for long-running services
- **Concurrency**: Uses RwLock for thread-safe access to shared state
- **Throughput**: Rate limiting configuration impacts maximum throughput
- **Latency**: Retries with backoff can increase delivery latency

## Future Extensions

The system is designed to support:
- Persistence layers for notification history
- Distributed rate limiting
- Advanced filtering and transformation pipelines
- Multi-language templates
- Analytics and reporting
- Webhook subscriptions
- Message signing and verification

## File Structure

```
core/src/
├── notifications.rs              # Core trait definitions
├── notification_router_impl.rs   # Default implementation
└── lib.rs                        # Module exports
```

## Module Documentation

See inline documentation in:
- `/Users/reuben/gauntlet/cap/descartes/core/src/notifications.rs` - Trait definitions
- `/Users/reuben/gauntlet/cap/descartes/core/src/notification_router_impl.rs` - Implementation

## Version Info

Descartes Notification System v0.1.0
Part of Descartes: Composable AI Agent Orchestration System
