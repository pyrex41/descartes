# ZMQ Transport Deployment Guide

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Deployment Scenarios](#deployment-scenarios)
6. [Best Practices](#best-practices)
7. [Monitoring and Observability](#monitoring-and-observability)
8. [Troubleshooting](#troubleshooting)
9. [Performance Tuning](#performance-tuning)
10. [Security](#security)

---

## Overview

The Descartes ZMQ transport layer provides a robust, scalable solution for distributed agent orchestration. This guide covers production deployment strategies, configuration, and best practices.

### Key Features

- **Distributed Agent Management**: Spawn and control agents across multiple servers
- **High Performance**: MessagePack serialization and efficient ZeroMQ transport
- **Fault Tolerance**: Automatic reconnection and error recovery
- **Scalability**: Support for hundreds of concurrent agents
- **Monitoring**: Built-in health checks and statistics

### Architecture Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Applications                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ ZmqClient 1  │  │ ZmqClient 2  │  │ ZmqClient N  │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
└─────────┼──────────────────┼──────────────────┼─────────────┘
          │                  │                  │
          │    ZMQ REQ/REP   │                  │
          ▼                  ▼                  ▼
┌─────────────────────────────────────────────────────────────┐
│                      ZmqAgentServer                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Request Handler                      │  │
│  │  • SpawnRequest    → LocalProcessRunner               │  │
│  │  • ControlCommand  → Agent Lifecycle                  │  │
│  │  • ListAgents      → Agent Registry                   │  │
│  │  • HealthCheck     → Server Statistics                │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              LocalProcessRunner                        │  │
│  │  Manages spawned agent processes                      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Installation

### Prerequisites

- Rust 1.70 or later
- ZeroMQ 4.x library (`libzmq`)
- Linux, macOS, or Windows

### Installing ZeroMQ

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install libzmq3-dev
```

#### macOS
```bash
brew install zeromq
```

#### Windows
Download pre-built binaries from [ZeroMQ downloads](https://zeromq.org/download/)

### Building from Source

```bash
cd descartes/core
cargo build --release --features zmq
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests (requires ZMQ server)
cargo test --test zmq_integration_tests -- --ignored

# Distributed integration tests
cargo test --test zmq_distributed_integration_tests -- --ignored
```

---

## Configuration

### Basic Configuration

Create a configuration file based on the example:

```bash
cp examples/zmq_deployment_config.toml config/production.toml
```

Edit the configuration for your environment:

```toml
[server]
server_id = "prod-server-01"
endpoint = "tcp://0.0.0.0:5555"
max_agents = 100

[client]
endpoint = "tcp://server-hostname:5555"
auto_reconnect = true
```

### Environment Variables

Override configuration with environment variables:

```bash
export DESCARTES_SERVER_ENDPOINT="tcp://0.0.0.0:5555"
export DESCARTES_MAX_AGENTS=200
export DESCARTES_LOG_LEVEL=info
```

### Configuration Precedence

1. Environment variables (highest priority)
2. Configuration file
3. Default values (lowest priority)

---

## Deployment Scenarios

### Scenario 1: Single Server Deployment

**Use Case**: Development, testing, small-scale production

```rust
use descartes_core::{ZmqAgentServer, ZmqServerConfig};

#[tokio::main]
async fn main() {
    let config = ZmqServerConfig {
        endpoint: "tcp://0.0.0.0:5555".to_string(),
        max_agents: 50,
        ..Default::default()
    };

    let server = ZmqAgentServer::new(config);
    server.start().await.unwrap();
}
```

### Scenario 2: Multi-Client Deployment

**Use Case**: Multiple applications spawning agents on shared server

```rust
use descartes_core::{ZmqClient, ZmqRunnerConfig, AgentConfig};

#[tokio::main]
async fn main() {
    let config = ZmqRunnerConfig {
        endpoint: "tcp://agent-server:5555".to_string(),
        auto_reconnect: true,
        ..Default::default()
    };

    let client = ZmqClient::new(config);
    client.connect("tcp://agent-server:5555").await.unwrap();

    // Spawn agents
    let agent = client.spawn_remote(agent_config, None).await.unwrap();
}
```

### Scenario 3: Load-Balanced Multi-Server

**Use Case**: High availability, horizontal scaling

```
┌─────────────┐
│ Load        │
│ Balancer    │
└──────┬──────┘
       │
   ────┼────────────────
   │   │              │
   ▼   ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Server 1 │  │ Server 2 │  │ Server 3 │
└──────────┘  └──────────┘  └──────────┘
```

**Configuration:**

```toml
[load_balancing]
enable_load_balancing = true
strategy = "least-loaded"
server_endpoints = [
    "tcp://server1:5555",
    "tcp://server2:5555",
    "tcp://server3:5555"
]
```

### Scenario 4: Docker Deployment

**Dockerfile:**

```dockerfile
FROM rust:1.70 as builder
RUN apt-get update && apt-get install -y libzmq3-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libzmq5
COPY --from=builder /app/target/release/descartes-server /usr/local/bin/
EXPOSE 5555
CMD ["descartes-server"]
```

**docker-compose.yml:**

```yaml
version: '3.8'

services:
  agent-server:
    build: .
    ports:
      - "5555:5555"
    environment:
      - DESCARTES_SERVER_ENDPOINT=tcp://0.0.0.0:5555
      - DESCARTES_MAX_AGENTS=100
      - RUST_LOG=info
    volumes:
      - agent-data:/var/lib/descartes
    restart: unless-stopped

volumes:
  agent-data:
```

### Scenario 5: Kubernetes Deployment

**deployment.yaml:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: descartes-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: descartes-server
  template:
    metadata:
      labels:
        app: descartes-server
    spec:
      containers:
      - name: server
        image: descartes-server:latest
        ports:
        - containerPort: 5555
        env:
        - name: DESCARTES_SERVER_ENDPOINT
          value: "tcp://0.0.0.0:5555"
        - name: DESCARTES_MAX_AGENTS
          value: "100"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          tcpSocket:
            port: 5555
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          tcpSocket:
            port: 5555
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: descartes-server
spec:
  selector:
    app: descartes-server
  ports:
  - port: 5555
    targetPort: 5555
  type: ClusterIP
```

---

## Best Practices

### 1. Connection Management

**DO:**
- Enable auto-reconnect for clients
- Set appropriate timeouts based on workload
- Implement exponential backoff for retries
- Handle connection failures gracefully

```rust
let config = ZmqRunnerConfig {
    auto_reconnect: true,
    max_reconnect_attempts: 5,
    reconnect_delay_secs: 2,
    ..Default::default()
};
```

**DON'T:**
- Use infinite timeouts
- Ignore connection errors
- Retry indefinitely without backoff

### 2. Resource Management

**DO:**
- Set max_agents based on server capacity
- Monitor memory and CPU usage
- Implement agent lifecycle limits
- Clean up completed agents

```rust
let config = ZmqServerConfig {
    max_agents: 100,
    runner_config: ProcessRunnerConfig {
        max_concurrent_agents: Some(100),
        ..Default::default()
    },
    ..Default::default()
};
```

**DON'T:**
- Allow unlimited agents
- Forget to stop agents
- Ignore resource exhaustion

### 3. Error Handling

**DO:**
- Check all operation results
- Log errors with context
- Implement retry logic for transient failures
- Use circuit breakers for persistent failures

```rust
match client.spawn_remote(config, Some(300)).await {
    Ok(agent) => {
        info!("Agent spawned: {}", agent.id);
    }
    Err(e) => {
        error!("Failed to spawn agent: {}", e);
        // Implement retry or fallback logic
    }
}
```

**DON'T:**
- Silently ignore errors
- Retry non-transient errors
- Let errors crash the application

### 4. Security

**DO:**
- Enable encryption in production
- Use authentication for multi-tenant setups
- Validate all input
- Rotate credentials regularly

```toml
[security]
enable_encryption = true
enable_auth = true
auth_type = "curve"
```

**DON'T:**
- Run without encryption over public networks
- Hard-code credentials
- Trust client input

### 5. Monitoring

**DO:**
- Export metrics (Prometheus, StatsD)
- Set up health check endpoints
- Monitor agent lifecycle events
- Track server statistics

```rust
// Regular health checks
let health = client.health_check().await?;
if !health.healthy {
    alert("Server unhealthy!");
}

// Monitor server stats
let stats = server.stats();
metrics.gauge("active_agents", stats.active_agents);
```

**DON'T:**
- Deploy without monitoring
- Ignore health check failures
- Skip alerting configuration

### 6. Testing

**DO:**
- Test in staging environment first
- Run load tests before production
- Test failure scenarios
- Verify recovery procedures

```bash
# Run load test
cargo run --example zmq_deployment_poc

# Run integration tests
cargo test --test zmq_distributed_integration_tests -- --ignored
```

**DON'T:**
- Deploy untested configurations
- Skip load testing
- Assume everything works

---

## Monitoring and Observability

### Health Checks

Implement regular health checks:

```rust
use tokio::time::{interval, Duration};

let mut health_check_interval = interval(Duration::from_secs(30));

loop {
    health_check_interval.tick().await;

    match client.health_check().await {
        Ok(health) => {
            if health.healthy {
                info!("Server healthy: {} agents", health.active_agents.unwrap_or(0));
            } else {
                warn!("Server unhealthy!");
            }
        }
        Err(e) => {
            error!("Health check failed: {}", e);
        }
    }
}
```

### Metrics Collection

Export Prometheus metrics:

```rust
use prometheus::{Registry, Counter, Gauge};

let registry = Registry::new();

let spawn_counter = Counter::new("agent_spawns_total", "Total agent spawns")?;
let active_agents = Gauge::new("active_agents", "Currently active agents")?;

registry.register(Box::new(spawn_counter.clone()))?;
registry.register(Box::new(active_agents.clone()))?;

// Update metrics
spawn_counter.inc();
active_agents.set(server.active_agent_count() as f64);
```

### Logging

Configure structured logging:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .json()
    .with_max_level(tracing::Level::INFO)
    .with_current_span(false)
    .init();
```

### Alerting

Set up alerts for critical conditions:

- Server unhealthy for > 5 minutes
- Agent spawn failure rate > 10%
- Active agents approaching max_agents
- Memory usage > 90%
- Request timeout rate > 5%

---

## Troubleshooting

### Common Issues

#### 1. Connection Refused

**Symptoms**: Client cannot connect to server

**Causes**:
- Server not running
- Firewall blocking port
- Wrong endpoint configuration

**Solutions**:
```bash
# Check if server is running
netstat -an | grep 5555

# Test connection
telnet server-hostname 5555

# Check firewall
sudo ufw status
```

#### 2. Agent Spawn Failures

**Symptoms**: Agents fail to spawn

**Causes**:
- Resource exhaustion
- Invalid configuration
- Timeout too short

**Solutions**:
```rust
// Check server capacity
let stats = server.stats();
println!("Active agents: {}/{}",
    server.active_agent_count(),
    server.config.max_agents);

// Increase timeout
client.spawn_remote(config, Some(300)).await?;
```

#### 3. Slow Performance

**Symptoms**: Operations taking longer than expected

**Causes**:
- Server overloaded
- Network latency
- Large message payloads

**Solutions**:
```toml
# Tune timeouts
[client]
request_timeout_secs = 120

# Increase server capacity
[server]
max_agents = 200

# Monitor performance
[monitoring]
enable_metrics = true
```

#### 4. Memory Leaks

**Symptoms**: Memory usage growing over time

**Causes**:
- Agents not cleaned up
- Large output buffers
- Message queue buildup

**Solutions**:
```rust
// Clean up completed agents
for agent_id in completed_agents {
    client.stop_agent(&agent_id).await?;
}

// Configure buffer limits
[limits]
max_output_buffer_bytes = 10485760
```

---

## Performance Tuning

### Server Optimization

1. **Tune max_agents**: Based on available CPU cores
   ```toml
   max_agents = num_cpus * 10
   ```

2. **Adjust timeouts**: Match your workload characteristics
   ```toml
   request_timeout_secs = 120  # For long-running operations
   ```

3. **Configure message buffers**: Prevent message loss
   ```toml
   [network]
   send_hwm = 1000
   receive_hwm = 1000
   ```

### Client Optimization

1. **Connection pooling**: Reuse connections
   ```rust
   let client = Arc::new(ZmqClient::new(config));
   // Share client across threads
   ```

2. **Batch operations**: Reduce round trips
   ```rust
   client.batch_control(agent_ids, ControlCommandType::Pause, None, false).await?;
   ```

3. **Async operations**: Don't block
   ```rust
   let spawn_futures: Vec<_> = configs
       .into_iter()
       .map(|config| client.spawn_remote(config, None))
       .collect();

   let results = futures::future::join_all(spawn_futures).await;
   ```

### Network Optimization

1. **Use Unix domain sockets**: For local communication
   ```toml
   endpoint = "ipc:///tmp/descartes.sock"
   ```

2. **Tune ZMQ options**: Based on network characteristics
   ```toml
   [network]
   send_timeout_ms = 5000
   receive_timeout_ms = 5000
   ```

---

## Security

### Encryption

Enable ZMQ CURVE encryption:

```rust
// Generate key pair
let (public_key, secret_key) = zmq::curve_keypair()?;

// Server configuration
let config = ZmqServerConfig {
    security: SecurityConfig {
        enable_encryption: true,
        server_secret_key: secret_key,
        ..Default::default()
    },
    ..Default::default()
};
```

### Authentication

Implement token-based authentication:

```rust
// Client sends auth token
let metadata = HashMap::from([
    ("auth_token".to_string(), token.to_string())
]);

let request = SpawnRequest {
    metadata: Some(metadata),
    ..request
};

// Server validates token
if !validate_token(&request.metadata) {
    return Err(AgentError::Unauthorized);
}
```

### Network Isolation

Use network policies in Kubernetes:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: descartes-policy
spec:
  podSelector:
    matchLabels:
      app: descartes-server
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          role: client
    ports:
    - protocol: TCP
      port: 5555
```

---

## Running the POC

### Quick Start

1. **Start the server**:
   ```bash
   cargo run --example zmq_server_example
   ```

2. **In another terminal, run the POC**:
   ```bash
   cargo run --example zmq_deployment_poc
   ```

3. **Monitor output** for each scenario demonstration

### Running Integration Tests

```bash
# All integration tests
cargo test --test zmq_distributed_integration_tests -- --ignored --test-threads=1

# Specific test
cargo test --test zmq_distributed_integration_tests test_multiple_clients_single_server -- --ignored
```

---

## Additional Resources

- **ZeroMQ Guide**: https://zguide.zeromq.org/
- **Rust Async Book**: https://rust-lang.github.io/async-book/
- **Descartes Documentation**: [Internal docs]
- **Issue Tracker**: [GitHub issues]

---

## Support

For questions or issues:

1. Check this guide first
2. Review the examples in `examples/`
3. Run the POC to understand behavior
4. Open an issue with detailed reproduction steps
