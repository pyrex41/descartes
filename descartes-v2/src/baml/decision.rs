//! Decision Engine using BAML
//!
//! Implements the Ralph Wiggum loop decision logic using BAML's
//! structured output parsing for reliable, type-safe decisions.

use std::time::Instant;

use serde_json::json;
use tracing::{debug, info, warn};

use crate::{Error, Result};

use super::runtime::{BamlRuntime, PromptBuilder};
use super::types::*;

/// Context for making a loop decision
#[derive(Debug, Clone)]
pub struct DecisionContext {
    /// Current task graph status
    pub task_status: BamlTaskGraphStatus,
    /// Output from the last agent run
    pub recent_output: String,
    /// Validation result if available
    pub validation: Option<BamlValidationResult>,
    /// Git status if available
    pub git_status: Option<BamlGitStatus>,
    /// Current iteration number
    pub iteration: i32,
    /// Session start time
    pub session_start: Instant,
}

impl DecisionContext {
    /// Create a new decision context
    pub fn new(task_status: BamlTaskGraphStatus, recent_output: String) -> Self {
        Self {
            task_status,
            recent_output,
            validation: None,
            git_status: None,
            iteration: 1,
            session_start: Instant::now(),
        }
    }

    /// Set the validation result
    pub fn with_validation(mut self, validation: BamlValidationResult) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Set the git status
    pub fn with_git_status(mut self, status: BamlGitStatus) -> Self {
        self.git_status = Some(status);
        self
    }

    /// Set the iteration number
    pub fn with_iteration(mut self, iteration: i32) -> Self {
        self.iteration = iteration;
        self
    }

    /// Convert to BAML context format
    pub fn to_baml_context(&self) -> BamlAgentContext {
        BamlAgentContext {
            task_status: self.task_status.clone(),
            recent_output: self.recent_output.clone(),
            validation: self.validation.clone(),
            git_status: self.git_status.clone(),
            iteration: self.iteration,
            elapsed_minutes: self.session_start.elapsed().as_secs() as i32 / 60,
        }
    }
}

/// A decision from the loop
#[derive(Debug, Clone)]
pub struct Decision {
    /// The decision type
    pub decision: LoopDecision,
    /// Confidence score (if available)
    pub confidence: Option<f32>,
    /// Raw response for debugging
    pub raw_response: Option<String>,
    /// Time taken to decide
    pub decision_time_ms: u64,
}

impl Decision {
    /// Create from a loop decision
    pub fn from_decision(decision: LoopDecision, decision_time_ms: u64) -> Self {
        Self {
            decision,
            confidence: None,
            raw_response: None,
            decision_time_ms,
        }
    }

    /// Get the action name
    pub fn action(&self) -> &str {
        self.decision.action_name()
    }

    /// Check if terminal
    pub fn is_terminal(&self) -> bool {
        self.decision.is_terminal()
    }

    /// Check if needs human
    pub fn needs_human(&self) -> bool {
        self.decision.needs_human()
    }

    /// Get summary for logging
    pub fn summary(&self) -> String {
        match &self.decision {
            LoopDecision::Continue(c) => {
                format!(
                    "Continue: {} (task: {:?})",
                    c.approach,
                    c.next_task_id
                )
            }
            LoopDecision::Replan(r) => {
                format!("Replan: {}", r.reason)
            }
            LoopDecision::Complete(c) => {
                format!("Complete: {} ({} artifacts)", c.summary, c.artifacts.len())
            }
            LoopDecision::Human(h) => {
                format!("Human needed: {}", h.question)
            }
            LoopDecision::Spawn(s) => {
                format!(
                    "Spawn {:?} for task {}: {}",
                    s.category,
                    s.task_id,
                    truncate(&s.prompt, 50)
                )
            }
            LoopDecision::Validate(v) => {
                format!("Validate: {} (continue_on_failure: {})", v.scope, v.continue_on_failure)
            }
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Decision engine using BAML
pub struct DecisionEngine {
    runtime: BamlRuntime,
}

impl DecisionEngine {
    /// Create a new decision engine
    pub fn new(runtime: BamlRuntime) -> Self {
        Self { runtime }
    }

    /// Create with default runtime from environment
    pub fn from_env() -> Result<Self> {
        let mut runtime = BamlRuntime::from_env();
        runtime.init()?;
        Ok(Self { runtime })
    }

    /// Make a decision based on the current context
    ///
    /// This is the core function that decides what the Ralph Wiggum loop
    /// should do next. It uses BAML's structured output to ensure reliable
    /// parsing of the LLM response.
    pub async fn decide(&self, context: &DecisionContext) -> Result<Decision> {
        let start = Instant::now();

        // Build the prompt
        let baml_context = context.to_baml_context();
        let prompt = self.build_decision_prompt(&baml_context);

        debug!("Decision prompt:\n{}", prompt);

        // For now, we'll use a simple heuristic-based decision
        // In production, this would call the BAML runtime
        let decision = self.make_heuristic_decision(&baml_context)?;

        let decision_time = start.elapsed().as_millis() as u64;
        info!(
            "Decision made in {}ms: {}",
            decision_time,
            decision.action_name()
        );

        Ok(Decision::from_decision(decision, decision_time))
    }

    /// Build the decision prompt
    fn build_decision_prompt(&self, context: &BamlAgentContext) -> String {
        let task_status = format!(
            "Total: {}, Completed: {}, In Progress: {}, Blocked: {}, Pending: {}",
            context.task_status.total_tasks,
            context.task_status.completed,
            context.task_status.in_progress,
            context.task_status.blocked,
            context.task_status.pending,
        );

        let next_task = context
            .task_status
            .next_ready
            .as_ref()
            .map(|t| format!("{}: {} ({})", t.id, t.title, format!("{:?}", t.complexity)))
            .unwrap_or_else(|| "None".to_string());

        let blockers = if context.task_status.blockers.is_empty() {
            "None".to_string()
        } else {
            context.task_status.blockers.join("\n- ")
        };

        let validation = context
            .validation
            .as_ref()
            .map(|v| {
                format!(
                    "Passed: {}, Failures: {}",
                    v.passed,
                    v.failures.len()
                )
            })
            .unwrap_or_else(|| "Not run".to_string());

        PromptBuilder::new()
            .section("Task Graph Status", &task_status)
            .section("Next Ready Task", &next_task)
            .section("Blockers", &blockers)
            .section("Recent Output", &context.recent_output)
            .section("Validation", &validation)
            .section(
                "Session Info",
                &format!(
                    "Iteration: {}, Elapsed: {} minutes",
                    context.iteration, context.elapsed_minutes
                ),
            )
            .output_format(&self.get_output_format())
            .build()
    }

    /// Get the output format instruction (BAML's ctx.output_format equivalent)
    fn get_output_format(&self) -> String {
        r#"
Respond with a JSON object with one of these formats:

For continuing: {"action": "continue", "next_task_id": "T1" | null, "approach": "..."}
For replanning: {"action": "replan", "reason": "...", "context": "...", "preserve_completed": true|false}
For completion: {"action": "complete", "summary": "...", "artifacts": ["..."]}
For human input: {"action": "human", "question": "...", "options": [...] | null, "blocking": true|false}
For spawning: {"action": "spawn", "category": "searcher"|"analyzer"|"builder"|"validator", "task_id": "...", "prompt": "...", "timeout_seconds": N | null}
For validation: {"action": "validate", "scope": "all"|"changed"|"path/...", "continue_on_failure": true|false}
"#
        .to_string()
    }

    /// Make a heuristic-based decision (fallback when BAML runtime is unavailable)
    fn make_heuristic_decision(&self, context: &BamlAgentContext) -> Result<LoopDecision> {
        // Rule 1: All tasks done and validation passed -> Complete
        if context.task_status.completed == context.task_status.total_tasks {
            if let Some(ref v) = context.validation {
                if v.passed {
                    return Ok(LoopDecision::Complete(Complete {
                                                summary: "All tasks completed and validation passed".to_string(),
                        artifacts: vec![],
                    }));
                }
            } else {
                // Need to run validation first
                return Ok(LoopDecision::Validate(RunValidation {
                                        scope: "all".to_string(),
                    continue_on_failure: false,
                }));
            }
        }

        // Rule 2: Blockers that need human clarification
        let ambiguous_blockers: Vec<_> = context
            .task_status
            .blockers
            .iter()
            .filter(|b| {
                b.to_lowercase().contains("unclear")
                    || b.to_lowercase().contains("ambiguous")
                    || b.to_lowercase().contains("decision")
                    || b.to_lowercase().contains("?")
            })
            .collect();

        if !ambiguous_blockers.is_empty() {
            return Ok(LoopDecision::Human(NeedHumanInput {
                                question: ambiguous_blockers.first().unwrap().to_string(),
                options: None,
                blocking: true,
            }));
        }

        // Rule 3: Technical blockers -> Replan
        if !context.task_status.blockers.is_empty() && context.task_status.next_ready.is_none() {
            return Ok(LoopDecision::Replan(RequestReplan {
                                reason: format!("Blocked: {}", context.task_status.blockers.join(", ")),
                context: context.recent_output.clone(),
                preserve_completed: true,
            }));
        }

        // Rule 4: Ready task available -> Spawn subagent
        if let Some(ref task) = context.task_status.next_ready {
            let category = self.select_category_for_task(task);
            return Ok(LoopDecision::Spawn(SpawnSubagent {
                                category,
                task_id: task.id.clone(),
                prompt: format!(
                    "Complete task {}: {}\n\nDescription: {}",
                    task.id, task.title, task.description
                ),
                timeout_seconds: match task.complexity {
                    BamlComplexity::Low => Some(120),
                    BamlComplexity::Medium => Some(300),
                    BamlComplexity::High => Some(600),
                },
            }));
        }

        // Rule 5: Validation failed -> need to fix
        if let Some(ref v) = context.validation {
            if !v.passed && !v.failures.is_empty() {
                return Ok(LoopDecision::Continue(ContinueBuilding {
                                        next_task_id: None,
                    approach: format!("Fix validation failures: {}", v.failures.join(", ")),
                }));
            }
        }

        // Default: Continue (waiting for dependencies)
        Ok(LoopDecision::Continue(ContinueBuilding {
                        next_task_id: None,
            approach: "Waiting for dependencies or blocked tasks to resolve".to_string(),
        }))
    }

    /// Select agent category based on task characteristics
    fn select_category_for_task(&self, task: &BamlTask) -> BamlAgentCategory {
        let title_lower = task.title.to_lowercase();
        let desc_lower = task.description.to_lowercase();

        // Keywords for each category
        if title_lower.contains("test")
            || title_lower.contains("verify")
            || title_lower.contains("check")
            || desc_lower.contains("run tests")
            || desc_lower.contains("validate")
        {
            return BamlAgentCategory::Validator;
        }

        if title_lower.contains("research")
            || title_lower.contains("analyze")
            || title_lower.contains("understand")
            || title_lower.contains("investigate")
            || desc_lower.contains("analyze")
        {
            return BamlAgentCategory::Analyzer;
        }

        if title_lower.contains("find")
            || title_lower.contains("search")
            || title_lower.contains("locate")
            || desc_lower.contains("grep")
            || desc_lower.contains("search")
        {
            return BamlAgentCategory::Searcher;
        }

        // Default to Builder for implementation tasks
        BamlAgentCategory::Builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_context(completed: i32, total: i32) -> DecisionContext {
        DecisionContext::new(
            BamlTaskGraphStatus {
                total_tasks: total,
                completed,
                in_progress: 0,
                blocked: 0,
                pending: total - completed,
                next_ready: if completed < total {
                    Some(BamlTask {
                        id: format!("T{}", completed + 1),
                        title: "Test task".to_string(),
                        description: "A test task".to_string(),
                        status: BamlTaskStatus::Pending,
                        complexity: BamlComplexity::Medium,
                        depends_on: vec![],
                    })
                } else {
                    None
                },
                blockers: vec![],
            },
            "Previous output".to_string(),
        )
    }

    #[test]
    fn test_decision_context_creation() {
        let ctx = make_test_context(2, 5);
        assert_eq!(ctx.task_status.completed, 2);
        assert_eq!(ctx.task_status.total_tasks, 5);
    }

    #[test]
    fn test_decision_summary() {
        let decision = Decision::from_decision(
            LoopDecision::Continue(ContinueBuilding {
                                next_task_id: Some("T3".to_string()),
                approach: "Keep going".to_string(),
            }),
            100,
        );

        let summary = decision.summary();
        assert!(summary.contains("Continue"));
        assert!(summary.contains("T3"));
    }

    #[test]
    fn test_spawn_decision_summary() {
        let decision = Decision::from_decision(
            LoopDecision::Spawn(SpawnSubagent {
                                category: BamlAgentCategory::Builder,
                task_id: "T1".to_string(),
                prompt: "Build the feature".to_string(),
                timeout_seconds: Some(300),
            }),
            50,
        );

        assert_eq!(decision.action(), "spawn");
        assert!(!decision.is_terminal());
    }

    #[test]
    fn test_category_selection() {
        let runtime = BamlRuntime::new(super::super::runtime::BamlConfig::default());
        let engine = DecisionEngine::new(runtime);

        let test_task = BamlTask {
            id: "T1".to_string(),
            title: "Run unit tests".to_string(),
            description: "Execute the test suite".to_string(),
            status: BamlTaskStatus::Pending,
            complexity: BamlComplexity::Low,
            depends_on: vec![],
        };

        let category = engine.select_category_for_task(&test_task);
        assert_eq!(category, BamlAgentCategory::Validator);

        let search_task = BamlTask {
            id: "T2".to_string(),
            title: "Find authentication code".to_string(),
            description: "Search for auth implementations".to_string(),
            status: BamlTaskStatus::Pending,
            complexity: BamlComplexity::Low,
            depends_on: vec![],
        };

        let category = engine.select_category_for_task(&search_task);
        assert_eq!(category, BamlAgentCategory::Searcher);
    }
}
