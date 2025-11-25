/// Example demonstrating custom message handlers and advanced routing
///
/// This example shows:
/// 1. Creating domain-specific message handlers
/// 2. Implementing custom business logic
/// 3. Advanced routing rules with filtering
/// 4. Error handling and recovery
/// 5. Statistics monitoring
use descartes_core::{
    IpcMessage, MessageBus, MessageBusConfig, MessageHandler, MessageType, RoutingRule,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::time::Duration;

// ============================================================================
// Custom Handlers for Domain-Specific Logic
// ============================================================================

/// Handler that processes market data and validates it
struct MarketDataValidator {
    messages_processed: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl MessageHandler for MarketDataValidator {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        self.messages_processed.fetch_add(1, Ordering::Relaxed);

        match message.payload.get("prices") {
            Some(prices) => {
                println!("[MarketValidator] Processing prices: {:?}", prices);
                // Validate prices are within reasonable bounds
                if let Some(obj) = prices.as_object() {
                    for (pair, price) in obj {
                        if let Some(p) = price.as_f64() {
                            if p <= 0.0 {
                                return Err(format!("Invalid price for {}: {}", pair, p));
                            }
                        }
                    }
                }
                println!(
                    "[MarketValidator] Validation passed. Processed {} messages total",
                    self.messages_processed.load(Ordering::Relaxed)
                );
                Ok(())
            }
            None => Err("No prices in message".to_string()),
        }
    }

    fn name(&self) -> &str {
        "market_validator"
    }
}

/// Handler that processes trade execution requests
struct TradeExecutor {
    orders_executed: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl MessageHandler for TradeExecutor {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        self.orders_executed.fetch_add(1, Ordering::Relaxed);

        match (
            message.payload.get("action"),
            message.payload.get("pair"),
            message.payload.get("amount"),
        ) {
            (Some(action), Some(pair), Some(amount)) => {
                println!(
                    "[TradeExecutor] Executing: {} {} of {}",
                    action, amount, pair
                );

                // Simulate execution
                let order_id = format!("order-{}", uuid::Uuid::new_v4());
                println!(
                    "[TradeExecutor] Order placed: {}. Total executed: {}",
                    order_id,
                    self.orders_executed.load(Ordering::Relaxed)
                );

                Ok(())
            }
            _ => Err("Missing required trade fields".to_string()),
        }
    }

    fn name(&self) -> &str {
        "trade_executor"
    }
}

/// Handler that aggregates results and generates reports
struct ResultAggregator {
    results_aggregated: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl MessageHandler for ResultAggregator {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        self.results_aggregated.fetch_add(1, Ordering::Relaxed);

        println!(
            "[ResultAggregator] Aggregating result from {}",
            message.sender
        );
        println!("[ResultAggregator] Data: {:?}", message.payload);
        println!(
            "[ResultAggregator] Total results processed: {}",
            self.results_aggregated.load(Ordering::Relaxed)
        );

        Ok(())
    }

    fn name(&self) -> &str {
        "result_aggregator"
    }
}

/// Handler that monitors system health and alerts on anomalies
struct HealthMonitor {
    alerts_issued: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl MessageHandler for HealthMonitor {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        if message.priority >= 80 {
            self.alerts_issued.fetch_add(1, Ordering::Relaxed);
            println!(
                "[HealthMonitor] HIGH PRIORITY ALERT from {}: {:?}",
                message.sender, message.payload
            );
            println!(
                "[HealthMonitor] Total alerts issued: {}",
                self.alerts_issued.load(Ordering::Relaxed)
            );
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "health_monitor"
    }
}

/// Handler that logs all messages for audit trail
struct AuditLogger {
    logged_messages: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl MessageHandler for AuditLogger {
    async fn handle(&self, message: &IpcMessage) -> Result<(), String> {
        self.logged_messages.fetch_add(1, Ordering::Relaxed);

        println!(
            "[AuditLog] [{}] {} -> {} | Type: {} | Priority: {} | Data: {:?}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            message.sender,
            message
                .recipient
                .as_ref()
                .unwrap_or(&"broadcast".to_string()),
            message.msg_type,
            message.priority,
            message.payload
        );

        Ok(())
    }

    fn name(&self) -> &str {
        "audit_logger"
    }
}

// ============================================================================
// Main Example
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Advanced Custom Handlers Example ===\n");

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

    // Create handlers with atomic counters for tracking
    let validator = Arc::new(MarketDataValidator {
        messages_processed: Arc::new(AtomicU64::new(0)),
    });

    let executor = Arc::new(TradeExecutor {
        orders_executed: Arc::new(AtomicU64::new(0)),
    });

    let aggregator = Arc::new(ResultAggregator {
        results_aggregated: Arc::new(AtomicU64::new(0)),
    });

    let health_monitor = Arc::new(HealthMonitor {
        alerts_issued: Arc::new(AtomicU64::new(0)),
    });

    let audit_logger = Arc::new(AuditLogger {
        logged_messages: Arc::new(AtomicU64::new(0)),
    });

    // Register handlers
    println!("--- Registering handlers ---\n");
    router.register_handler("market_validator".to_string(), Arc::clone(&validator))?;
    router.register_handler("trade_executor".to_string(), Arc::clone(&executor))?;
    router.register_handler("result_aggregator".to_string(), Arc::clone(&aggregator))?;
    router.register_handler("health_monitor".to_string(), Arc::clone(&health_monitor))?;
    router.register_handler("audit_logger".to_string(), Arc::clone(&audit_logger))?;

    println!("Registered 5 custom handlers\n");

    // Add routing rules with different priorities and filters
    println!("--- Adding routing rules ---\n");

    // Rule 1: Market data goes to validator (high priority)
    router
        .add_rule(RoutingRule {
            id: "market-validate".to_string(),
            msg_type_filter: Some(MessageType::PublishMessage),
            sender_filter: Some("market_data_source".to_string()),
            recipient_filter: None,
            topic_filter: Some("market_data".to_string()),
            handler: "market_validator".to_string(),
            priority: 50,
            enabled: true,
        })
        .await?;

    // Rule 2: Trade requests go to executor
    router
        .add_rule(RoutingRule {
            id: "execute-trade".to_string(),
            msg_type_filter: Some(MessageType::DirectMessage),
            sender_filter: None,
            recipient_filter: Some("trader".to_string()),
            topic_filter: None,
            handler: "trade_executor".to_string(),
            priority: 40,
            enabled: true,
        })
        .await?;

    // Rule 3: Results go to aggregator
    router
        .add_rule(RoutingRule {
            id: "aggregate-results".to_string(),
            msg_type_filter: Some(MessageType::PublishMessage),
            sender_filter: None,
            recipient_filter: None,
            topic_filter: Some("results".to_string()),
            handler: "result_aggregator".to_string(),
            priority: 30,
            enabled: true,
        })
        .await?;

    // Rule 4: High priority messages go to health monitor
    router
        .add_rule(RoutingRule {
            id: "health-alert".to_string(),
            msg_type_filter: Some(MessageType::Error),
            sender_filter: None,
            recipient_filter: None,
            topic_filter: None,
            handler: "health_monitor".to_string(),
            priority: 60,
            enabled: true,
        })
        .await?;

    // Rule 5: All messages logged for audit (lowest priority)
    router
        .add_rule(RoutingRule {
            id: "audit-all".to_string(),
            msg_type_filter: None,
            sender_filter: None,
            recipient_filter: None,
            topic_filter: None,
            handler: "audit_logger".to_string(),
            priority: 0,
            enabled: true,
        })
        .await?;

    println!("Added 5 routing rules\n");

    // ========================================================================
    // Simulate message flow
    // ========================================================================

    println!("--- Simulating message flow ---\n");

    // 1. Market data message
    println!("1. Publishing market data...");
    let market_msg = IpcMessage::new(
        MessageType::PublishMessage,
        "market_data_source".to_string(),
        serde_json::json!({
            "prices": {
                "ETH/USDC": 1950.0,
                "BTC/USDC": 45000.0,
                "SOL/USDC": 195.0
            },
            "timestamp": chrono::Local::now().to_rfc3339()
        }),
    )
    .with_topic("market_data".to_string())
    .with_priority(50);

    bus.send(market_msg).await?;
    println!();

    // 2. Trade execution message
    println!("2. Executing trade...");
    let trade_msg = IpcMessage::new(
        MessageType::DirectMessage,
        "analysis_engine".to_string(),
        serde_json::json!({
            "action": "BUY",
            "pair": "ETH/USDC",
            "amount": 10.5
        }),
    )
    .with_recipient("trader".to_string())
    .with_priority(70);

    bus.send(trade_msg).await?;
    println!();

    // 3. Another trade
    println!("3. Executing second trade...");
    let trade_msg2 = IpcMessage::new(
        MessageType::DirectMessage,
        "analysis_engine".to_string(),
        serde_json::json!({
            "action": "SELL",
            "pair": "BTC/USDC",
            "amount": 0.5
        }),
    )
    .with_recipient("trader".to_string())
    .with_priority(65);

    bus.send(trade_msg2).await?;
    println!();

    // 4. Results message
    println!("4. Publishing results...");
    let result_msg = IpcMessage::new(
        MessageType::PublishMessage,
        "trader".to_string(),
        serde_json::json!({
            "total_pnl": 1250.0,
            "trades_executed": 2,
            "portfolio_value": 150000.0
        }),
    )
    .with_topic("results".to_string())
    .with_priority(60);

    bus.send(result_msg).await?;
    println!();

    // 5. Health alert
    println!("5. Issuing health alert...");
    let alert_msg = IpcMessage::new(
        MessageType::Error,
        "system_monitor".to_string(),
        serde_json::json!({
            "alert": "high_latency",
            "latency_ms": 250,
            "threshold_ms": 100
        }),
    )
    .with_priority(95)
    .with_correlation_id("alert-health-1".to_string());

    bus.send(alert_msg).await?;
    println!();

    // 6. Market data with invalid price (should error in handler)
    println!("6. Publishing invalid market data...");
    let invalid_msg = IpcMessage::new(
        MessageType::PublishMessage,
        "market_data_source".to_string(),
        serde_json::json!({
            "prices": {
                "ETH/USDC": -100.0, // Invalid: negative price
                "BTC/USDC": 45000.0
            }
        }),
    )
    .with_topic("market_data".to_string());

    match bus.send(invalid_msg).await {
        Ok(id) => println!("Message sent: {}", id),
        Err(e) => println!("Error (expected): {}", e),
    }
    println!();

    // ========================================================================
    // Display handler statistics
    // ========================================================================

    println!("--- Handler Statistics ---\n");
    println!(
        "Market Validator: {} messages processed",
        validator.messages_processed.load(Ordering::Relaxed)
    );
    println!(
        "Trade Executor: {} orders executed",
        executor.orders_executed.load(Ordering::Relaxed)
    );
    println!(
        "Result Aggregator: {} results aggregated",
        aggregator.results_aggregated.load(Ordering::Relaxed)
    );
    println!(
        "Health Monitor: {} alerts issued",
        health_monitor.alerts_issued.load(Ordering::Relaxed)
    );
    println!(
        "Audit Logger: {} messages logged",
        audit_logger.logged_messages.load(Ordering::Relaxed)
    );

    // ========================================================================
    // Display bus statistics
    // ========================================================================

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

    println!("\n--- Dead Letter Queue ---");
    let dlq = bus.dlq();
    let failed = dlq.get_all().await;
    if failed.is_empty() {
        println!("No failed messages");
    } else {
        println!("Failed messages:");
        for (msg, reason) in failed {
            println!("  - {}: {}", msg.id, reason);
        }
    }

    println!("\n=== Example Complete ===");

    Ok(())
}
