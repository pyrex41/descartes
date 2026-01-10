//! OpenCode harness implementation
//!
//! Communicates with OpenCode TUI via IPC (Unix socket or named pipe).

use async_trait::async_trait;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult,
};
use crate::config::OpenCodeConfig;
use crate::{Error, Result};

/// OpenCode harness using IPC
pub struct OpenCodeHarness {
    /// Socket path for IPC
    socket_path: PathBuf,
    /// Default model
    model: String,
}

impl OpenCodeHarness {
    /// Create a new OpenCode harness
    pub fn new(config: &OpenCodeConfig) -> Result<Self> {
        let socket_path = config
            .socket_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp/opencode.sock"));

        let model = config
            .model
            .clone()
            .unwrap_or_else(|| "sonnet".to_string());

        Ok(Self { socket_path, model })
    }
}

#[async_trait]
impl Harness for OpenCodeHarness {
    fn name(&self) -> &str {
        "opencode"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::OpenCode
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        let session_id = Uuid::new_v4().to_string();

        let model = if config.model.is_empty() {
            self.model.clone()
        } else {
            config.model
        };

        info!(
            "Starting OpenCode session {} with model {}",
            session_id, model
        );

        // TODO: Connect to OpenCode socket and start session
        // For now, return a placeholder

        Ok(SessionHandle {
            id: session_id,
            harness: self.name().to_string(),
            model,
            parent: config.parent.map(|p| p.id),
        })
    }

    async fn send(&self, _session: &SessionHandle, _message: &str) -> Result<ResponseStream> {
        // TODO: Implement IPC communication with OpenCode
        Err(Error::Harness(
            "OpenCode harness not yet implemented".to_string(),
        ))
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        // OpenCode uses similar patterns to Claude Code
        match chunk {
            ResponseChunk::SubagentSpawn(req) => Some(req.clone()),
            _ => None,
        }
    }

    async fn inject_result(
        &self,
        _session: &SessionHandle,
        _result: SubagentResult,
    ) -> Result<()> {
        // TODO: Implement result injection via IPC
        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing OpenCode session {}", session.id);
        // TODO: Close IPC session
        Ok(())
    }
}
