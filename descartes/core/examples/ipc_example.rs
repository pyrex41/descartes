/// Example demonstrating the IPC/Message Bus System for inter-agent communication
///
/// This example shows:
/// 1. Creating a message bus
/// 2. Pub/sub messaging
/// 3. Request/response patterns
/// 4. Message routing
/// 5. Backpressure handling
/// 6. Dead letter queue management
use descartes_core::{
    BackpressureConfig, BackpressureController, DeadLetterQueue, IpcMessage, MemoryTransport,
    MessageBus, MessageBusConfig, MessageHandler, MessageRouter, MessageType,
    RequestResponseTracker, RoutingRule,
};
use std::sync::Arc;
use tokio::time::Duration;

/// Example message handler that processes messages
struct ExampleHandler {
    name: String,
}

#[async_trait::async_trait]
impl MessageHandler for ExampleHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        println!(
            "[{}] Received message: id={}, type={}, sender={}",
            self.name, message.id, message.msg_type, message.sender
        );
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== IPC Message Bus Example ===\n");

    // 1. Create and configure the message bus
    let config = MessageBusConfig {
        max_message_size: 10 * 1024 * 1024,
        request_timeout: Duration::from_secs(30),
        backpressure: BackpressureConfig {
            max_pending: 1000,
            wait_duration: Duration::from_millis(100),
            timeout: Duration::from_secs(30),
        },
        dlq_max_size: 1000,
        enable_history: true,
        max_history_size: 10000,
    };

    let bus = Arc::new(MessageBus::new(config));

    // 2. Register handlers with the router
    let router = bus.router();

    let handler1 = Arc::new(ExampleHandler {
        name: "Handler-1".to_string(),
    });

    router.register_handler("handler1".to_string(), handler1)?;

    // 3. Add routing rules
    let rule = RoutingRule {
        id: "rule1".to_string(),
        msg_type_filter: Some(MessageType::PublishMessage),
        sender_filter: None,
        recipient_filter: None,
        topic_filter: Some("events".to_string()),
        handler: "handler1".to_string(),
        priority: 10,
        enabled: true,
    };

    router.add_rule(rule).await?;

    // 4. Subscribe agents to topics
    bus.subscribe("events".to_string(), "agent1".to_string())?;
    bus.subscribe("events".to_string(), "agent2".to_string())?;

    println!(
        "Subscribers for 'events': {:?}\n",
        bus.get_subscribers("events")
    );

    // 5. Example: Send a direct message
    println!("--- Sending Direct Messages ---");
    let direct_msg = IpcMessage::new(
        MessageType::DirectMessage,
        "agent1".to_string(),
        serde_json::json!({
            "action": "update_config",
            "data": {"threshold": 0.75}
        }),
    )
    .with_recipient("agent2".to_string())
    .with_priority(75);

    match bus.send(direct_msg).await {
        Ok(msg_id) => println!("Direct message sent: {}\n", msg_id),
        Err(e) => println!("Failed to send direct message: {}\n", e),
    }

    // 6. Example: Publish a message (pub/sub)
    println!("--- Publishing Pub/Sub Message ---");
    let pub_msg = IpcMessage::new(
        MessageType::PublishMessage,
        "data-agent".to_string(),
        serde_json::json!({
            "event": "arbitrage_opportunity",
            "pair": "ETH/USDC",
            "opportunity": {
                "exchange1": {"price": 1950.0},
                "exchange2": {"price": 1945.0},
                "profit_percent": 0.26
            }
        }),
    )
    .with_topic("events".to_string())
    .with_priority(90)
    .require_ack();

    match bus.send(pub_msg).await {
        Ok(msg_id) => println!("Published message: {}\n", msg_id),
        Err(e) => println!("Failed to publish: {}\n", e),
    }

    // 7. Example: Send a request message
    println!("--- Sending Request Message ---");
    let request = IpcMessage::new(
        MessageType::Request,
        "client-agent".to_string(),
        serde_json::json!({
            "method": "get_portfolio_status",
            "params": {}
        }),
    )
    .with_recipient("portfolio-agent".to_string())
    .with_request_id("req-12345".to_string());

    match bus.send(request).await {
        Ok(msg_id) => println!("Request sent: {}\n", msg_id),
        Err(e) => println!("Failed to send request: {}\n", e),
    }

    // 8. Example: Send multiple high-priority messages
    println!("--- Sending High-Priority Alert ---");
    for i in 0..3 {
        let alert = IpcMessage::new(
            MessageType::Error,
            "monitor-agent".to_string(),
            serde_json::json!({
                "alert_type": "high_gas_price",
                "gas_price": 150.0,
                "threshold": 100.0,
                "action": "pause_transactions"
            }),
        )
        .with_priority(100) // Highest priority
        .with_correlation_id(format!("alert-correlation-{}", i));

        match bus.send(alert).await {
            Ok(msg_id) => println!("Alert {} sent: {}", i + 1, msg_id),
            Err(e) => println!("Failed to send alert: {}", e),
        }
    }
    println!();

    // 9. Get and display statistics
    println!("--- Message Bus Statistics ---");
    let stats = bus.get_stats().await;
    println!("Total messages processed: {}", stats.total_messages);
    println!("Total messages sent: {}", stats.total_sent);
    println!("Total messages failed: {}", stats.total_failed);
    println!("Pending messages: {}", stats.pending_messages);
    println!("DLQ size: {}\n", stats.dlq_size);

    // 10. Check dead letter queue
    println!("--- Dead Letter Queue ---");
    let dlq = bus.dlq();
    let dlq_messages = dlq.get_all().await;
    println!("DLQ messages: {}", dlq_messages.len());
    for (msg, reason) in dlq_messages {
        println!("  - Message {}: {}", msg.id, reason);
    }
    println!();

    // 11. Get message history
    println!("--- Message History ---");
    let history = bus.get_history().await;
    println!("History size: {}", history.len());
    for msg in history.iter().take(3) {
        println!("  - {}: {} (from: {})", msg.id, msg.msg_type, msg.sender);
    }
    println!();

    // 12. Demonstrate backpressure handling
    println!("--- Backpressure Demo ---");
    let backpressure = bus.backpressure();
    println!("Initial pending count: {}", backpressure.pending_count());

    backpressure.increment();
    backpressure.increment();
    backpressure.increment();
    println!("After incrementing 3x: {}", backpressure.pending_count());

    backpressure.decrement();
    println!("After decrementing 1x: {}\n", backpressure.pending_count());

    // 13. Demonstrate request/response tracking
    println!("--- Request/Response Tracking ---");
    let req_resp_tracker = RequestResponseTracker::new(Duration::from_secs(30));

    let request_msg = IpcMessage::new(
        MessageType::Request,
        "agent1".to_string(),
        serde_json::json!({"query": "status"}),
    );

    let request_id = req_resp_tracker.register_request(request_msg.clone());
    println!("Registered request: {}", request_id);
    println!("Pending requests: {}", req_resp_tracker.pending_count());

    if let Some(marked_msg) = req_resp_tracker.mark_responded(&request_id) {
        println!("Marked as responded: {}", marked_msg.id);
    }
    println!(
        "Pending after response: {}\n",
        req_resp_tracker.pending_count()
    );

    // 14. Message builder pattern examples
    println!("--- Message Builder Examples ---");

    let complex_msg = IpcMessage::new(
        MessageType::PublishMessage,
        "smart-agent".to_string(),
        serde_json::json!({
            "analysis": {
                "sentiment": "bullish",
                "confidence": 0.92,
                "recommendation": "LONG"
            }
        }),
    )
    .with_topic("market_analysis".to_string())
    .with_priority(80)
    .with_correlation_id("analysis-2024-01-15".to_string())
    .with_ttl(3600)
    .require_ack();

    println!("Complex message created:");
    println!("  - ID: {}", complex_msg.id);
    println!("  - Correlation ID: {:?}", complex_msg.correlation_id);
    println!("  - TTL (seconds): {:?}", complex_msg.ttl_secs);
    println!("  - Requires ACK: {}\n", complex_msg.requires_ack);

    println!("=== Example Complete ===");

    Ok(())
}
