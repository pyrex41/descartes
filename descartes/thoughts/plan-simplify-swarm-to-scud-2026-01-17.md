# Plan: Simplify Descartes Swarm to Use SCUD

## Overview

Replace Descartes's headless swarm executor with a thin wrapper that writes spec context to SCUD's guidance system and delegates execution to `scud swarm` or `scud spawn`. This gives us terminal visibility, working attach, and live output capture without reimplementing SCUD's proven infrastructure.

## Current State Analysis

**Descartes swarm executor** (`descartes/src/swarm_executor.rs`, 1432 lines):
- Reimplements wave computation (SCUD already has this)
- Headless API-based execution (no terminal visibility)
- Broken TUI attach (no real panes exist)
- Transcripts created but never saved
- Validation calls SCUD's `run_validation()` anyway

**SCUD already provides** (`scud/scud-cli/src/commands/`):
- Terminal spawning for Kitty, WezTerm, iTerm2, Zellij, tmux
- Wave-based orchestration with backpressure validation
- Live TUI with `tmux capture-pane` output
- Session tracking and locking
- Guidance system (`.scud/guidance/*.md` auto-loaded into prompts)
- Multiple harness support (claude, opencode) with agent assignment on graph

**Descartes unique value**:
- Spec building from `--plan` and `--spec-file`
- PRD processing pipeline (already shells out to SCUD)
- Future: streaming infrastructure for GUI/web

## Desired End State

```bash
# User runs:
descartes swarm --scud-tag transcript --plan thoughts/plan.md

# Descartes:
# 1. Builds spec from plan file
# 2. Writes to .scud/guidance/descartes-spec.md
# 3. Calls: scud swarm --tag transcript --harness claude
# 4. User sees agents in real terminal windows with live output
```

## Implementation Approach

Keep Descartes CLI interface, but delegate execution to SCUD. The SwarmExecutor becomes a "spec builder + SCUD caller" rather than its own orchestrator.

---

## Phases

### Phase 1: Spec-to-Guidance Writer

**Goal**: Build spec from Descartes flags and write to SCUD guidance directory.

**Changes**:
- [x] Create `write_spec_to_guidance()` function (`descartes/src/spec.rs`)
  - Takes `SpecConfig`, `working_dir`
  - Builds spec content via existing `build_task_spec()` logic (without task)
  - Writes to `.scud/guidance/descartes-spec.md`
  - Returns `Result<PathBuf>` with path written

- [x] Add `build_general_spec()` to `spec.rs` (around line 180)
  - Similar to `build_task_spec()` but without task-specific content
  - Combines plan file + spec files into guidance document
  - Respects `max_spec_tokens` limit

**Success Criteria - Automated**:
- [x] `cargo build --release` passes
- [x] `cargo test` passes
- [x] Unit test: `write_spec_to_guidance()` creates file at expected path

**Success Criteria - Manual**:
- [x] Running with `--plan` creates `.scud/guidance/descartes-spec.md`
- [x] Content includes plan file content with proper formatting

---

### Phase 2: SCUD Swarm Delegation

**Goal**: Replace SwarmExecutor::run() with subprocess call to `scud swarm`.

**Changes**:
- [x] Add `--use-scud` flag to swarm command (`main.rs:128-196`)
  - Default: true (use SCUD)
  - `--no-use-scud` falls back to old executor (for transition)

- [x] Create `run_scud_swarm()` function (`main.rs` or new `scud_bridge.rs`)
  - Maps Descartes flags to SCUD flags:
    - `--scud-tag` → `--tag`
    - `--round-size` → `--round-size` (same)
    - `--harness` → `--harness` (map: `claude-code`→`claude`, `opencode`→`opencode`)
    - `--no-validate` → `--no-validate`
    - `--dry-run` → `--dry-run`
  - Calls `std::process::Command::new("scud").args([...]).status()`

- [x] Update swarm command handler (`main.rs:408-531`)
  - Call `write_spec_to_guidance()` before execution
  - If `--use-scud`: call `run_scud_swarm()`
  - Else: use existing SwarmExecutor (deprecated path)

**Flag Mapping Table**:
| Descartes | SCUD swarm | Notes |
|-----------|------------|-------|
| `--scud-tag` | `--tag` | Direct mapping |
| `--round-size` | `--round-size` | Same meaning |
| `--harness claude-code` | `--harness claude` | Name translation |
| `--harness opencode` | `--harness opencode` | Direct |
| `--no-validate` | `--no-validate` | Same |
| `--dry-run` | `--dry-run` | Same |
| `--verify` | N/A | SCUD uses backpressure config |
| `--model` | N/A | SCUD uses agent definitions |
| `--plan` | N/A | Written to guidance |
| `--spec-file` | N/A | Written to guidance |

**Success Criteria - Automated**:
- [x] `cargo build --release` passes
- [x] `cargo test` passes

**Success Criteria - Manual**:
- [x] `descartes swarm --scud-tag test --plan plan.md --dry-run` delegates to SCUD correctly
- [ ] Full execution spawns visible terminals (requires interactive testing)
- [ ] Can switch to tmux windows and see agent output (requires interactive testing)
- [ ] Validation runs after waves complete (requires interactive testing)
- [ ] Tasks marked done/failed in SCUD (requires interactive testing)

---

### Phase 3: SCUD Spawn Support

**Goal**: Add `descartes scud-spawn` command as thin wrapper around `scud spawn`.

**Changes**:
- [x] Add `ScudSpawn` command variant to CLI (`main.rs`)
  ```rust
  ScudSpawn {
      #[arg(long)]
      scud_tag: Option<String>,
      #[arg(long)]
      plan: Option<PathBuf>,
      #[arg(long = "spec-file", action = ArgAction::Append)]
      spec_files: Vec<PathBuf>,
      #[arg(short = 'n', long, default_value = "5")]
      limit: usize,
      #[arg(long, default_value = "claude-code")]
      harness: String,
      #[arg(long, default_value = "true")]
      monitor: bool,
      #[arg(long)]
      no_monitor: bool,
  }
  ```

- [x] Create `run_scud_spawn()` function
  - Maps flags to `scud spawn` arguments
  - Always passes `--monitor` unless `--no-monitor` specified
  - Always passes `--claim` (mark tasks in-progress)

**Success Criteria - Automated**:
- [x] `cargo build --release` passes
- [x] `cargo test` passes
- [x] `cargo clippy -- -D warnings` passes

**Success Criteria - Manual**:
- [x] `descartes scud-spawn --scud-tag test --plan plan.md --dry-run` delegates to SCUD correctly
- [x] `--monitor` flag is passed by default (verified in dry-run output)
- [x] `descartes scud-spawn --no-monitor --dry-run` omits --monitor flag
- [ ] Full execution spawns terminals (requires interactive testing)

---

### Phase 4: Streaming Infrastructure (Future GUI Support)

**Goal**: Add file-based streaming so future GUI/web can read agent output.

**Changes**:
- [ ] Create `.descartes/streams/` directory structure
- [ ] For each spawned agent, write output to `<run-id>/<task-id>.log`
- [ ] Use append-only writes so `tail -f` works
- [ ] Add metadata file `<run-id>/manifest.json` with task list and status

**Implementation Options** (choose one):
1. **Hook into SCUD** - Add `--stream-dir` flag to SCUD spawn/swarm
2. **Capture tmux output** - Periodic `tmux capture-pane` to files
3. **Post-hoc** - Convert SCUD session JSON + transcripts to stream format

**Success Criteria - Automated**:
- [ ] Stream files created during execution

**Success Criteria - Manual**:
- [ ] `tail -f .descartes/streams/<run>/<task>.log` shows live output
- [ ] Multiple consumers can read simultaneously

---

### Phase 5: Deprecate Old Executor

**Goal**: Remove unused headless execution code.

**Changes**:
- [ ] Remove `--no-use-scud` flag (remove fallback)
- [ ] Mark `SwarmExecutor` as deprecated or remove
- [ ] Remove `swarm_tui.rs` (SCUD provides TUI)
- [ ] Update documentation

**Files to potentially remove/deprecate**:
- `descartes/src/swarm_executor.rs` (1432 lines)
- `descartes/src/swarm_tui.rs` (~520 lines)
- Related tests

**Success Criteria - Automated**:
- [ ] `cargo build --release` passes
- [ ] `cargo test` passes
- [ ] No unused code warnings

**Success Criteria - Manual**:
- [ ] All swarm functionality works via SCUD
- [ ] Documentation updated

---

## Open Questions

*None - all questions resolved during research.*

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| SCUD CLI not in PATH | Check and provide helpful error message |
| SCUD version incompatibility | Pin to known-good scud-cli version in Cargo.toml |
| Guidance file conflicts | Use distinctive filename (`descartes-spec.md`) |
| Loss of context handoff feature | Document as future SCUD enhancement opportunity |
| Users depend on old behavior | Keep `--no-use-scud` through deprecation period |

## Migration Path

1. **Phase 1-2**: Add SCUD delegation as default, keep old executor as fallback
2. **Phase 3**: Add spawn command with monitor default
3. **Phase 4**: Add streaming for GUI development
4. **Phase 5**: After 1-2 releases, remove fallback

## Code Size Impact

**Removed**: ~2000 lines (swarm_executor.rs + swarm_tui.rs)
**Added**: ~200 lines (spec-to-guidance + SCUD bridge)
**Net**: ~1800 lines removed

## Dependencies

- SCUD CLI must be installed and in PATH
- SCUD version 1.36+ (current dependency version)
- Terminal multiplexer (tmux recommended) for visibility
