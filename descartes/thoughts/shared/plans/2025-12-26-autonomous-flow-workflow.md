# Autonomous Flow Workflow Implementation Plan

## Overview

This plan transforms the Flow Workflow from semi-autonomous to fully autonomous execution. The goal is to allow a human to start a flow with a well-structured PRD, then step away while the agent executes all 6 phases autonomously. The human can re-enter the loop at any time, see everything that happened, and roll back if needed.

## Current State Analysis

### What Works Today
- 6 phases execute sequentially via `FlowExecutor::execute()`
- State persists to `.scud/flow-state.json` after each phase
- Resume capability via `--resume` flag
- Orchestrator agent consulted on failures
- Full JSON transcripts in `.scud/sessions/`

### Key Gaps Identified

1. **No Watchdog/Timeout**
   - If an agent hangs indefinitely, no automatic recovery
   - No way to detect stalled phases

2. **Retry Logic Not Implemented**
   - `handle_phase_error()` only checks for "abort" (line 452-454)
   - Orchestrator can respond with `retry` but code doesn't handle it
   - No retry counter or max retry limit

3. **No Git Checkpoints**
   - `FlowGitState` struct exists but is never populated
   - No automatic commits at phase boundaries
   - No rollback capability on failure

4. **Sequential QA**
   - Comment at line 282-283 notes QA should run concurrently
   - Currently runs after implement phase completes

5. **Unused Configuration**
   - `FlowConfig` has fields like `orchestrator_model`, `pause_between_phases` that are never used

## Desired End State

After implementation:

```
Human starts flow → [All 6 phases execute autonomously] → Human reviews anytime
                           ↑                                    ↓
                    Watchdog handles timeouts          Full playback available
                    Orchestrator retries failures      Git checkpoints for rollback
                    Git commits at boundaries          QA monitors in real-time
```

### Verification Checklist
- [ ] Flow completes 6 phases without human intervention (happy path)
- [ ] Stalled phase detected and recovered within configured timeout
- [ ] Failed phase retried up to max_retries with orchestrator guidance
- [ ] Git checkpoint created at each phase boundary
- [ ] Flow can be rolled back to any phase checkpoint
- [ ] QA feedback available during implementation (not just after)
- [ ] Full session transcript playable from `.scud/sessions/`

## What We're NOT Doing

- **Interactive human checkpoints**: The flow should run fully autonomously unless it fails
- **Automatic PRD generation**: Human provides well-structured PRD
- **External notification system**: No webhooks/email on completion (could add later)
- **Distributed execution**: All phases run in same process
- **Agent streaming to terminal**: Focus on transcript/playback over live streaming

## Implementation Approach

We'll implement in 4 phases, each adding a layer of autonomy:

1. **Phase 1**: Watchdog timeout system
2. **Phase 2**: Full retry logic with orchestrator
3. **Phase 3**: Git checkpoint and rollback
4. **Phase 4**: Concurrent QA monitoring

---

## Phase 1: Watchdog Timeout System

### Overview
Add configurable timeouts to phase execution so stalled agents are detected and handled.

### Changes Required:

#### 1.1 Add Timeout Configuration

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Add timeout fields to FlowConfig

```rust
/// Flow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    // ... existing fields ...

    /// Timeout per phase in seconds (default: 1800 = 30 minutes)
    pub phase_timeout_secs: u64,

    /// Watchdog check interval in seconds (default: 60)
    pub watchdog_interval_secs: u64,

    /// Maximum total flow duration in seconds (default: 14400 = 4 hours)
    pub max_flow_duration_secs: u64,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            phase_timeout_secs: 1800,        // 30 minutes per phase
            watchdog_interval_secs: 60,      // Check every minute
            max_flow_duration_secs: 14400,   // 4 hours max
        }
    }
}
```

#### 1.2 Add Timeout to Phase Execution

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Wrap `execute_phase()` call with timeout

```rust
use tokio::time::{timeout, Duration};

async fn execute_phase_with_timeout(&mut self, phase: &str) -> Result<()> {
    let timeout_duration = Duration::from_secs(self.state.config.phase_timeout_secs);

    match timeout(timeout_duration, self.execute_phase(phase)).await {
        Ok(result) => result,
        Err(_) => {
            // Phase timed out
            self.update_phase_status(phase, PhaseStatus::Failed);
            Err(anyhow::anyhow!(
                "Phase '{}' timed out after {} seconds",
                phase,
                self.state.config.phase_timeout_secs
            ))
        }
    }
}
```

#### 1.3 Add Flow-Level Timeout Check

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Check total duration in main execute loop

```rust
pub async fn execute(&mut self) -> Result<FlowResult> {
    let start_time = std::time::Instant::now();
    let max_duration = Duration::from_secs(self.state.config.max_flow_duration_secs);

    // ... existing phase loop ...

    for phase in ["ingest", "review_graph", "plan_tasks"] {
        // Check total flow timeout
        if start_time.elapsed() > max_duration {
            error!("Flow exceeded maximum duration of {} seconds",
                   self.state.config.max_flow_duration_secs);
            break;
        }

        // Use timeout-wrapped execution
        match self.execute_phase_with_timeout(phase).await {
            // ... existing handling ...
        }
    }

    // ... rest of method ...
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` compiles without errors
- [x] `cargo test -p descartes-core flow_executor` passes
- [x] New unit test verifies timeout triggers on slow phase

#### Manual Verification:
- [ ] Start flow with artificially low timeout (10s), verify it times out
- [ ] Verify timeout appears in flow-state.json as Failed status

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: Full Retry Logic with Orchestrator

### Overview
Implement proper retry/skip/abort parsing from orchestrator response and retry loop.

### Changes Required:

#### 2.1 Add Retry State Tracking

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Add retry tracking to PhaseState

```rust
/// Individual phase state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub status: PhaseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub data: serde_json::Value,

    // New fields
    #[serde(default)]
    pub retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}
```

#### 2.2 Add Orchestrator Decision Enum

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Create structured decision type

```rust
/// Orchestrator decision for error handling
#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorDecision {
    Retry { reason: String },
    Skip { reason: String },
    Abort { reason: String },
    Continue { reason: String },
}

impl OrchestratorDecision {
    /// Parse from orchestrator response text
    fn parse(output: &str) -> Self {
        let lower = output.to_lowercase();

        // Extract reason if present
        let reason = lower
            .lines()
            .find(|l| l.starts_with("reason:"))
            .map(|l| l.trim_start_matches("reason:").trim().to_string())
            .unwrap_or_else(|| "No reason provided".to_string());

        if lower.contains("decision: retry") || lower.contains("decision:retry") {
            OrchestratorDecision::Retry { reason }
        } else if lower.contains("decision: skip") || lower.contains("decision:skip") {
            OrchestratorDecision::Skip { reason }
        } else if lower.contains("decision: abort") || lower.contains("decision:abort") {
            OrchestratorDecision::Abort { reason }
        } else {
            // Default to continue if unclear
            OrchestratorDecision::Continue { reason }
        }
    }
}
```

#### 2.3 Implement Retry Loop

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Replace single execution with retry loop

```rust
/// Add max_retries to FlowConfig
pub struct FlowConfig {
    // ... existing fields ...

    /// Maximum retries per phase (default: 3)
    pub max_retries_per_phase: u32,
}

/// Execute phase with retry support
async fn execute_phase_with_retry(&mut self, phase: &str) -> Result<()> {
    let max_retries = self.state.config.max_retries_per_phase;

    loop {
        // Check if we've exceeded max retries
        let retry_count = self.get_phase_retry_count(phase);
        if retry_count >= max_retries {
            return Err(anyhow::anyhow!(
                "Phase '{}' failed after {} retries",
                phase, retry_count
            ));
        }

        match self.execute_phase_with_timeout(phase).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                // Increment retry count
                self.increment_phase_retry(phase);
                self.set_phase_error(phase, &e.to_string());

                // Ask orchestrator what to do
                let decision = self.get_orchestrator_decision(phase, &e).await?;

                match decision {
                    OrchestratorDecision::Retry { reason } => {
                        info!("Orchestrator decided to retry phase '{}': {}", phase, reason);
                        // Loop continues
                    }
                    OrchestratorDecision::Skip { reason } => {
                        info!("Orchestrator decided to skip phase '{}': {}", phase, reason);
                        self.update_phase_status(phase, PhaseStatus::Skipped);
                        return Ok(());
                    }
                    OrchestratorDecision::Abort { reason } => {
                        error!("Orchestrator decided to abort: {}", reason);
                        return Err(anyhow::anyhow!("Aborted: {}", reason));
                    }
                    OrchestratorDecision::Continue { reason } => {
                        info!("Orchestrator decided to continue: {}", reason);
                        return Ok(());
                    }
                }
            }
        }
    }
}
```

#### 2.4 Update handle_phase_error to return decision

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Return structured decision instead of bool

```rust
/// Get orchestrator decision for error handling
async fn get_orchestrator_decision(
    &mut self,
    phase: &str,
    error: &anyhow::Error,
) -> Result<OrchestratorDecision> {
    // Check if orchestrator agent exists
    if !self.agent_loader.agent_exists("flow-orchestrator") {
        warn!("Orchestrator agent not found, defaulting to abort");
        return Ok(OrchestratorDecision::Abort {
            reason: "Orchestrator agent not found".to_string(),
        });
    }

    let context = WorkflowContext::new(
        self.working_dir.clone(),
        self.state.tag.as_deref().unwrap_or("flow"),
    )
    .map_err(|e| anyhow::anyhow!("Failed to create workflow context: {}", e))?;

    let retry_count = self.get_phase_retry_count(phase);

    let task = format!(
        r#"Phase '{}' failed with error: {}

Retry count: {} of {}
Flow tag: {}
Current phase: {}

Analyze the error and decide the best course of action.

Respond with:
Decision: <retry|skip|abort>
Reason: <brief explanation>
"#,
        phase,
        error,
        retry_count,
        self.state.config.max_retries_per_phase,
        self.state.tag.as_deref().unwrap_or("flow"),
        phase
    );

    let step = WorkflowStep {
        name: "Flow: Error Recovery".to_string(),
        agent: "flow-orchestrator".to_string(),
        task: task.clone(),
        parallel: false,
        output: None,
    };

    let config = WorkflowExecutorConfig::default();
    let result = execute_step(&step, &task, &context, self.backend.as_ref(), &config)
        .await
        .map_err(|e| anyhow::anyhow!("Orchestrator execution failed: {}", e))?;

    Ok(OrchestratorDecision::parse(&result.output))
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` compiles without errors
- [x] `cargo test -p descartes-core flow_executor` passes
- [x] Unit test for `OrchestratorDecision::parse()` covers all variants
- [ ] Integration test simulates failure → retry → success

#### Manual Verification:
- [ ] Intentionally fail a phase, verify retry occurs
- [ ] Verify retry_count increments in flow-state.json
- [ ] Verify orchestrator "skip" decision marks phase as Skipped

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Git Checkpoint and Rollback

### Overview
Create git commits at phase boundaries and enable rollback to any checkpoint.

### Changes Required:

#### 3.1 Add Git Operations Module

**File**: `descartes/core/src/flow_git.rs` (new file)
**Changes**: Create git operations wrapper

```rust
//! Git operations for flow workflow checkpoints

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Git checkpoint operations for flow workflow
pub struct FlowGit {
    working_dir: std::path::PathBuf,
}

impl FlowGit {
    pub fn new(working_dir: impl AsRef<Path>) -> Self {
        Self {
            working_dir: working_dir.as_ref().to_path_buf(),
        }
    }

    /// Get current HEAD commit hash
    pub fn current_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get current commit"))
        }
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch == "HEAD" {
                Ok(None) // Detached HEAD
            } else {
                Ok(Some(branch))
            }
        } else {
            Ok(None)
        }
    }

    /// Check if working directory is clean
    pub fn is_clean(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.working_dir)
            .output()?;

        Ok(output.status.success() && output.stdout.is_empty())
    }

    /// Create checkpoint commit for phase
    pub fn create_checkpoint(&self, phase: &str, tag: &str) -> Result<String> {
        // Stage all changes
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.working_dir)
            .output()?;

        // Create commit
        let message = format!(
            "flow({}): checkpoint after {} phase\n\nAutomated checkpoint by Descartes flow workflow",
            tag, phase
        );

        let output = Command::new("git")
            .args(["commit", "-m", &message, "--allow-empty"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            self.current_commit()
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to create checkpoint: {}", stderr))
        }
    }

    /// Rollback to a specific commit
    pub fn rollback(&self, commit: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["reset", "--hard", commit])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to rollback: {}", stderr))
        }
    }

    /// Stash current changes
    pub fn stash(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["stash", "push", "-m", message])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to stash: {}", stderr))
        }
    }
}
```

#### 3.2 Add Phase Checkpoints to FlowGitState

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Track checkpoint per phase

```rust
/// Git tracking for flow
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowGitState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    // New: checkpoint after each phase
    #[serde(default)]
    pub phase_checkpoints: std::collections::HashMap<String, String>,
}
```

#### 3.3 Integrate Checkpoints into Phase Execution

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Create checkpoint after each successful phase

```rust
use crate::flow_git::FlowGit;

impl FlowExecutor {
    /// Initialize git state at flow start
    async fn init_git_state(&mut self) -> Result<()> {
        if !self.state.config.auto_commit {
            return Ok(());
        }

        let git = FlowGit::new(&self.working_dir);

        self.state.git.start_commit = Some(git.current_commit()?);
        self.state.git.branch = git.current_branch()?;

        Ok(())
    }

    /// Create checkpoint after phase completion
    async fn create_phase_checkpoint(&mut self, phase: &str) -> Result<()> {
        if !self.state.config.auto_commit {
            return Ok(());
        }

        let git = FlowGit::new(&self.working_dir);
        let tag = self.state.tag.as_deref().unwrap_or("flow");

        let commit = git.create_checkpoint(phase, tag)?;
        self.state.git.phase_checkpoints.insert(phase.to_string(), commit.clone());

        info!("Created git checkpoint for phase '{}': {}", phase, &commit[..8]);
        Ok(())
    }

    /// Rollback to checkpoint from specific phase
    pub async fn rollback_to_phase(&mut self, phase: &str) -> Result<()> {
        let commit = self.state.git.phase_checkpoints.get(phase)
            .ok_or_else(|| anyhow::anyhow!("No checkpoint found for phase '{}'", phase))?
            .clone();

        let git = FlowGit::new(&self.working_dir);
        git.rollback(&commit)?;

        // Reset subsequent phases to pending
        self.reset_phases_after(phase);

        info!("Rolled back to phase '{}' checkpoint: {}", phase, &commit[..8]);
        Ok(())
    }
}
```

#### 3.4 Add Rollback Command to CLI

**File**: `descartes/cli/src/commands/workflow.rs`
**Changes**: Add rollback subcommand

```rust
/// Rollback flow to a specific phase checkpoint
#[command(name = "flow-rollback")]
FlowRollback {
    /// Phase to rollback to
    #[arg(long)]
    phase: String,

    /// Working directory
    #[arg(short, long)]
    dir: Option<PathBuf>,
},
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` compiles without errors
- [x] `cargo test -p descartes-core flow_executor` passes
- [x] `cargo test -p descartes-core flow_git` passes
- [x] Unit tests for FlowGit operations pass

#### Manual Verification:
- [ ] Run flow, verify git commits appear after each phase
- [ ] Verify phase_checkpoints populated in flow-state.json
- [ ] Test `descartes workflow flow-rollback --phase ingest`
- [ ] Verify rollback resets files to checkpoint state

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 4.

---

## Phase 4: Concurrent QA Monitoring

### Overview
Run QA agent concurrently with implementation to provide real-time quality feedback.

### Changes Required:

#### 4.1 Add QA Monitoring State

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Add QA monitoring tracking

```rust
/// QA monitoring state during implementation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QAMonitorState {
    /// Whether QA is actively monitoring
    pub active: bool,
    /// Number of issues found
    pub issues_found: u32,
    /// Last commit reviewed
    pub last_reviewed_commit: Option<String>,
    /// QA log entries
    #[serde(default)]
    pub log_entries: Vec<QALogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QALogEntry {
    pub timestamp: DateTime<Utc>,
    pub commit: String,
    pub severity: String,  // info, warning, error
    pub message: String,
}
```

#### 4.2 Implement Concurrent QA During Implementation

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Run QA monitoring as background task

```rust
use tokio::sync::mpsc;

impl FlowExecutor {
    /// Execute implement phase with concurrent QA monitoring
    async fn execute_implement_with_qa(&mut self) -> Result<()> {
        // Create channel for QA events
        let (qa_tx, mut qa_rx) = mpsc::channel::<QALogEntry>(100);

        // Clone what we need for the QA task
        let backend = self.backend.clone();
        let working_dir = self.working_dir.clone();
        let tag = self.state.tag.clone();
        let agent_loader = self.agent_loader.clone();

        // Spawn QA monitoring task
        let qa_handle = tokio::spawn(async move {
            Self::run_qa_monitor(
                working_dir,
                tag,
                agent_loader,
                backend,
                qa_tx,
            ).await
        });

        // Execute implement phase
        let implement_result = self.execute_phase_with_retry("implement").await;

        // Signal QA to stop (by dropping sender when phase completes)
        // Collect QA results
        drop(qa_rx);

        // Wait for QA to finish
        match qa_handle.await {
            Ok(Ok(())) => info!("QA monitoring completed"),
            Ok(Err(e)) => warn!("QA monitoring error: {}", e),
            Err(e) => warn!("QA task join error: {}", e),
        }

        implement_result
    }

    /// Background QA monitoring task
    async fn run_qa_monitor(
        working_dir: PathBuf,
        tag: Option<String>,
        agent_loader: AgentDefinitionLoader,
        backend: Arc<dyn ModelBackend + Send + Sync>,
        tx: mpsc::Sender<QALogEntry>,
    ) -> Result<()> {
        let git = FlowGit::new(&working_dir);
        let mut last_commit = git.current_commit()?;
        let check_interval = Duration::from_secs(30);

        loop {
            tokio::time::sleep(check_interval).await;

            // Check if channel is closed (implement phase finished)
            if tx.is_closed() {
                break;
            }

            // Check for new commits
            let current_commit = git.current_commit()?;
            if current_commit != last_commit {
                // New commit detected - run QA check
                let entry = Self::qa_check_commit(
                    &working_dir,
                    &tag,
                    &agent_loader,
                    backend.as_ref(),
                    &last_commit,
                    &current_commit,
                ).await?;

                let _ = tx.send(entry).await;
                last_commit = current_commit;
            }
        }

        Ok(())
    }

    /// Run QA check on commit diff
    async fn qa_check_commit(
        working_dir: &Path,
        tag: &Option<String>,
        agent_loader: &AgentDefinitionLoader,
        backend: &dyn ModelBackend,
        from_commit: &str,
        to_commit: &str,
    ) -> Result<QALogEntry> {
        // Get diff
        let diff_output = Command::new("git")
            .args(["diff", from_commit, to_commit])
            .current_dir(working_dir)
            .output()?;

        let diff = String::from_utf8_lossy(&diff_output.stdout);

        // Create QA task
        let context = WorkflowContext::new(
            working_dir.to_path_buf(),
            tag.as_deref().unwrap_or("flow"),
        )?;

        let task = format!(
            "Review this commit diff for quality issues:\n\n```diff\n{}\n```\n\nProvide a brief assessment (1-2 sentences). Format: SEVERITY: <info|warning|error> MESSAGE: <assessment>",
            &diff[..diff.len().min(4000)]  // Limit diff size
        );

        let step = WorkflowStep {
            name: "QA: Commit Review".to_string(),
            agent: "flow-qa".to_string(),
            task: task.clone(),
            parallel: false,
            output: None,
        };

        let config = WorkflowExecutorConfig::default();
        let result = execute_step(&step, &task, &context, backend, &config).await?;

        // Parse response
        let (severity, message) = Self::parse_qa_response(&result.output);

        Ok(QALogEntry {
            timestamp: Utc::now(),
            commit: to_commit.to_string(),
            severity,
            message,
        })
    }

    fn parse_qa_response(output: &str) -> (String, String) {
        let lower = output.to_lowercase();
        let severity = if lower.contains("error") {
            "error"
        } else if lower.contains("warning") {
            "warning"
        } else {
            "info"
        }.to_string();

        (severity, output.to_string())
    }
}
```

#### 4.3 Update Execute to Use Concurrent QA

**File**: `descartes/core/src/flow_executor.rs`
**Changes**: Use new method in execute loop

```rust
pub async fn execute(&mut self) -> Result<FlowResult> {
    // ... Phase 1-3 as before ...

    // Phase 4: Implement with concurrent QA
    if !self.is_phase_completed("implement") {
        match self.execute_implement_with_qa().await {
            Ok(_) => phases_completed.push("implement".to_string()),
            Err(e) => {
                phases_failed.push("implement".to_string());
                error!("Implement phase failed: {}", e);
            }
        }
        self.save_state().await?;
    }

    // Phase 5: Final QA (now just a summary of monitoring)
    // ...

    // Phase 6: Summarize
    // ...
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` compiles without errors
- [x] `cargo test -p descartes-core flow_executor` passes
- [x] Unit test for QA log entry parsing

#### Manual Verification:
- [x] Run flow, verify QA log entries appear in flow-state.json
- [x] Verify QA checks happen during implementation (not just after)
- [x] Check `.scud/qa-log.json` for monitoring entries

**Implementation Note**: After completing this phase and all automated verification passes, the full autonomous flow is ready for end-to-end testing.

---

## Testing Strategy

### Unit Tests

**FlowConfig defaults**:
- Verify timeout defaults are reasonable
- Verify max_retries default

**OrchestratorDecision parsing**:
- `"Decision: retry\nReason: transient"` → `Retry`
- `"Decision: skip\nReason: optional"` → `Skip`
- `"Decision: abort\nReason: critical"` → `Abort`
- `"Some unclear response"` → `Continue`

**FlowGit operations**:
- Mock git commands or use tempdir with real git

### Integration Tests

**End-to-end flow**:
1. Create mock PRD
2. Run flow with test backend
3. Verify all 6 phases complete
4. Verify git checkpoints created
5. Verify state file accurate

**Failure recovery**:
1. Configure backend to fail on phase 2
2. Verify retry occurs
3. Configure to succeed on retry
4. Verify flow completes

### Manual Testing Steps

1. Run full flow with real PRD
2. Intentionally kill mid-phase, verify resume works
3. Test rollback command
4. Verify QA monitoring produces entries
5. Test with very long phase (beyond timeout)

## Performance Considerations

- **Timeout tuning**: 30 min default may be too short for large codebases
- **QA check interval**: 30s may cause too many agent calls, make configurable
- **Git operations**: All sync, could bottleneck on large repos
- **Memory**: QA log could grow large, consider rotation

## Migration Notes

Existing flow-state.json files will load with defaults for new fields:
- `phase_timeout_secs`: 1800
- `retry_count`: 0
- `phase_checkpoints`: empty

No breaking changes to existing flows.

## References

- Research doc: `thoughts/shared/research/2025-12-26-flow-orchestration-architecture.md`
- Flow docs: `docs/FLOW-WORKFLOW.md`
- Existing patterns: `body_restore.rs`, `time_travel_integration.rs`
- Timeout patterns: `daemon/src/rpc_client.rs`, `daemon/src/agent_monitor.rs`
