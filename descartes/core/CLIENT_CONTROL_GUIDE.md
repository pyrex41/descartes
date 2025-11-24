# ZMQ Client-Side Agent Control Guide

**Quick Reference for Phase 3:2.4 Implementation**

## Table of Contents
- [Quick Start](#quick-start)
- [Custom Actions](#custom-actions)
- [Batch Operations](#batch-operations)
- [Output Querying](#output-querying)
- [Status Streaming](#status-streaming)
- [Connection Management](#connection-management)
- [Error Handling](#error-handling)

## Quick Start

```rust
use descartes_core::{ZmqClient, ZmqRunnerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client configuration
    let config = ZmqRunnerConfig {
        endpoint: "tcp://localhost:5555".to_string(),
        request_timeout_secs: 30,
        auto_reconnect: true,
        max_reconnect_attempts: 3,
        ..Default::default()
    };

    // Create client
    let client = ZmqClient::new(config);

    // Connect to server
    client.connect("tcp://localhost:5555").await?;

    // Client is ready to use
    Ok(())
}
```

## Custom Actions

### Send a Custom Action

```rust
use uuid::Uuid;

async fn send_custom_action(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    // Prepare action parameters
    let params = serde_json::json!({
        "operation": "analyze_logs",
        "filter": "ERROR",
        "limit": 1000,
        "timeframe": "last_24h"
    });

    // Send action with 5-minute timeout
    let response = client.send_action_to_agent(
        agent_id,
        "analyze",
        Some(params),
        Some(300)
    ).await?;

    // Check result
    if let Some(data) = response.data {
        println!("Result: {:?}", data);
    }

    Ok(())
}
```

### Action Without Parameters

```rust
async fn simple_action(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.send_action_to_agent(
        agent_id,
        "ping",
        None,  // No parameters
        None   // Default timeout
    ).await?;

    println!("Agent responded: {:?}", response.status);
    Ok(())
}
```

## Batch Operations

### Pause Multiple Agents

```rust
use descartes_core::{AgentStatus, ControlCommandType};

async fn pause_all_running_agents(
    client: &ZmqClient,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all running agents
    let agents = client.list_remote_agents(
        Some(AgentStatus::Running),
        None
    ).await?;

    let agent_ids: Vec<Uuid> = agents.iter().map(|a| a.id).collect();

    // Pause all agents (continue on error)
    let response = client.batch_control(
        agent_ids,
        ControlCommandType::Pause,
        None,
        false  // Don't fail fast
    ).await?;

    println!("Paused: {}, Failed: {}",
             response.successful, response.failed);

    // Print errors
    for result in response.results {
        if !result.success {
            eprintln!("Agent {} failed: {}",
                      result.agent_id,
                      result.error.unwrap_or_default());
        }
    }

    Ok(())
}
```

### Batch Stop with Fail-Fast

```rust
async fn stop_agents_fail_fast(
    client: &ZmqClient,
    agent_ids: Vec<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Stop all agents, but fail on first error
    let response = client.batch_control(
        agent_ids,
        ControlCommandType::Stop,
        None,
        true  // Fail fast
    ).await?;

    if !response.success {
        eprintln!("Batch operation failed after {} successful operations",
                  response.successful);
        return Err("Batch stop failed".into());
    }

    println!("All agents stopped successfully");
    Ok(())
}
```

### Batch Custom Action

```rust
async fn batch_custom_action(
    client: &ZmqClient,
    agent_ids: Vec<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = serde_json::json!({
        "command": "refresh_cache"
    });

    let response = client.batch_control(
        agent_ids,
        ControlCommandType::CustomAction,
        Some(payload),
        false
    ).await?;

    println!("Success rate: {}/{}",
             response.successful,
             response.results.len());

    Ok(())
}
```

## Output Querying

### Query Recent Errors

```rust
use descartes_core::ZmqOutputStream;

async fn query_errors(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.query_agent_output(
        agent_id,
        ZmqOutputStream::Both,  // stdout and stderr
        Some("ERROR".to_string()),  // Filter for ERROR
        Some(50),  // Last 50 lines
        None       // No offset
    ).await?;

    for line in response.lines {
        println!("{}", line);
    }

    if response.has_more {
        println!("... {} more lines available",
                 response.total_lines.unwrap_or(0) - 50);
    }

    Ok(())
}
```

### Paginated Output Query

```rust
async fn query_all_output(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut all_lines = Vec::new();
    let mut offset = 0;
    let limit = 100;

    loop {
        let response = client.query_agent_output(
            agent_id,
            ZmqOutputStream::Stdout,
            None,  // No filter
            Some(limit),
            Some(offset)
        ).await?;

        all_lines.extend(response.lines);

        if !response.has_more {
            break;
        }

        offset += limit;
    }

    Ok(all_lines)
}
```

### Query with Regex Filter

```rust
async fn query_with_pattern(
    client: &ZmqClient,
    agent_id: &Uuid,
    pattern: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.query_agent_output(
        agent_id,
        ZmqOutputStream::Both,
        Some(pattern.to_string()),  // Regex pattern
        Some(100),
        None
    ).await?;

    println!("Found {} matching lines (of {} total)",
             response.lines.len(),
             response.total_lines.unwrap_or(0));

    for line in response.lines {
        println!("{}", line);
    }

    Ok(())
}
```

## Status Streaming

### Basic Status Monitoring

```rust
use descartes_core::StatusUpdateType;

async fn monitor_agent_status(
    client: &ZmqClient,
    agent_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    client.stream_agent_status(Some(agent_id), |update| {
        Box::pin(async move {
            match update.update_type {
                StatusUpdateType::StatusChanged => {
                    println!("Agent status: {:?}", update.status);
                }
                StatusUpdateType::Error => {
                    eprintln!("Agent error: {}",
                              update.message.unwrap_or_default());
                }
                StatusUpdateType::Completed => {
                    println!("Agent completed successfully");
                }
                _ => {}
            }
            Ok(())
        })
    }).await?;

    Ok(())
}
```

### Monitor All Agents

```rust
async fn monitor_all_agents(
    client: &ZmqClient,
) -> Result<(), Box<dyn std::error::Error>> {
    client.stream_agent_status(None, |update| {
        Box::pin(async move {
            println!("[{}] {:?}: {:?}",
                     update.agent_id,
                     update.update_type,
                     update.message);
            Ok(())
        })
    }).await?;

    Ok(())
}
```

### Collect Status Updates

```rust
use tokio::sync::mpsc;

async fn collect_status_updates(
    client: &ZmqClient,
    duration: Duration,
) -> Result<Vec<StatusUpdate>, Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Clone tx for the callback
    let tx_clone = tx.clone();

    // Start monitoring
    client.stream_agent_status(None, move |update| {
        let tx = tx_clone.clone();
        Box::pin(async move {
            let _ = tx.send(update);
            Ok(())
        })
    }).await?;

    // Collect updates for specified duration
    let mut updates = Vec::new();
    let deadline = tokio::time::Instant::now() + duration;

    while tokio::time::Instant::now() < deadline {
        if let Ok(update) = tokio::time::timeout(
            Duration::from_secs(1),
            rx.recv()
        ).await {
            if let Some(upd) = update {
                updates.push(upd);
            }
        }
    }

    Ok(updates)
}
```

## Connection Management

### Check Queue Status

```rust
async fn check_queue(client: &ZmqClient) {
    let count = client.queued_command_count().await;
    if count > 0 {
        println!("Warning: {} commands queued (disconnected?)", count);
    }
}
```

### Reconnection Handling

```rust
async fn handle_reconnection(
    client: &ZmqClient,
    endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Reconnecting to {}...", endpoint);

    // Check queued commands before reconnect
    let queued = client.queued_command_count().await;
    if queued > 0 {
        println!("Will process {} queued commands", queued);
    }

    // Reconnect (automatically processes queue)
    client.connect(endpoint).await?;

    // Verify queue was processed
    let remaining = client.queued_command_count().await;
    println!("Queue processed. Remaining: {}", remaining);

    Ok(())
}
```

### Health Check

```rust
async fn health_check(
    client: &ZmqClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let health = client.health_check().await?;

    if health.healthy {
        println!("Server is healthy");
        println!("  Protocol version: {}", health.protocol_version);
        if let Some(uptime) = health.uptime_secs {
            println!("  Uptime: {} seconds", uptime);
        }
        if let Some(active) = health.active_agents {
            println!("  Active agents: {}", active);
        }
    } else {
        eprintln!("Server is unhealthy!");
        return Err("Server health check failed".into());
    }

    Ok(())
}
```

## Error Handling

### Handle Connection Errors

```rust
async fn robust_connect(
    client: &ZmqClient,
    endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match client.connect(endpoint).await {
        Ok(_) => {
            println!("Connected successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);

            // Check if commands were queued
            let queued = client.queued_command_count().await;
            if queued > 0 {
                println!("Commands queued for retry: {}", queued);
            }

            Err(e.into())
        }
    }
}
```

### Handle Batch Errors

```rust
async fn handle_batch_errors(
    client: &ZmqClient,
    agent_ids: Vec<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.batch_control(
        agent_ids,
        ControlCommandType::Stop,
        None,
        false
    ).await?;

    if response.failed > 0 {
        eprintln!("Batch operation had {} failures:", response.failed);

        for result in response.results {
            if !result.success {
                eprintln!("  Agent {}: {}",
                          result.agent_id,
                          result.error.unwrap_or_else(|| "Unknown error".to_string()));
            }
        }
    }

    Ok(())
}
```

### Handle Output Query Errors

```rust
async fn safe_query_output(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    match client.query_agent_output(
        agent_id,
        ZmqOutputStream::Both,
        None,
        Some(100),
        None
    ).await {
        Ok(response) => Ok(response.lines),
        Err(e) => {
            eprintln!("Output query failed: {}", e);
            // Return empty vec instead of propagating error
            Ok(Vec::new())
        }
    }
}
```

### Timeout Handling

```rust
async fn action_with_timeout(
    client: &ZmqClient,
    agent_id: &Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = Duration::from_secs(10);

    let result = tokio::time::timeout(
        timeout,
        client.send_action_to_agent(
            agent_id,
            "long_running_task",
            None,
            Some(10)
        )
    ).await;

    match result {
        Ok(Ok(response)) => {
            println!("Action completed: {:?}", response);
            Ok(())
        }
        Ok(Err(e)) => {
            eprintln!("Action failed: {}", e);
            Err(e.into())
        }
        Err(_) => {
            eprintln!("Action timed out after {:?}", timeout);
            Err("Timeout".into())
        }
    }
}
```

## Complete Example

```rust
use descartes_core::{
    ZmqClient, ZmqRunnerConfig, AgentConfig, AgentStatus,
    ControlCommandType, ZmqOutputStream, StatusUpdateType,
};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);
    client.connect("tcp://localhost:5555").await?;

    // 1. Spawn an agent
    let agent_config = AgentConfig {
        name: "demo-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "Process data".to_string(),
        context: None,
        system_prompt: None,
        environment: std::collections::HashMap::new(),
    };

    let agent = client.spawn_remote(agent_config, Some(60)).await?;
    println!("Spawned agent: {} ({})", agent.name, agent.id);

    // 2. Send custom action
    let params = serde_json::json!({
        "dataset": "logs",
        "filter": "ERROR"
    });

    let action_response = client.send_action_to_agent(
        &agent.id,
        "analyze",
        Some(params),
        Some(120)
    ).await?;

    println!("Action result: {:?}", action_response.data);

    // 3. Query output
    let output = client.query_agent_output(
        &agent.id,
        ZmqOutputStream::Both,
        Some("ERROR".to_string()),
        Some(50),
        None
    ).await?;

    println!("Found {} error lines", output.lines.len());

    // 4. Batch operation - pause multiple agents
    let all_agents = client.list_remote_agents(
        Some(AgentStatus::Running),
        None
    ).await?;

    let agent_ids: Vec<Uuid> = all_agents.iter().map(|a| a.id).collect();

    let batch_response = client.batch_control(
        agent_ids,
        ControlCommandType::Pause,
        None,
        false
    ).await?;

    println!("Paused {}/{} agents",
             batch_response.successful,
             batch_response.results.len());

    // 5. Monitor status updates
    client.stream_agent_status(Some(agent.id), |update| {
        Box::pin(async move {
            println!("Status: {:?}", update.update_type);
            Ok(())
        })
    }).await?;

    // Wait a bit for updates
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 6. Cleanup
    client.stop_agent(&agent.id).await?;
    client.disconnect().await?;

    println!("Demo complete");
    Ok(())
}
```

## Best Practices

### 1. Connection Management
- Always check health before operations
- Monitor queue size during disconnections
- Set appropriate timeouts for your use case
- Enable auto-reconnect in production

### 2. Batch Operations
- Use batches for 10+ agents
- Use fail-fast for critical operations
- Use continue-on-error for best-effort operations
- Monitor success/failure ratios

### 3. Output Querying
- Use pagination for large outputs
- Apply filters to reduce data transfer
- Cache results when appropriate
- Set reasonable limits (100-1000 lines)

### 4. Status Streaming
- Filter by agent ID when monitoring specific agents
- Handle errors in callbacks gracefully
- Don't block in callbacks
- Use channels to communicate with main thread

### 5. Error Handling
- Always check response success flags
- Log errors for debugging
- Implement retry logic for transient failures
- Monitor error rates

## Configuration Reference

```rust
ZmqRunnerConfig {
    // Connection
    endpoint: String,                    // "tcp://host:port"
    connection_timeout_secs: u64,        // Default: 30
    request_timeout_secs: u64,           // Default: 30

    // Reconnection
    auto_reconnect: bool,                // Default: true
    max_reconnect_attempts: u32,         // Default: 3
    reconnect_delay_secs: u64,           // Default: 5

    // Health
    enable_heartbeat: bool,              // Default: true
    heartbeat_interval_secs: u64,        // Default: 30

    // Server
    server_id: Option<String>,           // Default: None
}
```

## Troubleshooting

### Queue Growing
```rust
// Check queue size
let size = client.queued_command_count().await;
if size > 100 {
    eprintln!("Warning: Queue size is {}", size);
    // Consider reconnecting or dropping old commands
}
```

### Slow Batch Operations
```rust
// Split large batches
let chunk_size = 50;
for chunk in agent_ids.chunks(chunk_size) {
    let response = client.batch_control(
        chunk.to_vec(),
        ControlCommandType::Pause,
        None,
        false
    ).await?;
}
```

### Large Output Queries
```rust
// Use smaller limits with pagination
let small_limit = 50;
let response = client.query_agent_output(
    &agent_id,
    ZmqOutputStream::Stdout,
    None,
    Some(small_limit),
    Some(0)
).await?;
```

---

For more information, see the full implementation report: `PHASE3_2_4_IMPLEMENTATION_REPORT.md`
