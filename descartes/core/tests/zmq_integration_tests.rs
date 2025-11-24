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
