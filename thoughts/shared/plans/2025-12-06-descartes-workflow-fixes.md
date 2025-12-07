# Descartes Workflow System Fixes Implementation Plan

## Overview

Fix the Descartes workflow system to:
1. Use `.descartes/` instead of `.scud/` for sessions (`.scud/` is for external SCUD CLI tool)
2. Enable actual workflow execution (not just display commands)
3. Support passing workflows to any model/headless CLI system
4. Clean separation between SCUD integration (plugin-like) and core Descartes functionality

## Current State Analysis

### Key Issues Identified:

1. **Session path hardcoded to `.scud/`**:
   - `session.rs:57-59` uses `.scud/` directory for session metadata
   - `session_manager.rs:62-64` checks for `.scud/` to identify workspaces
   - `session_manager.rs:107` loads session from `.scud/session.json`
   - `session_manager.rs:265-279` creates `.scud/` directory structure

2. **Workflows only print commands, don't execute**:
   - `workflow.rs:180-191` just prints `descartes spawn` commands
   - No actual execution of workflow steps
   - No parallel execution support for steps marked `parallel: true`

3. **SCUD integration is too tightly coupled**:
   - Session management uses `.scud/` paths throughout
   - Needs to be decoupled so SCUD CLI can remain external

4. **Thoughts storage is at `~/.descartes/thoughts/`**:
   - This is correct - should remain global
   - Per-project access via symlinks works well

### Key Discoveries:
- `workflow_commands.rs:289-316` - `prepare_workflow()` prepares steps but doesn't execute
- `spawn.rs` - Has all the infrastructure to execute agents with any provider
- `agent_definitions.rs` - Agent loading from `~/.descartes/agents/` works correctly
- Model backend abstraction supports multiple providers (Anthropic, OpenAI, Ollama, DeepSeek, Groq, Grok)

## Desired End State

After this plan is complete:
1. `descartes workflow research --topic "..."` will actually execute the workflow steps
2. Sessions will be stored in `.descartes/` directory (not `.scud/`)
3. SCUD CLI can still read/write to `.scud/` for its own purposes (plugin-like integration)
4. Workflows support parallel execution where marked
5. Workflow outputs are saved to the thoughts storage

### Verification:
- `descartes workflow research --topic "test"` spawns agents and produces output
- `ls .descartes/` shows session data (not in `.scud/`)
- External SCUD CLI tool continues to work with `.scud/` directory

## What We're NOT Doing

- Removing SCUD CLI compatibility entirely (it stays as external tool)
- Changing the global `~/.descartes/` structure
- Modifying agent definition format
- Adding new workflow types (just fixing execution of existing ones)

## Implementation Approach

We'll make incremental changes:
1. First, migrate session storage from `.scud/` to `.descartes/`
2. Then implement actual workflow execution
3. Finally, ensure SCUD CLI can still work as a plugin/external tool

---

## Phase 1: Migrate Session Storage to `.descartes/`

### Overview
Change session/workspace detection and storage from `.scud/` to `.descartes/`.

### Changes Required:

#### 1. Update Session Path Constants
**File**: `descartes/core/src/session.rs`
**Changes**: Add `.descartes` constant, update methods

```rust
// Line 56-59, change scud_path to descartes_path
/// Get the path to the .descartes directory
pub fn descartes_path(&self) -> PathBuf {
    self.path.join(".descartes")
}

/// Get the path to the session metadata file
pub fn metadata_path(&self) -> PathBuf {
    self.descartes_path().join("session.json")
}

// Keep scud_path as alias for backward compatibility with SCUD CLI
/// Get the path to the .scud directory (for SCUD CLI plugin compatibility)
pub fn scud_path(&self) -> PathBuf {
    self.path.join(".scud")
}
```

#### 2. Update FileSystemSessionManager
**File**: `descartes/core/src/session_manager.rs`
**Changes**: Update workspace detection and creation

```rust
// Line 62-64, update is_workspace to check for .descartes
fn is_workspace(&self, path: &Path) -> bool {
    // Check for .descartes/ directory (Descartes sessions)
    // Also check .scud/ for backward compatibility and SCUD CLI integration
    path.join(".descartes").exists()
        || path.join(".scud").exists()
        || path.join("config.toml").exists()
}

// Line 104-116, update load_session_from_path to try .descartes first
fn load_session_from_path(&self, path: &Path) -> Option<Session> {
    // Try .descartes/session.json first (new format)
    let descartes_session = path.join(".descartes/session.json");
    if descartes_session.exists() {
        if let Ok(content) = std::fs::read_to_string(&descartes_session) {
            if let Ok(mut session) = serde_json::from_str::<Session>(&content) {
                session.path = path.to_path_buf();
                return Some(session);
            }
        }
    }

    // Fall back to .scud/session.json for backward compatibility
    let scud_session = path.join(".scud/session.json");
    if scud_session.exists() {
        // ... existing code
    }

    // Create new session if directory exists but no metadata
    // ...
}

// Line 257-279, update create_session to use .descartes
async fn create_session(&self, name: &str, path: &Path) -> Result<Session, SessionError> {
    // Create .descartes directory structure
    std::fs::create_dir_all(path)?;
    std::fs::create_dir_all(path.join(".descartes"))?;
    std::fs::create_dir_all(path.join(".descartes/sessions"))?;
    std::fs::create_dir_all(path.join("data"))?;
    std::fs::create_dir_all(path.join("thoughts"))?;
    std::fs::create_dir_all(path.join("logs"))?;

    // ... rest of session creation
}
```

#### 3. Update save_session_metadata
**File**: `descartes/core/src/session_manager.rs`
**Changes**: Save to `.descartes/` instead of `.scud/`

```rust
// Line 143-156
fn save_session_metadata(&self, session: &Session) -> Result<(), SessionError> {
    let descartes_dir = session.descartes_path();  // Changed from scud_path()
    if !descartes_dir.exists() {
        std::fs::create_dir_all(&descartes_dir)?;
    }
    // ... rest unchanged
}
```

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test -p descartes-core`
- [x] Build succeeds: `cargo build -p descartes-cli`
- [x] Existing `.scud/` workspaces are still discovered (backward compat)

#### Manual Verification:
- [ ] `descartes init --name test-project` creates `.descartes/` directory
- [ ] `descartes ps` discovers both old `.scud/` and new `.descartes/` sessions

---

## Phase 2: Implement Workflow Execution

### Overview
Replace the "print commands" behavior with actual workflow step execution.

### Changes Required:

#### 1. Add Workflow Executor Module
**File**: `descartes/core/src/workflow_executor.rs` (new file)
**Changes**: Create workflow execution engine

```rust
//! Workflow Executor for Descartes
//!
//! Executes workflow steps using the appropriate agents and providers.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::{
    get_system_prompt, get_tools, Message, MessageRole, ModelBackend, ModelRequest,
    ProviderFactory, ThoughtsStorage, ToolLevel, WorkflowContext, WorkflowStep,
};

/// Result of executing a single workflow step
#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub step_name: String,
    pub success: bool,
    pub output: String,
    pub saved_to: Option<PathBuf>,
    pub duration_ms: u64,
}

/// Workflow executor configuration
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

/// Execute a workflow step
pub async fn execute_step(
    step: &WorkflowStep,
    task: &str,
    context: &WorkflowContext,
    backend: &dyn ModelBackend,
    config: &WorkflowExecutorConfig,
) -> Result<StepExecutionResult, WorkflowError> {
    let start = std::time::Instant::now();

    // Load agent definition for tool level and system prompt
    let agent_def = context.agent_loader.load_agent(&step.agent)?;

    // Get tools for this agent's level
    let tools = get_tools(agent_def.tool_level);

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
        system_prompt: Some(agent_def.system_prompt),
        tools: Some(tools),
    };

    // Execute
    let response = backend.complete(request).await?;

    // Save output if configured
    let saved_to = if config.save_outputs {
        if let Some(output_path) = &step.output {
            let content = format!("# {}\n\n{}", step.name, response.content);
            let saved = context.thoughts.save_research(output_path, &content)?;
            Some(saved)
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
    })
}

/// Execute multiple steps, respecting parallel flags
pub async fn execute_workflow(
    steps: Vec<(WorkflowStep, String)>,
    context: &WorkflowContext,
    backend: Arc<dyn ModelBackend>,
    config: &WorkflowExecutorConfig,
) -> Result<Vec<StepExecutionResult>, WorkflowError> {
    let mut results = Vec::new();
    let semaphore = Arc::new(Semaphore::new(config.max_parallel));

    let mut i = 0;
    while i < steps.len() {
        // Find consecutive parallel steps
        let mut parallel_batch = vec![&steps[i]];
        let mut j = i + 1;
        while j < steps.len() && steps[j].0.parallel {
            parallel_batch.push(&steps[j]);
            j += 1;
        }

        if parallel_batch.len() == 1 {
            // Execute sequentially
            let (step, task) = &steps[i];
            let result = execute_step(step, task, context, backend.as_ref(), config).await?;
            results.push(result);
            i += 1;
        } else {
            // Execute in parallel
            let handles: Vec<_> = parallel_batch.iter().map(|(step, task)| {
                let step = step.clone();
                let task = task.clone();
                let ctx = context.clone();
                let backend = backend.clone();
                let config = config.clone();
                let sem = semaphore.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    execute_step(&step, &task, &ctx, backend.as_ref(), &config).await
                })
            }).collect();

            for handle in handles {
                let result = handle.await??;
                results.push(result);
            }

            i = j;
        }
    }

    Ok(results)
}
```

#### 2. Update lib.rs to export new module
**File**: `descartes/core/src/lib.rs`
**Changes**: Add module and re-exports

```rust
pub mod workflow_executor;

pub use workflow_executor::{
    execute_step, execute_workflow, StepExecutionResult, WorkflowExecutorConfig,
};
```

#### 3. Update CLI Workflow Command
**File**: `descartes/cli/src/commands/workflow.rs`
**Changes**: Call actual executor instead of printing commands

```rust
use descartes_core::{
    execute_workflow as run_workflow, get_workflow, list_workflows, prepare_workflow,
    WorkflowContext, WorkflowExecutorConfig, ProviderFactory,
};

async fn execute_workflow(
    workflow_name: &str,
    topic: &str,
    context: Option<&str>,
    dir: Option<PathBuf>,
    config: &DescaratesConfig,
) -> Result<()> {
    println!();
    println!("{}", format!("┌─ Workflow: {} ─", workflow_name).cyan());
    println!();

    let workflow = get_workflow(workflow_name)
        .ok_or_else(|| anyhow::anyhow!("Workflow '{}' not found", workflow_name))?;

    println!("  {}", workflow.description.dimmed());
    println!("  Topic: {}", topic.yellow());

    let working_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut wf_context = WorkflowContext::new(working_dir.clone(), topic)?;
    if let Some(ctx) = context {
        wf_context = wf_context.with_context(ctx);
    }

    let prepared_steps = prepare_workflow(&workflow, &wf_context)?;

    println!("\n{}", "Executing workflow steps:".green().bold());

    // Create backend
    let provider = &config.providers.primary;
    let backend = create_backend(config, provider)?;

    let exec_config = WorkflowExecutorConfig {
        provider: provider.clone(),
        model: get_model_for_provider(config, provider, None)?,
        max_parallel: 3,
        save_outputs: true,
    };

    // Execute workflow
    let results = run_workflow(prepared_steps, &wf_context, Arc::new(backend), &exec_config).await?;

    // Display results
    println!("\n{}", "─".repeat(50).dimmed());
    println!("\n{}", "Workflow Results:".green().bold());

    for result in &results {
        let status = if result.success { "✓".green() } else { "✗".red() };
        println!("  {} {} ({}ms)", status, result.step_name, result.duration_ms);
        if let Some(path) = &result.saved_to {
            println!("    Output: {}", path.display().to_string().yellow());
        }
    }

    Ok(())
}
```

#### 4. Update CLI main.rs for config access
**File**: `descartes/cli/src/main.rs`
**Changes**: Pass config to workflow execute

```rust
Commands::Workflow(cmd) => {
    let config = load_config(args.config.as_deref())?;
    workflow::execute(&cmd, &config).await?;
}
```

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test -p descartes-core`
- [x] New workflow_executor tests pass
- [x] Build succeeds: `cargo build -p descartes-cli`

#### Manual Verification:
- [ ] `descartes workflow research --topic "test topic"` executes agents
- [ ] Output files are created in `~/.descartes/thoughts/research/`
- [ ] Parallel steps execute concurrently (visible in logs)

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: SCUD CLI Plugin Integration

### Overview
Ensure SCUD CLI can work alongside Descartes as an external tool/plugin.

### Changes Required:

#### 1. Add SCUD Plugin Module
**File**: `descartes/core/src/scud_plugin.rs` (new file)
**Changes**: SCUD CLI integration helpers

```rust
//! SCUD CLI Plugin Integration
//!
//! Allows the external SCUD CLI tool to work alongside Descartes.
//! SCUD manages its own state in `.scud/` while Descartes uses `.descartes/`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if SCUD CLI is available
pub fn scud_available() -> bool {
    Command::new("scud")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the SCUD directory for a workspace
pub fn scud_dir(workspace: &Path) -> PathBuf {
    workspace.join(".scud")
}

/// Check if a workspace has SCUD initialized
pub fn has_scud(workspace: &Path) -> bool {
    scud_dir(workspace).exists()
}

/// Sync Descartes tasks to SCUD (write-only plugin)
pub fn sync_tasks_to_scud(workspace: &Path, tasks_json: &str) -> std::io::Result<()> {
    let scud_tasks = scud_dir(workspace).join("tasks/tasks.json");
    if scud_tasks.parent().map_or(false, |p| p.exists()) {
        std::fs::write(scud_tasks, tasks_json)?;
    }
    Ok(())
}

/// Read SCUD workflow state (read-only)
pub fn read_scud_workflow_state(workspace: &Path) -> Option<String> {
    let state_file = scud_dir(workspace).join("workflow-state.json");
    std::fs::read_to_string(state_file).ok()
}
```

#### 2. Update Session Discovery
**File**: `descartes/core/src/session_manager.rs`
**Changes**: Report SCUD status alongside Descartes

```rust
// In load_session_from_path, add SCUD status detection
let has_scud = path.join(".scud").exists();
// Can add to Session struct if needed for UI display
```

### Success Criteria:

#### Automated Verification:
- [ ] Build succeeds: `cargo build -p descartes-cli`
- [ ] SCUD plugin module compiles without errors

#### Manual Verification:
- [ ] SCUD CLI (if installed) can still read/write to `.scud/` directory
- [ ] Descartes and SCUD can coexist in the same project
- [ ] `descartes ps` shows both Descartes and SCUD-managed workspaces

---

## Phase 4: Headless CLI Adapter Support

### Overview
Ensure workflows can be executed via any headless CLI adapter (Claude Code CLI, OpenCode, etc.).

### Changes Required:

#### 1. Update HeadlessCliAdapter
**File**: `descartes/core/src/providers/headless_cli.rs`
**Changes**: Ensure workflow execution works with CLI adapters

The existing HeadlessCliAdapter should already support this, but we need to verify:
- Task passing works correctly
- Tool definitions are passed to the CLI
- Output is captured properly

#### 2. Add CLI Adapter Factory
**File**: `descartes/cli/src/commands/workflow.rs`
**Changes**: Support `--adapter` flag for CLI execution

```rust
#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    Research {
        // ... existing fields ...

        /// Use a headless CLI adapter (claude-code, opencode)
        #[arg(long)]
        adapter: Option<String>,
    },
    // ...
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Build succeeds: `cargo build -p descartes-cli`
- [ ] Headless CLI adapter tests pass

#### Manual Verification:
- [ ] `descartes workflow research --adapter claude-code --topic "test"` works
- [ ] Workflow outputs are captured correctly

---

## Testing Strategy

### Unit Tests:
- Session path methods return correct `.descartes/` paths
- Workflow executor handles sequential and parallel steps
- SCUD plugin detection works correctly

### Integration Tests:
- End-to-end workflow execution with mock provider
- Session creation/discovery with new directory structure
- Backward compatibility with existing `.scud/` workspaces

### Manual Testing Steps:
1. Create new project with `descartes init`
2. Verify `.descartes/` directory structure
3. Run `descartes workflow research --topic "codebase structure"`
4. Verify output files in `~/.descartes/thoughts/research/`
5. Test with existing `.scud/` project for backward compatibility

## Performance Considerations

- Parallel workflow step execution uses semaphore to limit concurrency
- Default max_parallel of 3 prevents API rate limiting
- Streaming responses where supported for faster feedback

## Migration Notes

- Existing `.scud/` workspaces will continue to work (backward compat)
- New workspaces will use `.descartes/` directory
- Optional migration command could be added later: `descartes migrate-to-new-format`

## References

- Session module: `descartes/core/src/session.rs`
- Session manager: `descartes/core/src/session_manager.rs`
- Workflow commands: `descartes/core/src/workflow_commands.rs`
- CLI workflow: `descartes/cli/src/commands/workflow.rs`
- Spawn command: `descartes/cli/src/commands/spawn.rs`
