//! Integration tests for Swank protocol.
//!
//! These tests require SBCL to be installed and available in PATH.
//! Run with: cargo test -p descartes-core swank -- --ignored --nocapture

use super::{find_available_port, SwankClient, SwankLauncher, SwankMessage, DEFAULT_SWANK_PORT};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Check if SBCL is available
fn sbcl_available() -> bool {
    std::process::Command::new("sbcl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_swank_connection() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();

    let client = SwankClient::connect(agent_id, port, event_tx).await;
    assert!(client.is_ok(), "Failed to connect to Swank: {:?}", client.err());

    let client = client.unwrap();
    assert!(client.is_connected().await);
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_simple_eval() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Simple arithmetic
    let result = client.eval("(+ 1 2)", "CL-USER").await;
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    let value = result.unwrap();
    assert!(value.contains('3'), "Expected 3, got: {}", value);
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_multiline_eval() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Multi-line code with special characters
    let code = r#"(progn
  (format nil "Hello~%World"))"#;

    let result = client.eval(code, "CL-USER").await;
    assert!(result.is_ok(), "Multiline eval failed: {:?}", result.err());
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_debugger_triggered() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let _child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, mut event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    // Trigger division by zero - this will enter the debugger
    // We need to spawn this as a task since it will block waiting for debugger resolution
    let client_clone = client.clone();
    let eval_task = tokio::spawn(async move {
        client_clone.eval("(/ 1 0)", "CL-USER").await
    });

    // Wait for debugger event
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            while let Some(msg) = event_rx.recv().await {
                if let SwankMessage::Debug { condition, restarts, .. } = msg {
                    // Verify we got the debug event
                    assert!(condition.contains("DIVISION-BY-ZERO") || condition.to_lowercase().contains("division"),
                        "Expected division error, got: {}", condition);
                    assert!(!restarts.is_empty(), "Expected at least one restart");

                    // Find and invoke ABORT restart
                    let abort_idx = restarts.iter()
                        .find(|r| r.name.to_uppercase().contains("ABORT"))
                        .map(|r| r.index)
                        .unwrap_or(0);

                    return Some(abort_idx);
                }
            }
            None
        }
    ).await;

    assert!(timeout.is_ok(), "Timed out waiting for debugger event");
    let abort_idx = timeout.unwrap();
    assert!(abort_idx.is_some(), "Did not receive debug event");

    // Invoke abort restart
    let abort_idx = abort_idx.unwrap();
    let restart_result = client.invoke_restart(abort_idx).await;
    assert!(restart_result.is_ok(), "Restart failed: {:?}", restart_result.err());

    // The eval task should complete (with an abort)
    let eval_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        eval_task
    ).await;
    assert!(eval_result.is_ok(), "Eval task didn't complete after restart");
}

#[tokio::test]
#[ignore] // Requires SBCL
async fn test_connection_cleanup() {
    if !sbcl_available() {
        eprintln!("Skipping test: SBCL not available");
        return;
    }

    let port = find_available_port(DEFAULT_SWANK_PORT).await.unwrap();
    let mut child = SwankLauncher::start_sbcl(port).await.unwrap();

    let (event_tx, _event_rx) = mpsc::channel(64);
    let agent_id = Uuid::new_v4();
    let client = SwankClient::connect(agent_id, port, event_tx).await.unwrap();

    assert!(client.is_connected().await);

    // Disconnect
    client.disconnect().await.unwrap();
    assert!(!client.is_connected().await);

    // Kill SBCL
    child.kill().await.unwrap();
}
