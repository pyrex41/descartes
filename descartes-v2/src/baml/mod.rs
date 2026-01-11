//! BAML client for calling the baml-cli serve REST API
//!
//! This module provides typed Rust functions that call the BAML HTTP server.
//! Start the server with: `npx @boundaryml/baml dev --preview`
//! Server runs at: http://localhost:2024
//! API docs at: http://localhost:2024/docs

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default BAML server URL
pub const DEFAULT_BAML_URL: &str = "http://localhost:2024";

#[derive(Error, Debug)]
pub enum BamlError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("BAML server not running at {0}. Start with: npx @boundaryml/baml dev --preview")]
    ServerNotRunning(String),
    #[error("BAML function error: {0}")]
    FunctionError(String),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

/// BAML client for making typed function calls
#[derive(Clone)]
pub struct BamlClient {
    client: Client,
    base_url: String,
}

impl BamlClient {
    pub fn new() -> Self {
        Self::with_url(DEFAULT_BAML_URL)
    }

    pub fn with_url(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check if the BAML server is running
    pub async fn health_check(&self) -> Result<String, BamlError> {
        let url = format!("{}/_debug/ping", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                BamlError::ServerNotRunning(self.base_url.clone())
            } else {
                BamlError::Http(e)
            }
        })?;
        Ok(resp.text().await?)
    }

    /// Generic function call to BAML endpoint
    async fn call_function<Req: Serialize, Resp: for<'de> Deserialize<'de>>(
        &self,
        function_name: &str,
        request: &Req,
    ) -> Result<Resp, BamlError> {
        let url = format!("{}/call/{}", self.base_url, function_name);
        let resp = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    BamlError::ServerNotRunning(self.base_url.clone())
                } else {
                    BamlError::Http(e)
                }
            })?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(BamlError::FunctionError(error_text));
        }

        resp.json().await.map_err(|e| BamlError::ParseError(e.to_string()))
    }
}

// ============================================================================
// Orchestrator types (from orchestrator.baml)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum NextAction {
    Continue,
    Replan,
    Validate,
    Complete,
    AskHuman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopDecision {
    pub action: NextAction,
    pub reasoning: String,
    pub next_task_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecideNextActionRequest {
    pub completed_tasks: Vec<String>,
    pub current_task: Option<String>,
    pub remaining_tasks: Vec<String>,
    pub blockers: Vec<String>,
    pub recent_output: String,
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubagentCategory {
    Searcher,
    Analyzer,
    Builder,
    Validator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentSelection {
    pub category: SubagentCategory,
    pub prompt: String,
    pub timeout_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectSubagentRequest {
    pub task_title: String,
    pub task_description: String,
    pub available_context: String,
    pub additional_context: Option<String>,
}

// ============================================================================
// Planning types (from planning.baml)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub complexity: TaskComplexity,
    pub depends_on: Vec<String>,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    pub goal: String,
    pub approach: String,
    pub tasks: Vec<Task>,
    pub risks: Vec<String>,
    pub out_of_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanRequest {
    pub objective: String,
    pub research_context: String,
    pub constraints: Option<String>,
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBreakdown {
    pub subtasks: Vec<Task>,
    pub estimated_minutes: i32,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownTaskRequest {
    pub task: Task,
    pub context: String,
    pub additional_context: Option<String>,
}

// ============================================================================
// Handoff types (from handoff.baml)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageHandoff {
    pub from_stage: String,
    pub to_stage: String,
    pub summary: String,
    pub key_artifacts: Vec<String>,
    pub context_for_next: String,
    pub open_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateHandoffRequest {
    pub from_stage: String,
    pub to_stage: String,
    pub work_done: String,
    pub outputs: String,
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitType {
    Feat,
    Fix,
    Docs,
    Refactor,
    Test,
    Chore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMessage {
    #[serde(rename = "type")]
    pub commit_type: CommitType,
    pub scope: Option<String>,
    pub subject: String,
    pub body: Option<String>,
    pub breaking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateCommitMessageRequest {
    pub diff: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRDescription {
    pub title: String,
    pub summary: String,
    pub changes: Vec<String>,
    pub testing: String,
    pub checklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePRDescriptionRequest {
    pub commits: String,
    pub diff_summary: String,
    pub related_issues: Option<String>,
    pub additional_context: Option<String>,
}

// ============================================================================
// Client implementation for each BAML function
// ============================================================================

impl BamlClient {
    // Orchestrator functions

    pub async fn decide_next_action(
        &self,
        req: DecideNextActionRequest,
    ) -> Result<LoopDecision, BamlError> {
        self.call_function("DecideNextAction", &req).await
    }

    pub async fn select_subagent(
        &self,
        req: SelectSubagentRequest,
    ) -> Result<SubagentSelection, BamlError> {
        self.call_function("SelectSubagent", &req).await
    }

    // Planning functions

    pub async fn create_plan(&self, req: CreatePlanRequest) -> Result<ImplementationPlan, BamlError> {
        self.call_function("CreatePlan", &req).await
    }

    pub async fn breakdown_task(&self, req: BreakdownTaskRequest) -> Result<TaskBreakdown, BamlError> {
        self.call_function("BreakdownTask", &req).await
    }

    // Handoff functions

    pub async fn generate_handoff(
        &self,
        req: GenerateHandoffRequest,
    ) -> Result<StageHandoff, BamlError> {
        self.call_function("GenerateHandoff", &req).await
    }

    pub async fn generate_commit_message(
        &self,
        req: GenerateCommitMessageRequest,
    ) -> Result<CommitMessage, BamlError> {
        self.call_function("GenerateCommitMessage", &req).await
    }

    pub async fn generate_pr_description(
        &self,
        req: GeneratePRDescriptionRequest,
    ) -> Result<PRDescription, BamlError> {
        self.call_function("GeneratePRDescription", &req).await
    }
}

impl Default for BamlClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_when_server_not_running() {
        let client = BamlClient::with_url("http://localhost:19999");
        let result = client.health_check().await;
        assert!(matches!(result, Err(BamlError::ServerNotRunning(_))));
    }
}
