//! Subagent spawning with visibility and depth control

use futures::StreamExt;
use tracing::{debug, info, warn};

use super::AgentCategory;
use crate::harness::{
    Harness, ResponseChunk, SessionConfig, SubagentMetrics, SubagentRequest,
};
use crate::transcript::Transcript;
use crate::Result;

/// Result from a subagent execution
#[derive(Debug, Clone)]
pub struct SubagentResult {
    /// Session ID of the subagent
    pub session_id: String,
    /// Final output/summary
    pub output: String,
    /// Whether the subagent succeeded
    pub success: bool,
    /// Metrics about the execution
    pub metrics: SubagentMetrics,
}

impl SubagentResult {
    /// Create a blocked result (for nested spawn attempts)
    pub fn blocked(reason: &str) -> Self {
        Self {
            session_id: String::new(),
            output: format!("Blocked: {}", reason),
            success: false,
            metrics: SubagentMetrics::default(),
        }
    }

    /// Check if the subagent passed (for validator)
    pub fn passed(&self) -> bool {
        self.success
    }

    /// Get a summary of the result
    pub fn summary(&self) -> String {
        if self.success {
            format!(
                "Session {}: completed in {}ms, {} tool calls",
                self.session_id, self.metrics.duration_ms, self.metrics.tools_called
            )
        } else {
            format!("Session {}: failed - {}", self.session_id, self.output)
        }
    }
}

/// Spawn a subagent with full transcript capture
///
/// # Arguments
/// * `harness` - The harness to use for execution
/// * `category` - The agent category (determines model, tools)
/// * `prompt` - The task for the subagent
/// * `parent_transcript` - Optional parent transcript to link to
///
/// # Returns
/// Result containing the subagent's output and metrics
pub async fn spawn_subagent(
    harness: &dyn Harness,
    category: AgentCategory,
    prompt: String,
    parent_transcript: Option<&mut Transcript>,
) -> Result<SubagentResult> {
    info!(
        "Spawning {} subagent: {}",
        category,
        truncate(&prompt, 50)
    );

    let category_config = category.default_config();

    // Create session config
    let session_config = SessionConfig {
        model: category_config.model.clone(),
        tools: category_config.tools.clone(),
        system_prompt: None,
        parent: None,
        is_subagent: true,
    };

    // Start session
    let session = harness.start_session(session_config).await?;

    // Create transcript for this subagent
    let mut transcript = Transcript::new()
        .with_harness(harness.name())
        .with_category(category.name());

    if let Some(parent) = &parent_transcript {
        transcript = transcript.with_parent(parent.id());
    }

    // Record the initial prompt
    transcript.record_user_message(&prompt);

    // Execute
    let start = std::time::Instant::now();
    let mut response = harness.send(&session, &prompt).await?;
    let mut output = String::new();
    let mut tools_called = 0;
    let mut success = true;

    while let Some(chunk) = response.next().await {
        // Record chunk in transcript
        transcript.record_chunk(&chunk);

        match &chunk {
            ResponseChunk::Text(text) => {
                output.push_str(text);
            }
            ResponseChunk::ToolCall(tool) => {
                tools_called += 1;
                debug!("Subagent tool call: {} ({})", tool.name, tool.id);
            }
            ResponseChunk::SubagentSpawn(nested_req) => {
                // Block nested spawns - this is the 1-level enforcement
                warn!(
                    "Subagent attempted nested spawn: {} - {}",
                    nested_req.category,
                    truncate(&nested_req.prompt, 30)
                );

                let blocked =
                    crate::harness::SubagentResult::blocked("Subagents cannot spawn subagents");
                harness.inject_result(&session, blocked).await?;
            }
            ResponseChunk::Error(e) => {
                warn!("Subagent error: {}", e);
                output = format!("Error: {}", e);
                success = false;
                break;
            }
            ResponseChunk::Done => {
                debug!("Subagent completed");
                break;
            }
            _ => {}
        }
    }

    let duration = start.elapsed();

    // Close session
    harness.close_session(&session).await?;

    // Finalize transcript
    transcript.finalize();

    // Record in parent transcript if provided
    if let Some(parent) = parent_transcript {
        parent.record_subagent(transcript.id(), category.name(), &prompt);
    }

    let result = SubagentResult {
        session_id: session.id,
        output,
        success,
        metrics: SubagentMetrics {
            tokens_in: 0,  // TODO: Get from harness metrics
            tokens_out: 0, // TODO: Get from harness metrics
            tokens_total: 0,
            duration_ms: duration.as_millis() as u64,
            tools_called,
        },
    };

    info!(
        "Subagent {} {} in {}ms",
        result.session_id,
        if success { "completed" } else { "failed" },
        result.metrics.duration_ms
    );

    Ok(result)
}

/// Spawn multiple subagents in parallel
pub async fn spawn_parallel(
    harness: &dyn Harness,
    requests: Vec<(AgentCategory, String)>,
    parent_transcript: Option<&mut Transcript>,
) -> Vec<Result<SubagentResult>> {
    use futures::future::join_all;

    info!("Spawning {} subagents in parallel", requests.len());

    // We can't easily share the parent transcript across parallel futures,
    // so we'll record after completion
    let futures: Vec<_> = requests
        .into_iter()
        .map(|(category, prompt)| spawn_subagent(harness, category, prompt, None))
        .collect();

    let results = join_all(futures).await;

    // Record all subagents in parent transcript
    if let Some(parent) = parent_transcript {
        for result in &results {
            if let Ok(r) = result {
                parent.record_subagent_completion(&r.session_id, r.success);
            }
        }
    }

    results
}

/// Truncate a string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_result_blocked() {
        let result = SubagentResult::blocked("test reason");
        assert!(!result.success);
        assert!(!result.passed());
        assert!(result.output.contains("Blocked"));
    }
}
