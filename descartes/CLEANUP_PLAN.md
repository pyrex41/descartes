# Plan: Descartes Cleanup and OpenCode Completion

## Overview
Three-phase cleanup: (1) Remove dead workflow module and dependencies, (2) Delete archive directory (preserved in git), (3) Complete OpenCode harness for non-Claude agent execution.

## Current State Analysis

**workflow/ module** (~72KB, 6 files):
- `config.rs` (11KB) - WorkflowConfig, GateConfig, TransitionConfig
- `gate.rs` (11KB) - CliGate, GateController
- `notify.rs` (15KB) - Telegram, Slack, Desktop notifications
- `runner.rs` (17KB) - WorkflowRunner
- `state.rs` (15KB) - StateManager, WorkflowState
- `mod.rs` (2KB) - re-exports

**Files that import workflow:**
- `lib.rs:45,66` - module declaration and re-exports
- `main.rs:11-12,530,549-585,805+` - Workflow command handling
- `handoff/mod.rs:10` - uses `AutoContext, TransitionConfig`
- `interactive/session.rs:16` - uses `WorkflowConfig`

**archive/** (6.1MB):
- `descartes-v1/` - 40,000+ lines of abandoned v1 code
- Confirmed in git history: `fbc532f refactor: archive descartes v1`

**OpenCode harness** (`src/harness/opencode.rs`):
- 577 lines, structurally complete
- Uses Unix socket IPC protocol
- Missing: actual OpenCode protocol verification (IPC format is speculative)

## Desired End State

1. No `workflow/` module - ~72KB removed
2. No `archive/` directory - ~6.1MB removed
3. Working OpenCode harness that can drive opencode agents
4. Clean `cargo build` and `cargo test`

## Implementation Approach

Remove workflow first (most invasive), then archive (trivial), then complete OpenCode (additive).

## Phases

### Phase 1: Remove Workflow Module
**Goal**: Delete workflow/ and all references to it

**Changes**:
- [ ] Delete `src/workflow/` directory
- [ ] Edit `src/lib.rs:45` - remove `pub mod workflow;`
- [ ] Edit `src/lib.rs:66` - remove `pub use workflow::{...};`
- [ ] Edit `src/main.rs:11-12` - remove workflow imports
- [ ] Edit `src/main.rs` - remove `Commands::Workflow` variant (~lines 239-311)
- [ ] Edit `src/main.rs` - remove workflow match arm (~line 530)
- [ ] Edit `src/main.rs` - remove Handoff command's workflow usage (~lines 549-555)
- [ ] Edit `src/main.rs` - simplify Interactive command (~lines 565-585)
- [ ] Edit `src/main.rs` - delete `handle_workflow_command` function (~lines 805-900)
- [ ] Edit `src/main.rs` - delete `load_workflow_config` function
- [ ] Edit `src/handoff/mod.rs:10` - remove workflow import, inline or delete `AutoContext`/`TransitionConfig`
- [ ] Edit `src/interactive/session.rs:16` - remove `WorkflowConfig` usage

**Success Criteria - Automated**:
- [ ] `cargo check` passes with no workflow references
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes

**Success Criteria - Manual**:
- [ ] `descartes --help` shows no workflow commands
- [ ] `descartes ralph --help` still works

### Phase 2: Delete Archive
**Goal**: Remove archive/ directory (already in git history)

**Changes**:
- [ ] Verify archive is in git: `git log --oneline archive/ | head -1`
- [ ] Delete `archive/` directory: `rm -rf archive/`

**Success Criteria - Automated**:
- [ ] `ls archive/` returns "No such file or directory"
- [ ] `git log --oneline --all -- archive/ | head -1` shows commit exists

**Success Criteria - Manual**:
- [ ] Can recover with `git checkout fbc532f -- archive/` if ever needed

### Phase 3: Complete OpenCode Harness
**Goal**: Make OpenCode harness functional for non-Claude agents

**OpenCode has two interfaces:**

1. **CLI mode**: `opencode run --format json "prompt"` - streaming JSON output
2. **Server mode**: `opencode serve` on port 4096, REST API with sessions

**Recommended: CLI mode** (simpler, matches ClaudeCode pattern)

```bash
# CLI invocation pattern
opencode run --format json --model "anthropic/claude-sonnet" "Your prompt"

# With session persistence
opencode run --format json --session <id> "Follow-up prompt"

# Output: streaming JSON events
```

**Changes**:
- [ ] Rewrite `src/harness/opencode.rs` to use CLI invocation (delete Unix socket code)
- [ ] Update struct to match ClaudeCode pattern:
  ```rust
  pub struct OpenCodeHarness {
      binary: String,      // "opencode"
      model: String,       // "anthropic/claude-sonnet" or "openai/gpt-4"
      sessions: Arc<Mutex<HashMap<String, OpenCodeSession>>>,
  }
  ```
- [ ] Implement `execute_opencode()` similar to `ClaudeCodeHarness::execute_claude()`
- [ ] Parse JSON streaming output (format TBD from `--format json`)
- [ ] Update `src/config.rs:OpenCodeConfig`:
  ```rust
  pub struct OpenCodeConfig {
      pub binary: Option<String>,      // default: "opencode"
      pub model: Option<String>,       // default: "anthropic/claude-sonnet"
      pub server_url: Option<String>,  // optional: for server mode
  }
  ```
- [ ] Add test for JSON output parsing

**Alternative: Server mode** (if CLI doesn't support streaming well)
- Start server: `opencode serve --port 4096`
- Use HTTP client to POST to `/session/:id/message`
- SSE stream from `/event` for real-time output

**Success Criteria - Automated**:
- [ ] `cargo check` passes
- [ ] `cargo test harness::opencode` passes
- [ ] JSON parsing test works

**Success Criteria - Manual**:
- [ ] `descartes ralph --harness opencode` executes a task
- [ ] Output streams correctly to TUI

## Open Questions

None - all resolved:
1. Workflow removal: straightforward deletion
2. Archive removal: trivial, git preserves history
3. OpenCode: **CLI mode confirmed** - `opencode run --format json` matches ClaudeCode pattern

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Workflow removal breaks something hidden | Run full test suite; grep for any missed references |
| Archive accidentally needed | Git history preserves it; can checkout from `fbc532f` |
| OpenCode JSON format unknown | Test with actual output; fallback to server mode if needed |
| OpenCode not installed on user's system | Graceful error: "OpenCode binary not found" |

## Estimated Scope

| Phase | Files Changed | Lines Removed | Lines Added |
|-------|---------------|---------------|-------------|
| 1: Workflow | 5 files | ~1,200 | 0 |
| 2: Archive | 1 directory | ~40,000 | 0 |
| 3: OpenCode | 2-3 files | ~100 | ~150 |

**Net effect**: -41,000+ lines, cleaner codebase, working OpenCode support.
