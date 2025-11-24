/// Integration tests for ZMQ communication layer
///
/// These tests verify the functionality of the ZMQ communication layer,
/// including socket setup, message serialization/deserialization, connection
/// management, and request/response correlation.

use descartes_core::{
    AgentConfig, AgentInfo, AgentStatus, ControlCommandType, HealthCheckRequest,
    HealthCheckResponse, ListAgentsRequest, ListAgentsResponse, SocketType, SpawnRequest,
    SpawnResponse, ZmqClient, ZmqConnection, ZmqMessage, ZmqMessageRouter, ZmqRunnerConfig,
    deserialize_zmq_message, serialize_zmq_message, validate_message_size,
};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[test]
fn test_message_serialization_health_check() {
    let request = HealthCheckRequest {
        request_id: "test-123".to_string(),
    };

    let msg = ZmqMessage::HealthCheckRequest(request);
    let bytes = serialize_zmq_message(&msg).unwrap();

    assert!(bytes.len() > 0);
    assert!(bytes.len() < 1000); // Should be small

    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::HealthCheckRequest(req) => {
            assert_eq!(req.request_id, "test-123");
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_message_serialization_spawn_request() {
    let request = SpawnRequest {
        request_id: "spawn-456".to_string(),
        config: AgentConfig {
            name: "test-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "Test task".to_string(),
            context: Some("Test context".to_string()),
            system_prompt: None,
            environment: HashMap::new(),
        },
        timeout_secs: Some(300),
        metadata: None,
    };

    let msg = ZmqMessage::SpawnRequest(request);
    let bytes = serialize_zmq_message(&msg).unwrap();

    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::SpawnRequest(req) => {
            assert_eq!(req.request_id, "spawn-456");
            assert_eq!(req.config.name, "test-agent");
            assert_eq!(req.config.model_backend, "claude");
            assert_eq!(req.timeout_secs, Some(300));
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_message_serialization_spawn_response() {
    let response = SpawnResponse {
        request_id: "spawn-456".to_string(),
        success: true,
        agent_info: Some(AgentInfo {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            status: AgentStatus::Running,
            model_backend: "claude".to_string(),
            started_at: SystemTime::now(),
            task: "Test task".to_string(),
        }),
        error: None,
        server_id: Some("server-01".to_string()),
    };

    let msg = ZmqMessage::SpawnResponse(response);
    let bytes = serialize_zmq_message(&msg).unwrap();

    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::SpawnResponse(resp) => {
            assert_eq!(resp.request_id, "spawn-456");
            assert!(resp.success);
            assert!(resp.agent_info.is_some());
            assert_eq!(resp.server_id, Some("server-01".to_string()));
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_message_serialization_list_agents() {
    let request = ListAgentsRequest {
        request_id: "list-789".to_string(),
        filter_status: Some(AgentStatus::Running),
        limit: Some(10),
    };

    let msg = ZmqMessage::ListAgentsRequest(request);
    let bytes = serialize_zmq_message(&msg).unwrap();

    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::ListAgentsRequest(req) => {
            assert_eq!(req.request_id, "list-789");
            assert_eq!(req.filter_status, Some(AgentStatus::Running));
            assert_eq!(req.limit, Some(10));
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_message_serialization_list_agents_response() {
    let response = ListAgentsResponse {
        request_id: "list-789".to_string(),
        success: true,
        agents: vec![
            AgentInfo {
                id: Uuid::new_v4(),
                name: "agent-1".to_string(),
                status: AgentStatus::Running,
                model_backend: "claude".to_string(),
                started_at: SystemTime::now(),
                task: "Task 1".to_string(),
            },
            AgentInfo {
                id: Uuid::new_v4(),
                name: "agent-2".to_string(),
                status: AgentStatus::Running,
                model_backend: "gpt-4".to_string(),
                started_at: SystemTime::now(),
                task: "Task 2".to_string(),
            },
        ],
        error: None,
    };

    let msg = ZmqMessage::ListAgentsResponse(response);
    let bytes = serialize_zmq_message(&msg).unwrap();

    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::ListAgentsResponse(resp) => {
            assert_eq!(resp.request_id, "list-789");
            assert!(resp.success);
            assert_eq!(resp.agents.len(), 2);
            assert_eq!(resp.agents[0].name, "agent-1");
            assert_eq!(resp.agents[1].name, "agent-2");
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_message_size_validation() {
    // Valid size
    assert!(validate_message_size(1024).is_ok());
    assert!(validate_message_size(1024 * 1024).is_ok());
    assert!(validate_message_size(10 * 1024 * 1024).is_ok());

    // Invalid size (too large)
    assert!(validate_message_size(10 * 1024 * 1024 + 1).is_err());
    assert!(validate_message_size(100 * 1024 * 1024).is_err());
}

#[test]
fn test_zmq_connection_creation() {
    let config = ZmqRunnerConfig {
        endpoint: "tcp://localhost:5555".to_string(),
        ..Default::default()
    };

    let connection = ZmqConnection::new(SocketType::Req, "tcp://localhost:5555", config);

    // Should start disconnected
    assert!(!connection.is_connected());

    // Stats should be zero
    let stats = connection.stats();
    assert_eq!(stats.messages_sent, 0);
    assert_eq!(stats.messages_received, 0);
    assert_eq!(stats.bytes_sent, 0);
    assert_eq!(stats.bytes_received, 0);
}

#[tokio::test]
async fn test_message_router_operations() {
    let router = ZmqMessageRouter::new();

    assert_eq!(router.pending_count().await, 0);

    // Register multiple requests
    let req1_id = Uuid::new_v4().to_string();
    let req2_id = Uuid::new_v4().to_string();

    let rx1 = router.register_request(req1_id.clone()).await;
    let rx2 = router.register_request(req2_id.clone()).await;

    assert_eq!(router.pending_count().await, 2);

    // Route responses
    let response1 = ZmqMessage::HealthCheckResponse(HealthCheckResponse {
        request_id: req1_id.clone(),
        healthy: true,
        protocol_version: "1.0.0".to_string(),
        uptime_secs: Some(100),
        active_agents: Some(5),
        metadata: None,
    });

    router.route_response(&req1_id, Ok(response1)).await.unwrap();

    // Should have one pending now
    assert_eq!(router.pending_count().await, 1);

    // Receive response 1
    let received1 = rx1.await.unwrap().unwrap();
    match received1 {
        ZmqMessage::HealthCheckResponse(resp) => {
            assert!(resp.healthy);
        }
        _ => panic!("Wrong message type"),
    }

    // Route response 2
    let response2 = ZmqMessage::HealthCheckResponse(HealthCheckResponse {
        request_id: req2_id.clone(),
        healthy: false,
        protocol_version: "1.0.0".to_string(),
        uptime_secs: None,
        active_agents: None,
        metadata: None,
    });

    router.route_response(&req2_id, Ok(response2)).await.unwrap();

    // Should have zero pending now
    assert_eq!(router.pending_count().await, 0);

    // Receive response 2
    let received2 = rx2.await.unwrap().unwrap();
    match received2 {
        ZmqMessage::HealthCheckResponse(resp) => {
            assert!(!resp.healthy);
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_zmq_client_creation() {
    let config = ZmqRunnerConfig {
        endpoint: "tcp://localhost:5555".to_string(),
        connection_timeout_secs: 30,
        request_timeout_secs: 30,
        auto_reconnect: true,
        max_reconnect_attempts: 3,
        reconnect_delay_secs: 5,
        enable_heartbeat: true,
        heartbeat_interval_secs: 30,
        server_id: Some("test-server".to_string()),
    };

    let client = ZmqClient::new(config);

    // Just verify it constructs successfully
    assert!(true);
}

#[test]
fn test_zmq_runner_config_defaults() {
    let config = ZmqRunnerConfig::default();

    assert_eq!(config.endpoint, "tcp://localhost:5555");
    assert_eq!(config.connection_timeout_secs, 30);
    assert_eq!(config.request_timeout_secs, 30);
    assert!(config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 3);
    assert_eq!(config.reconnect_delay_secs, 5);
    assert!(config.enable_heartbeat);
    assert_eq!(config.heartbeat_interval_secs, 30);
    assert_eq!(config.server_id, None);
}

#[test]
fn test_zmq_runner_config_custom() {
    let config = ZmqRunnerConfig {
        endpoint: "tcp://192.168.1.100:6000".to_string(),
        connection_timeout_secs: 60,
        request_timeout_secs: 120,
        auto_reconnect: false,
        max_reconnect_attempts: 5,
        reconnect_delay_secs: 10,
        enable_heartbeat: false,
        heartbeat_interval_secs: 60,
        server_id: Some("custom-server".to_string()),
    };

    assert_eq!(config.endpoint, "tcp://192.168.1.100:6000");
    assert_eq!(config.connection_timeout_secs, 60);
    assert_eq!(config.request_timeout_secs, 120);
    assert!(!config.auto_reconnect);
    assert_eq!(config.max_reconnect_attempts, 5);
    assert_eq!(config.reconnect_delay_secs, 10);
    assert!(!config.enable_heartbeat);
    assert_eq!(config.heartbeat_interval_secs, 60);
    assert_eq!(config.server_id, Some("custom-server".to_string()));
}

#[test]
fn test_serialization_efficiency() {
    // Create a complex message
    let request = SpawnRequest {
        request_id: "efficiency-test".to_string(),
        config: AgentConfig {
            name: "test-agent".to_string(),
            model_backend: "claude".to_string(),
            task: "A".repeat(1000), // 1KB task
            context: Some("B".repeat(1000)), // 1KB context
            system_prompt: Some("C".repeat(1000)), // 1KB system prompt
            environment: {
                let mut env = HashMap::new();
                for i in 0..10 {
                    env.insert(format!("KEY_{}", i), format!("VALUE_{}", i));
                }
                env
            },
        },
        timeout_secs: Some(300),
        metadata: Some({
            let mut meta = HashMap::new();
            meta.insert("key1".to_string(), "value1".to_string());
            meta.insert("key2".to_string(), "value2".to_string());
            meta
        }),
    };

    let msg = ZmqMessage::SpawnRequest(request);

    // Serialize
    let bytes = serialize_zmq_message(&msg).unwrap();

    // Verify size is reasonable (should be compressed)
    assert!(bytes.len() > 3000); // At least 3KB of data
    assert!(bytes.len() < 10000); // But less than 10KB (MessagePack is efficient)

    // Deserialize and verify
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::SpawnRequest(req) => {
            assert_eq!(req.request_id, "efficiency-test");
            assert_eq!(req.config.task.len(), 1000);
            assert_eq!(req.config.context.as_ref().unwrap().len(), 1000);
            assert_eq!(req.config.system_prompt.as_ref().unwrap().len(), 1000);
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test that demonstrates the expected usage pattern for client/server communication
#[test]
fn test_usage_pattern_documentation() {
    // This test documents the expected usage pattern

    // 1. Create configuration
    let config = ZmqRunnerConfig {
        endpoint: "tcp://localhost:5555".to_string(),
        request_timeout_secs: 30,
        ..Default::default()
    };

    // 2. Create client
    let _client = ZmqClient::new(config);

    // 3. In actual usage, client would:
    //    - Connect to server
    //    - Spawn remote agents
    //    - Control agents
    //    - Monitor status
    //    - Handle errors

    // This test just verifies the pattern compiles
    assert!(true);
}

// ============================================================================
// Phase 3:2.4 - Client-Side Agent Control Tests
// ============================================================================

#[test]
fn test_custom_action_request_serialization() {
    use descartes_core::CustomActionRequest;

    let request = CustomActionRequest {
        request_id: "action-123".to_string(),
        agent_id: Uuid::new_v4(),
        action: "process_data".to_string(),
        params: Some(serde_json::json!({
            "dataset": "logs",
            "filter": "ERROR",
            "limit": 100
        })),
        timeout_secs: Some(60),
    };

    let msg = ZmqMessage::CustomActionRequest(request);
    let bytes = serialize_zmq_message(&msg).unwrap();
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::CustomActionRequest(req) => {
            assert_eq!(req.request_id, "action-123");
            assert_eq!(req.action, "process_data");
            assert_eq!(req.timeout_secs, Some(60));
            assert!(req.params.is_some());
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_batch_control_command_serialization() {
    use descartes_core::BatchControlCommand;

    let agent_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

    let command = BatchControlCommand {
        request_id: "batch-456".to_string(),
        agent_ids: agent_ids.clone(),
        command_type: ControlCommandType::Pause,
        payload: None,
        fail_fast: false,
    };

    let msg = ZmqMessage::BatchControlCommand(command);
    let bytes = serialize_zmq_message(&msg).unwrap();
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::BatchControlCommand(cmd) => {
            assert_eq!(cmd.request_id, "batch-456");
            assert_eq!(cmd.agent_ids.len(), 3);
            assert_eq!(cmd.command_type, ControlCommandType::Pause);
            assert!(!cmd.fail_fast);
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_batch_control_response_serialization() {
    use descartes_core::{BatchControlResponse, BatchAgentResult};

    let agent_id_1 = Uuid::new_v4();
    let agent_id_2 = Uuid::new_v4();

    let response = BatchControlResponse {
        request_id: "batch-456".to_string(),
        success: false, // One failed
        results: vec![
            BatchAgentResult {
                agent_id: agent_id_1,
                success: true,
                status: Some(AgentStatus::Paused),
                error: None,
            },
            BatchAgentResult {
                agent_id: agent_id_2,
                success: false,
                status: Some(AgentStatus::Running),
                error: Some("Agent is busy".to_string()),
            },
        ],
        successful: 1,
        failed: 1,
    };

    let msg = ZmqMessage::BatchControlResponse(response);
    let bytes = serialize_zmq_message(&msg).unwrap();
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::BatchControlResponse(resp) => {
            assert_eq!(resp.request_id, "batch-456");
            assert!(!resp.success);
            assert_eq!(resp.results.len(), 2);
            assert_eq!(resp.successful, 1);
            assert_eq!(resp.failed, 1);
            assert!(resp.results[0].success);
            assert!(!resp.results[1].success);
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_output_query_request_serialization() {
    use descartes_core::{OutputQueryRequest, ZmqOutputStream};

    let request = OutputQueryRequest {
        request_id: "output-789".to_string(),
        agent_id: Uuid::new_v4(),
        stream: ZmqOutputStream::Both,
        filter: Some("ERROR|WARN".to_string()),
        limit: Some(50),
        offset: Some(100),
    };

    let msg = ZmqMessage::OutputQueryRequest(request);
    let bytes = serialize_zmq_message(&msg).unwrap();
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::OutputQueryRequest(req) => {
            assert_eq!(req.request_id, "output-789");
            assert_eq!(req.stream, ZmqOutputStream::Both);
            assert_eq!(req.filter, Some("ERROR|WARN".to_string()));
            assert_eq!(req.limit, Some(50));
            assert_eq!(req.offset, Some(100));
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_output_query_response_serialization() {
    use descartes_core::OutputQueryResponse;

    let response = OutputQueryResponse {
        request_id: "output-789".to_string(),
        agent_id: Uuid::new_v4(),
        success: true,
        lines: vec![
            "ERROR: Connection failed".to_string(),
            "WARN: Retrying connection".to_string(),
            "ERROR: Maximum retries exceeded".to_string(),
        ],
        total_lines: Some(150),
        has_more: true,
        error: None,
    };

    let msg = ZmqMessage::OutputQueryResponse(response);
    let bytes = serialize_zmq_message(&msg).unwrap();
    let deserialized = deserialize_zmq_message(&bytes).unwrap();

    match deserialized {
        ZmqMessage::OutputQueryResponse(resp) => {
            assert_eq!(resp.request_id, "output-789");
            assert!(resp.success);
            assert_eq!(resp.lines.len(), 3);
            assert_eq!(resp.total_lines, Some(150));
            assert!(resp.has_more);
        }
        _ => panic!("Wrong message type after deserialization"),
    }
}

#[test]
fn test_output_stream_types() {
    use descartes_core::ZmqOutputStream;

    // Verify serialization of enum variants
    let streams = vec![
        ZmqOutputStream::Stdout,
        ZmqOutputStream::Stderr,
        ZmqOutputStream::Both,
    ];

    for stream in streams {
        let json = serde_json::to_string(&stream).unwrap();
        let deserialized: ZmqOutputStream = serde_json::from_str(&json).unwrap();
        assert_eq!(stream, deserialized);
    }
}

#[tokio::test]
async fn test_client_queued_command_count() {
    let config = ZmqRunnerConfig::default();
    let client = ZmqClient::new(config);

    // Initially should be zero
    assert_eq!(client.queued_command_count().await, 0);
}

#[test]
fn test_control_command_type_extensions() {
    // Test the new control command types
    let custom_action = ControlCommandType::CustomAction;
    let query_output = ControlCommandType::QueryOutput;
    let stream_logs = ControlCommandType::StreamLogs;

    // Verify they serialize correctly
    let json1 = serde_json::to_string(&custom_action).unwrap();
    let json2 = serde_json::to_string(&query_output).unwrap();
    let json3 = serde_json::to_string(&stream_logs).unwrap();

    assert_eq!(json1, "\"custom_action\"");
    assert_eq!(json2, "\"query_output\"");
    assert_eq!(json3, "\"stream_logs\"");

    // Verify deserialization
    let deser1: ControlCommandType = serde_json::from_str(&json1).unwrap();
    let deser2: ControlCommandType = serde_json::from_str(&json2).unwrap();
    let deser3: ControlCommandType = serde_json::from_str(&json3).unwrap();

    assert_eq!(deser1, ControlCommandType::CustomAction);
    assert_eq!(deser2, ControlCommandType::QueryOutput);
    assert_eq!(deser3, ControlCommandType::StreamLogs);
}

#[test]
fn test_batch_operation_message_size() {
    use descartes_core::BatchControlCommand;

    // Create a batch command for 100 agents
    let agent_ids: Vec<Uuid> = (0..100).map(|_| Uuid::new_v4()).collect();

    let command = BatchControlCommand {
        request_id: "large-batch".to_string(),
        agent_ids,
        command_type: ControlCommandType::GetStatus,
        payload: None,
        fail_fast: false,
    };

    let msg = ZmqMessage::BatchControlCommand(command);
    let bytes = serialize_zmq_message(&msg).unwrap();

    // Verify it's under the max message size
    assert!(bytes.len() < descartes_core::MAX_MESSAGE_SIZE);

    // Verify it deserializes correctly
    let deserialized = deserialize_zmq_message(&bytes).unwrap();
    match deserialized {
        ZmqMessage::BatchControlCommand(cmd) => {
            assert_eq!(cmd.agent_ids.len(), 100);
            assert_eq!(cmd.request_id, "large-batch");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_output_query_with_large_results() {
    use descartes_core::OutputQueryResponse;

    // Create a response with many lines
    let lines: Vec<String> = (0..1000)
        .map(|i| format!("Log line {}: Some output data here", i))
        .collect();

    let response = OutputQueryResponse {
        request_id: "large-output".to_string(),
        agent_id: Uuid::new_v4(),
        success: true,
        lines: lines.clone(),
        total_lines: Some(10000),
        has_more: true,
        error: None,
    };

    let msg = ZmqMessage::OutputQueryResponse(response);
    let bytes = serialize_zmq_message(&msg).unwrap();

    // Verify it's under the max message size
    assert!(bytes.len() < descartes_core::MAX_MESSAGE_SIZE);

    // Verify deserialization
    let deserialized = deserialize_zmq_message(&bytes).unwrap();
    match deserialized {
        ZmqMessage::OutputQueryResponse(resp) => {
            assert_eq!(resp.lines.len(), 1000);
            assert_eq!(resp.total_lines, Some(10000));
            assert!(resp.has_more);
        }
        _ => panic!("Wrong message type"),
    }
}
