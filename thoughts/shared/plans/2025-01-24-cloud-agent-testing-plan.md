# Cloud Agent Testing Plan - Descartes System

## Overview

This plan provides a comprehensive testing strategy for the Descartes cloud agent orchestration system. The Descartes system is a production-grade Rust framework for building, deploying, and orchestrating multi-agent AI systems with unified abstractions over multiple LLM backends (APIs, local models, CLI tools).

## Current State Analysis

### Architecture Components

The system consists of five primary workspace members:
1. **descartes-core**: Core library providing traits, providers, and orchestration utilities
2. **descartes-cli**: Command-line interface for agent management
3. **descartes-gui**: Native Iced-based GUI for visual monitoring and control
4. **descartes-daemon**: JSON-RPC daemon for remote agent control
5. **agent-runner**: Semantic code parsing and RAG capabilities

### Key Features Requiring Testing

Based on comprehensive codebase analysis, the following features need testing:

| Component | Feature | Current Test Coverage | Priority |
|-----------|---------|----------------------|----------|
| Core | Agent Runner | Partial | High |
| Core | Swarm Parser | Partial | High |
| Core | DAG Operations | Full | Medium |
| Core | State Machine | Full | Medium |
| Core | ZMQ Communication | Partial | High |
| Core | Time Travel | Partial | High |
| CLI | Init Command | Partial | Medium |
| CLI | Spawn Command | None | Critical |
| CLI | PS/Kill/Logs | Partial | Medium |
| Daemon | RPC Server | Full | Low |
| Daemon | Agent Monitor | Full | Low |
| GUI | Swarm Handler | None | High |
| GUI | DAG Editor | Partial | Medium |
| GUI | Time Travel UI | None | High |

---

## Phase 1: Core Agent Testing

### Overview
Test the fundamental agent lifecycle, spawning, and communication mechanisms.

### Changes Required:

#### 1.1 Agent Runner Integration Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/core/tests/agent_runner_integration_tests.rs` (new)

**Tests to implement**:

```rust
// Test 1: Full agent lifecycle
#[tokio::test]
async fn test_full_agent_lifecycle() {
    // Create LocalProcessRunner
    // Spawn agent with mock backend
    // Verify status transitions: Idle -> Initializing -> Running -> Completed
    // Verify cleanup on completion
}

// Test 2: Agent spawning with real-ish backends
#[tokio::test]
async fn test_spawn_echo_agent() {
    // Spawn agent using echo/cat as mock backend
    // Send input, verify output
    // Kill agent, verify termination
}

// Test 3: Multiple concurrent agents
#[tokio::test]
async fn test_concurrent_agent_spawning() {
    // Spawn 5 agents concurrently
    // Verify each has unique ID
    // Verify list_agents returns all 5
    // Kill all, verify cleanup
}

// Test 4: Agent signal handling
#[tokio::test]
async fn test_agent_signal_handling() {
    // Spawn long-running process
    // Send SIGTERM, verify graceful shutdown
    // Send SIGKILL, verify immediate termination
}

// Test 5: Health check monitoring
#[tokio::test]
async fn test_agent_health_monitoring() {
    // Spawn agent with health checks enabled
    // Verify periodic health check calls
    // Simulate unhealthy agent, verify status transition
}
```

#### 1.2 Provider Factory Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/core/tests/provider_factory_tests.rs` (new)

**Tests to implement**:

```rust
// Test 1: Create all supported providers
#[test]
fn test_create_all_providers() {
    let providers = ["openai", "anthropic", "ollama", "claude-code-cli", "deepseek", "groq"];
    // Verify each can be created with valid config
}

// Test 2: Provider config validation
#[test]
fn test_provider_config_validation() {
    // Test missing api_key
    // Test invalid endpoint
    // Test unknown provider name
}

// Test 3: Provider model resolution
#[test]
fn test_provider_model_defaults() {
    // Verify each provider has correct default model
    // Anthropic: claude-3-5-sonnet-20241022
    // OpenAI: gpt-4-turbo
    // etc.
}
```

### Success Criteria:

#### Automated Verification:
- [ ] All agent runner tests pass: `cargo test -p descartes-core agent_runner`
- [ ] Provider factory tests pass: `cargo test -p descartes-core provider_factory`
- [ ] No memory leaks detected (run with valgrind or ASAN)
- [ ] Type checking passes: `cargo check -p descartes-core`

#### Manual Verification:
- [ ] Spawn real echo agent, verify output visible
- [ ] Kill agent during execution, verify clean termination
- [ ] Run with tracing enabled, verify log output sensible

---

## Phase 2: CLI Command Testing

### Overview
Test all CLI commands end-to-end with real database and file system operations.

### Changes Required:

#### 2.1 Spawn Command Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/cli/tests/spawn_tests.rs` (new)

**Tests to implement**:

```rust
// Test 1: Spawn with explicit provider and model
#[tokio::test]
async fn test_spawn_explicit_provider() {
    // Mock API server
    // Call spawn with --provider anthropic --model claude-3-haiku
    // Verify request sent to correct endpoint
    // Verify response displayed
}

// Test 2: Spawn with stdin pipe
#[tokio::test]
async fn test_spawn_with_piped_input() {
    // Pipe text to spawn command
    // Verify text appended to task
}

// Test 3: Streaming vs non-streaming
#[tokio::test]
async fn test_spawn_streaming_output() {
    // Mock streaming API
    // Verify chunks displayed as received
}

// Test 4: Error handling
#[tokio::test]
async fn test_spawn_missing_api_key() {
    // Unset API key
    // Verify helpful error message
}

// Test 5: Provider config from file
#[tokio::test]
async fn test_spawn_uses_config_defaults() {
    // Create config with anthropic as default
    // Spawn without --provider
    // Verify anthropic used
}
```

#### 2.2 Init Command Integration Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/cli/tests/init_integration_tests.rs` (new)

```rust
// Test 1: Full initialization flow
#[tokio::test]
async fn test_init_complete_flow() {
    // Run init in temp directory
    // Verify all directories created
    // Verify database initialized with schema
    // Verify config file created
    // Verify example files exist
}

// Test 2: Init with custom name
#[tokio::test]
async fn test_init_custom_project_name() {
    // Run init --name my-custom-project
    // Verify project name in config
}

// Test 3: Idempotent initialization
#[tokio::test]
async fn test_init_twice_no_error() {
    // Run init twice
    // Verify no errors
    // Verify existing files not overwritten
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Spawn tests pass: `cargo test -p descartes-cli spawn`
- [ ] Init tests pass: `cargo test -p descartes-cli init`
- [ ] All CLI tests pass: `cargo test -p descartes-cli`
- [ ] Linting passes: `cargo clippy -p descartes-cli`

#### Manual Verification:
- [ ] Run `descartes init` in fresh directory, verify all files created
- [ ] Run `descartes spawn --task "hello" --provider ollama` with Ollama running
- [ ] Verify `descartes ps` shows running agent
- [ ] Verify `descartes logs --follow` streams events
- [ ] Verify `descartes kill <id>` terminates agent

**Implementation Note**: After completing this phase, pause for manual CLI testing to ensure commands work end-to-end before proceeding.

---

## Phase 3: GUI Component Testing

### Overview
Test the GUI components: Swarm Handler, DAG Editor, and Time Travel functionality.

### Changes Required:

#### 3.1 Swarm Handler Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/gui/tests/swarm_handler_tests.rs` (new)

```rust
// Test 1: Stream handler callbacks
#[test]
fn test_on_status_update() {
    let handler = GuiStreamHandler::new();
    let agent_id = Uuid::new_v4();

    handler.on_status_update(agent_id, AgentStatus::Running, Utc::now());

    let agents = handler.get_agents();
    assert_eq!(agents.get(&agent_id).unwrap().status, AgentStatus::Running);
}

// Test 2: Thought update auto-transitions
#[test]
fn test_thought_update_transitions_status() {
    let handler = GuiStreamHandler::new();
    let agent_id = Uuid::new_v4();

    handler.on_thought_update(agent_id, "Thinking...".to_string(), Utc::now());

    let agents = handler.get_agents();
    assert_eq!(agents.get(&agent_id).unwrap().status, AgentStatus::Thinking);
}

// Test 3: Concurrent access
#[tokio::test]
async fn test_concurrent_agent_updates() {
    let handler = Arc::new(GuiStreamHandler::new());

    let handles: Vec<_> = (0..10).map(|_| {
        let h = handler.clone();
        let id = Uuid::new_v4();
        tokio::spawn(async move {
            for _ in 0..100 {
                h.on_heartbeat(id, Utc::now());
            }
        })
    }).collect();

    futures::future::join_all(handles).await;
    // Verify no deadlock
}

// Test 4: Agent auto-creation
#[test]
fn test_auto_creates_unknown_agent() {
    let handler = GuiStreamHandler::new();
    let agent_id = Uuid::new_v4();

    // First event for unknown agent
    handler.on_heartbeat(agent_id, Utc::now());

    let agents = handler.get_agents();
    assert!(agents.contains_key(&agent_id));
    assert!(agents.get(&agent_id).unwrap().name.starts_with("agent-"));
}

// Test 5: Error handling
#[test]
fn test_on_error_sets_failed_status() {
    let handler = GuiStreamHandler::new();
    let agent_id = Uuid::new_v4();

    handler.on_error(agent_id, "Connection lost".to_string(), Utc::now());

    let agents = handler.get_agents();
    assert_eq!(agents.get(&agent_id).unwrap().status, AgentStatus::Failed);
}
```

#### 3.2 DAG Editor Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/gui/tests/dag_editor_comprehensive_tests.rs` (new)

```rust
// Test 1: Node CRUD operations
#[test]
fn test_dag_node_operations() {
    let mut state = DAGEditorState::default();

    // Add node
    state.update(DAGEditorMessage::AddNode(Point { x: 100.0, y: 100.0 }));
    assert_eq!(state.dag.nodes.len(), 1);

    // Remove node
    let node_id = state.dag.nodes.keys().next().unwrap().clone();
    state.update(DAGEditorMessage::RemoveNode(node_id));
    assert_eq!(state.dag.nodes.len(), 0);
}

// Test 2: Edge creation with cycle detection
#[test]
fn test_edge_cycle_detection() {
    let mut state = DAGEditorState::default();

    // Add three nodes
    let node1 = state.add_node(Point { x: 0.0, y: 0.0 });
    let node2 = state.add_node(Point { x: 100.0, y: 0.0 });
    let node3 = state.add_node(Point { x: 200.0, y: 0.0 });

    // Create chain: 1 -> 2 -> 3
    state.create_edge(node1, node2, EdgeType::Dependency);
    state.create_edge(node2, node3, EdgeType::Dependency);

    // Attempt to create cycle: 3 -> 1
    let result = state.create_edge(node3, node1, EdgeType::Dependency);
    assert!(result.is_err());
}

// Test 3: Undo/Redo
#[test]
fn test_undo_redo_operations() {
    let mut state = DAGEditorState::default();

    // Add node
    state.update(DAGEditorMessage::AddNode(Point { x: 100.0, y: 100.0 }));
    assert_eq!(state.dag.nodes.len(), 1);

    // Undo
    state.update(DAGEditorMessage::Undo);
    assert_eq!(state.dag.nodes.len(), 0);

    // Redo
    state.update(DAGEditorMessage::Redo);
    assert_eq!(state.dag.nodes.len(), 1);
}

// Test 4: Coordinate transformations
#[test]
fn test_coordinate_transforms() {
    let state = DAGEditorState::default();

    let screen_point = Point { x: 500.0, y: 300.0 };
    let world = state.screen_to_world(screen_point);
    let back = state.world_to_screen(world);

    assert!((screen_point.x - back.x).abs() < 0.01);
    assert!((screen_point.y - back.y).abs() < 0.01);
}

// Test 5: Box selection
#[test]
fn test_box_selection() {
    let mut state = DAGEditorState::default();

    // Add nodes in a grid
    for x in 0..3 {
        for y in 0..3 {
            state.add_node(Point {
                x: (x * 100) as f32,
                y: (y * 100) as f32
            });
        }
    }

    // Box select top-left quadrant
    state.start_box_selection(Point { x: -10.0, y: -10.0 });
    state.update_box_selection(Point { x: 160.0, y: 160.0 });
    state.finish_box_selection();

    // Should select 4 nodes (0,0), (0,1), (1,0), (1,1)
    assert_eq!(state.selected_nodes().len(), 4);
}
```

#### 3.3 Time Travel Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/gui/tests/time_travel_tests.rs` (new)

```rust
// Test 1: Event navigation
#[test]
fn test_event_navigation() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::new(events, vec![]);

    state.jump_to_event(0);
    assert_eq!(state.selected_index, Some(0));

    state.next_event();
    assert_eq!(state.selected_index, Some(1));

    state.prev_event();
    assert_eq!(state.selected_index, Some(0));
}

// Test 2: Playback controls
#[test]
fn test_playback_controls() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::new(events, vec![]);

    state.toggle_playback();
    assert!(state.playback.playing);

    state.set_speed(2.0);
    assert_eq!(state.playback.speed, 2.0);

    state.toggle_playback();
    assert!(!state.playback.playing);
}

// Test 3: Snapshot navigation
#[test]
fn test_snapshot_navigation() {
    let events = generate_sample_events(20);
    let snapshots = vec![
        HistorySnapshot { timestamp: events[5].timestamp, label: "Snapshot 1".to_string() },
        HistorySnapshot { timestamp: events[15].timestamp, label: "Snapshot 2".to_string() },
    ];
    let mut state = TimeTravelState::new(events, snapshots);

    state.jump_to_snapshot(0);
    assert_eq!(state.selected_index, Some(5));

    state.jump_to_snapshot(1);
    assert_eq!(state.selected_index, Some(15));
}

// Test 4: Time range calculation
#[test]
fn test_time_range() {
    let events = generate_sample_events(10);
    let state = TimeTravelState::new(events, vec![]);

    let (start, end) = state.time_range();
    assert!(start < end);
}

// Test 5: Zoom and scroll
#[test]
fn test_zoom_scroll() {
    let events = generate_sample_events(100);
    let mut state = TimeTravelState::new(events, vec![]);

    let initial_zoom = state.zoom_level;
    state.zoom_in();
    assert!(state.zoom_level > initial_zoom);

    state.zoom_out();
    assert!((state.zoom_level - initial_zoom).abs() < 0.01);

    state.scroll_timeline(10);
    assert_eq!(state.scroll_offset, 10);
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Swarm handler tests pass: `cargo test -p descartes-gui swarm_handler`
- [ ] DAG editor tests pass: `cargo test -p descartes-gui dag_editor`
- [ ] Time travel tests pass: `cargo test -p descartes-gui time_travel`
- [ ] All GUI tests pass: `cargo test -p descartes-gui`
- [ ] Type checking passes: `cargo check -p descartes-gui`

#### Manual Verification:
- [ ] Launch GUI: `cargo run --bin descartes-gui`
- [ ] Connect to daemon, verify swarm monitor shows agents
- [ ] Create DAG with 5+ nodes and edges, verify rendering
- [ ] Use time travel slider, verify event details update
- [ ] Test all keyboard shortcuts (Ctrl+Z, Delete, Space, etc.)

**Implementation Note**: After completing this phase, pause for manual GUI testing to verify visual components work correctly before proceeding.

---

## Phase 4: Daemon and RPC Testing

### Overview
Test the daemon's RPC server, event bus, and client integration.

### Changes Required:

#### 4.1 RPC Server Stress Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/daemon/tests/rpc_stress_tests.rs` (new)

```rust
// Test 1: High concurrency
#[tokio::test]
async fn test_100_concurrent_requests() {
    let (server, socket_path, _) = setup_test_server().await;
    let handle = server.start().await.unwrap();

    let mut tasks = vec![];
    for i in 0..100 {
        let socket = socket_path.clone();
        tasks.push(tokio::spawn(async move {
            let request = create_rpc_request("list_tasks", json!([null]), i);
            send_rpc_request(&socket, &request).await
        }));
    }

    let results = futures::future::join_all(tasks).await;
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    handle.stop().unwrap();
}

// Test 2: Sustained load
#[tokio::test]
async fn test_sustained_load() {
    let (server, socket_path, _) = setup_test_server().await;
    let handle = server.start().await.unwrap();

    let start = Instant::now();
    let mut count = 0;

    while start.elapsed() < Duration::from_secs(5) {
        let request = create_rpc_request("list_tasks", json!([null]), count);
        send_rpc_request(&socket_path, &request).await.unwrap();
        count += 1;
    }

    println!("Processed {} requests in 5 seconds ({} req/s)", count, count / 5);
    assert!(count > 100); // At least 20 req/s

    handle.stop().unwrap();
}

// Test 3: Connection recovery
#[tokio::test]
async fn test_connection_recovery() {
    let (server, socket_path, _) = setup_test_server().await;

    // Start server
    let handle = server.start().await.unwrap();

    // Make request
    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await;
    assert!(response.is_ok());

    // Stop server
    handle.stop().unwrap();
    handle.stopped().await;

    // Restart server
    let handle2 = server.start().await.unwrap();

    // Make another request
    let response2 = send_rpc_request(&socket_path, &request).await;
    assert!(response2.is_ok());

    handle2.stop().unwrap();
}
```

#### 4.2 Event System Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/daemon/tests/event_system_tests.rs` (new)

```rust
// Test 1: Event bus pub/sub
#[tokio::test]
async fn test_event_bus_pubsub() {
    let bus = Arc::new(EventBus::new());

    let rx = bus.subscribe(EventFilter::default());

    bus.publish(DescartesEvent::System {
        message: "Test event".to_string(),
        timestamp: Utc::now(),
    });

    let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");

    assert!(matches!(event, DescartesEvent::System { .. }));
}

// Test 2: Event filtering
#[tokio::test]
async fn test_event_filtering() {
    let bus = Arc::new(EventBus::new());

    let agent_id = Uuid::new_v4();
    let filter = EventFilter {
        agent_ids: vec![agent_id],
        ..Default::default()
    };

    let rx = bus.subscribe(filter);

    // Publish event for our agent
    bus.publish(DescartesEvent::Agent(AgentEvent {
        agent_id,
        event_type: AgentEventType::Started,
        ..Default::default()
    }));

    // Publish event for different agent (should be filtered)
    bus.publish(DescartesEvent::Agent(AgentEvent {
        agent_id: Uuid::new_v4(),
        event_type: AgentEventType::Started,
        ..Default::default()
    }));

    // Should only receive one event
    let event = rx.recv().await.unwrap();
    assert_eq!(event.agent_id(), Some(agent_id));

    // Should timeout on second recv
    assert!(tokio::time::timeout(Duration::from_millis(100), rx.recv()).await.is_err());
}

// Test 3: Multiple subscribers
#[tokio::test]
async fn test_multiple_subscribers() {
    let bus = Arc::new(EventBus::new());

    let rx1 = bus.subscribe(EventFilter::default());
    let rx2 = bus.subscribe(EventFilter::default());
    let rx3 = bus.subscribe(EventFilter::default());

    bus.publish(DescartesEvent::System {
        message: "Test".to_string(),
        timestamp: Utc::now(),
    });

    // All subscribers should receive the event
    assert!(rx1.recv().await.is_ok());
    assert!(rx2.recv().await.is_ok());
    assert!(rx3.recv().await.is_ok());
}
```

### Success Criteria:

#### Automated Verification:
- [ ] RPC stress tests pass: `cargo test -p descartes-daemon rpc_stress`
- [ ] Event system tests pass: `cargo test -p descartes-daemon event_system`
- [ ] All daemon tests pass: `cargo test -p descartes-daemon`
- [ ] Linting passes: `cargo clippy -p descartes-daemon`

#### Manual Verification:
- [ ] Start daemon: `cargo run --bin descartes-daemon`
- [ ] Connect GUI to daemon, verify connection status
- [ ] Spawn agent via CLI, verify events appear in GUI
- [ ] Stop and restart daemon, verify clients reconnect

---

## Phase 5: Integration Testing

### Overview
Test complete end-to-end workflows across all components.

### Changes Required:

#### 5.1 End-to-End Workflow Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/tests/e2e_workflow_tests.rs` (new)

```rust
// Test 1: Complete agent lifecycle
#[tokio::test]
async fn test_complete_agent_lifecycle() {
    // 1. Initialize project
    let temp_dir = tempdir().unwrap();
    init::execute(Some("test-project"), Some(temp_dir.path())).await.unwrap();

    // 2. Start daemon
    let daemon = start_test_daemon(temp_dir.path()).await;

    // 3. Spawn agent via CLI (using mock provider)
    let agent_id = spawn_test_agent(&temp_dir, "Test task").await.unwrap();

    // 4. Verify agent appears in ps
    let agents = list_agents(&temp_dir).await.unwrap();
    assert!(agents.iter().any(|a| a.id == agent_id));

    // 5. Wait for completion
    wait_for_agent_completion(&temp_dir, &agent_id, Duration::from_secs(30)).await.unwrap();

    // 6. Verify logs contain events
    let logs = get_agent_logs(&temp_dir, &agent_id).await.unwrap();
    assert!(logs.iter().any(|l| l.event_type == "agent_started"));
    assert!(logs.iter().any(|l| l.event_type == "agent_completed"));

    // Cleanup
    daemon.shutdown().await;
}

// Test 2: Multi-agent coordination
#[tokio::test]
async fn test_multi_agent_workflow() {
    let temp_dir = tempdir().unwrap();
    init::execute(Some("multi-agent"), Some(temp_dir.path())).await.unwrap();
    let daemon = start_test_daemon(temp_dir.path()).await;

    // Spawn 3 agents
    let agents: Vec<_> = futures::future::join_all((0..3).map(|i| {
        spawn_test_agent(&temp_dir, &format!("Task {}", i))
    })).await;

    // Verify all running
    let running = list_agents(&temp_dir).await.unwrap();
    assert_eq!(running.len(), 3);

    // Kill one agent
    kill_agent(&temp_dir, &agents[1].unwrap(), false).await.unwrap();

    // Verify only 2 remaining
    let remaining = list_running_agents(&temp_dir).await.unwrap();
    assert_eq!(remaining.len(), 2);

    daemon.shutdown().await;
}

// Test 3: GUI-Daemon integration
#[tokio::test]
async fn test_gui_daemon_integration() {
    let temp_dir = tempdir().unwrap();
    init::execute(Some("gui-test"), Some(temp_dir.path())).await.unwrap();
    let daemon = start_test_daemon(temp_dir.path()).await;

    // Create GUI RPC client
    let client = GuiRpcClient::connect(daemon.socket_path()).await.unwrap();
    assert!(client.is_connected());

    // Spawn agent via daemon RPC
    let agent_id = client.spawn_agent("test-agent", "claude", json!({})).await.unwrap();

    // Subscribe to events
    let mut events = client.subscribe_events(EventFilter::default()).await.unwrap();

    // Verify events received
    let event = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("Timeout waiting for event")
        .unwrap();

    assert!(matches!(event, DescartesEvent::Agent { .. }));

    daemon.shutdown().await;
}
```

#### 5.2 Error Recovery Tests

**File**: `/Users/reuben/gauntlet/cap/descartes/tests/error_recovery_tests.rs` (new)

```rust
// Test 1: Daemon crash recovery
#[tokio::test]
async fn test_daemon_crash_recovery() {
    let temp_dir = tempdir().unwrap();
    init::execute(Some("crash-test"), Some(temp_dir.path())).await.unwrap();

    // Start daemon
    let mut daemon = start_test_daemon(temp_dir.path()).await;

    // Spawn agent
    let agent_id = spawn_test_agent(&temp_dir, "Long task").await.unwrap();

    // Kill daemon abruptly
    daemon.kill().await;

    // Restart daemon
    let daemon2 = start_test_daemon(temp_dir.path()).await;

    // Verify agent state was persisted
    let agents = list_all_agents(&temp_dir).await.unwrap();
    assert!(agents.iter().any(|a| a.id == agent_id));

    daemon2.shutdown().await;
}

// Test 2: Database corruption handling
#[tokio::test]
async fn test_database_corruption() {
    let temp_dir = tempdir().unwrap();
    init::execute(Some("db-test"), Some(temp_dir.path())).await.unwrap();

    // Corrupt database
    let db_path = temp_dir.path().join("data/descartes.db");
    std::fs::write(&db_path, "corrupted data").unwrap();

    // Attempt to connect - should fail gracefully
    let result = connect_to_database(&db_path).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("database"));
}

// Test 3: Network timeout handling
#[tokio::test]
async fn test_api_timeout() {
    // Create mock server with 10s delay
    let mock = MockServer::start();
    mock.mock(|when, then| {
        when.any_request();
        then.delay(Duration::from_secs(10));
    });

    // Create provider with 1s timeout
    let config = HashMap::from([
        ("endpoint".to_string(), mock.url("/v1")),
        ("api_key".to_string(), "test".to_string()),
        ("timeout_secs".to_string(), "1".to_string()),
    ]);

    let mut provider = ProviderFactory::create("openai", config).unwrap();
    provider.initialize().await.unwrap();

    let result = provider.complete(ModelRequest::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}
```

### Success Criteria:

#### Automated Verification:
- [ ] E2E tests pass: `cargo test -p descartes --test e2e_workflow_tests`
- [ ] Error recovery tests pass: `cargo test -p descartes --test error_recovery_tests`
- [ ] All workspace tests pass: `cargo test --workspace`
- [ ] Type checking passes: `cargo check --workspace`

#### Manual Verification:
- [ ] Complete full workflow: init → spawn → monitor in GUI → kill
- [ ] Test recovery: kill daemon during agent execution, restart, verify state
- [ ] Test API failures: use invalid API key, verify error messages

---

## Phase 6: Performance Testing

### Overview
Benchmark system performance and identify bottlenecks.

### Changes Required:

#### 6.1 Performance Benchmarks

**File**: `/Users/reuben/gauntlet/cap/descartes/core/benches/performance_tests.rs` (new)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_dag_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("DAG Operations");

    // Node addition
    group.bench_function("add_100_nodes", |b| {
        b.iter(|| {
            let mut dag = DAG::new("bench");
            for i in 0..100 {
                dag.add_node(DAGNode::new_auto(&format!("Node {}", i))).unwrap();
            }
            black_box(dag)
        })
    });

    // Edge addition with cycle detection
    group.bench_function("add_100_edges", |b| {
        let mut dag = DAG::new("bench");
        let nodes: Vec<_> = (0..101).map(|i| {
            let node = DAGNode::new_auto(&format!("Node {}", i));
            let id = node.node_id;
            dag.add_node(node).unwrap();
            id
        }).collect();

        b.iter(|| {
            let mut dag = dag.clone();
            for i in 0..100 {
                dag.add_edge(DAGEdge::dependency(nodes[i], nodes[i+1])).unwrap();
            }
            black_box(dag)
        })
    });

    // Topological sort
    group.bench_function("topo_sort_1000_nodes", |b| {
        let dag = create_dag_with_nodes(1000);
        b.iter(|| {
            black_box(dag.topological_sort())
        })
    });

    group.finish();
}

fn benchmark_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("Serialization");

    // JSON serialization
    group.bench_function("serialize_dag_100_nodes", |b| {
        let dag = create_dag_with_nodes(100);
        b.iter(|| {
            black_box(serde_json::to_string(&dag).unwrap())
        })
    });

    // JSON deserialization
    group.bench_function("deserialize_dag_100_nodes", |b| {
        let dag = create_dag_with_nodes(100);
        let json = serde_json::to_string(&dag).unwrap();
        b.iter(|| {
            black_box(serde_json::from_str::<DAG>(&json).unwrap())
        })
    });

    // ZMQ message serialization
    group.bench_function("serialize_zmq_message", |b| {
        let msg = SpawnRequest {
            request_id: "test".to_string(),
            config: AgentConfig::default(),
            timeout: Some(30),
            metadata: HashMap::new(),
        };
        b.iter(|| {
            black_box(serialize_zmq_message(&ZmqMessage::SpawnRequest(msg.clone())).unwrap())
        })
    });

    group.finish();
}

fn benchmark_state_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("State Store");

    // Event insertion
    group.bench_function("insert_1000_events", |b| {
        b.iter_batched(
            || setup_temp_database(),
            |store| async {
                for i in 0..1000 {
                    store.save_event(&create_event(i)).await.unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Event query
    group.bench_function("query_events_by_session", |b| {
        let store = setup_populated_database(10000);
        let session_id = "test-session";
        b.iter(|| {
            black_box(store.get_events(session_id))
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_dag_operations, benchmark_serialization, benchmark_state_store);
criterion_main!(benches);
```

### Success Criteria:

#### Automated Verification:
- [ ] Benchmarks run successfully: `cargo bench`
- [ ] No performance regressions from baseline
- [ ] DAG operations < 1ms for 100 nodes
- [ ] Serialization < 10ms for 100 nodes
- [ ] Database operations < 100ms for 1000 events

#### Manual Verification:
- [ ] Profile GUI with 100+ agents, verify 60 FPS maintained
- [ ] Profile daemon under load (100 concurrent clients), verify < 1s response
- [ ] Monitor memory usage over 1 hour, verify no leaks

---

## Testing Strategy

### Unit Tests
- Test individual functions and methods in isolation
- Use mocks for external dependencies
- Focus on edge cases and error handling
- Key areas: DAG operations, coordinate transforms, message parsing

### Integration Tests
- Test component interactions
- Use real databases and file systems
- Focus on data flow and state consistency
- Key areas: CLI commands, RPC communication, event streaming

### End-to-End Tests
- Test complete user workflows
- Use real (or mock) API providers
- Focus on user scenarios
- Key areas: Agent lifecycle, multi-agent coordination, crash recovery

### Manual Testing Steps
1. **Initial Setup**
   - Clone repository
   - Run `cargo build --workspace`
   - Verify no build errors

2. **CLI Testing**
   - Run `descartes init --name test-project`
   - Verify directories created
   - Run `descartes spawn --task "hello" --provider ollama` (with Ollama running)
   - Verify agent starts

3. **GUI Testing**
   - Run `cargo run --bin descartes-gui`
   - Connect to daemon
   - Create DAG with 5 nodes
   - Test undo/redo
   - Test zoom/pan

4. **Integration Testing**
   - Start daemon
   - Connect GUI
   - Spawn agents via CLI
   - Verify events appear in GUI
   - Test time travel with agent history

## Performance Considerations

- **Database**: SQLite with WAL mode for concurrent reads
- **Memory**: Arc/Mutex for shared state, avoid cloning large structures
- **Rendering**: Cache canvas frames, zoom-dependent detail levels
- **Network**: Connection pooling for HTTP, ZMQ for IPC

## References

- Project README: `descartes/README.md`
- Core library: `descartes/core/src/lib.rs`
- CLI implementation: `descartes/cli/src/main.rs`
- GUI main: `descartes/gui/src/main.rs`
- Daemon main: `descartes/daemon/src/main.rs`
- Existing tests: `descartes/*/tests/*.rs`
- Benchmarks: `descartes/core/benches/`
