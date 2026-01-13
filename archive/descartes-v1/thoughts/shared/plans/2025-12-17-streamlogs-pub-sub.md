---
date: 2025-12-17
status: completed
related_plan: 2025-12-15-critical-features.md
---

# StreamLogs PUB/SUB Implementation Plan

This plan implements real-time log streaming via ZMQ PUB/SUB, the final deferred feature from the critical features plan.

## Background

The current ZMQ infrastructure uses REQ/REP (request-response) for all communication:
- Client sends request → Server processes → Server sends response
- Synchronous, blocking pattern
- Works for commands but not for continuous streaming

**StreamLogs requires PUB/SUB (publish-subscribe):**
- Server publishes logs as they arrive
- Clients subscribe to specific agent topics
- Asynchronous, non-blocking pattern
- Multiple clients can receive same logs

### Existing Infrastructure We Build On

1. **`LocalAgentHandle` already has broadcast channels** (`agent_runner.rs:736-738`):
   - `stdout_broadcast: broadcast::Sender<Vec<u8>>`
   - `stderr_broadcast: broadcast::Sender<Vec<u8>>`
   - Background tasks forward output line-by-line

2. **`StatusUpdate` message type exists** (`zmq_agent_runner.rs:281-303`):
   - Already designed for streaming updates
   - Includes `StatusUpdateType::OutputAvailable`

3. **`ZmqMessage::StatusUpdate` variant exists** (`zmq_agent_runner.rs:555`):
   - Can be reused for log streaming

4. **Server has status update task** (`zmq_server.rs:860-893`):
   - Comment at line 882: "In a real implementation, we would broadcast this via PUB socket"
   - Placeholder infrastructure ready

---

## Scope

### What We're Implementing

1. **PUB/SUB socket types** in ZMQ communication layer
2. **Secondary PUB socket** on server for log broadcasting
3. **Log forwarding** from agent stdout/stderr to PUB socket
4. **SUB socket client** with topic filtering by agent_id
5. **StreamLogs command** to initiate subscription

### What We're NOT Implementing

- Log persistence/replay (use QueryOutput for historical)
- Log level filtering (send everything, filter client-side)
- Backpressure/flow control (ZMQ handles via HWM)
- Web interface for log viewing

---

## Implementation Phases

### Phase 1: PUB/SUB Socket Types

**Goal**: Add PUB/SUB support to the ZMQ communication layer.

**Files to modify:**

1. **`core/src/zmq_communication.rs`**

   Add socket types to enum (around line 102-113):
   ```rust
   pub enum SocketType {
       Req,
       Rep,
       Dealer,
       Router,
       Pub,    // NEW
       Sub,    // NEW
   }
   ```

   Add socket wrappers (similar to `ReqSocketWrapper`/`RepSocketWrapper`):
   ```rust
   struct PubSocketWrapper {
       socket: zeromq::PubSocket,
   }

   struct SubSocketWrapper {
       socket: zeromq::SubSocket,
   }
   ```

   Implement `SocketWrapper` trait for both:
   - `PubSocketWrapper::send()` - broadcasts to all subscribers
   - `SubSocketWrapper::recv()` - receives from publisher
   - `SubSocketWrapper::subscribe(topic)` - subscribe to topic filter

   Update `ZmqConnection::connect()` (line 246-275) to handle new types.

2. **`core/src/zmq_agent_runner.rs`**

   Add `LogStreamMessage` for log data (around line 280):
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct LogStreamMessage {
       pub agent_id: Uuid,
       pub stream_type: LogStreamType,
       pub data: Vec<u8>,
       pub timestamp: SystemTime,
       pub sequence: u64,  // For ordering
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum LogStreamType {
       Stdout,
       Stderr,
   }
   ```

   Add to `ZmqMessage` enum (around line 539):
   ```rust
   LogStream(LogStreamMessage),
   ```

**Success Criteria:**
- [ ] `cargo check` passes with new socket types
- [ ] Unit tests for PubSocketWrapper and SubSocketWrapper

---

### Phase 2: Server PUB Socket

**Goal**: Add secondary PUB socket to ZmqAgentServer for broadcasting logs.

**Files to modify:**

1. **`core/src/zmq_server.rs`**

   Add PUB socket to server struct (around line 41-49):
   ```rust
   pub struct ZmqAgentServer {
       // ... existing fields
       pub_socket: Option<Arc<Mutex<ZmqConnection>>>,  // NEW
       pub_endpoint: String,  // NEW - e.g., "tcp://0.0.0.0:5556"
   }
   ```

   Update `ZmqServerConfig` (line 70-101):
   ```rust
   pub struct ZmqServerConfig {
       // ... existing fields
       pub pub_endpoint: Option<String>,  // None = disabled
   }
   ```

   Modify `ZmqAgentServer::new()` (line 143) to create PUB socket:
   ```rust
   let pub_socket = if let Some(pub_endpoint) = &config.pub_endpoint {
       let connection = ZmqConnection::new(SocketType::Pub)?;
       connection.bind(pub_endpoint).await?;
       Some(Arc::new(Mutex::new(connection)))
   } else {
       None
   };
   ```

   Add method to publish log message:
   ```rust
   async fn publish_log(&self, message: LogStreamMessage) -> AgentResult<()> {
       if let Some(pub_socket) = &self.pub_socket {
           let topic = message.agent_id.to_string();
           let data = serialize_zmq_message(&ZmqMessage::LogStream(message))?;
           pub_socket.lock().await.send_with_topic(&topic, &data).await?;
       }
       Ok(())
   }
   ```

2. **`core/src/zmq_communication.rs`**

   Add `send_with_topic()` method to `ZmqConnection`:
   ```rust
   pub async fn send_with_topic(&self, topic: &str, data: &[u8]) -> AgentResult<()> {
       // PUB socket: prepend topic as first frame
       // Format: [topic][data]
   }
   ```

**Success Criteria:**
- [ ] `cargo check` passes
- [ ] Server starts with PUB socket bound
- [ ] Can call `publish_log()` without errors

---

### Phase 3: Agent Output Forwarding

**Goal**: Forward agent stdout/stderr to the PUB socket.

**Files to modify:**

1. **`core/src/zmq_server.rs`**

   The key insight is that `ZmqAgentServer` uses `LocalProcessRunner` internally. We need to:

   a. Subscribe to the agent's broadcast channels when spawned
   b. Forward received data to the PUB socket

   In `handle_spawn_request()` (around line 400), after spawning:
   ```rust
   // Subscribe to agent output broadcasts
   if let Some(handle) = self.runner.agents.get(&agent_id) {
       let stdout_rx = handle.subscribe_stdout();
       let stderr_rx = handle.subscribe_stderr();

       self.spawn_output_forwarder(agent_id, stdout_rx, LogStreamType::Stdout);
       self.spawn_output_forwarder(agent_id, stderr_rx, LogStreamType::Stderr);
   }
   ```

   Add forwarder task:
   ```rust
   fn spawn_output_forwarder(
       &self,
       agent_id: Uuid,
       mut rx: broadcast::Receiver<Vec<u8>>,
       stream_type: LogStreamType,
   ) {
       let pub_socket = self.pub_socket.clone();
       let mut sequence = 0u64;

       tokio::spawn(async move {
           while let Ok(data) = rx.recv().await {
               if let Some(socket) = &pub_socket {
                   let msg = LogStreamMessage {
                       agent_id,
                       stream_type: stream_type.clone(),
                       data,
                       timestamp: SystemTime::now(),
                       sequence,
                   };
                   sequence += 1;

                   let topic = agent_id.to_string();
                   // Publish to PUB socket
                   let _ = socket.lock().await.send_with_topic(&topic, &serialize(&msg));
               }
           }
       });
   }
   ```

**Success Criteria:**
- [ ] `cargo check` passes
- [ ] Spawned agent output is published to PUB socket
- [ ] No performance regression in agent spawning

---

### Phase 4: Client SUB Socket

**Goal**: Implement client-side subscription to log streams.

**Files to modify:**

1. **`core/src/zmq_client.rs`**

   Add SUB socket to client (around line 50):
   ```rust
   pub struct ZmqClient {
       // ... existing fields
       sub_socket: Option<Arc<Mutex<ZmqConnection>>>,
       log_subscribers: Arc<RwLock<HashMap<Uuid, Vec<mpsc::UnboundedSender<LogStreamMessage>>>>>,
   }
   ```

   Update `ZmqClientConfig` or constructor to accept SUB endpoint.

   Add subscription method:
   ```rust
   pub async fn subscribe_logs(
       &self,
       agent_id: Uuid,
   ) -> AgentResult<mpsc::UnboundedReceiver<LogStreamMessage>> {
       let (tx, rx) = mpsc::unbounded_channel();

       // Add to subscribers
       self.log_subscribers.write().entry(agent_id).or_default().push(tx);

       // Ensure SUB socket is subscribed to this agent's topic
       if let Some(sub_socket) = &self.sub_socket {
           sub_socket.lock().await.subscribe(&agent_id.to_string()).await?;
       }

       Ok(rx)
   }
   ```

   Add background task to receive and dispatch:
   ```rust
   fn spawn_sub_receiver(&self) {
       let sub_socket = self.sub_socket.clone();
       let subscribers = self.log_subscribers.clone();

       tokio::spawn(async move {
           loop {
               if let Some(socket) = &sub_socket {
                   if let Ok((topic, data)) = socket.lock().await.recv_with_topic().await {
                       if let Ok(ZmqMessage::LogStream(msg)) = deserialize(&data) {
                           if let Some(senders) = subscribers.read().get(&msg.agent_id) {
                               for tx in senders {
                                   let _ = tx.send(msg.clone());
                               }
                           }
                       }
                   }
               }
           }
       });
   }
   ```

**Success Criteria:**
- [ ] `cargo check` passes
- [ ] Client can subscribe to agent logs
- [ ] Received logs are dispatched to correct subscriber

---

### Phase 5: StreamLogs Command Handler

**Goal**: Implement the StreamLogs ZMQ command for initiating subscriptions.

**Files to modify:**

1. **`core/src/zmq_server.rs`**

   Replace placeholder at line 730-735:
   ```rust
   ControlCommandType::StreamLogs => {
       // StreamLogs via REQ/REP returns subscription info
       // Actual streaming happens via PUB/SUB socket

       let pub_endpoint = self.pub_endpoint.clone();

       if pub_endpoint.is_empty() {
           return CommandResponse::error(
               cmd.request_id.clone(),
               "Log streaming not enabled on this server".to_string(),
           );
       }

       CommandResponse::success(
           cmd.request_id.clone(),
           Some(json!({
               "pub_endpoint": pub_endpoint,
               "topic": agent_id.to_string(),
               "instructions": "Connect SUB socket to pub_endpoint, subscribe to topic"
           })),
       )
   }
   ```

2. **`core/src/zmq_client.rs`**

   Add convenience method:
   ```rust
   pub async fn stream_logs(&self, agent_id: Uuid) -> AgentResult<impl Stream<Item = LogStreamMessage>> {
       // First, query server for PUB endpoint via REQ/REP
       let response = self.send_command(ControlCommand {
           command_type: ControlCommandType::StreamLogs,
           agent_id,
           ..
       }).await?;

       // Then subscribe via SUB socket
       let rx = self.subscribe_logs(agent_id).await?;

       Ok(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
   }
   ```

**Success Criteria:**
- [ ] `cargo check` passes
- [ ] `StreamLogs` command returns subscription info
- [ ] Client can stream logs end-to-end

---

### Phase 6: Integration Tests

**Goal**: Verify end-to-end functionality.

**Files to create:**

1. **`core/src/zmq_server.rs` (add tests)**

   ```rust
   #[cfg(test)]
   mod stream_logs_tests {
       #[tokio::test]
       async fn test_pub_sub_log_streaming() {
           // 1. Start server with PUB socket
           // 2. Create client with SUB socket
           // 3. Spawn agent that produces output
           // 4. Subscribe to agent's logs
           // 5. Verify logs received via subscription
       }

       #[tokio::test]
       async fn test_multiple_subscribers() {
           // 1. Start server
           // 2. Create multiple clients
           // 3. All subscribe to same agent
           // 4. Verify all clients receive same logs
       }

       #[tokio::test]
       async fn test_topic_filtering() {
           // 1. Start server
           // 2. Spawn two agents
           // 3. Subscribe to only one agent's topic
           // 4. Verify only subscribed agent's logs received
       }
   }
   ```

**Success Criteria:**
- [ ] All new tests pass
- [ ] No regression in existing 382 tests
- [ ] `cargo test -p descartes-core --lib` passes

---

## Success Criteria Summary

### Automated Verification

```bash
cargo check                           # Workspace compiles
cargo test -p descartes-core --lib    # All tests pass (382 + new)
cargo clippy -p descartes-core        # No new warnings
```

### Manual Verification

1. **Server startup**: `ZmqAgentServer` starts with both REP and PUB sockets
2. **Client connection**: Client connects to both REQ and SUB endpoints
3. **Log streaming**: Spawn agent, subscribe, see stdout/stderr in real-time
4. **Multiple subscribers**: Two clients subscribing see same logs
5. **Topic isolation**: Subscribing to agent A doesn't receive agent B's logs

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| ZMQ PUB/SUB complexity | Use simple topic-based filtering, no complex patterns |
| Performance impact | Use non-blocking channels, spawn dedicated tasks |
| Memory growth from slow subscribers | ZMQ high-water mark (HWM) handles backpressure |
| Breaking existing REQ/REP | PUB/SUB is additive, doesn't change existing flow |

---

## Appendix: Key File References

| File | Purpose |
|------|---------|
| `core/src/zmq_communication.rs:102-113` | SocketType enum to extend |
| `core/src/zmq_communication.rs:246-275` | Socket connection logic |
| `core/src/zmq_server.rs:41-49` | Server struct to add PUB socket |
| `core/src/zmq_server.rs:730-735` | StreamLogs placeholder to implement |
| `core/src/zmq_server.rs:860-893` | Existing status update task pattern |
| `core/src/zmq_client.rs:839-853` | Existing subscription pattern |
| `core/src/zmq_agent_runner.rs:141-179` | ControlCommandType enum |
| `core/src/zmq_agent_runner.rs:281-303` | StatusUpdate for reference |
| `core/src/agent_runner.rs:736-738` | Existing broadcast channels |
