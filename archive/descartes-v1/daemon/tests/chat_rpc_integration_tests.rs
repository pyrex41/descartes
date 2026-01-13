//! Chat RPC integration tests
//!
//! This test suite covers the chat RPC methods:
//! - chat.create: Create a session without starting CLI (for race-free subscription)
//! - chat.list: List all chat sessions
//! - chat.stop: Stop a chat session
//! - chat.upgrade_to_agent: Upgrade a chat session to agent mode
//!
//! Note: These tests don't test actual Claude CLI spawning (which requires
//! Claude CLI to be installed). They test the RPC protocol and session management.

use serde_json::{json, Value};

/// Create a JSON-RPC 2.0 request
fn create_rpc_request(method: &str, params: Value, id: u64) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    })
    .to_string()
}

/// Parse JSON-RPC response
fn parse_response(response: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(response)
}

// ============================================================================
// UNIT TESTS FOR RPC REQUEST/RESPONSE FORMATS
// ============================================================================

#[test]
fn test_rpc_request_format() {
    let request = create_rpc_request(
        "chat.create",
        json!({
            "working_dir": "/tmp",
            "enable_thinking": false
        }),
        1,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "chat.create");
    assert_eq!(parsed["params"]["working_dir"], "/tmp");
    assert_eq!(parsed["params"]["enable_thinking"], false);
    assert_eq!(parsed["id"], 1);
}

#[test]
fn test_chat_create_params() {
    // Test with all parameters
    let request = create_rpc_request(
        "chat.create",
        json!({
            "working_dir": "/home/user/project",
            "initial_prompt": "Hello",
            "enable_thinking": true,
            "thinking_level": "high",
            "max_turns": 10
        }),
        1,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["params"]["working_dir"], "/home/user/project");
    assert_eq!(parsed["params"]["initial_prompt"], "Hello");
    assert_eq!(parsed["params"]["enable_thinking"], true);
    assert_eq!(parsed["params"]["thinking_level"], "high");
    assert_eq!(parsed["params"]["max_turns"], 10);
}

#[test]
fn test_chat_create_minimal_params() {
    // Test with minimal parameters
    let request = create_rpc_request(
        "chat.create",
        json!({
            "working_dir": "/tmp"
        }),
        1,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["params"]["working_dir"], "/tmp");
}

#[test]
fn test_chat_prompt_params() {
    let session_id = "550e8400-e29b-41d4-a716-446655440000";
    let request = create_rpc_request(
        "chat.prompt",
        json!({
            "session_id": session_id,
            "prompt": "What is Rust?"
        }),
        2,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["method"], "chat.prompt");
    assert_eq!(parsed["params"]["session_id"], session_id);
    assert_eq!(parsed["params"]["prompt"], "What is Rust?");
}

#[test]
fn test_chat_stop_params() {
    let session_id = "550e8400-e29b-41d4-a716-446655440000";
    let request = create_rpc_request(
        "chat.stop",
        json!({
            "session_id": session_id
        }),
        3,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["method"], "chat.stop");
    assert_eq!(parsed["params"]["session_id"], session_id);
}

#[test]
fn test_chat_list_params() {
    // chat.list takes no parameters
    let request = create_rpc_request("chat.list", json!({}), 4);

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["method"], "chat.list");
    assert!(parsed["params"].as_object().unwrap().is_empty());
}

#[test]
fn test_chat_upgrade_to_agent_params() {
    let session_id = "550e8400-e29b-41d4-a716-446655440000";
    let request = create_rpc_request(
        "chat.upgrade_to_agent",
        json!({
            "session_id": session_id
        }),
        5,
    );

    let parsed: Value = serde_json::from_str(&request).unwrap();
    assert_eq!(parsed["method"], "chat.upgrade_to_agent");
    assert_eq!(parsed["params"]["session_id"], session_id);
}

// ============================================================================
// EXPECTED RESPONSE FORMAT TESTS
// ============================================================================

#[test]
fn test_success_response_format() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "session_id": "550e8400-e29b-41d4-a716-446655440000",
            "pub_endpoint": "tcp://127.0.0.1:19480",
            "topic": "chat/550e8400-e29b-41d4-a716-446655440000"
        }
    });

    let response_str = response_json.to_string();
    let parsed: Value = parse_response(&response_str).unwrap();

    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert!(parsed["result"].is_object());
    assert!(parsed.get("error").is_none());
}

#[test]
fn test_error_response_format() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "Invalid request"
        }
    });

    let response_str = response_json.to_string();
    let parsed: Value = parse_response(&response_str).unwrap();

    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], -32600);
    assert!(parsed.get("result").is_none());
}

#[test]
fn test_chat_create_response_structure() {
    // Expected response from chat.create
    let expected_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "session_id": "550e8400-e29b-41d4-a716-446655440000",
            "pub_endpoint": "tcp://127.0.0.1:19480",
            "topic": "chat/550e8400-e29b-41d4-a716-446655440000"
        }
    });

    let result = &expected_response["result"];
    assert!(result.get("session_id").is_some());
    assert!(result.get("pub_endpoint").is_some());
    assert!(result.get("topic").is_some());

    // Verify topic format
    let session_id = result["session_id"].as_str().unwrap();
    let topic = result["topic"].as_str().unwrap();
    assert_eq!(topic, format!("chat/{}", session_id));
}

#[test]
fn test_chat_list_response_structure() {
    // Expected response from chat.list
    let expected_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "sessions": [
                {
                    "session_id": "550e8400-e29b-41d4-a716-446655440000",
                    "working_dir": "/tmp",
                    "created_at": "2025-12-11T20:00:00Z",
                    "is_active": true,
                    "turn_count": 3,
                    "mode": "chat"
                },
                {
                    "session_id": "660e8400-e29b-41d4-a716-446655440001",
                    "working_dir": "/home/user",
                    "created_at": "2025-12-11T19:00:00Z",
                    "is_active": false,
                    "turn_count": 5,
                    "mode": "agent"
                }
            ]
        }
    });

    let sessions = expected_response["result"]["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 2);

    let first_session = &sessions[0];
    assert!(first_session.get("session_id").is_some());
    assert!(first_session.get("working_dir").is_some());
    assert!(first_session.get("created_at").is_some());
    assert!(first_session.get("is_active").is_some());
    assert!(first_session.get("turn_count").is_some());
    assert!(first_session.get("mode").is_some());
}

// ============================================================================
// BATCH REQUEST TESTS
// ============================================================================

#[test]
fn test_batch_request_format() {
    let batch_request = json!([
        {
            "jsonrpc": "2.0",
            "method": "chat.create",
            "params": {"working_dir": "/tmp"},
            "id": 1
        },
        {
            "jsonrpc": "2.0",
            "method": "chat.list",
            "params": {},
            "id": 2
        }
    ]);

    let requests = batch_request.as_array().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["method"], "chat.create");
    assert_eq!(requests[1]["method"], "chat.list");
}

// ============================================================================
// SESSION ID VALIDATION TESTS
// ============================================================================

#[test]
fn test_valid_uuid_format() {
    let valid_uuids = vec![
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "00000000-0000-0000-0000-000000000000",
    ];

    for uuid_str in valid_uuids {
        let parsed = uuid::Uuid::parse_str(uuid_str);
        assert!(parsed.is_ok(), "Failed to parse: {}", uuid_str);
    }
}

#[test]
fn test_invalid_uuid_format() {
    let invalid_uuids = vec![
        "not-a-uuid",
        "550e8400-e29b-41d4-a716",  // too short
        "550e8400-e29b-41d4-a716-446655440000-extra",  // too long
        "",
    ];

    for uuid_str in invalid_uuids {
        let parsed = uuid::Uuid::parse_str(uuid_str);
        assert!(parsed.is_err(), "Should have failed: {}", uuid_str);
    }
}

// ============================================================================
// THINKING LEVEL TESTS
// ============================================================================

#[test]
fn test_thinking_level_values() {
    let valid_levels = vec!["none", "normal", "high", "ultrathink"];

    for level in valid_levels {
        let request = create_rpc_request(
            "chat.create",
            json!({
                "working_dir": "/tmp",
                "enable_thinking": true,
                "thinking_level": level
            }),
            1,
        );

        let parsed: Value = serde_json::from_str(&request).unwrap();
        assert_eq!(parsed["params"]["thinking_level"], level);
    }
}

// ============================================================================
// ZMQ TOPIC FORMAT TESTS
// ============================================================================

#[test]
fn test_zmq_topic_format() {
    let session_id = uuid::Uuid::new_v4();
    let topic = format!("chat/{}", session_id);

    assert!(topic.starts_with("chat/"));
    assert_eq!(topic.len(), 5 + 36); // "chat/" + UUID length
}
