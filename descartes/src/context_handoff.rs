//! Context handoff system for managing agent context usage
//!
//! Monitors token accumulation during agent execution and performs handoffs
//! when approaching context window limits. This prevents context overflow
//! by killing the current agent and spawning a fresh one with a summary.

use tracing::{debug, info};

/// Estimate token count from text using the 4 chars ≈ 1 token heuristic
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Monitor for tracking context usage during agent execution
#[derive(Debug, Clone)]
pub struct ContextMonitor {
    /// Total context window size in tokens
    pub context_window: usize,
    /// Threshold percentage (0.0-1.0) to trigger handoff
    pub threshold: f64,
    /// Cumulative tokens used so far
    pub tokens_used: usize,
    /// Number of handoffs performed
    pub handoffs: usize,
    /// Whether monitoring is enabled
    pub enabled: bool,
}

impl ContextMonitor {
    /// Create a new context monitor
    ///
    /// # Arguments
    ///
    /// * `context_window` - Total context window size in tokens (e.g., 200_000)
    /// * `threshold` - Percentage (0.0-1.0) at which to trigger handoff (e.g., 0.6 for 60%)
    pub fn new(context_window: usize, threshold: f64) -> Self {
        Self {
            context_window,
            threshold: threshold.clamp(0.0, 1.0),
            tokens_used: 0,
            handoffs: 0,
            enabled: true,
        }
    }

    /// Create a monitor with default settings (200K context, 60% threshold)
    pub fn default() -> Self {
        Self::new(200_000, 0.6)
    }

    /// Create a disabled monitor (useful for testing or single-task execution)
    pub fn disabled() -> Self {
        Self {
            context_window: 200_000,
            threshold: 0.6,
            tokens_used: 0,
            handoffs: 0,
            enabled: false,
        }
    }

    /// Record token usage from a text chunk
    pub fn record_usage(&mut self, text: &str) {
        if !self.enabled {
            return;
        }
        let tokens = estimate_tokens(text);
        self.tokens_used += tokens;
        debug!(
            "Context usage: {} / {} tokens ({:.1}%)",
            self.tokens_used,
            self.context_window,
            self.usage_percentage() * 100.0
        );
    }

    /// Get the current usage as a percentage (0.0-1.0)
    pub fn usage_percentage(&self) -> f64 {
        if self.context_window == 0 {
            return 1.0;
        }
        (self.tokens_used as f64) / (self.context_window as f64)
    }

    /// Check if handoff should be triggered
    pub fn should_handoff(&self) -> bool {
        self.enabled && self.usage_percentage() >= self.threshold
    }

    /// Get tokens remaining before handoff threshold
    pub fn tokens_until_handoff(&self) -> usize {
        if !self.enabled {
            return usize::MAX;
        }
        let threshold_tokens = (self.context_window as f64 * self.threshold) as usize;
        threshold_tokens.saturating_sub(self.tokens_used)
    }

    /// Reset the monitor (called after handoff)
    pub fn reset(&mut self) {
        self.tokens_used = 0;
        self.handoffs += 1;
        info!(
            "Context monitor reset. Handoff #{} completed.",
            self.handoffs
        );
    }

    /// Get a status summary string
    pub fn status(&self) -> String {
        if !self.enabled {
            return "Context monitoring disabled".to_string();
        }
        format!(
            "Context: {}/{} tokens ({:.1}%), {} handoffs, {} tokens until handoff",
            self.tokens_used,
            self.context_window,
            self.usage_percentage() * 100.0,
            self.handoffs,
            self.tokens_until_handoff()
        )
    }
}

/// Summarize agent progress from a transcript
///
/// Extracts key accomplishments, decisions, and state from the agent's
/// full response text. This is used to create a compact context summary
/// for the next agent after handoff.
///
/// # Arguments
///
/// * `transcript` - Full response text from the agent
///
/// # Returns
///
/// A concise summary suitable for prepending to the next agent's prompt
pub fn summarize_agent_progress(transcript: &str) -> String {
    // Strategy: Extract key information while being token-efficient
    // 1. Look for explicit success/completion markers
    // 2. Extract file modifications
    // 3. Extract test results
    // 4. Extract error/blocking information
    // 5. Preserve last few substantive lines

    let mut summary_parts = Vec::new();

    // Check for task completion markers
    if transcript.contains("DONE]") || transcript.contains("completed successfully") {
        summary_parts.push("Status: Task completed".to_string());
    } else if transcript.contains("BLOCKED") {
        // Extract blocked reason
        for line in transcript.lines().rev() {
            if let Some(reason) = line.trim().strip_prefix("TASK_BLOCKED:") {
                summary_parts.push(format!("Status: Blocked - {}", reason.trim()));
                break;
            }
        }
    } else {
        summary_parts.push("Status: In progress".to_string());
    }

    // Extract file modifications (look for common patterns)
    let mut modified_files = Vec::new();
    for line in transcript.lines() {
        let line_lower = line.to_lowercase();
        if (line_lower.contains("modified:")
            || line_lower.contains("created:")
            || line_lower.contains("updated:"))
            && !line_lower.contains("git")
        {
            if let Some(file_part) = line.split(':').nth(1) {
                modified_files.push(file_part.trim().to_string());
            }
        }
    }

    if !modified_files.is_empty() {
        summary_parts.push(format!("Files modified: {}", modified_files.join(", ")));
    }

    // Extract test results
    if transcript.contains("test result:") || transcript.contains("tests passing") {
        for line in transcript.lines() {
            if line.contains("test result:") || line.contains("passed") && line.contains("failed") {
                summary_parts.push(format!("Tests: {}", line.trim()));
                break;
            }
        }
    }

    // Get last substantive content (skip empty lines and common markers)
    let substantive_lines: Vec<&str> = transcript
        .lines()
        .rev()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("===")
                && !trimmed.starts_with("---")
                && trimmed.len() > 10
        })
        .take(3)
        .collect();

    if !substantive_lines.is_empty() {
        summary_parts.push("Recent progress:".to_string());
        for line in substantive_lines.iter().rev() {
            summary_parts.push(format!("  {}", line.trim()));
        }
    }

    // Build final summary
    if summary_parts.is_empty() {
        return "Previous agent progress: No significant updates recorded.".to_string();
    }

    format!(
        "=== Previous Agent Summary ===\n\n{}\n\n===",
        summary_parts.join("\n")
    )
}

/// Handoff context for creating a new agent with summarized history
///
/// This struct captures the state needed to spawn a fresh agent after
/// a context handoff, including a summary of previous work.
#[derive(Debug, Clone)]
pub struct HandoffContext {
    /// Summary of previous agent's work
    pub summary: String,
    /// Original task spec/prompt (remains constant)
    pub task_spec: String,
    /// Number of previous handoffs (for tracking)
    pub handoff_count: usize,
}

impl HandoffContext {
    /// Create a new handoff context
    pub fn new(summary: String, task_spec: String, handoff_count: usize) -> Self {
        Self {
            summary,
            task_spec,
            handoff_count,
        }
    }

    /// Build a fresh prompt with the handoff context
    ///
    /// Combines the original task spec with the progress summary
    /// to give the new agent necessary context while staying within limits.
    pub fn build_handoff_prompt(&self) -> String {
        format!(
            r#"# Context Handoff (Iteration {})

You are continuing work on a task after a context handoff. The previous agent made progress before reaching context limits.

{}

{}

Continue from where the previous agent left off. Review the progress summary above and proceed with the next logical step."#,
            self.handoff_count + 1,
            self.summary,
            self.task_spec
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("1234"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
        assert_eq!(estimate_tokens("123456789012345678901234567890"), 7);
    }

    #[test]
    fn test_context_monitor_new() {
        let monitor = ContextMonitor::new(100_000, 0.7);
        assert_eq!(monitor.context_window, 100_000);
        assert_eq!(monitor.threshold, 0.7);
        assert_eq!(monitor.tokens_used, 0);
        assert_eq!(monitor.handoffs, 0);
        assert!(monitor.enabled);
    }

    #[test]
    fn test_context_monitor_default() {
        let monitor = ContextMonitor::default();
        assert_eq!(monitor.context_window, 200_000);
        assert_eq!(monitor.threshold, 0.6);
        assert!(monitor.enabled);
    }

    #[test]
    fn test_context_monitor_disabled() {
        let monitor = ContextMonitor::disabled();
        assert!(!monitor.enabled);
        assert!(!monitor.should_handoff());
    }

    #[test]
    fn test_record_usage() {
        let mut monitor = ContextMonitor::new(1000, 0.5);
        assert_eq!(monitor.tokens_used, 0);

        monitor.record_usage("1234567890"); // 10 chars = 2 tokens
        assert_eq!(monitor.tokens_used, 2);

        monitor.record_usage("12345678901234567890"); // 20 chars = 5 tokens
        assert_eq!(monitor.tokens_used, 7);
    }

    #[test]
    fn test_usage_percentage() {
        let mut monitor = ContextMonitor::new(1000, 0.5);
        assert_eq!(monitor.usage_percentage(), 0.0);

        monitor.tokens_used = 500;
        assert_eq!(monitor.usage_percentage(), 0.5);

        monitor.tokens_used = 750;
        assert_eq!(monitor.usage_percentage(), 0.75);

        monitor.tokens_used = 1000;
        assert_eq!(monitor.usage_percentage(), 1.0);
    }

    #[test]
    fn test_should_handoff() {
        let mut monitor = ContextMonitor::new(1000, 0.6);
        assert!(!monitor.should_handoff());

        monitor.tokens_used = 599;
        assert!(!monitor.should_handoff());

        monitor.tokens_used = 600;
        assert!(monitor.should_handoff());

        monitor.tokens_used = 800;
        assert!(monitor.should_handoff());
    }

    #[test]
    fn test_should_handoff_disabled() {
        let mut monitor = ContextMonitor::disabled();
        monitor.tokens_used = 1_000_000; // Way over any limit
        assert!(!monitor.should_handoff());
    }

    #[test]
    fn test_tokens_until_handoff() {
        let mut monitor = ContextMonitor::new(1000, 0.6);
        assert_eq!(monitor.tokens_until_handoff(), 600);

        monitor.tokens_used = 300;
        assert_eq!(monitor.tokens_until_handoff(), 300);

        monitor.tokens_used = 600;
        assert_eq!(monitor.tokens_until_handoff(), 0);

        monitor.tokens_used = 800;
        assert_eq!(monitor.tokens_until_handoff(), 0); // Saturating sub
    }

    #[test]
    fn test_reset() {
        let mut monitor = ContextMonitor::new(1000, 0.6);
        monitor.tokens_used = 700;
        monitor.reset();

        assert_eq!(monitor.tokens_used, 0);
        assert_eq!(monitor.handoffs, 1);

        monitor.tokens_used = 650;
        monitor.reset();
        assert_eq!(monitor.handoffs, 2);
    }

    #[test]
    fn test_threshold_clamping() {
        let monitor = ContextMonitor::new(1000, 1.5);
        assert_eq!(monitor.threshold, 1.0);

        let monitor = ContextMonitor::new(1000, -0.5);
        assert_eq!(monitor.threshold, 0.0);

        let monitor = ContextMonitor::new(1000, 0.7);
        assert_eq!(monitor.threshold, 0.7);
    }

    #[test]
    fn test_summarize_agent_progress_completed() {
        let transcript = r#"I've implemented the feature successfully.

Modified: src/lib.rs (lines 10-20)
Modified: src/tests.rs (lines 5-15)

Running tests...
test result: ok. 12 passed; 0 failed

[DONE] Task completed"#;

        let summary = summarize_agent_progress(transcript);
        assert!(summary.contains("Status: Task completed"));
        assert!(summary.contains("Files modified"));
        assert!(summary.contains("Tests:"));
    }

    #[test]
    fn test_summarize_agent_progress_blocked() {
        let transcript = r#"I attempted to implement the feature.

However, I encountered an issue.

TASK_BLOCKED: Missing auth module dependency"#;

        let summary = summarize_agent_progress(transcript);
        assert!(summary.contains("Status: Blocked"));
        assert!(summary.contains("Missing auth module dependency"));
    }

    #[test]
    fn test_summarize_agent_progress_in_progress() {
        let transcript = r#"Working on the implementation.

Created: src/new_feature.rs
Modified: src/lib.rs

Still working on tests..."#;

        let summary = summarize_agent_progress(transcript);
        assert!(summary.contains("Status: In progress"));
        assert!(summary.contains("Files modified"));
    }

    #[test]
    fn test_summarize_agent_progress_empty() {
        let summary = summarize_agent_progress("");
        // Empty transcript should generate a fallback status message
        assert!(summary.contains("Previous agent progress") || summary.contains("Status:"));
    }

    #[test]
    fn test_handoff_context_build_prompt() {
        let context = HandoffContext::new(
            "Previous work summary".to_string(),
            "Task: Build feature X".to_string(),
            1,
        );

        let prompt = context.build_handoff_prompt();
        assert!(prompt.contains("Iteration 2")); // handoff_count + 1
        assert!(prompt.contains("Previous work summary"));
        assert!(prompt.contains("Task: Build feature X"));
        assert!(prompt.contains("context handoff"));
    }

    #[test]
    fn test_handoff_context_first_iteration() {
        let context =
            HandoffContext::new("Initial summary".to_string(), "Task spec".to_string(), 0);

        let prompt = context.build_handoff_prompt();
        assert!(prompt.contains("Iteration 1"));
    }
}
