//! Codex harness implementation
//!
//! Uses the Codex API directly for agent execution.

use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult,
};
use crate::config::CodexConfig;
use crate::{Error, Result};

/// Codex harness using the API directly
pub struct CodexHarness {
    /// API base URL
    api_base: String,
    /// API key
    api_key: String,
    /// HTTP client
    client: reqwest::Client,
}

impl CodexHarness {
    /// Create a new Codex harness
    pub fn new(config: &CodexConfig) -> Result<Self> {
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| Error::Config("Codex API key not configured".to_string()))?;

        let client = reqwest::Client::new();

        Ok(Self {
            api_base,
            api_key,
            client,
        })
    }
}

#[async_trait]
impl Harness for CodexHarness {
    fn name(&self) -> &str {
        "codex"
    }

    fn kind(&self) -> HarnessKind {
        HarnessKind::Codex
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        let session_id = Uuid::new_v4().to_string();

        info!(
            "Starting Codex session {} with model {}",
            session_id, config.model
        );

        Ok(SessionHandle {
            id: session_id,
            harness: self.name().to_string(),
            model: config.model,
            parent: config.parent.map(|p| p.id),
        })
    }

    async fn send(&self, _session: &SessionHandle, _message: &str) -> Result<ResponseStream> {
        // TODO: Implement Codex API communication
        // This would use the OpenAI-compatible API with function calling
        Err(Error::Harness(
            "Codex harness not yet implemented".to_string(),
        ))
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        // Codex uses function-calling schema for agent dispatch
        match chunk {
            ResponseChunk::SubagentSpawn(req) => Some(req.clone()),
            ResponseChunk::ToolCall(tool) => {
                // Look for Codex-style agent dispatch
                if tool.name == "dispatch_agent" || tool.name == "spawn_agent" {
                    let prompt = tool
                        .arguments
                        .get("task")
                        .or_else(|| tool.arguments.get("prompt"))
                        .and_then(|p| p.as_str())?;

                    let category = tool
                        .arguments
                        .get("agent_type")
                        .or_else(|| tool.arguments.get("category"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("searcher")
                        .to_string();

                    Some(SubagentRequest {
                        category,
                        prompt: prompt.to_string(),
                        model: None,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    async fn inject_result(
        &self,
        _session: &SessionHandle,
        _result: SubagentResult,
    ) -> Result<()> {
        // TODO: Implement function result injection
        Ok(())
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        info!("Closing Codex session {}", session.id);
        Ok(())
    }
}
