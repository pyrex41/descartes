//! Workflow runner/orchestrator
//!
//! Executes workflows, handling:
//! - Stage transitions
//! - Gate checks
//! - Handoff generation
//! - Pre/post hooks
//! - State persistence

use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use super::config::{GateType, WorkflowConfig};
use super::gate::{ApprovalMethod, CliGate, GateController, GateResult};
use super::notify::{create_channels, Notification};
use super::state::{StateManager, StageStatus, WorkflowState, WorkflowStatus};
use crate::handoff::{Handoff, HandoffBuilder};
use crate::harness::{Harness, SessionConfig};
use crate::{Config, Error, Result};

/// Options for running a workflow
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Force all gates to manual (step-by-step mode)
    pub step_by_step: bool,
    /// Force all gates to auto (one-shot mode)
    pub one_shot: bool,
    /// Start from a specific stage
    pub from_stage: Option<String>,
    /// Stop after a specific stage
    pub to_stage: Option<String>,
    /// Override specific gates
    pub gate_overrides: Vec<(String, GateType)>,
    /// Extra context to inject
    pub extra_context: Option<String>,
    /// Resume a specific workflow run
    pub resume_id: Option<String>,
    /// Dry run (don't actually execute agents)
    pub dry_run: bool,
}

/// Workflow runner
pub struct WorkflowRunner {
    /// Workflow configuration
    workflow_config: WorkflowConfig,
    /// Application configuration
    app_config: Config,
    /// State manager
    state_manager: StateManager,
    /// Harness for running agents
    harness: Box<dyn Harness>,
}

impl WorkflowRunner {
    /// Create a new workflow runner
    pub fn new(
        workflow_config: WorkflowConfig,
        app_config: Config,
        harness: Box<dyn Harness>,
    ) -> Self {
        Self {
            workflow_config,
            app_config,
            state_manager: StateManager::default(),
            harness,
        }
    }

    /// Run the workflow
    pub async fn run(&self, options: RunOptions) -> Result<WorkflowState> {
        // Load or create state
        let mut state = if let Some(id) = &options.resume_id {
            info!("Resuming workflow run: {}", id);
            self.state_manager
                .load(&self.workflow_config.workflow.name, id)?
        } else {
            info!(
                "Starting new workflow: {}",
                self.workflow_config.workflow.name
            );
            let mut state = WorkflowState::new(
                &self.workflow_config.workflow.name,
                &self.workflow_config.workflow.stages,
            );

            // Apply run options
            state.config_overrides.step_by_step = options.step_by_step;
            state.config_overrides.one_shot = options.one_shot;
            state.config_overrides.extra_context = options.extra_context.clone();

            state
        };

        // Determine starting stage
        let stages = self.workflow_config.stages();
        let start_stage = options
            .from_stage
            .as_ref()
            .or(Some(&state.current_stage))
            .unwrap();

        let start_idx = stages
            .iter()
            .position(|s| s == start_stage)
            .ok_or_else(|| Error::Config(format!("Unknown stage: {}", start_stage)))?;

        // Run through stages
        for i in start_idx..stages.len() {
            let stage = &stages[i];

            // Check if we should stop
            if let Some(ref to) = options.to_stage {
                if stage == to {
                    info!("Reached stop stage: {}", to);
                    break;
                }
            }

            // Run this stage
            match self.run_stage(&mut state, stage, &options).await {
                Ok(true) => {
                    // Stage completed, continue
                    info!("Stage {} completed", stage);
                }
                Ok(false) => {
                    // Stage paused or waiting
                    info!("Workflow paused at stage {}", stage);
                    self.state_manager.save(&state)?;
                    return Ok(state);
                }
                Err(e) => {
                    error!("Stage {} failed: {}", stage, e);
                    state.fail_stage(stage, &e.to_string());
                    self.state_manager.save(&state)?;
                    return Err(e);
                }
            }

            // Save state after each stage
            self.state_manager.save(&state)?;

            // Check for next stage gate
            if let Some(next_stage) = self.workflow_config.next_stage(stage) {
                match self
                    .check_gate(&mut state, stage, next_stage, &options)
                    .await?
                {
                    GateResult::Approved { .. } => {
                        info!("Gate approved: {} → {}", stage, next_stage);
                    }
                    GateResult::Rejected { reason } => {
                        info!("Gate rejected: {}", reason);
                        state.gate_rejected(stage, next_stage, &reason);
                        self.state_manager.save(&state)?;
                        return Ok(state);
                    }
                    GateResult::Skip => {
                        info!("Skipping stage: {}", next_stage);
                        state.skip_stage(next_stage);
                    }
                    GateResult::Waiting { .. } => {
                        info!("Waiting at gate: {} → {}", stage, next_stage);
                        state.gate_waiting(stage, next_stage);
                        self.state_manager.save(&state)?;
                        return Ok(state);
                    }
                    GateResult::EditRequested => {
                        info!("Edit requested for handoff");
                        // TODO: Open editor for handoff
                        state.gate_waiting(stage, next_stage);
                        self.state_manager.save(&state)?;
                        return Ok(state);
                    }
                }
            }
        }

        // Workflow complete
        state.complete();
        self.state_manager.save(&state)?;

        info!("Workflow completed: {}", state.id);
        Ok(state)
    }

    /// Run a single stage
    async fn run_stage(
        &self,
        state: &mut WorkflowState,
        stage: &str,
        options: &RunOptions,
    ) -> Result<bool> {
        // Skip if already completed
        if let Some(stage_state) = state.stages.get(stage) {
            if matches!(
                stage_state.status,
                StageStatus::Completed | StageStatus::Skipped
            ) {
                debug!("Stage {} already completed, skipping", stage);
                return Ok(true);
            }
        }

        info!("Running stage: {}", stage);

        // Get previous handoff
        let previous_handoff = state.get_previous_handoff(stage, self.workflow_config.stages());

        // Get transition config
        let prev_stage = self.get_previous_stage(stage);
        let transition = prev_stage.and_then(|p| self.workflow_config.get_transition(p, stage));

        // Build the prompt
        let prompt = self.build_stage_prompt(stage, previous_handoff, transition, options)?;

        if options.dry_run {
            println!("\n=== DRY RUN: Stage {} ===", stage);
            println!("Prompt:\n{}", prompt);
            println!("=========================\n");
            state.complete_stage(stage, Some("(dry run)".to_string()), None);
            return Ok(true);
        }

        // Run pre-hooks
        if let Some(trans) = transition {
            for hook in &trans.pre_hooks {
                info!("Running pre-hook: {}", hook);
                self.run_hook(hook).await?;
            }
        }

        // Start session
        let session_config = SessionConfig {
            model: self.get_model_for_stage(stage),
            system_prompt: Some(prompt.clone()),
            ..Default::default()
        };

        let session = self.harness.start_session(session_config).await?;
        state.start_stage(stage, &session.id);

        // For now, we just send the prompt and wait for completion
        // In a full implementation, this would be more interactive
        let mut stream = self.harness.send(&session, &prompt).await?;

        // Collect output
        use futures::StreamExt;
        let mut output = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                crate::harness::ResponseChunk::Text(text) => {
                    output.push_str(&text);
                }
                crate::harness::ResponseChunk::Done => break,
                crate::harness::ResponseChunk::Error(e) => {
                    self.harness.close_session(&session).await?;
                    return Err(Error::Harness(e));
                }
                _ => {}
            }
        }

        self.harness.close_session(&session).await?;

        // Run post-hooks
        if let Some(trans) = transition {
            for hook in &trans.post_hooks {
                info!("Running post-hook: {}", hook);
                self.run_hook(hook).await?;
            }
        }

        // Generate handoff for next stage
        let handoff = self.generate_handoff(stage, &output).await?;
        state.complete_stage(stage, Some(handoff), Some(output));

        Ok(true)
    }

    /// Check gate between stages
    async fn check_gate(
        &self,
        state: &mut WorkflowState,
        from: &str,
        to: &str,
        options: &RunOptions,
    ) -> Result<GateResult> {
        let mut gate_config = self.workflow_config.get_gate(from, to);

        // Apply overrides
        if options.one_shot {
            gate_config.gate_type = GateType::Auto;
        } else if options.step_by_step {
            gate_config.gate_type = GateType::Manual;
        }

        // Check for specific gate override
        let gate_key = format!("{}_to_{}", from, to);
        for (key, gate_type) in &options.gate_overrides {
            if key == &gate_key {
                gate_config.gate_type = *gate_type;
            }
        }

        // Build notification
        let handoff = state
            .stages
            .get(from)
            .and_then(|s| s.handoff.as_ref())
            .map(|h| h.as_str())
            .unwrap_or("");

        let transition = self.workflow_config.get_transition(from, to);
        let command = transition.and_then(|t| t.command.as_ref());

        let notification = Notification::new(&self.workflow_config.workflow.name, from, to)
            .with_summary(&format!("Stage {} complete", from))
            .with_handoff(handoff);

        let notification = if let Some(cmd) = command {
            notification.with_command(cmd)
        } else {
            notification
        };

        let notification = if let Some(timeout) = gate_config.timeout {
            notification.with_timeout(timeout)
        } else {
            notification
        };

        // Handle gate based on type
        match gate_config.gate_type {
            GateType::Auto => {
                state.gate_approved(from, to, ApprovalMethod::Auto, None);
                Ok(GateResult::Approved {
                    method: ApprovalMethod::Auto,
                    message: None,
                })
            }
            GateType::Manual => {
                // Use CLI gate for manual approval
                let result = CliGate::prompt(&notification).await?;
                if let GateResult::Approved { ref method, ref message } = result {
                    state.gate_approved(from, to, method.clone(), message.clone());
                }
                Ok(result)
            }
            GateType::Notify => {
                // Create notification channels
                let channels = create_channels(
                    &self.workflow_config.notifications,
                    &gate_config.notify,
                );

                // Create gate controller
                let mut controller = GateController::new(gate_config.clone(), channels);

                // Check gate
                let result = controller.check(&notification).await?;
                if let GateResult::Approved { ref method, ref message } = result {
                    state.gate_approved(from, to, method.clone(), message.clone());
                }
                Ok(result)
            }
        }
    }

    /// Build the prompt for a stage
    fn build_stage_prompt(
        &self,
        stage: &str,
        previous_handoff: Option<&str>,
        transition: Option<&super::config::TransitionConfig>,
        options: &RunOptions,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add previous handoff if available
        if let Some(handoff) = previous_handoff {
            prompt.push_str("## Previous Stage Handoff\n\n");
            prompt.push_str(handoff);
            prompt.push_str("\n\n");
        }

        // Add command if specified
        if let Some(trans) = transition {
            if let Some(cmd) = &trans.command {
                prompt.push_str(&format!("Execute: {}\n\n", cmd));
            }
        }

        // Add extra context if specified
        if let Some(extra) = &options.extra_context {
            prompt.push_str("## Additional Context\n\n");
            prompt.push_str(extra);
            prompt.push_str("\n\n");
        }

        Ok(prompt)
    }

    /// Generate handoff from stage output
    async fn generate_handoff(&self, stage: &str, output: &str) -> Result<String> {
        let next_stage = self.workflow_config.next_stage(stage);

        let handoff = if let Some(next) = next_stage {
            let transition = self.workflow_config.get_transition(stage, next);

            let builder = HandoffBuilder::new(stage, next);

            let builder = if let Some(trans) = transition {
                builder.with_transition_config(trans.clone())
            } else {
                builder
            };

            // Extract summary from output (simplified - could use LLM for better extraction)
            let summary = if output.len() > 500 {
                format!("{}...", &output[..500])
            } else {
                output.to_string()
            };

            builder
                .summary(&summary)
                .populate_auto_context()
                .await?
                .render()
        } else {
            // Final stage, just return output summary
            output.to_string()
        };

        Ok(handoff)
    }

    /// Get previous stage
    fn get_previous_stage(&self, current: &str) -> Option<&str> {
        let stages = self.workflow_config.stages();
        stages
            .iter()
            .position(|s| s == current)
            .and_then(|i| if i > 0 { Some(stages[i - 1].as_str()) } else { None })
    }

    /// Get model for a stage
    fn get_model_for_stage(&self, stage: &str) -> String {
        // Map stages to categories
        let category = match stage {
            "research" => "analyzer",
            "plan" => "planner",
            "implement" => "builder",
            "validate" => "validator",
            _ => "builder",
        };

        self.app_config
            .get_category(category)
            .map(|c| c.model.clone())
            .unwrap_or_else(|| "sonnet".to_string())
    }

    /// Run a hook command
    async fn run_hook(&self, hook: &str) -> Result<()> {
        let output = Command::new("sh")
            .args(["-c", hook])
            .output()
            .await
            .map_err(|e| Error::Command(format!("Hook failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Hook returned non-zero: {}", stderr);
        }

        Ok(())
    }
}

/// Quick handoff command - generates handoff for next stage
pub async fn quick_handoff(
    workflow_config: &WorkflowConfig,
    from_stage: &str,
    extra_context: Option<&str>,
) -> Result<String> {
    let next_stage = workflow_config
        .next_stage(from_stage)
        .ok_or_else(|| Error::Config(format!("No next stage after {}", from_stage)))?;

    let transition = workflow_config.get_transition(from_stage, next_stage);

    let mut builder = HandoffBuilder::new(from_stage, next_stage);

    if let Some(trans) = transition {
        builder = builder.with_transition_config(trans.clone());
    }

    if let Some(extra) = extra_context {
        builder = builder.extra(extra);
    }

    let builder = builder.populate_auto_context().await?;

    Ok(builder.render())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::config::default_workflow;

    #[test]
    fn test_run_options_defaults() {
        let options = RunOptions::default();
        assert!(!options.step_by_step);
        assert!(!options.one_shot);
        assert!(options.from_stage.is_none());
    }

    #[test]
    fn test_get_previous_stage() {
        // Would need a mock harness to fully test
    }
}
