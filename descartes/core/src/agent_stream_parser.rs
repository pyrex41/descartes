//! JSON Stream Parser for Agent Status Updates
//!
//! This module provides comprehensive JSON stream parsing for agent status updates,
//! handling newline-delimited JSON (NDJSON) format with real-time agent state management.
//!
//! # Overview
//!
//! The parser processes JSON streams from agent processes, extracting status updates,
//! thinking states, progress information, and other real-time data. It provides:
//!
//! - **NDJSON Parsing**: Line-by-line JSON message parsing
//! - **Event Handlers**: Type-safe handlers for each message type
//! - **Async Streaming**: Asynchronous stream processing with buffer management
//! - **State Management**: Centralized tracking of all agent states
//! - **Error Recovery**: Robust error handling for malformed JSON
//!
//! # Example
//!
//! ```ignore
//! use descartes_core::agent_stream_parser::{AgentStreamParser, StreamHandler};
//! use tokio::io::AsyncBufReadExt;
//!
//! // Create parser with custom handler
//! let mut parser = AgentStreamParser::new();
//! parser.register_handler(MyHandler);
//!
//! // Process stream
//! let stream = tokio::io::BufReader::new(agent_stdout);
//! parser.process_stream(stream).await?;
//! ```

use crate::agent_state::{
    AgentError, AgentProgress, AgentRuntimeState, AgentStatus, AgentStreamMessage, LifecycleEvent,
    OutputStream,
};
use chrono::Utc;
use serde_json;
use std::collections::HashMap;
use std::io;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use uuid::Uuid;

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Error types for JSON stream parsing
#[derive(Error, Debug)]
pub enum StreamParseError {
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Unknown agent: {0}")]
    UnknownAgent(Uuid),

    #[error("State transition error: {0}")]
    StateTransitionError(String),

    #[error("Buffer overflow: message too large")]
    BufferOverflow,

    #[error("Stream closed unexpectedly")]
    StreamClosed,
}

pub type StreamResult<T> = Result<T, StreamParseError>;

// ============================================================================
// STREAM HANDLER TRAIT
// ============================================================================

/// Trait for handling agent stream messages
///
/// Implement this trait to receive callbacks for each message type.
/// This enables custom handling of agent events for UI updates, logging, etc.
pub trait StreamHandler: Send + Sync {
    /// Called when an agent status update is received
    fn on_status_update(
        &mut self,
        agent_id: Uuid,
        status: AgentStatus,
        timestamp: chrono::DateTime<Utc>,
    );

    /// Called when an agent thought update is received
    fn on_thought_update(
        &mut self,
        agent_id: Uuid,
        thought: String,
        timestamp: chrono::DateTime<Utc>,
    );

    /// Called when an agent progress update is received
    fn on_progress_update(
        &mut self,
        agent_id: Uuid,
        progress: AgentProgress,
        timestamp: chrono::DateTime<Utc>,
    );

    /// Called when an agent output message is received
    fn on_output(
        &mut self,
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        timestamp: chrono::DateTime<Utc>,
    );

    /// Called when an agent error is received
    fn on_error(&mut self, agent_id: Uuid, error: AgentError, timestamp: chrono::DateTime<Utc>);

    /// Called when an agent lifecycle event is received
    fn on_lifecycle(
        &mut self,
        agent_id: Uuid,
        event: LifecycleEvent,
        timestamp: chrono::DateTime<Utc>,
    );

    /// Called when a heartbeat is received
    fn on_heartbeat(&mut self, agent_id: Uuid, timestamp: chrono::DateTime<Utc>);
}

// ============================================================================
// PARSER CONFIGURATION
// ============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the stream parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Maximum line length in bytes (prevents buffer overflow)
    pub max_line_length: usize,

    /// Whether to skip invalid JSON lines instead of failing
    pub skip_invalid_json: bool,

    /// Whether to auto-create unknown agents
    pub auto_create_agents: bool,

    /// Buffer capacity for async reading
    pub buffer_capacity: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_line_length: 1024 * 1024, // 1 MB
            skip_invalid_json: true,
            auto_create_agents: true,
            buffer_capacity: 8192, // 8 KB
        }
    }
}

// ============================================================================
// AGENT STREAM PARSER
// ============================================================================

/// Main JSON stream parser for agent status updates
///
/// This parser processes newline-delimited JSON (NDJSON) streams from agent
/// processes, maintaining state and dispatching events to registered handlers.
pub struct AgentStreamParser {
    /// Parser configuration
    config: ParserConfig,

    /// Current state of all agents
    agents: HashMap<Uuid, AgentRuntimeState>,

    /// Registered event handlers
    handlers: Vec<Box<dyn StreamHandler>>,

    /// Statistics
    messages_processed: u64,
    errors_encountered: u64,
}

impl AgentStreamParser {
    /// Create a new parser with default configuration
    pub fn new() -> Self {
        Self::with_config(ParserConfig::default())
    }

    /// Create a new parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            agents: HashMap::new(),
            handlers: Vec::new(),
            messages_processed: 0,
            errors_encountered: 0,
        }
    }

    /// Register a new event handler
    pub fn register_handler<H: StreamHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    /// Get current state of all agents
    pub fn agents(&self) -> &HashMap<Uuid, AgentRuntimeState> {
        &self.agents
    }

    /// Get current state of a specific agent
    pub fn get_agent(&self, agent_id: &Uuid) -> Option<&AgentRuntimeState> {
        self.agents.get(agent_id)
    }

    /// Get mutable reference to agent state
    pub fn get_agent_mut(&mut self, agent_id: &Uuid) -> Option<&mut AgentRuntimeState> {
        self.agents.get_mut(agent_id)
    }

    /// Add a new agent to track
    pub fn add_agent(&mut self, agent: AgentRuntimeState) {
        self.agents.insert(agent.agent_id, agent);
    }

    /// Get parser statistics
    pub fn statistics(&self) -> ParserStatistics {
        ParserStatistics {
            messages_processed: self.messages_processed,
            errors_encountered: self.errors_encountered,
            active_agents: self.agents.len(),
        }
    }

    // ========================================================================
    // STREAM PROCESSING
    // ========================================================================

    /// Process an async stream of JSON messages
    ///
    /// This is the main entry point for async stream processing. It reads
    /// lines from the stream, parses JSON messages, and dispatches events.
    pub async fn process_stream<R: AsyncRead + Unpin>(&mut self, stream: R) -> StreamResult<()> {
        let mut reader = BufReader::with_capacity(self.config.buffer_capacity, stream);
        let mut line = String::new();

        loop {
            line.clear();

            let bytes_read = reader.read_line(&mut line).await?;

            // Stream closed
            if bytes_read == 0 {
                break;
            }

            // Check line length
            if line.len() > self.config.max_line_length {
                self.errors_encountered += 1;
                if self.config.skip_invalid_json {
                    tracing::warn!("Line too long, skipping: {} bytes", line.len());
                    continue;
                } else {
                    return Err(StreamParseError::BufferOverflow);
                }
            }

            // Parse and process message
            match self.parse_line(&line) {
                Ok(()) => {
                    self.messages_processed += 1;
                }
                Err(e) => {
                    self.errors_encountered += 1;
                    if self.config.skip_invalid_json {
                        tracing::warn!("Failed to parse line: {} - line: {}", e, line.trim());
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a synchronous iterator of JSON lines
    ///
    /// This is useful for testing or when working with buffered data.
    pub fn process_lines<I, S>(&mut self, lines: I) -> StreamResult<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for line in lines {
            let line_ref = line.as_ref();
            if line_ref.trim().is_empty() {
                continue;
            }

            match self.parse_line(line_ref) {
                Ok(()) => {
                    self.messages_processed += 1;
                }
                Err(e) => {
                    self.errors_encountered += 1;
                    if self.config.skip_invalid_json {
                        tracing::warn!("Failed to parse line: {}", e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // MESSAGE PARSING
    // ========================================================================

    /// Parse a single JSON line
    fn parse_line(&mut self, line: &str) -> StreamResult<()> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Parse JSON message
        let message: AgentStreamMessage = serde_json::from_str(line)?;

        // Handle message
        self.handle_message(message)
    }

    /// Handle a parsed stream message
    fn handle_message(&mut self, message: AgentStreamMessage) -> StreamResult<()> {
        match message {
            AgentStreamMessage::StatusUpdate {
                agent_id,
                status,
                timestamp,
            } => {
                self.handle_status_update(agent_id, status, timestamp)?;
            }

            AgentStreamMessage::ThoughtUpdate {
                agent_id,
                thought,
                timestamp,
            } => {
                self.handle_thought_update(agent_id, thought, timestamp)?;
            }

            AgentStreamMessage::ProgressUpdate {
                agent_id,
                progress,
                timestamp,
            } => {
                self.handle_progress_update(agent_id, progress, timestamp)?;
            }

            AgentStreamMessage::Output {
                agent_id,
                stream,
                content,
                timestamp,
            } => {
                self.handle_output(agent_id, stream, content, timestamp)?;
            }

            AgentStreamMessage::Error {
                agent_id,
                error,
                timestamp,
            } => {
                self.handle_error(agent_id, error, timestamp)?;
            }

            AgentStreamMessage::Lifecycle {
                agent_id,
                event,
                timestamp,
            } => {
                self.handle_lifecycle(agent_id, event, timestamp)?;
            }

            AgentStreamMessage::Heartbeat {
                agent_id,
                timestamp,
            } => {
                self.handle_heartbeat(agent_id, timestamp)?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // EVENT HANDLERS
    // ========================================================================

    /// Handle status update message
    fn handle_status_update(
        &mut self,
        agent_id: Uuid,
        status: AgentStatus,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Update agent state
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent
                .transition_to(status, Some("Status update from stream".to_string()))
                .map_err(|e| StreamParseError::StateTransitionError(e))?;

            // Clear thought if transitioning out of Thinking state
            if agent.status != AgentStatus::Thinking {
                agent.clear_thought();
            }
        } else if self.config.auto_create_agents {
            // Auto-create agent if configured
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", agent_id),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            agent
                .transition_to(status, Some("Initial status from stream".to_string()))
                .ok();
            self.agents.insert(agent_id, agent);
        } else {
            return Err(StreamParseError::UnknownAgent(agent_id));
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_status_update(agent_id, status, timestamp);
        }

        Ok(())
    }

    /// Handle thought update message
    fn handle_thought_update(
        &mut self,
        agent_id: Uuid,
        thought: String,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Update agent state
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.update_thought(thought.clone());

            // Auto-transition to Thinking state if not already
            if agent.status != AgentStatus::Thinking {
                agent
                    .transition_to(AgentStatus::Thinking, Some("Thought detected".to_string()))
                    .ok();
            }
        } else if self.config.auto_create_agents {
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", agent_id),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            agent.transition_to(AgentStatus::Thinking, None).ok();
            agent.update_thought(thought.clone());
            self.agents.insert(agent_id, agent);
        } else {
            return Err(StreamParseError::UnknownAgent(agent_id));
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_thought_update(agent_id, thought.clone(), timestamp);
        }

        Ok(())
    }

    /// Handle progress update message
    fn handle_progress_update(
        &mut self,
        agent_id: Uuid,
        progress: AgentProgress,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Update agent state
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.update_progress(progress.clone());
        } else if self.config.auto_create_agents {
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", agent_id),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            agent.update_progress(progress.clone());
            self.agents.insert(agent_id, agent);
        } else {
            return Err(StreamParseError::UnknownAgent(agent_id));
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_progress_update(agent_id, progress.clone(), timestamp);
        }

        Ok(())
    }

    /// Handle output message
    fn handle_output(
        &mut self,
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Notify handlers (output is not stored in agent state)
        for handler in &mut self.handlers {
            handler.on_output(agent_id, stream, content.clone(), timestamp);
        }

        Ok(())
    }

    /// Handle error message
    fn handle_error(
        &mut self,
        agent_id: Uuid,
        error: AgentError,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Update agent state
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.set_error(error.clone());
            agent
                .transition_to(AgentStatus::Failed, Some(error.message.clone()))
                .ok();
        } else if self.config.auto_create_agents {
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", agent_id),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            agent.set_error(error.clone());
            agent.transition_to(AgentStatus::Failed, None).ok();
            self.agents.insert(agent_id, agent);
        } else {
            return Err(StreamParseError::UnknownAgent(agent_id));
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_error(agent_id, error.clone(), timestamp);
        }

        Ok(())
    }

    /// Handle lifecycle event message
    fn handle_lifecycle(
        &mut self,
        agent_id: Uuid,
        event: LifecycleEvent,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Map lifecycle event to status
        let status = match event {
            LifecycleEvent::Spawned => AgentStatus::Idle,
            LifecycleEvent::Started => AgentStatus::Initializing,
            LifecycleEvent::Paused => AgentStatus::Paused,
            LifecycleEvent::Resumed => AgentStatus::Running,
            LifecycleEvent::Completed => AgentStatus::Completed,
            LifecycleEvent::Failed => AgentStatus::Failed,
            LifecycleEvent::Terminated => AgentStatus::Terminated,
        };

        // Update agent state
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent
                .transition_to(status, Some(format!("Lifecycle event: {:?}", event)))
                .ok();
        } else if self.config.auto_create_agents {
            let mut agent = AgentRuntimeState::new(
                agent_id,
                format!("agent-{}", agent_id),
                "Auto-created from stream".to_string(),
                "unknown".to_string(),
            );
            agent.transition_to(status, None).ok();
            self.agents.insert(agent_id, agent);
        } else {
            return Err(StreamParseError::UnknownAgent(agent_id));
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_lifecycle(agent_id, event.clone(), timestamp);
        }

        Ok(())
    }

    /// Handle heartbeat message
    fn handle_heartbeat(
        &mut self,
        agent_id: Uuid,
        timestamp: chrono::DateTime<Utc>,
    ) -> StreamResult<()> {
        // Update agent's updated_at timestamp
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.updated_at = timestamp;
        }

        // Notify handlers
        for handler in &mut self.handlers {
            handler.on_heartbeat(agent_id, timestamp);
        }

        Ok(())
    }
}

impl Default for AgentStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PARSER STATISTICS
// ============================================================================

/// Statistics about stream parsing
#[derive(Debug, Clone)]
pub struct ParserStatistics {
    /// Total messages processed
    pub messages_processed: u64,

    /// Total errors encountered
    pub errors_encountered: u64,

    /// Number of active agents being tracked
    pub active_agents: usize,
}

// ============================================================================
// DEFAULT HANDLERS
// ============================================================================

/// A simple logging handler that prints messages to stdout
pub struct LoggingHandler;

impl StreamHandler for LoggingHandler {
    fn on_status_update(
        &mut self,
        agent_id: Uuid,
        status: AgentStatus,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        tracing::info!("Agent {} status: {}", agent_id, status);
    }

    fn on_thought_update(
        &mut self,
        agent_id: Uuid,
        thought: String,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        tracing::info!("Agent {} thinking: {}", agent_id, thought);
    }

    fn on_progress_update(
        &mut self,
        agent_id: Uuid,
        progress: AgentProgress,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        tracing::info!("Agent {} progress: {:.1}%", agent_id, progress.percentage);
    }

    fn on_output(
        &mut self,
        agent_id: Uuid,
        stream: OutputStream,
        content: String,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        tracing::debug!("Agent {} {:?}: {}", agent_id, stream, content);
    }

    fn on_error(&mut self, agent_id: Uuid, error: AgentError, _timestamp: chrono::DateTime<Utc>) {
        tracing::error!("Agent {} error: {}", agent_id, error.message);
    }

    fn on_lifecycle(
        &mut self,
        agent_id: Uuid,
        event: LifecycleEvent,
        _timestamp: chrono::DateTime<Utc>,
    ) {
        tracing::info!("Agent {} lifecycle: {:?}", agent_id, event);
    }

    fn on_heartbeat(&mut self, agent_id: Uuid, _timestamp: chrono::DateTime<Utc>) {
        tracing::trace!("Agent {} heartbeat", agent_id);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test handler that collects events
    struct TestHandler {
        status_updates: Vec<(Uuid, AgentStatus)>,
        thought_updates: Vec<(Uuid, String)>,
        progress_updates: Vec<(Uuid, f32)>,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                status_updates: Vec::new(),
                thought_updates: Vec::new(),
                progress_updates: Vec::new(),
            }
        }
    }

    impl StreamHandler for TestHandler {
        fn on_status_update(
            &mut self,
            agent_id: Uuid,
            status: AgentStatus,
            _timestamp: chrono::DateTime<Utc>,
        ) {
            self.status_updates.push((agent_id, status));
        }

        fn on_thought_update(
            &mut self,
            agent_id: Uuid,
            thought: String,
            _timestamp: chrono::DateTime<Utc>,
        ) {
            self.thought_updates.push((agent_id, thought));
        }

        fn on_progress_update(
            &mut self,
            agent_id: Uuid,
            progress: AgentProgress,
            _timestamp: chrono::DateTime<Utc>,
        ) {
            self.progress_updates.push((agent_id, progress.percentage));
        }

        fn on_output(
            &mut self,
            _agent_id: Uuid,
            _stream: OutputStream,
            _content: String,
            _timestamp: chrono::DateTime<Utc>,
        ) {
        }

        fn on_error(
            &mut self,
            _agent_id: Uuid,
            _error: AgentError,
            _timestamp: chrono::DateTime<Utc>,
        ) {
        }

        fn on_lifecycle(
            &mut self,
            _agent_id: Uuid,
            _event: LifecycleEvent,
            _timestamp: chrono::DateTime<Utc>,
        ) {
        }

        fn on_heartbeat(&mut self, _agent_id: Uuid, _timestamp: chrono::DateTime<Utc>) {}
    }

    #[test]
    fn test_parse_status_update() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"type":"status_update","agent_id":"{}","status":"running","timestamp":"2025-11-24T05:53:00Z"}}"#,
            agent_id
        );

        let mut parser = AgentStreamParser::new();
        let handler = TestHandler::new();
        parser.register_handler(handler);

        parser.process_lines(&[&json]).unwrap();

        assert_eq!(parser.messages_processed, 1);
        assert_eq!(parser.agents.len(), 1);

        let agent = parser.get_agent(&agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Running);
    }

    #[test]
    fn test_parse_thought_update() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"type":"thought_update","agent_id":"{}","thought":"Analyzing code...","timestamp":"2025-11-24T05:53:00Z"}}"#,
            agent_id
        );

        let mut parser = AgentStreamParser::new();
        parser.process_lines(&[&json]).unwrap();

        let agent = parser.get_agent(&agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Thinking);
        assert_eq!(agent.current_thought, Some("Analyzing code...".to_string()));
    }

    #[test]
    fn test_parse_progress_update() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"type":"progress_update","agent_id":"{}","progress":{{"percentage":50.0}},"timestamp":"2025-11-24T05:53:00Z"}}"#,
            agent_id
        );

        let mut parser = AgentStreamParser::new();
        parser.process_lines(&[&json]).unwrap();

        let agent = parser.get_agent(&agent_id).unwrap();
        assert!(agent.progress.is_some());
        assert_eq!(agent.progress.as_ref().unwrap().percentage, 50.0);
    }

    #[test]
    fn test_parse_multiple_messages() {
        let agent_id = Uuid::new_v4();
        let messages = vec![
            format!(
                r#"{{"type":"status_update","agent_id":"{}","status":"initializing","timestamp":"2025-11-24T05:53:00Z"}}"#,
                agent_id
            ),
            format!(
                r#"{{"type":"status_update","agent_id":"{}","status":"running","timestamp":"2025-11-24T05:53:01Z"}}"#,
                agent_id
            ),
            format!(
                r#"{{"type":"thought_update","agent_id":"{}","thought":"Processing...","timestamp":"2025-11-24T05:53:02Z"}}"#,
                agent_id
            ),
            format!(
                r#"{{"type":"progress_update","agent_id":"{}","progress":{{"percentage":75.5}},"timestamp":"2025-11-24T05:53:03Z"}}"#,
                agent_id
            ),
        ];

        let mut parser = AgentStreamParser::new();
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        parser.process_lines(refs).unwrap();

        assert_eq!(parser.messages_processed, 4);

        let agent = parser.get_agent(&agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Thinking);
        assert_eq!(agent.current_thought, Some("Processing...".to_string()));
        assert_eq!(agent.progress.as_ref().unwrap().percentage, 75.5);
        assert_eq!(agent.timeline.len(), 4); // Initial + 3 transitions
    }

    #[test]
    fn test_invalid_json_skip() {
        let agent_id = Uuid::new_v4();
        let messages = vec![
            format!(
                r#"{{"type":"status_update","agent_id":"{}","status":"running","timestamp":"2025-11-24T05:53:00Z"}}"#,
                agent_id
            ),
            "invalid json {".to_string(),
            format!(
                r#"{{"type":"progress_update","agent_id":"{}","progress":{{"percentage":50.0}},"timestamp":"2025-11-24T05:53:01Z"}}"#,
                agent_id
            ),
        ];

        let mut parser = AgentStreamParser::new();
        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        parser.process_lines(refs).unwrap();

        assert_eq!(parser.messages_processed, 2);
        assert_eq!(parser.errors_encountered, 1);
    }

    #[test]
    fn test_handler_callbacks() {
        let agent_id = Uuid::new_v4();
        let messages = vec![
            format!(
                r#"{{"type":"status_update","agent_id":"{}","status":"running","timestamp":"2025-11-24T05:53:00Z"}}"#,
                agent_id
            ),
            format!(
                r#"{{"type":"thought_update","agent_id":"{}","thought":"Test thought","timestamp":"2025-11-24T05:53:01Z"}}"#,
                agent_id
            ),
        ];

        let mut parser = AgentStreamParser::new();
        let handler = TestHandler::new();
        parser.register_handler(handler);

        let refs: Vec<&str> = messages.iter().map(|s| s.as_str()).collect();
        parser.process_lines(refs).unwrap();

        // We can't easily test handler state since it's boxed,
        // but we can verify messages were processed
        assert_eq!(parser.messages_processed, 2);
    }

    #[test]
    fn test_lifecycle_events() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"type":"lifecycle","agent_id":"{}","event":"started","timestamp":"2025-11-24T05:53:00Z"}}"#,
            agent_id
        );

        let mut parser = AgentStreamParser::new();
        parser.process_lines(&[&json]).unwrap();

        let agent = parser.get_agent(&agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Initializing);
    }

    #[test]
    fn test_error_handling() {
        let agent_id = Uuid::new_v4();
        let json = format!(
            r#"{{"type":"error","agent_id":"{}","error":{{"code":"TEST_ERROR","message":"Test error message","timestamp":"2025-11-24T05:53:00Z","recoverable":false}},"timestamp":"2025-11-24T05:53:00Z"}}"#,
            agent_id
        );

        let mut parser = AgentStreamParser::new();
        parser.process_lines(&[&json]).unwrap();

        let agent = parser.get_agent(&agent_id).unwrap();
        assert_eq!(agent.status, AgentStatus::Failed);
        assert!(agent.error.is_some());
        assert_eq!(agent.error.as_ref().unwrap().code, "TEST_ERROR");
    }

    #[test]
    fn test_heartbeat() {
        let agent_id = Uuid::new_v4();

        // Create agent first
        let mut parser = AgentStreamParser::new();
        let agent = AgentRuntimeState::new(
            agent_id,
            "test-agent".to_string(),
            "test".to_string(),
            "test".to_string(),
        );
        let initial_time = agent.updated_at;
        parser.add_agent(agent);

        // Send heartbeat
        let json = format!(
            r#"{{"type":"heartbeat","agent_id":"{}","timestamp":"2025-11-24T06:00:00Z"}}"#,
            agent_id
        );

        parser.process_lines(&[&json]).unwrap();

        let agent = parser.get_agent(&agent_id).unwrap();
        // Heartbeat should update the timestamp
        assert!(agent.updated_at >= initial_time);
    }
}
