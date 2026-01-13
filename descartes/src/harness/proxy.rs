//! Subagent proxy for intercepting and routing subagent spawns
//!
//! This module wraps a harness and intercepts subagent spawn requests,
//! routing them to isolated sessions with full transcript capture.

use async_trait::async_trait;
use futures::StreamExt;
use tracing::{debug, info, warn};

use super::{
    Harness, HarnessKind, ResponseChunk, ResponseStream, SessionConfig, SessionHandle,
    SubagentRequest, SubagentResult,
};
use crate::agent::AgentCategory;
use crate::transcript::Transcript;
use crate::{Config, Result};

/// Proxy that intercepts subagent spawns
pub struct SubagentProxy<H: Harness> {
    /// Underlying harness
    inner: H,
    /// Configuration
    config: Config,
    /// Current nesting depth (0 = top level)
    depth: usize,
    /// Maximum allowed depth (1 = one level of subagents)
    max_depth: usize,
}

impl<H: Harness> SubagentProxy<H> {
    /// Create a new proxy wrapping a harness
    pub fn new(inner: H, config: Config) -> Self {
        Self {
            inner,
            config,
            depth: 0,
            max_depth: 1, // Only allow 1 level of subagents
        }
    }

    /// Create a child proxy for subagent execution
    fn child_proxy(&self) -> Self {
        Self {
            inner: todo!("Need to clone harness or use Arc"),
            config: self.config.clone(),
            depth: self.depth + 1,
            max_depth: self.max_depth,
        }
    }

    /// Check if we can spawn a subagent
    fn can_spawn(&self) -> bool {
        self.depth < self.max_depth
    }

    /// Handle a subagent spawn request
    pub async fn handle_subagent_spawn(
        &self,
        request: SubagentRequest,
        parent_transcript: &mut Transcript,
    ) -> Result<SubagentResult> {
        if !self.can_spawn() {
            warn!(
                "Blocking nested subagent spawn at depth {} (max: {})",
                self.depth, self.max_depth
            );
            return Ok(SubagentResult::blocked(
                "Subagents cannot spawn further subagents",
            ));
        }

        info!(
            "Spawning {} subagent: {}",
            request.category,
            truncate(&request.prompt, 50)
        );

        // Get category config
        let category: AgentCategory = request.category.parse()?;
        let category_config = self
            .config
            .get_category(&request.category)
            .cloned()
            .unwrap_or_else(|| category.default_config());

        // Determine model
        let model = request
            .model
            .unwrap_or_else(|| category_config.model.clone());

        // Create session config
        let session_config = SessionConfig {
            model,
            tools: category_config.tools.clone(),
            system_prompt: None,
            parent: None, // Will be set when we have parent handle
            is_subagent: true,
        };

        // Start subagent session
        let session = self.inner.start_session(session_config).await?;

        // Create transcript for subagent
        let mut transcript = Transcript::new()
            .with_harness(self.inner.name())
            .with_parent(parent_transcript.id())
            .with_category(&request.category);

        // Execute subagent
        let start = std::time::Instant::now();
        let mut response = self.inner.send(&session, &request.prompt).await?;
        let mut output = String::new();
        let mut tools_called = 0;

        while let Some(chunk) = response.next().await {
            // Record in transcript
            transcript.record_chunk(&chunk);

            match &chunk {
                ResponseChunk::Text(text) => {
                    output.push_str(text);
                }
                ResponseChunk::ToolCall(_) => {
                    tools_called += 1;
                }
                ResponseChunk::SubagentSpawn(nested_req) => {
                    // Block nested spawns
                    warn!("Subagent attempted nested spawn: {:?}", nested_req);
                    let blocked = SubagentResult::blocked(
                        "Subagents cannot spawn further subagents",
                    );
                    self.inner.inject_result(&session, blocked).await?;
                }
                ResponseChunk::Done => break,
                ResponseChunk::Error(e) => {
                    warn!("Subagent error: {}", e);
                    output = format!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        let duration = start.elapsed();

        // Close session
        self.inner.close_session(&session).await?;

        // Save transcript
        let transcript_path = self.config.transcript_dir().join(format!(
            "{}.scg",
            transcript.id()
        ));
        transcript.save_scg(&transcript_path)?;

        // Record subagent in parent transcript
        parent_transcript.record_subagent(
            transcript.id(),
            &request.category,
            &request.prompt,
        );

        // Build result
        let result = SubagentResult {
            session_id: session.id,
            output,
            success: true,
            metrics: super::SubagentMetrics {
                tokens_in: 0,  // TODO: Track from harness
                tokens_out: 0, // TODO: Track from harness
                tokens_total: 0,
                duration_ms: duration.as_millis() as u64,
                tools_called,
            },
        };

        debug!(
            "Subagent {} completed in {}ms",
            result.session_id, result.metrics.duration_ms
        );

        Ok(result)
    }
}

#[async_trait]
impl<H: Harness + Clone> Harness for SubagentProxy<H> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn kind(&self) -> HarnessKind {
        self.inner.kind()
    }

    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle> {
        self.inner.start_session(config).await
    }

    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream> {
        self.inner.send(session, message).await
    }

    fn detect_subagent_spawn(&self, chunk: &ResponseChunk) -> Option<SubagentRequest> {
        self.inner.detect_subagent_spawn(chunk)
    }

    async fn inject_result(
        &self,
        session: &SessionHandle,
        result: SubagentResult,
    ) -> Result<()> {
        self.inner.inject_result(session, result).await
    }

    async fn close_session(&self, session: &SessionHandle) -> Result<()> {
        self.inner.close_session(session).await
    }
}

/// Truncate a string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
