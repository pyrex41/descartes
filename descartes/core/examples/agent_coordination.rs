/// Example demonstrating IPC integration with agent coordination
///
/// This example shows how to use the IPC message bus to coordinate
/// multiple agents in a workflow, including pub/sub, request/response,
/// and state sharing patterns.
use descartes_core::{
    IpcMessage, MessageBus, MessageBusConfig, MessageHandler, MessageType, RoutingRule,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::Duration;

/// Represents an agent in our system
struct Agent {
    id: String,
    bus: Arc<MessageBus>,
    is_ready: Arc<AtomicBool>,
}

impl Agent {
    fn new(id: String, bus: Arc<MessageBus>) -> Self {
        Self {
            id,
            bus,
            is_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[{}] Starting...", self.id);
        self.is_ready.store(true, Ordering::SeqCst);

        // Announce that this agent is ready
        let startup_msg = IpcMessage::new(
            MessageType::PublishMessage,
            self.id.clone(),
            serde_json::json!({
                "event": "agent_ready",
                "agent_id": self.id,
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .with_topic("lifecycle".to_string())
        .with_priority(50);

        self.bus.send(startup_msg).await?;
        println!("[{}] Startup notification sent", self.id);

        Ok(())
    }

    async fn send_message(
        &self,
        recipient: String,
        payload: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let msg = IpcMessage::new(MessageType::DirectMessage, self.id.clone(), payload)
            .with_recipient(recipient);

        Ok(self.bus.send(msg).await?)
    }

    async fn request(
        &self,
        recipient: String,
        method: String,
        params: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = IpcMessage::new(
            MessageType::Request,
            self.id.clone(),
            serde_json::json!({
                "method": method,
                "params": params
            }),
        )
        .with_recipient(recipient.clone())
        .with_request_id(format!("req-{}-{}", self.id, uuid::Uuid::new_v4()));

        Ok(self.bus.send(request).await?)
    }

    async fn publish(
        &self,
        topic: String,
        event: String,
        data: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let msg = IpcMessage::new(
            MessageType::PublishMessage,
            self.id.clone(),
            serde_json::json!({
                "event": event,
                "data": data
            }),
        )
        .with_topic(topic);

        Ok(self.bus.send(msg).await?)
    }
}

/// Coordinator handler - processes coordination messages
struct CoordinatorHandler;

#[async_trait::async_trait]
impl MessageHandler for CoordinatorHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        println!(
            "[Coordinator] Received message from {}: {:?}",
            message.sender, message.payload
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "coordinator"
    }
}

/// Data processing handler - processes data-related messages
struct DataProcessorHandler;

#[async_trait::async_trait]
impl MessageHandler for DataProcessorHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        println!(
            "[DataProcessor] Processing message: {} with payload: {:?}",
            message.id, message.payload
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "data_processor"
    }
}

/// Result aggregator handler - collects results from other agents
struct ResultAggregatorHandler;

#[async_trait::async_trait]
impl MessageHandler for ResultAggregatorHandler {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        println!(
            "[ResultAggregator] Aggregating result from {}: {:?}",
            message.sender, message.payload
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "result_aggregator"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Agent Coordination Example ===\n");

    // Create message bus
    let config = MessageBusConfig {
        max_message_size: 10 * 1024 * 1024,
        request_timeout: Duration::from_secs(30),
        backpressure: Default::default(),
        dlq_max_size: 1000,
        enable_history: true,
        max_history_size: 10000,
    };

    let bus = Arc::new(MessageBus::new(config));
    let router = bus.router();

    // Register handlers
    router.register_handler("coordinator".to_string(), Arc::new(CoordinatorHandler))?;
    router.register_handler("data_processor".to_string(), Arc::new(DataProcessorHandler))?;
    router.register_handler(
        "result_aggregator".to_string(),
        Arc::new(ResultAggregatorHandler),
    )?;

    // Add routing rules
    router
        .add_rule(RoutingRule {
            id: "route-lifecycle".to_string(),
            msg_type_filter: Some(MessageType::PublishMessage),
            sender_filter: None,
            recipient_filter: None,
            topic_filter: Some("lifecycle".to_string()),
            handler: "coordinator".to_string(),
            priority: 10,
            enabled: true,
        })
        .await?;

    router
        .add_rule(RoutingRule {
            id: "route-data".to_string(),
            msg_type_filter: Some(MessageType::PublishMessage),
            sender_filter: None,
            recipient_filter: None,
            topic_filter: Some("data".to_string()),
            handler: "data_processor".to_string(),
            priority: 10,
            enabled: true,
        })
        .await?;

    router
        .add_rule(RoutingRule {
            id: "route-results".to_string(),
            msg_type_filter: Some(MessageType::PublishMessage),
            sender_filter: None,
            recipient_filter: None,
            topic_filter: Some("results".to_string()),
            handler: "result_aggregator".to_string(),
            priority: 10,
            enabled: true,
        })
        .await?;

    println!("--- Setting up agents ---\n");

    // Create agents
    let coordinator = Arc::new(Agent::new("coordinator".to_string(), Arc::clone(&bus)));
    let data_collector = Arc::new(Agent::new("data_collector".to_string(), Arc::clone(&bus)));
    let analyzer = Arc::new(Agent::new("analyzer".to_string(), Arc::clone(&bus)));
    let trader = Arc::new(Agent::new("trader".to_string(), Arc::clone(&bus)));

    // Subscribe agents to relevant topics
    bus.subscribe("lifecycle".to_string(), "coordinator".to_string())?;
    bus.subscribe("data".to_string(), "analyzer".to_string())?;
    bus.subscribe("data".to_string(), "trader".to_string())?;
    bus.subscribe("results".to_string(), "coordinator".to_string())?;
    bus.subscribe("results".to_string(), "trader".to_string())?;

    // Start agents
    coordinator.start().await?;
    data_collector.start().await?;
    analyzer.start().await?;
    trader.start().await?;

    println!("\n--- Agent workflow execution ---\n");

    // Simulate workflow

    // 1. Data Collector publishes market data
    println!("1. Data Collector publishing market data...");
    data_collector
        .publish(
            "data".to_string(),
            "market_data_ready".to_string(),
            serde_json::json!({
                "pairs": ["ETH/USDC", "BTC/USDC"],
                "prices": {
                    "ETH/USDC": 1950.0,
                    "BTC/USDC": 45000.0
                },
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await?;

    // 2. Analyzer makes a request to get more data
    println!("\n2. Analyzer requesting detailed market data from Data Collector...");
    analyzer
        .request(
            "data_collector".to_string(),
            "get_market_analysis".to_string(),
            serde_json::json!({
                "pairs": ["ETH/USDC"],
                "indicators": ["RSI", "MACD"],
                "timeframe": "1h"
            }),
        )
        .await?;

    // 3. Analyzer publishes analysis results
    println!("\n3. Analyzer publishing analysis results...");
    analyzer
        .publish(
            "results".to_string(),
            "analysis_complete".to_string(),
            serde_json::json!({
                "pair": "ETH/USDC",
                "recommendation": "BULLISH",
                "confidence": 0.85,
                "indicators": {
                    "RSI": 65.2,
                    "MACD": "positive"
                },
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await?;

    // 4. Trader receives and acts on analysis
    println!("\n4. Trader sending execution request based on analysis...");
    trader
        .send_message(
            "order_executor".to_string(),
            serde_json::json!({
                "action": "buy",
                "pair": "ETH/USDC",
                "amount": 10.5,
                "order_type": "market"
            }),
        )
        .await?;

    // 5. Trader publishes trade execution results
    println!("\n5. Trader publishing execution results...");
    trader
        .publish(
            "results".to_string(),
            "trade_executed".to_string(),
            serde_json::json!({
                "pair": "ETH/USDC",
                "order_id": "order-12345",
                "status": "filled",
                "amount": 10.5,
                "price": 1950.0,
                "total": 20572.5,
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await?;

    // 6. Coordinator publishes workflow completion
    println!("\n6. Coordinator publishing workflow completion...");
    coordinator
        .publish(
            "lifecycle".to_string(),
            "workflow_complete".to_string(),
            serde_json::json!({
                "workflow_id": "wf-123",
                "status": "success",
                "stages_completed": [
                    "data_collection",
                    "analysis",
                    "execution"
                ],
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await?;

    // Display results
    println!("\n--- Message Bus Statistics ---");
    let stats = bus.get_stats().await;
    println!("Total messages: {}", stats.total_messages);
    println!("Successfully sent: {}", stats.total_sent);
    println!("Failed: {}", stats.total_failed);
    println!("Pending: {}", stats.pending_messages);

    println!("\nMessages by sender:");
    for (sender, count) in stats.per_sender.iter() {
        println!("  - {}: {}", sender, count);
    }

    println!("\nMessages by topic:");
    for (topic, count) in stats.per_topic.iter() {
        println!("  - {}: {}", topic, count);
    }

    println!("\n--- Message History (Last 5 messages) ---");
    let history = bus.get_history().await;
    for msg in history.iter().rev().take(5) {
        println!(
            "  [{}] {} -> {} | {}",
            msg.id,
            msg.sender,
            msg.recipient.as_ref().unwrap_or(&"broadcast".to_string()),
            msg.msg_type
        );
    }

    println!("\n--- Dead Letter Queue ---");
    let dlq = bus.dlq();
    let failed = dlq.get_all().await;
    if failed.is_empty() {
        println!("No failed messages (DLQ is empty)");
    } else {
        for (msg, reason) in failed {
            println!("  - {}: {}", msg.id, reason);
        }
    }

    println!("\n=== Coordination Example Complete ===");

    Ok(())
}
