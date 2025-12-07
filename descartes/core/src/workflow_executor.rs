//! Workflow Executor for Descartes
//!
//! Executes workflow steps using the appropriate agents and providers.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::agent_definitions::AgentDefinitionError;
use crate::thoughts::ThoughtsError;
use crate::workflow_commands::WorkflowError;
use crate::{
    get_tools, Message, MessageRole, ModelBackend, ModelRequest, WorkflowContext, WorkflowStep,
};

/// Result of executing a single workflow step
#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub step_name: String,
    pub success: bool,
    pub output: String,
    pub saved_to: Option<PathBuf>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Workflow executor configuration
#[derive(Debug, Clone)]
pub struct WorkflowExecutorConfig {
    pub provider: String,
    pub model: String,
    pub max_parallel: usize,
    pub save_outputs: bool,
}

impl Default for WorkflowExecutorConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            max_parallel: 3,
            save_outputs: true,
        }
    }
}

/// Errors specific to workflow execution
#[derive(Debug, thiserror::Error)]
pub enum WorkflowExecutionError {
    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Step failed: {0}")]
    StepFailed(String),

    #[error("Task join error: {0}")]
    JoinError(String),

    #[error("Workflow error: {0}")]
    WorkflowError(#[from] WorkflowError),
}

impl From<AgentDefinitionError> for WorkflowExecutionError {
    fn from(e: AgentDefinitionError) -> Self {
        WorkflowExecutionError::AgentError(e.to_string())
    }
}

impl From<ThoughtsError> for WorkflowExecutionError {
    fn from(e: ThoughtsError) -> Self {
        WorkflowExecutionError::StorageError(e.to_string())
    }
}

impl From<crate::errors::AgentError> for WorkflowExecutionError {
    fn from(e: crate::errors::AgentError) -> Self {
        WorkflowExecutionError::ProviderError(e.to_string())
    }
}

/// Execute a single workflow step
pub async fn execute_step(
    step: &WorkflowStep,
    task: &str,
    context: &WorkflowContext,
    backend: &dyn ModelBackend,
    config: &WorkflowExecutorConfig,
) -> Result<StepExecutionResult, WorkflowExecutionError> {
    let start = std::time::Instant::now();

    info!("Executing step: {} with agent: {}", step.name, step.agent);

    // Load agent definition for tool level and system prompt
    let agent_def = match context.agent_loader.load_agent(&step.agent) {
        Ok(def) => def,
        Err(e) => {
            warn!("Failed to load agent {}: {}", step.agent, e);
            // Use default readonly system prompt if agent not found
            return Ok(StepExecutionResult {
                step_name: step.name.clone(),
                success: false,
                output: String::new(),
                saved_to: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Agent '{}' not found: {}", step.agent, e)),
            });
        }
    };

    // Get tools for this agent's level
    let tools = get_tools(agent_def.tool_level);
    debug!(
        "Using {} tools for agent {}",
        tools.len(),
        agent_def.name
    );

    // Build the full task with context
    let full_task = format!(
        "Topic: {}\n\nTask: {}\n\nContext: {}",
        context.topic,
        task,
        context.context.as_deref().unwrap_or("None")
    );

    // Create model request
    let messages = vec![Message {
        role: MessageRole::User,
        content: full_task,
    }];

    let request = ModelRequest {
        messages,
        model: config.model.clone(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        system_prompt: Some(agent_def.system_prompt.clone()),
        tools: Some(tools),
    };

    // Execute
    let response = match backend.complete(request).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(StepExecutionResult {
                step_name: step.name.clone(),
                success: false,
                output: String::new(),
                saved_to: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Provider error: {}", e)),
            });
        }
    };

    // Save output if configured
    let saved_to = if config.save_outputs {
        if let Some(output_path) = &step.output {
            let content = format!("# {}\n\n{}", step.name, response.content);
            match context.thoughts.save_research(output_path, &content) {
                Ok(saved) => {
                    info!("Saved output to: {:?}", saved);
                    Some(saved)
                }
                Err(e) => {
                    warn!("Failed to save output: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(StepExecutionResult {
        step_name: step.name.clone(),
        success: true,
        output: response.content,
        saved_to,
        duration_ms: start.elapsed().as_millis() as u64,
        error: None,
    })
}

/// Execute multiple steps, respecting parallel flags
pub async fn execute_workflow(
    steps: Vec<(WorkflowStep, String)>,
    context: &WorkflowContext,
    backend: Arc<dyn ModelBackend + Send + Sync>,
    config: &WorkflowExecutorConfig,
) -> Result<Vec<StepExecutionResult>, WorkflowExecutionError> {
    let mut results = Vec::new();
    let semaphore = Arc::new(Semaphore::new(config.max_parallel));

    info!(
        "Executing workflow with {} steps, max_parallel={}",
        steps.len(),
        config.max_parallel
    );

    let mut i = 0;
    while i < steps.len() {
        // Find consecutive parallel steps
        let mut parallel_batch = vec![i];
        let mut j = i + 1;
        while j < steps.len() && steps[j].0.parallel {
            parallel_batch.push(j);
            j += 1;
        }

        if parallel_batch.len() == 1 {
            // Execute sequentially
            let (step, task) = &steps[i];
            info!("Executing step {} sequentially", step.name);
            let result = execute_step(step, task, context, backend.as_ref(), config).await?;
            results.push(result);
            i += 1;
        } else {
            // Execute in parallel
            info!(
                "Executing {} steps in parallel",
                parallel_batch.len()
            );

            let mut handles = Vec::new();

            for &idx in &parallel_batch {
                let (step, task) = steps[idx].clone();
                let backend = backend.clone();
                let config = config.clone();
                let sem = semaphore.clone();

                // Clone context fields for the async block
                let topic = context.topic.clone();
                let ctx_context = context.context.clone();
                let working_dir = context.working_dir.clone();

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");

                    // Recreate context in spawn
                    let wf_context = match WorkflowContext::new(working_dir, &topic) {
                        Ok(mut c) => {
                            if let Some(ctx) = ctx_context {
                                c = c.with_context(ctx);
                            }
                            c
                        }
                        Err(e) => {
                            return StepExecutionResult {
                                step_name: step.name.clone(),
                                success: false,
                                output: String::new(),
                                saved_to: None,
                                duration_ms: 0,
                                error: Some(format!("Context error: {}", e)),
                            };
                        }
                    };

                    match execute_step(&step, &task, &wf_context, backend.as_ref(), &config).await {
                        Ok(result) => result,
                        Err(e) => StepExecutionResult {
                            step_name: step.name.clone(),
                            success: false,
                            output: String::new(),
                            saved_to: None,
                            duration_ms: 0,
                            error: Some(format!("Execution error: {}", e)),
                        },
                    }
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        return Err(WorkflowExecutionError::JoinError(e.to_string()));
                    }
                }
            }

            i = j;
        }
    }

    info!("Workflow execution complete: {} steps", results.len());
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_default() {
        let config = WorkflowExecutorConfig::default();
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.max_parallel, 3);
        assert!(config.save_outputs);
    }

    #[test]
    fn test_step_execution_result() {
        let result = StepExecutionResult {
            step_name: "test-step".to_string(),
            success: true,
            output: "test output".to_string(),
            saved_to: Some(PathBuf::from("/tmp/test.md")),
            duration_ms: 100,
            error: None,
        };

        assert!(result.success);
        assert_eq!(result.step_name, "test-step");
        assert!(result.saved_to.is_some());
    }
}
