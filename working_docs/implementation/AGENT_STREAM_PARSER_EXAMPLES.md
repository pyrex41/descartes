# Agent Stream Parser - Usage Examples and Integration Guide

**Module**: `descartes-core::agent_stream_parser`
**Date**: 2025-11-24
**Phase**: 3:5.2

---

## Table of Contents

1. [Basic Usage](#basic-usage)
2. [Custom Handlers](#custom-handlers)
3. [Integration Patterns](#integration-patterns)
4. [Advanced Scenarios](#advanced-scenarios)
5. [Testing Utilities](#testing-utilities)
6. [Performance Tuning](#performance-tuning)

---

## Basic Usage

### Minimal Example

```rust
use descartes_core::agent_stream_parser::AgentStreamParser;

fn main() {
    let json_lines = vec![
        r#"{"type":"status_update","agent_id":"550e8400-e29b-41d4-a716-446655440000","status":"running","timestamp":"2025-11-24T05:53:00Z"}"#,
    ];

    let mut parser = AgentStreamParser::new();
    parser.process_lines(&json_lines).unwrap();

    println!("Processed {} messages", parser.statistics().messages_processed);
}
```

### With Logging Handler

```rust
use descartes_core::agent_stream_parser::{AgentStreamParser, LoggingHandler};

fn main() {
    let mut parser = AgentStreamParser::new();
    parser.register_handler(LoggingHandler);

    let json_lines = vec![
        r#"{"type":"status_update","agent_id":"550e8400-e29b-41d4-a716-446655440000","status":"thinking","timestamp":"2025-11-24T05:53:00Z"}"#,
        r#"{"type":"thought_update","agent_id":"550e8400-e29b-41d4-a716-446655440000","thought":"Analyzing the problem...","timestamp":"2025-11-24T05:53:01Z"}"#,
    ];

    parser.process_lines(&json_lines).unwrap();
}
```

### Async Stream Processing

```rust
use descartes_core::agent_stream_parser::{AgentStreamParser, LoggingHandler};
use tokio::io::BufReader;
use tokio::fs::File;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = AgentStreamParser::new();
    parser.register_handler(LoggingHandler);

    // Read from file
    let file = File::open("agent_output.ndjson").await?;
    let reader = BufReader::new(file);

    parser.process_stream(reader).await?;

    println!("Stats: {:?}", parser.statistics());
    Ok(())
}
```

---

## Custom Handlers

### Simple Console Handler

```rust
use descartes_core::agent_stream_parser::StreamHandler;
use descartes_core::agent_state::{AgentStatus, AgentProgress, AgentError, OutputStream, LifecycleEvent};
use uuid::Uuid;
use chrono::{DateTime, Utc};

struct ConsoleHandler;

impl StreamHandler for ConsoleHandler {
    fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, _timestamp: DateTime<Utc>) {
        println!("[STATUS] Agent {}: {}", &agent_id.to_string()[..8], status);
    }

    fn on_thought_update(&mut self, agent_id: Uuid, thought: String, _timestamp: DateTime<Utc>) {
        println!("[THOUGHT] Agent {}: {}", &agent_id.to_string()[..8], thought);
    }

    fn on_progress_update(&mut self, agent_id: Uuid, progress: AgentProgress, _timestamp: DateTime<Utc>) {
        let bar = "=".repeat((progress.percentage / 2.0) as usize);
        println!("[PROGRESS] Agent {}: [{}] {:.1}%",
            &agent_id.to_string()[..8], bar, progress.percentage);
    }

    fn on_output(&mut self, agent_id: Uuid, stream: OutputStream, content: String, _timestamp: DateTime<Utc>) {
        let prefix = match stream {
            OutputStream::Stdout => "[OUT]",
            OutputStream::Stderr => "[ERR]",
        };
        println!("{} Agent {}: {}", prefix, &agent_id.to_string()[..8], content);
    }

    fn on_error(&mut self, agent_id: Uuid, error: AgentError, _timestamp: DateTime<Utc>) {
        eprintln!("[ERROR] Agent {}: {} (code: {})",
            &agent_id.to_string()[..8], error.message, error.code);
    }

    fn on_lifecycle(&mut self, agent_id: Uuid, event: LifecycleEvent, _timestamp: DateTime<Utc>) {
        println!("[LIFECYCLE] Agent {}: {:?}", &agent_id.to_string()[..8], event);
    }

    fn on_heartbeat(&mut self, _agent_id: Uuid, _timestamp: DateTime<Utc>) {
        // Silent - heartbeats are too frequent for console
    }
}
```

### Metrics Collection Handler

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Default, Clone)]
struct MetricsData {
    status_changes: HashMap<Uuid, usize>,
    thought_count: usize,
    error_count: usize,
    total_messages: usize,
}

struct MetricsHandler {
    metrics: Arc<Mutex<MetricsData>>,
}

impl MetricsHandler {
    fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(MetricsData::default())),
        }
    }

    fn get_metrics(&self) -> MetricsData {
        self.metrics.lock().unwrap().clone()
    }
}

impl StreamHandler for MetricsHandler {
    fn on_status_update(&mut self, agent_id: Uuid, _status: AgentStatus, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        *metrics.status_changes.entry(agent_id).or_insert(0) += 1;
        metrics.total_messages += 1;
    }

    fn on_thought_update(&mut self, _agent_id: Uuid, _thought: String, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.thought_count += 1;
        metrics.total_messages += 1;
    }

    fn on_progress_update(&mut self, _agent_id: Uuid, _progress: AgentProgress, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_messages += 1;
    }

    fn on_output(&mut self, _agent_id: Uuid, _stream: OutputStream, _content: String, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_messages += 1;
    }

    fn on_error(&mut self, _agent_id: Uuid, _error: AgentError, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.error_count += 1;
        metrics.total_messages += 1;
    }

    fn on_lifecycle(&mut self, _agent_id: Uuid, _event: LifecycleEvent, _timestamp: DateTime<Utc>) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_messages += 1;
    }

    fn on_heartbeat(&mut self, _agent_id: Uuid, _timestamp: DateTime<Utc>) {
        // Don't count heartbeats in total
    }
}
```

### File Storage Handler

```rust
use std::fs::OpenOptions;
use std::io::Write;

struct FileStorageHandler {
    file: std::fs::File,
}

impl FileStorageHandler {
    fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }
}

impl StreamHandler for FileStorageHandler {
    fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, timestamp: DateTime<Utc>) {
        let line = format!("[{}] {} -> {}\n", timestamp, agent_id, status);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_thought_update(&mut self, agent_id: Uuid, thought: String, timestamp: DateTime<Utc>) {
        let line = format!("[{}] {} THOUGHT: {}\n", timestamp, agent_id, thought);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_progress_update(&mut self, agent_id: Uuid, progress: AgentProgress, timestamp: DateTime<Utc>) {
        let line = format!("[{}] {} PROGRESS: {:.1}%\n", timestamp, agent_id, progress.percentage);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_output(&mut self, agent_id: Uuid, stream: OutputStream, content: String, timestamp: DateTime<Utc>) {
        let stream_name = match stream {
            OutputStream::Stdout => "STDOUT",
            OutputStream::Stderr => "STDERR",
        };
        let line = format!("[{}] {} {}: {}\n", timestamp, agent_id, stream_name, content);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_error(&mut self, agent_id: Uuid, error: AgentError, timestamp: DateTime<Utc>) {
        let line = format!("[{}] {} ERROR: {} ({})\n", timestamp, agent_id, error.message, error.code);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_lifecycle(&mut self, agent_id: Uuid, event: LifecycleEvent, timestamp: DateTime<Utc>) {
        let line = format!("[{}] {} LIFECYCLE: {:?}\n", timestamp, agent_id, event);
        self.file.write_all(line.as_bytes()).ok();
    }

    fn on_heartbeat(&mut self, _agent_id: Uuid, _timestamp: DateTime<Utc>) {
        // Don't log heartbeats to avoid file bloat
    }
}
```

---

## Integration Patterns

### Pattern 1: Agent Process Monitor

```rust
use descartes_core::agent_stream_parser::{AgentStreamParser, LoggingHandler};
use tokio::process::Command;
use tokio::io::BufReader;

async fn monitor_agent(agent_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Start agent process
    let mut child = Command::new(agent_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();

    // Create parser
    let mut parser = AgentStreamParser::new();
    parser.register_handler(LoggingHandler);

    // Process stdout in background
    let parse_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        parser.process_stream(reader).await.ok();
        parser
    });

    // Wait for process to complete
    let status = child.wait().await?;
    println!("Agent exited with status: {}", status);

    // Get final parser state
    let parser = parse_task.await?;
    println!("Final stats: {:?}", parser.statistics());

    Ok(())
}
```

### Pattern 2: Multi-Agent Orchestrator

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;

struct AgentOrchestrator {
    agents: HashMap<Uuid, AgentHandle>,
    parser: AgentStreamParser,
}

struct AgentHandle {
    process: tokio::process::Child,
    status: AgentStatus,
}

impl AgentOrchestrator {
    fn new() -> Self {
        let mut parser = AgentStreamParser::new();
        parser.register_handler(LoggingHandler);

        Self {
            agents: HashMap::new(),
            parser,
        }
    }

    async fn spawn_agent(&mut self, agent_id: Uuid, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = Command::new(command)
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();

        // Create agent state
        let agent = AgentRuntimeState::new(
            agent_id,
            format!("agent-{}", agent_id),
            "Task description".to_string(),
            "anthropic".to_string(),
        );
        self.parser.add_agent(agent);

        // Store handle
        self.agents.insert(agent_id, AgentHandle {
            process: child,
            status: AgentStatus::Idle,
        });

        // Process stdout in background
        let parser_ref = &mut self.parser;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            // Note: In practice, you'd need to share parser across tasks
            // This is simplified for illustration
        });

        Ok(())
    }

    fn get_active_agents(&self) -> Vec<Uuid> {
        self.parser
            .agents()
            .iter()
            .filter(|(_, state)| state.is_active())
            .map(|(id, _)| *id)
            .collect()
    }

    fn get_agent_state(&self, agent_id: &Uuid) -> Option<&AgentRuntimeState> {
        self.parser.get_agent(agent_id)
    }
}
```

### Pattern 3: WebSocket Stream Relay

```rust
use tokio_tungstenite::WebSocketStream;
use futures::{SinkExt, StreamExt};

struct WebSocketRelay {
    parser: AgentStreamParser,
}

impl WebSocketRelay {
    fn new() -> Self {
        Self {
            parser: AgentStreamParser::new(),
        }
    }

    async fn relay_to_websocket(
        &mut self,
        agent_stream: impl AsyncRead + Unpin,
        ws_stream: WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Custom handler that forwards to WebSocket
        struct WsHandler {
            ws_tx: mpsc::UnboundedSender<String>,
        }

        impl StreamHandler for WsHandler {
            fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, timestamp: DateTime<Utc>) {
                let msg = serde_json::json!({
                    "type": "status",
                    "agent_id": agent_id,
                    "status": status,
                    "timestamp": timestamp,
                });
                self.ws_tx.send(msg.to_string()).ok();
            }

            fn on_thought_update(&mut self, agent_id: Uuid, thought: String, timestamp: DateTime<Utc>) {
                let msg = serde_json::json!({
                    "type": "thought",
                    "agent_id": agent_id,
                    "thought": thought,
                    "timestamp": timestamp,
                });
                self.ws_tx.send(msg.to_string()).ok();
            }

            // ... implement other handlers similarly
        }

        // Set up channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.parser.register_handler(WsHandler { ws_tx: tx });

        // Process stream
        let parse_task = tokio::spawn(async move {
            self.parser.process_stream(agent_stream).await
        });

        // Forward to WebSocket
        let (mut ws_tx, _ws_rx) = ws_stream.split();
        while let Some(msg) = rx.recv().await {
            ws_tx.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await?;
        }

        parse_task.await??;
        Ok(())
    }
}
```

### Pattern 4: Database Storage

```rust
use sqlx::SqlitePool;

struct DatabaseHandler {
    pool: SqlitePool,
}

impl DatabaseHandler {
    async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    async fn insert_event(&self, agent_id: Uuid, event_type: &str, data: serde_json::Value, timestamp: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO agent_events (agent_id, event_type, data, timestamp) VALUES (?, ?, ?, ?)"
        )
        .bind(agent_id.to_string())
        .bind(event_type)
        .bind(data.to_string())
        .bind(timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl StreamHandler for DatabaseHandler {
    fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, timestamp: DateTime<Utc>) {
        let data = serde_json::json!({ "status": status });
        tokio::spawn({
            let pool = self.pool.clone();
            async move {
                // Store in database
                sqlx::query("INSERT INTO agent_events (agent_id, event_type, data, timestamp) VALUES (?, ?, ?, ?)")
                    .bind(agent_id.to_string())
                    .bind("status_update")
                    .bind(data.to_string())
                    .bind(timestamp.to_rfc3339())
                    .execute(&pool)
                    .await
                    .ok();
            }
        });
    }

    // ... implement other handlers similarly
}
```

---

## Advanced Scenarios

### Scenario 1: Filtering Messages by Agent

```rust
struct AgentFilterHandler {
    target_agent_id: Uuid,
}

impl StreamHandler for AgentFilterHandler {
    fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, timestamp: DateTime<Utc>) {
        if agent_id == self.target_agent_id {
            println!("Target agent status: {}", status);
        }
    }

    // ... only process messages from target agent
}
```

### Scenario 2: Aggregating Statistics

```rust
use std::time::Duration;

struct StatsAggregator {
    window_start: DateTime<Utc>,
    window_duration: Duration,
    message_count: usize,
    error_count: usize,
}

impl StatsAggregator {
    fn new(window_duration: Duration) -> Self {
        Self {
            window_start: Utc::now(),
            window_duration,
            message_count: 0,
            error_count: 0,
        }
    }

    fn check_window(&mut self, timestamp: DateTime<Utc>) {
        if timestamp.signed_duration_since(self.window_start) > chrono::Duration::from_std(self.window_duration).unwrap() {
            // Window expired, report and reset
            println!("Window stats: {} messages, {} errors", self.message_count, self.error_count);
            self.message_count = 0;
            self.error_count = 0;
            self.window_start = timestamp;
        }
    }
}

impl StreamHandler for StatsAggregator {
    fn on_status_update(&mut self, _agent_id: Uuid, _status: AgentStatus, timestamp: DateTime<Utc>) {
        self.check_window(timestamp);
        self.message_count += 1;
    }

    fn on_error(&mut self, _agent_id: Uuid, _error: AgentError, timestamp: DateTime<Utc>) {
        self.check_window(timestamp);
        self.message_count += 1;
        self.error_count += 1;
    }

    // ... implement other handlers
}
```

### Scenario 3: Rate Limiting

```rust
use std::time::Instant;

struct RateLimitedHandler {
    last_output: Instant,
    min_interval: Duration,
}

impl RateLimitedHandler {
    fn new(min_interval: Duration) -> Self {
        Self {
            last_output: Instant::now(),
            min_interval,
        }
    }

    fn should_output(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_output) >= self.min_interval {
            self.last_output = now;
            true
        } else {
            false
        }
    }
}

impl StreamHandler for RateLimitedHandler {
    fn on_progress_update(&mut self, agent_id: Uuid, progress: AgentProgress, _timestamp: DateTime<Utc>) {
        if self.should_output() {
            println!("Agent {} progress: {:.1}%", agent_id, progress.percentage);
        }
    }

    // ... other handlers
}
```

---

## Testing Utilities

### Mock Stream Generator

```rust
fn generate_mock_stream(agent_id: Uuid, message_count: usize) -> Vec<String> {
    let mut messages = Vec::new();

    // Initial status
    messages.push(format!(
        r#"{{"type":"status_update","agent_id":"{}","status":"initializing","timestamp":"2025-11-24T05:53:00Z"}}"#,
        agent_id
    ));

    // Progress updates
    for i in 1..=message_count {
        let percentage = (i as f32 / message_count as f32) * 100.0;
        messages.push(format!(
            r#"{{"type":"progress_update","agent_id":"{}","progress":{{"percentage":{}}},"timestamp":"2025-11-24T05:53:{}Z"}}"#,
            agent_id, percentage, i
        ));
    }

    // Completion
    messages.push(format!(
        r#"{{"type":"status_update","agent_id":"{}","status":"completed","timestamp":"2025-11-24T05:53:{}Z"}}"#,
        agent_id, message_count + 1
    ));

    messages
}

#[test]
fn test_mock_stream() {
    let agent_id = Uuid::new_v4();
    let messages = generate_mock_stream(agent_id, 10);

    let mut parser = AgentStreamParser::new();
    let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
    parser.process_lines(refs).unwrap();

    let agent = parser.get_agent(&agent_id).unwrap();
    assert_eq!(agent.status, AgentStatus::Completed);
}
```

### Test Harness

```rust
struct TestHarness {
    parser: AgentStreamParser,
    events: Arc<Mutex<Vec<String>>>,
}

impl TestHarness {
    fn new() -> Self {
        let events = Arc::new(Mutex::new(Vec::new()));

        struct TestHandler {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl StreamHandler for TestHandler {
            fn on_status_update(&mut self, agent_id: Uuid, status: AgentStatus, _timestamp: DateTime<Utc>) {
                self.events.lock().unwrap().push(format!("status:{}:{}", agent_id, status));
            }
            // ... other handlers
        }

        let mut parser = AgentStreamParser::new();
        parser.register_handler(TestHandler { events: events.clone() });

        Self { parser, events }
    }

    fn get_events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}
```

---

## Performance Tuning

### Configuration Tuning

```rust
use descartes_core::agent_stream_parser::ParserConfig;

// High-throughput configuration
let high_throughput = ParserConfig {
    max_line_length: 64 * 1024,      // 64 KB
    skip_invalid_json: true,          // Don't fail on errors
    auto_create_agents: true,         // Reduce lookups
    buffer_capacity: 64 * 1024,       // Large buffer
};

// Low-latency configuration
let low_latency = ParserConfig {
    max_line_length: 4 * 1024,        // 4 KB
    skip_invalid_json: false,         // Fail fast
    auto_create_agents: false,        // Strict validation
    buffer_capacity: 1024,            // Small buffer
};

// Memory-constrained configuration
let memory_constrained = ParserConfig {
    max_line_length: 1024,            // 1 KB
    skip_invalid_json: true,
    auto_create_agents: false,
    buffer_capacity: 512,             // Tiny buffer
};
```

### Batch Processing

```rust
// Process messages in batches for better throughput
async fn process_batch(parser: &mut AgentStreamParser, batch: Vec<String>) {
    let refs: Vec<&str> = batch.iter().map(|s| s.as_str()).collect();
    parser.process_lines(refs).ok();
}
```

---

## Conclusion

This guide provides comprehensive examples for integrating the Agent Stream Parser into various scenarios. The parser is designed to be flexible and extensible, allowing you to build custom handlers for your specific use case.

For more information, see:
- `/home/user/descartes/descartes/core/src/agent_stream_parser.rs` - Implementation
- `/home/user/descartes/working_docs/implementation/PHASE3_5_2_COMPLETION_REPORT.md` - Completion report
- `/home/user/descartes/working_docs/implementation/AGENT_STATUS_MODELS.md` - Status models documentation
