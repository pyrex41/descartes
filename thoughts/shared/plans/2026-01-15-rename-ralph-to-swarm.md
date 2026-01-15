# Rename Ralph to Swarm Implementation Plan

## Overview

Remove the `loop`, `run`, and `plan` CLI commands (which use the BAML-driven `ralph_loop.rs`) and rename the `ralph` command to `swarm`. Keep BAML scaffolding in place for potential future use. Update all references to use "Swarm" terminology while preserving documentation references to "Ralph Wiggum" as the inspirational pattern.

## Current State Analysis

The codebase has two orchestration systems:
1. **`loop`/`run`/`plan` commands** - BAML-driven via `ralph_loop.rs` (being removed)
2. **`ralph` command** - SCUD-integrated via `ralph_executor.rs` (being renamed to `swarm`)

### Key Discoveries:
- `ralph_loop.rs` (801 lines) handles loop/run/plan commands
- `ralph_executor.rs` (1337 lines) handles the ralph command
- `ralph_tui.rs` (520 lines) provides TUI visualization
- Config has `RalphLoopConfig` struct and `[ralph_loop]` TOML section
- Tests in `ralph_integration.rs` and `ralph_e2e.rs`
- Slash commands in `.claude/commands/rw/` directory

## Desired End State

After implementation:
- `descartes swarm --scud-tag <tag>` replaces `descartes ralph --scud-tag <tag>`
- No `loop`, `run`, or `plan` commands exist
- All source files use "swarm" naming (SwarmExecutor, SwarmTui, etc.)
- Config uses `[swarm]` section
- Documentation explains Swarm as inspired by Ralph Wiggum principles
- BAML files remain in place (unused but available)

### Verification:
```bash
# Command works
descartes swarm --help

# Old commands removed
descartes loop 2>&1 | grep -q "error" && echo "loop removed"
descartes run 2>&1 | grep -q "error" && echo "run removed"
descartes ralph 2>&1 | grep -q "error" && echo "ralph removed"

# Tests pass
cargo test
```

## What We're NOT Doing

- NOT removing `ralph_loop.rs` entirely (keeping BAML scaffolding)
- NOT removing BAML files (`orchestrator.baml`, `planning.baml`, etc.)
- NOT changing the fundamental architecture or behavior
- NOT updating archived SCUD tasks or research documents (historical record)

## Implementation Approach

Incremental changes with verification at each phase. Source code changes first, then tests, then documentation.

---

## Phase 1: Remove loop/run/plan Commands from CLI

### Overview
Remove the three command variants that use `ralph_loop.rs` from the CLI, but keep the module for BAML scaffolding.

### Changes Required:

#### 1.1 Remove Command Enum Variants

**File**: `descartes/src/main.rs`
**Changes**: Remove Loop, Run, Plan from Commands enum and their handlers

Remove lines 31-47 (command definitions):
```rust
// DELETE: Commands::Loop variant (lines 32-41)
// DELETE: Commands::Run variant (lines 43-44)
// DELETE: Commands::Plan variant (lines 46-47)
```

Remove lines 236-272 (command handlers):
```rust
// DELETE: Commands::Loop handler (lines 237-252)
// DELETE: Commands::Run handler (lines 254-262)
// DELETE: Commands::Plan handler (lines 264-272)
```

Remove import on line 11:
```rust
// CHANGE: Remove LoopConfig, LoopMode from import
use descartes::{Config, Result};  // Remove LoopConfig, LoopMode
```

#### 1.2 Keep ralph_loop.rs Module (BAML scaffolding)

**File**: `descartes/src/lib.rs`
**Changes**: Keep module declaration but remove public exports

```rust
// KEEP: Line 40 - pub mod ralph_loop;  (internal use only)

// REMOVE from line 58:
// pub use ralph_loop::{LoopConfig, LoopMode};
```

### Success Criteria:

#### Automated Verification:
- [x] Project compiles: `cargo build`
- [x] Old commands not available: `cargo run -- loop 2>&1 | grep -q "unrecognized"`
- [x] Old commands not available: `cargo run -- run 2>&1 | grep -q "unrecognized"`
- [x] Old commands not available: `cargo run -- plan 2>&1 | grep -q "unrecognized"`
- [x] Ralph command still works: `cargo run -- ralph --help`

#### Manual Verification:
- [x] Confirm `descartes --help` no longer shows loop/run/plan commands

---

## Phase 2: Rename Ralph to Swarm in Source Files

### Overview
Rename source files, structs, and internal references from "ralph" to "swarm".

### Changes Required:

#### 2.1 Rename Source Files

```bash
# In descartes/src/
mv ralph_executor.rs swarm_executor.rs
mv ralph_tui.rs swarm_tui.rs
# Keep ralph_loop.rs as-is (BAML scaffolding)
```

#### 2.2 Update swarm_executor.rs (formerly ralph_executor.rs)

**File**: `descartes/src/swarm_executor.rs`
**Changes**: Update module doc, struct names, and internal references

```rust
// Line 1-10: Update module doc
//! Swarm executor
//!
//! Implements fresh-context-per-task execution inspired by the Ralph Wiggum pattern:
//! 1. Load spec sources (task + plan + custom files)
//! ...

// Line 44: Rename struct
pub struct SwarmExecutor {
    // ... fields unchanged
}

// Line 69: Update impl block
impl SwarmExecutor {
    // ... methods unchanged
}

// Line 130: Update output text
println!("=== Swarm Execution Plan ===");

// Line 187-189: Update logging
info!(
    "Starting Swarm loop for tag '{}' in {:?}",
    ...
);

// Line 222: Update output
println!(
    "Swarm loop for tag '{}' - {} wave(s), {} task(s)",
    ...
);

// Line 278: Update pane naming
let pane_name = format!("swarm-{}", task.id);

// Line 462: Update comment
/// This implements the core fresh-context-per-task pattern:
```

#### 2.3 Update swarm_tui.rs (formerly ralph_tui.rs)

**File**: `descartes/src/swarm_tui.rs`
**Changes**: Update module doc, struct names, and display text

```rust
// Line 1-4: Update module doc
//! Swarm orchestrator TUI
//!
//! Terminal UI for monitoring wave progress and agent status.

// Line 87: Rename struct
pub struct SwarmTui {
    // ... fields unchanged
}

// Line 98: Update impl
impl SwarmTui {
    // ... methods unchanged
}

// Line 270: Update display title
Print("=== Swarm Orchestrator ===\n"),

// Line 402: Update Default impl
impl Default for SwarmTui {

// Line 421: Rename function
pub fn create_tui_from_config(config: &Config) -> Result<SwarmTui> {
```

#### 2.4 Update lib.rs Exports

**File**: `descartes/src/lib.rs`
**Changes**: Update module declarations and exports

```rust
// Line 5: Update doc
//! Swarm: Fresh-context-per-task orchestration

// Line 16: Update ASCII art comment
//! Swarm Loop (outer)

// Lines 39-41: Update module declarations
pub mod swarm_executor;
pub mod ralph_loop;  // Keep for BAML scaffolding
pub mod swarm_tui;

// Lines 57-59: Update exports
pub use swarm_executor::SwarmExecutor;
pub use swarm_tui::{SwarmTui, create_tui_from_config};
// Remove: pub use ralph_loop::{LoopConfig, LoopMode};
```

#### 2.5 Update main.rs Command

**File**: `descartes/src/main.rs`
**Changes**: Rename Ralph command to Swarm

```rust
// Line 3: Update module doc
//! Visible subagent orchestration with Swarm loops.

// Line 116-117: Rename command
/// Run Swarm loop for SCUD tasks
Swarm {
    // ... all fields unchanged
}

// Line 419: Update match arm
Commands::Swarm {
    // ... unchanged
} => {
    // Line 447: Update default tag
    let tag_name = tag.unwrap_or_else(|| {
        prd_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "swarm".to_string())  // Changed from "ralph"
    });

    // Line 526-527: Update executor creation
    let executor = descartes::SwarmExecutor::new(
        // ... unchanged params
    )?;
}
```

#### 2.6 Update spec.rs Prompt

**File**: `descartes/src/spec.rs`
**Changes**: Update prompt text

```rust
// Line 331: Update prompt
format!(
    r#"You are implementing SCUD task {} for tag '{}' using the Swarm technique.
    ...
```

#### 2.7 Update Internal References in ralph_loop.rs

**File**: `descartes/src/ralph_loop.rs`
**Changes**: Update imports to use new module names (keep file as BAML scaffolding)

```rust
// Line 24: Update import if it references ralph_tui
use crate::swarm_tui::SwarmTui;  // If applicable
```

### Success Criteria:

#### Automated Verification:
- [x] Project compiles: `cargo build`
- [x] Swarm command available: `cargo run -- swarm --help`
- [x] Ralph command removed: `cargo run -- ralph 2>&1 | grep -q "unrecognized"`
- [x] No remaining "Ralph" in public API: `grep -r "pub.*Ralph" src/ | grep -v ralph_loop` (only RalphLoopConfig remains, addressed in Phase 3)

#### Manual Verification:
- [x] `descartes --help` shows `swarm` command with correct description

---

## Phase 3: Update Configuration

### Overview
Rename configuration struct and TOML section from `ralph_loop` to `swarm`.

### Changes Required:

#### 3.1 Update config.rs

**File**: `descartes/src/config.rs`
**Changes**: Rename struct and field

```rust
// Line 22: Rename field
pub swarm: SwarmConfig,

// Lines 43-59: Rename struct
/// Swarm orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Whether to try fast-builder first for applicable tasks
    #[serde(default)]
    pub use_fast_first: bool,

    /// Whether to always review fast-builder changes
    #[serde(default)]
    pub always_review: bool,

    /// Heuristic for orchestration decisions
    #[serde(default = "default_heuristic")]
    pub heuristic: String,
}

// Line 61-68: Update Default impl
impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            use_fast_first: true,
            always_review: false,
            heuristic: default_heuristic(),
        }
    }
}

// Line 203: Update instantiation
swarm: SwarmConfig::default(),
```

#### 3.2 Update ralph_loop.rs Config References

**File**: `descartes/src/ralph_loop.rs`
**Changes**: Update config field references

```rust
// Line 383: Update reference
config.swarm.heuristic

// Line 406: Update reference
config.swarm.use_fast_first

// Line 438: Update reference
config.swarm.always_review

// Line 448: Update reference
config.swarm.heuristic
```

#### 3.3 Update Example Config File

**File**: `descartes/.descartes/config.toml`
**Changes**: Rename section

```toml
# Line 61: Rename section
[swarm]
use_fast_first = true
always_review = false
heuristic = "prefer_speed"
```

### Success Criteria:

#### Automated Verification:
- [x] Project compiles: `cargo build`
- [x] Config loads correctly: `cargo run -- config`
- [x] No "ralph_loop" in config output: `cargo run -- config | grep -v ralph`

#### Manual Verification:
- [x] Config file section correctly named `[swarm]`

---

## Phase 4: Update Tests

### Overview
Rename test files and update test code to use new names.

### Changes Required:

#### 4.1 Rename Test Files

```bash
# In descartes/tests/
mv ralph_integration.rs swarm_integration.rs

# In descartes/tests/e2e/
mv ralph_e2e.rs swarm_e2e.rs
```

#### 4.2 Update swarm_integration.rs

**File**: `descartes/tests/swarm_integration.rs`
**Changes**: Update imports, struct names, and test names

```rust
// Line 1-4: Update module doc
//! Integration tests for Swarm executor

// Line 11: Update import
use descartes::SwarmExecutor;

// All test functions: rename from test_ralph_* to test_swarm_*
// All SwarmExecutor::new() calls are already correct after Phase 2
```

#### 4.3 Update e2e/mod.rs

**File**: `descartes/tests/e2e/mod.rs`
**Changes**: Update module reference

```rust
// Line 3: Update doc
//! full Swarm loop with mock harnesses

// Line 8: Update module
mod swarm_e2e;
```

#### 4.4 Update swarm_e2e.rs

**File**: `descartes/tests/e2e/swarm_e2e.rs`
**Changes**: Update imports and test names

```rust
// Line 1: Update module doc
//! End-to-end tests for Swarm executor

// Line 8: Update import
use descartes::SwarmExecutor;

// Rename all test_ralph_* functions to test_swarm_*
```

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test`
- [x] Integration tests run: `cargo test --test swarm_integration`
- [x] E2E tests run: `cargo test --test e2e_tests`

#### Manual Verification:
- [x] No test failures related to naming

---

## Phase 5: Update Slash Commands

### Overview
Rename the slash command directory and update command files.

### Changes Required:

#### 5.1 Rename Directory

```bash
mv .claude/commands/rw .claude/commands/swarm
```

#### 5.2 Rename and Update cancel-ralph.md

**File**: `.claude/commands/swarm/cancel.md` (renamed from cancel-ralph.md)
**Changes**: Update content

```markdown
---
description: Cancel active Swarm loop
---

# Cancel Swarm Loop

...update all references from "Ralph" to "Swarm"...
```

#### 5.3 Update help.md

**File**: `.claude/commands/swarm/help.md`
**Changes**: Update content to explain Swarm (inspired by Ralph Wiggum)

```markdown
---
description: Explain Swarm technique and commands
---

# Swarm Help

Swarm is a fresh-context-per-task orchestration pattern inspired by the
Ralph Wiggum loop principles. Each task gets a clean context without
accumulated baggage from previous tasks.

...
```

#### 5.4 Update loop.md

**File**: `.claude/commands/swarm/loop.md`
**Changes**: Update content

```markdown
---
description: Start Swarm loop for SCUD tag
---

# Swarm Loop

...update command examples to use `descartes swarm`...
```

### Success Criteria:

#### Automated Verification:
- [x] Slash command directory exists: `ls .claude/commands/swarm/`
- [x] Old directory removed: `! ls .claude/commands/rw/ 2>/dev/null`

#### Manual Verification:
- [x] Slash commands work in Claude Code session

---

## Phase 6: Update Documentation

### Overview
Update all documentation files to use Swarm terminology while explaining Ralph Wiggum as the inspirational pattern.

### Changes Required:

#### 6.1 Rename and Update ralph-loop.md

**File**: `descartes/docs/swarm.md` (renamed from ralph-loop.md)
**Changes**: Complete rewrite with Swarm focus

```markdown
# Swarm Orchestration

Swarm is Descartes' fresh-context-per-task orchestration pattern, inspired by
the Ralph Wiggum loop principles.

## Background: The Ralph Wiggum Pattern

The pattern is named after the Simpsons character who famously lives in the
moment. The core principle: give each task a completely fresh context to
prevent drift, error accumulation, and hallucination creep.

## How Swarm Extends the Pattern

Swarm implements these principles with specific extensions:
- SCUD integration for DAG-based task management
- Wave computation using Kahn's algorithm
- Backpressure validation with failure tracking
- Context handoff when approaching token limits
- TUI visualization for monitoring

## Usage

```bash
descartes swarm --scud-tag my-feature
```

...rest of documentation...
```

#### 6.2 Update README.md

**File**: `descartes/README.md`
**Changes**: Update all command examples and descriptions

- Replace `descartes ralph` with `descartes swarm`
- Replace `Ralph Wiggum loop` with `Swarm` in product references
- Keep "inspired by Ralph Wiggum" in explanatory text
- Update ASCII art if it mentions "Ralph"

#### 6.3 Update getting-started.md

**File**: `descartes/docs/getting-started.md`
**Changes**: Update command examples and terminology

#### 6.4 Update configuration.md

**File**: `descartes/docs/configuration.md`
**Changes**: Update `[ralph_loop]` examples to `[swarm]`

#### 6.5 Update Cargo.toml Description

**File**: `descartes/Cargo.toml`
**Changes**: Update package description

```toml
description = "Visible subagent orchestration with Swarm loops"
```

#### 6.6 Update docs/README.md

**File**: `descartes/docs/README.md`
**Changes**: Update index and links

### Success Criteria:

#### Automated Verification:
- [x] No "descartes ralph" in docs: `! grep -r "descartes ralph" descartes/docs/`
- [x] Swarm documentation exists: `ls descartes/docs/swarm.md`
- [x] Old file removed: `! ls descartes/docs/ralph-loop.md 2>/dev/null`

#### Manual Verification:
- [x] Documentation reads coherently
- [x] Ralph Wiggum is referenced as inspiration, not product name
- [x] All command examples use `swarm`

---

## Testing Strategy

### Unit Tests:
- SwarmExecutor construction and configuration
- Wave computation algorithm
- Task result parsing (TASK_BLOCKED detection)

### Integration Tests:
- Full swarm execution with mock harness
- PRD parsing and task generation
- Backpressure validation flow

### Manual Testing Steps:
1. Run `descartes --help` and verify `swarm` appears, `loop`/`run`/`ralph` don't
2. Run `descartes swarm --help` and verify all options documented
3. Run `descartes swarm --dry-run --scud-tag test` with a test tag
4. Verify config file with `[swarm]` section loads correctly

## Migration Notes

Users with existing config files using `[ralph_loop]` will need to rename the section to `[swarm]`. The TOML parser will use defaults if the section is missing, so this is non-breaking.

## References

- Earlier research: `thoughts/shared/research/2025-01-15-ralph-vs-loop-commands.md`
- Ralph Wiggum pattern explanation: Original in `docs/swarm.md` after rename
