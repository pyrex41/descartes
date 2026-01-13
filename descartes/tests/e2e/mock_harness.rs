//! Mock harness for integration testing
//!
//! Provides canned responses without making real API calls.

use async_trait::async_trait;
use descartes::harness::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult,
};
use descartes::Result;
use futures::stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Configured response for a task
#[derive(Clone)]
pub enum MockResponse {
    /// Task completes successfully
    Success { output: String },

    /// Task is blocked
    Blocked { reason: String },

    /// Task reports context overflow (triggers handoff)
    ContextOverflow { usage_percent: u8 },

    /// Task times out
    Timeout,
}

impl Default for MockResponse {
    fn default() -> Self {
        MockResponse::Success {
            output: "Task completed successfully".to_string(),
        }
    }
}

/// Mock harness for testing
pub struct MockHarness {
    /// Task ID -> Response mapping
    responses: Arc<Mutex<HashMap<String, MockResponse>>>,

    /// Log of all messages sent
    call_log: Arc<Mutex<Vec<MockCall>>>,

    /// Default response for unmapped tasks
    default_response: MockResponse,

    /// Session counter
    session_counter: Arc<Mutex<u64>>,
}

/// Record of a call to the mock harness
#[derive(Debug, Clone)]
pub struct MockCall {
    pub session_id: String,
    pub message: String,
    pub timestamp: std::time::Instant,
}

impl MockHarness {
    /// Create a new mock harness
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            call_log: Arc::new(Mutex::new(Vec::new())),
            default_response: MockResponse::default(),
            session_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Set response for a specific task ID
    pub fn when_task(&mut self, task_id: &str, response: MockResponse) {
        self.responses
            .lock()
            .unwrap()
            .insert(task_id.to_string(), response);
    }

    /// Set default response for all tasks
    pub fn with_default_response(mut self, response: MockResponse) -> Self {
        self.default_response = response;
        self
    }

    /// Get all calls that were made
    pub fn calls(&self) -> Vec<MockCall> {
        self.call_log.lock().unwrap().clone()
    }

    /// Get calls for a specific task (by searching message content)
    pub fn calls_for_task(&self, task_id: &str) -> Vec<MockCall> {
        self.call_log
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.message.contains(task_id))
            .cloned()
            .collect()
    }

    /// Extract task ID from a message (looks for "Task X:" pattern)
    fn extract_task_id(message: &str) -> Option<String> {
        // Look for patterns like "Task 1:", "Task 1.2:", etc.
        let re = regex::Regex::new(r"Task\s+(\d+(?:\.\d+)*):").ok()?;
        re.captures(message)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    }

    /// Get response for a task
    fn get_response(&self, task_id: &str) -> MockResponse {
        self.responses
            .lock()
            .unwrap()
            .get(task_id)
            .cloned()
            .unwrap_or_else(|| self.default_response.clone())
    }
}

impl Default for MockHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Harness for MockHarness {
    fn name(&self) -> &str {
        "mock"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::ClaudeCode
    }

    async fn start_session(&self, _config: SessionConfig) -> Result<SessionHandle> {
        let mut counter = self.session_counter.lock().unwrap();
        *counter += 1;

        Ok(SessionHandle {
            id: format!("mock-session-{}", *counter),
            harness: "mock".to_string(),
            model: "mock-model".to_string(),
            parent: None,
        })
    }

    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream> {
        // Log the call
        self.call_log.lock().unwrap().push(MockCall {
            session_id: session.id.clone(),
            message: message.to_string(),
            timestamp: std::time::Instant::now(),
        });

        // Determine response based on task ID
        let task_id = Self::extract_task_id(message);
        let response = task_id
            .map(|id| self.get_response(&id))
            .unwrap_or_else(|| self.default_response.clone());

        // Build response chunks
        let chunks: Vec<ResponseChunk> = match response {
            MockResponse::Success { output } => {
                vec![ResponseChunk::Text(output), ResponseChunk::Done]
            }
            MockResponse::Blocked { reason } => {
                vec![
                    ResponseChunk::Text(format!("TASK_BLOCKED: {}", reason)),
                    ResponseChunk::Done,
                ]
            }
            MockResponse::ContextOverflow { usage_percent } => {
                vec![
                    ResponseChunk::Text(format!(
                        "Context usage at {}%. Need to handoff.",
                        usage_percent
                    )),
                    ResponseChunk::Done,
                ]
            }
            MockResponse::Timeout => {
                vec![ResponseChunk::Error("Request timed out".to_string())]
            }
        };

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn detect_subagent_spawn(&self, _chunk: &ResponseChunk) -> Option<SubagentRequest> {
        // Mock doesn't spawn subagents
        None
    }

    async fn inject_result(&self, _session: &SessionHandle, _result: SubagentResult) -> Result<()> {
        Ok(())
    }

    async fn close_session(&self, _session: &SessionHandle) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_harness_default_response() {
        let harness = MockHarness::new();
        let session = harness.start_session(SessionConfig::default()).await.unwrap();

        let mut stream = harness.send(&session, "Do something").await.unwrap();

        let mut responses = vec![];
        while let Some(chunk) = stream.next().await {
            responses.push(chunk);
        }

        assert!(matches!(responses[0], ResponseChunk::Text(_)));
        assert!(matches!(responses[1], ResponseChunk::Done));
    }

    #[tokio::test]
    async fn test_mock_harness_configured_response() {
        let mut harness = MockHarness::new();
        harness.when_task(
            "1",
            MockResponse::Blocked {
                reason: "Missing dependency".to_string(),
            },
        );

        let session = harness.start_session(SessionConfig::default()).await.unwrap();
        let mut stream = harness
            .send(&session, "Task 1: Do something")
            .await
            .unwrap();

        let mut responses = vec![];
        while let Some(chunk) = stream.next().await {
            responses.push(chunk);
        }

        if let ResponseChunk::Text(text) = &responses[0] {
            assert!(text.contains("TASK_BLOCKED"));
        } else {
            panic!("Expected text response");
        }
    }

    #[tokio::test]
    async fn test_mock_harness_call_logging() {
        let harness = MockHarness::new();
        let session = harness.start_session(SessionConfig::default()).await.unwrap();

        let _ = harness.send(&session, "Task 1: First").await.unwrap();
        let _ = harness.send(&session, "Task 2: Second").await.unwrap();

        let calls = harness.calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].message.contains("Task 1"));
        assert!(calls[1].message.contains("Task 2"));
    }
}
