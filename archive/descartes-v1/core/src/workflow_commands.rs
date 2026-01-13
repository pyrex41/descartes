//! Workflow Commands for Descartes
//!
//! Provides high-level workflow commands that orchestrate multiple agents
//! to perform common development tasks like research, planning, and implementation.
//!
//! These commands follow the `/cl:*` pattern from Claude Code.

use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info};

use crate::agent_definitions::AgentDefinitionLoader;
use crate::thoughts::ThoughtsStorage;

/// Errors that can occur during workflow execution
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Workflow step failed: {0}")]
    StepFailed(String),
}

/// Result type for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// A single step in a workflow
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    /// Name of the step
    pub name: String,
    /// Agent to use for this step (from ~/.descartes/agents/)
    pub agent: String,
    /// Task/prompt for this step
    pub task: String,
    /// Whether this step can run in parallel with previous steps
    pub parallel: bool,
    /// Output file path (relative to thoughts directory)
    pub output: Option<String>,
}

/// A workflow command definition
#[derive(Debug, Clone)]
pub struct WorkflowCommand {
    /// Name of the workflow
    pub name: String,
    /// Description of what this workflow does
    pub description: String,
    /// Ordered list of steps to execute
    pub steps: Vec<WorkflowStep>,
}

impl WorkflowCommand {
    /// Create a new workflow command
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            steps: Vec::new(),
        }
    }

    /// Add a step to the workflow
    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add a sequential step (waits for previous steps)
    pub fn then(mut self, agent: impl Into<String>, task: impl Into<String>) -> Self {
        let step_num = self.steps.len() + 1;
        self.steps.push(WorkflowStep {
            name: format!("Step {}", step_num),
            agent: agent.into(),
            task: task.into(),
            parallel: false,
            output: None,
        });
        self
    }

    /// Add a parallel step (runs concurrently with previous step)
    pub fn parallel(mut self, agent: impl Into<String>, task: impl Into<String>) -> Self {
        let step_num = self.steps.len() + 1;
        self.steps.push(WorkflowStep {
            name: format!("Step {}", step_num),
            agent: agent.into(),
            task: task.into(),
            parallel: true,
            output: None,
        });
        self
    }
}

/// Registry of built-in workflow commands
pub struct WorkflowRegistry {
    commands: Vec<WorkflowCommand>,
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowRegistry {
    /// Create a new registry with built-in commands
    pub fn new() -> Self {
        let mut registry = Self {
            commands: Vec::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Register the built-in workflow commands
    fn register_builtins(&mut self) {
        // research_codebase: Find and analyze code
        self.commands.push(
            WorkflowCommand::new(
                "research_codebase",
                "Research the codebase to understand file locations and implementations",
            )
            .add_step(WorkflowStep {
                name: "Locate Files".to_string(),
                agent: "codebase-locator".to_string(),
                task: "Find all files related to the topic. Report file paths organized by purpose."
                    .to_string(),
                parallel: false,
                output: Some("research/locations.md".to_string()),
            })
            .add_step(WorkflowStep {
                name: "Analyze Implementation".to_string(),
                agent: "codebase-analyzer".to_string(),
                task:
                    "Analyze how the code works. Trace data flow and explain key implementation details."
                        .to_string(),
                parallel: true, // Can run in parallel with locator
                output: Some("research/analysis.md".to_string()),
            })
            .add_step(WorkflowStep {
                name: "Find Patterns".to_string(),
                agent: "codebase-pattern-finder".to_string(),
                task: "Find existing patterns and examples that can be used as templates."
                    .to_string(),
                parallel: true,
                output: Some("research/patterns.md".to_string()),
            }),
        );

        // create_plan: Create an implementation plan
        self.commands.push(
            WorkflowCommand::new(
                "create_plan",
                "Create an implementation plan with phases and specific steps",
            )
            .add_step(WorkflowStep {
                name: "Research".to_string(),
                agent: "researcher".to_string(),
                task: "Research the codebase to understand the current state and constraints."
                    .to_string(),
                parallel: false,
                output: Some("research/context.md".to_string()),
            })
            .add_step(WorkflowStep {
                name: "Plan".to_string(),
                agent: "planner".to_string(),
                task: "Create a detailed implementation plan with phases, steps, and verification criteria."
                    .to_string(),
                parallel: false,
                output: Some("plans/implementation.md".to_string()),
            }),
        );

        // implement_plan: Execute an implementation plan
        self.commands.push(
            WorkflowCommand::new(
                "implement_plan",
                "Implement a plan from the thoughts/plans directory",
            )
            .add_step(WorkflowStep {
                name: "Read Plan".to_string(),
                agent: "researcher".to_string(),
                task: "Read and summarize the implementation plan. Identify the next incomplete phase."
                    .to_string(),
                parallel: false,
                output: None,
            }),
            // Note: Actual implementation requires a more capable agent
            // This is a starting point for the workflow structure
        );
    }

    /// Get a workflow command by name
    pub fn get(&self, name: &str) -> Option<&WorkflowCommand> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// List all available workflow commands
    pub fn list(&self) -> Vec<&str> {
        self.commands.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get all workflow commands
    pub fn all(&self) -> &[WorkflowCommand] {
        &self.commands
    }
}

/// Context for executing a workflow
pub struct WorkflowContext {
    /// Working directory for the workflow
    pub working_dir: PathBuf,
    /// The topic or subject of the workflow
    pub topic: String,
    /// Additional context to pass to agents
    pub context: Option<String>,
    /// Thoughts storage for saving outputs
    pub thoughts: ThoughtsStorage,
    /// Agent loader for loading agent definitions
    pub agent_loader: AgentDefinitionLoader,
}

impl WorkflowContext {
    /// Create a new workflow context
    pub fn new(
        working_dir: PathBuf,
        topic: impl Into<String>,
    ) -> Result<Self, WorkflowError> {
        let thoughts = ThoughtsStorage::new()
            .map_err(|e| WorkflowError::StorageError(e.to_string()))?;
        let agent_loader = AgentDefinitionLoader::new()
            .map_err(|e| WorkflowError::AgentError(e.to_string()))?;

        Ok(Self {
            working_dir,
            topic: topic.into(),
            context: None,
            thoughts,
            agent_loader,
        })
    }

    /// Add additional context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Result of executing a workflow step
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Name of the step
    pub step_name: String,
    /// Whether the step succeeded
    pub success: bool,
    /// Output from the step
    pub output: String,
    /// Path where output was saved (if any)
    pub saved_to: Option<PathBuf>,
}

/// Result of executing a complete workflow
#[derive(Debug)]
pub struct WorkflowExecutionResult {
    /// Name of the workflow
    pub workflow_name: String,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Overall success
    pub success: bool,
    /// Combined output or summary
    pub summary: String,
}

/// Execute a workflow command
///
/// This is a synchronous preparation function that returns the steps to execute.
/// Actual execution happens through the CLI or other runners.
pub fn prepare_workflow(
    command: &WorkflowCommand,
    context: &WorkflowContext,
) -> WorkflowResult<Vec<(WorkflowStep, String)>> {
    info!("Preparing workflow: {}", command.name);

    let mut prepared_steps = Vec::new();

    for step in &command.steps {
        // Verify the agent exists
        if !context.agent_loader.agent_exists(&step.agent) {
            return Err(WorkflowError::AgentError(format!(
                "Agent '{}' not found for step '{}'",
                step.agent, step.name
            )));
        }

        // Prepare the task with topic substitution
        let task = step
            .task
            .replace("{topic}", &context.topic)
            .replace("{context}", context.context.as_deref().unwrap_or(""));

        debug!("Prepared step '{}' with agent '{}'", step.name, step.agent);
        prepared_steps.push((step.clone(), task));
    }

    Ok(prepared_steps)
}

/// Get information about available workflows for display
pub fn list_workflows() -> Vec<(String, String)> {
    let registry = WorkflowRegistry::new();
    registry
        .all()
        .iter()
        .map(|cmd| (cmd.name.clone(), cmd.description.clone()))
        .collect()
}

/// Get workflow by name
pub fn get_workflow(name: &str) -> Option<WorkflowCommand> {
    let registry = WorkflowRegistry::new();
    registry.get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_command_builder() {
        let workflow = WorkflowCommand::new("test", "Test workflow")
            .then("agent1", "Task 1")
            .parallel("agent2", "Task 2")
            .then("agent3", "Task 3");

        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.steps.len(), 3);
        assert!(!workflow.steps[0].parallel);
        assert!(workflow.steps[1].parallel);
        assert!(!workflow.steps[2].parallel);
    }

    #[test]
    fn test_workflow_registry() {
        let registry = WorkflowRegistry::new();
        let names = registry.list();

        assert!(names.contains(&"research_codebase"));
        assert!(names.contains(&"create_plan"));
        assert!(names.contains(&"implement_plan"));
    }

    #[test]
    fn test_get_workflow() {
        let workflow = get_workflow("research_codebase");
        assert!(workflow.is_some());

        let workflow = workflow.unwrap();
        assert_eq!(workflow.name, "research_codebase");
        assert!(!workflow.steps.is_empty());
    }

    #[test]
    fn test_list_workflows() {
        let workflows = list_workflows();
        assert!(!workflows.is_empty());

        let names: Vec<&str> = workflows.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"research_codebase"));
    }

    #[test]
    fn test_workflow_step_structure() {
        let registry = WorkflowRegistry::new();
        let research = registry.get("research_codebase").unwrap();

        // First step should be sequential
        assert!(!research.steps[0].parallel);
        assert_eq!(research.steps[0].agent, "codebase-locator");

        // Second step can be parallel
        assert!(research.steps[1].parallel);
        assert_eq!(research.steps[1].agent, "codebase-analyzer");
    }
}
