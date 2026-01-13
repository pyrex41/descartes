//! Handoff generation and management
//!
//! Handoffs are the structured context passed between workflow stages.
//! They capture what happened, what was decided, and what comes next.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::workflow::config::{AutoContext, TransitionConfig};
use crate::{Error, Result};

/// A handoff between workflow stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    /// Source stage
    pub from_stage: String,
    /// Target stage
    pub to_stage: String,
    /// Summary of what was accomplished
    pub summary: String,
    /// Key findings or decisions
    pub findings: Vec<String>,
    /// Recommendations for next stage
    pub recommendations: Vec<String>,
    /// Artifacts produced (file paths, etc.)
    pub artifacts: Vec<Artifact>,
    /// Command to run in next stage
    pub command: Option<String>,
    /// Additional context sections
    pub context: HashMap<String, String>,
    /// Extra user-provided context
    pub extra: Option<String>,
}

/// An artifact produced during a stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact type
    pub kind: ArtifactKind,
    /// Path or identifier
    pub path: String,
    /// Description
    pub description: Option<String>,
}

/// Types of artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    File,
    Directory,
    Url,
    Plan,
    ScudTask,
    Transcript,
}

impl Handoff {
    /// Create a new handoff
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from_stage: from.to_string(),
            to_stage: to.to_string(),
            summary: String::new(),
            findings: Vec::new(),
            recommendations: Vec::new(),
            artifacts: Vec::new(),
            command: None,
            context: HashMap::new(),
            extra: None,
        }
    }

    /// Set the summary
    pub fn with_summary(mut self, summary: &str) -> Self {
        self.summary = summary.to_string();
        self
    }

    /// Add a finding
    pub fn with_finding(mut self, finding: &str) -> Self {
        self.findings.push(finding.to_string());
        self
    }

    /// Add findings
    pub fn with_findings(mut self, findings: &[String]) -> Self {
        self.findings.extend(findings.iter().cloned());
        self
    }

    /// Add a recommendation
    pub fn with_recommendation(mut self, rec: &str) -> Self {
        self.recommendations.push(rec.to_string());
        self
    }

    /// Add an artifact
    pub fn with_artifact(mut self, kind: ArtifactKind, path: &str, desc: Option<&str>) -> Self {
        self.artifacts.push(Artifact {
            kind,
            path: path.to_string(),
            description: desc.map(|s| s.to_string()),
        });
        self
    }

    /// Set the command
    pub fn with_command(mut self, command: &str) -> Self {
        self.command = Some(command.to_string());
        self
    }

    /// Add context section
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }

    /// Set extra user context
    pub fn with_extra(mut self, extra: &str) -> Self {
        self.extra = Some(extra.to_string());
        self
    }

    /// Render the handoff using a template
    pub fn render(&self, template: &str) -> String {
        let mut output = template.to_string();

        // Replace template variables
        output = output.replace("{{summary}}", &self.summary);
        output = output.replace(
            "{{findings}}",
            &self
                .findings
                .iter()
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        output = output.replace(
            "{{recommendations}}",
            &self
                .recommendations
                .iter()
                .map(|r| format!("- {}", r))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        output = output.replace(
            "{{artifacts}}",
            &self
                .artifacts
                .iter()
                .map(|a| {
                    if let Some(desc) = &a.description {
                        format!("- {} ({})", a.path, desc)
                    } else {
                        format!("- {}", a.path)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );

        // Replace context variables
        for (key, value) in &self.context {
            output = output.replace(&format!("{{{{{}}}}}", key), value);
        }

        // Add extra context if present
        if let Some(extra) = &self.extra {
            output.push_str(&format!("\n\n## Additional Context\n{}", extra));
        }

        // Clean up any remaining template variables
        let re = regex::Regex::new(r"\{\{[^}]+\}\}").unwrap();
        output = re.replace_all(&output, "").to_string();

        output
    }

    /// Render to default format
    pub fn render_default(&self) -> String {
        let mut sections = Vec::new();

        sections.push(format!(
            "# Handoff: {} → {}\n",
            self.from_stage, self.to_stage
        ));

        if !self.summary.is_empty() {
            sections.push(format!("## Summary\n{}\n", self.summary));
        }

        if !self.findings.is_empty() {
            sections.push(format!(
                "## Key Findings\n{}\n",
                self.findings
                    .iter()
                    .map(|f| format!("- {}", f))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if !self.recommendations.is_empty() {
            sections.push(format!(
                "## Recommendations\n{}\n",
                self.recommendations
                    .iter()
                    .map(|r| format!("- {}", r))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if !self.artifacts.is_empty() {
            sections.push(format!(
                "## Artifacts\n{}\n",
                self.artifacts
                    .iter()
                    .map(|a| {
                        if let Some(desc) = &a.description {
                            format!("- `{}` - {}", a.path, desc)
                        } else {
                            format!("- `{}`", a.path)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        for (key, value) in &self.context {
            sections.push(format!("## {}\n{}\n", key, value));
        }

        if let Some(extra) = &self.extra {
            sections.push(format!("## Additional Context\n{}\n", extra));
        }

        if let Some(cmd) = &self.command {
            sections.push(format!("---\n**Next Command:** `{}`\n", cmd));
        }

        sections.join("\n")
    }

    /// Create prompt for the next agent
    pub fn to_prompt(&self) -> String {
        let mut prompt = self.render_default();

        if let Some(cmd) = &self.command {
            prompt.push_str(&format!("\n\nPlease execute: {}\n", cmd));
        }

        prompt
    }
}

/// Builder for creating handoffs with auto-context
pub struct HandoffBuilder {
    handoff: Handoff,
    transition_config: Option<TransitionConfig>,
}

impl HandoffBuilder {
    /// Create a new handoff builder
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            handoff: Handoff::new(from, to),
            transition_config: None,
        }
    }

    /// Set transition configuration
    pub fn with_transition_config(mut self, config: TransitionConfig) -> Self {
        if let Some(cmd) = &config.command {
            self.handoff.command = Some(cmd.clone());
        }
        self.transition_config = Some(config);
        self
    }

    /// Set summary
    pub fn summary(mut self, summary: &str) -> Self {
        self.handoff.summary = summary.to_string();
        self
    }

    /// Add findings
    pub fn findings(mut self, findings: &[String]) -> Self {
        self.handoff.findings = findings.to_vec();
        self
    }

    /// Add recommendations
    pub fn recommendations(mut self, recs: &[String]) -> Self {
        self.handoff.recommendations = recs.to_vec();
        self
    }

    /// Add extra context
    pub fn extra(mut self, extra: &str) -> Self {
        self.handoff.extra = Some(extra.to_string());
        self
    }

    /// Populate auto-context
    pub async fn populate_auto_context(mut self) -> Result<Self> {
        if let Some(config) = &self.transition_config {
            for ctx in &config.auto_context {
                match ctx {
                    AutoContext::ScudTasks => {
                        if let Ok(tasks) = self.get_scud_tasks().await {
                            self.handoff.context.insert("scud_tasks".to_string(), tasks);
                        }
                    }
                    AutoContext::ScudWaves => {
                        if let Ok(waves) = self.get_scud_waves().await {
                            self.handoff.context.insert("scud_waves".to_string(), waves);
                        }
                    }
                    AutoContext::ScudDeps => {
                        if let Ok(deps) = self.get_scud_deps().await {
                            self.handoff.context.insert("scud_deps".to_string(), deps);
                        }
                    }
                    AutoContext::GitDiff => {
                        if let Ok(diff) = self.get_git_diff().await {
                            self.handoff.context.insert("git_diff".to_string(), diff);
                        }
                    }
                    AutoContext::GitStatus => {
                        if let Ok(status) = self.get_git_status().await {
                            self.handoff.context.insert("git_status".to_string(), status);
                        }
                    }
                    AutoContext::TranscriptSummary => {
                        // TODO: Implement transcript summarization
                        debug!("Transcript summary not yet implemented");
                    }
                    AutoContext::Custom(cmd) => {
                        if let Ok(output) = self.run_custom_context(cmd).await {
                            self.handoff.context.insert(cmd.clone(), output);
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    /// Build the handoff
    pub fn build(self) -> Handoff {
        self.handoff
    }

    /// Render using transition template
    pub fn render(self) -> String {
        if let Some(config) = &self.transition_config {
            if let Some(template) = &config.handoff_template {
                return self.handoff.render(template);
            }
        }
        self.handoff.render_default()
    }

    // Helper methods for auto-context

    async fn get_scud_tasks(&self) -> Result<String> {
        let output = tokio::process::Command::new("scud")
            .args(["list", "--format", "markdown"])
            .output()
            .await
            .map_err(|e| Error::Command(format!("scud list failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_scud_waves(&self) -> Result<String> {
        let output = tokio::process::Command::new("scud")
            .args(["waves", "--format", "markdown"])
            .output()
            .await
            .map_err(|e| Error::Command(format!("scud waves failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_scud_deps(&self) -> Result<String> {
        let output = tokio::process::Command::new("scud")
            .args(["deps", "--format", "markdown"])
            .output()
            .await
            .map_err(|e| Error::Command(format!("scud deps failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_git_diff(&self) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["diff", "--stat"])
            .output()
            .await
            .map_err(|e| Error::Command(format!("git diff failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn get_git_status(&self) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["status", "--short"])
            .output()
            .await
            .map_err(|e| Error::Command(format!("git status failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn run_custom_context(&self, cmd: &str) -> Result<String> {
        let output = tokio::process::Command::new("sh")
            .args(["-c", cmd])
            .output()
            .await
            .map_err(|e| Error::Command(format!("custom context command failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_render_default() {
        let handoff = Handoff::new("research", "plan")
            .with_summary("Analyzed the auth module")
            .with_finding("Uses JWT tokens")
            .with_finding("No refresh token support")
            .with_recommendation("Add refresh token flow")
            .with_command("/create_plan");

        let rendered = handoff.render_default();

        assert!(rendered.contains("research → plan"));
        assert!(rendered.contains("Analyzed the auth module"));
        assert!(rendered.contains("JWT tokens"));
        assert!(rendered.contains("refresh token"));
        assert!(rendered.contains("/create_plan"));
    }

    #[test]
    fn test_handoff_render_template() {
        let handoff = Handoff::new("plan", "implement")
            .with_summary("Created implementation plan")
            .with_context("scud_waves", "Wave 1: task-1, task-2");

        let template = "## Plan\n{{summary}}\n\n## Waves\n{{scud_waves}}";
        let rendered = handoff.render(template);

        assert!(rendered.contains("Created implementation plan"));
        assert!(rendered.contains("Wave 1: task-1, task-2"));
    }

    #[test]
    fn test_handoff_with_extra() {
        let handoff = Handoff::new("a", "b")
            .with_summary("Test")
            .with_extra("Focus on the auth module first");

        let rendered = handoff.render_default();
        assert!(rendered.contains("Focus on the auth module first"));
    }
}
