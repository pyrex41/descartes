# GitHub Pages Documentation Review

**Date**: 2026-01-15
**Scope**: Review of `/descartes/docs/` for consistency with actual codebase functionality
**E2E Test Coverage**: 267 tests (US-23 through US-50)

---

## Executive Summary

The documentation is **mostly accurate** but contains several **non-existent commands and flags** that should be fixed. Most documented features match the implementation, but a few aspirational features are documented that don't exist in code.

---

## Critical Issues (Must Fix)

### 1. `descartes run` Command Does Not Exist

**Location**: `docs/workflows.md:112`
```bash
# DOCUMENTED (WRONG):
descartes run --task TASK-001
```

**Reality**: There is no `run` subcommand. The actual CLI commands are:
- `swarm` - Execute tasks for a SCUD tag
- `spawn` - Spawn a single subagent
- `next` - Get next ready task
- `complete` - Mark task done

**Fix**: Replace with `descartes spawn builder "Implement TASK-001: ..."` or document proper workflow.

---

### 2. `--plan-only` and `--output` Flags Do Not Exist

**Location**: `docs/workflows.md:72-74`
```bash
# DOCUMENTED (WRONG):
descartes swarm \
    --prd ./docs/complex-feature.md \
    --tag complex \
    --plan-only \                    # DOES NOT EXIST
    --output ./docs/IMPLEMENTATION_PLAN.md  # DOES NOT EXIST
```

**Reality**: The `swarm` command does not support plan-only mode or output path. The actual flags are:
- `--dry-run` - Preview execution without running
- `--plan <path>` - Use existing plan document as context

**Fix**: Remove this workflow or implement the feature.

---

### 3. `--last` Flag for Transcripts Does Not Exist

**Location**: `docs/workflows.md:310-311`
```bash
# DOCUMENTED (WRONG):
descartes transcripts --last 5
```

**Reality**: The `transcripts` command only supports:
- `--today` - Show only today's transcripts
- `--session <id>` - Show only transcripts from session

**Fix**: Update documentation to use actual flags.

---

## Minor Issues

### 4. Default Harness Inconsistency

**Documentation states** (`docs/harnesses.md`): "OpenCode (Default)"

**Reality**: In `main.rs:145`, default is `claude-code`:
```rust
#[arg(long, env = "DESCARTES_HARNESS", default_value = "claude-code")]
harness: String,
```

In `config.rs:277`, default function returns `opencode`:
```rust
std::env::var("DESCARTES_HARNESS").unwrap_or_else(|_| "opencode".to_string())
```

**Status**: CLI default wins (`claude-code`), but config default is `opencode`. Documentation says OpenCode is default, which matches config but not CLI.

---

### 5. `orchestrator` Category Not in Enum

**Documentation** (`docs/configuration.md:115-116`): Lists `orchestrator` as a category

**Reality**: `AgentCategory` enum has:
```rust
Searcher, Analyzer, Builder, FastBuilder, BuilderReviewer, Validator, Planner, Custom(String)
```

The default config includes `orchestrator`, but it uses the `Custom("orchestrator")` variant since it's not in the enum.

**Status**: Works correctly (falls back to Custom), but could be confusing.

---

## Verified Correct

### CLI Commands (All Exist)

| Command | Status | Notes |
|---------|--------|-------|
| `descartes swarm` | CORRECT | Main execution loop |
| `descartes spawn <cat> <prompt>` | CORRECT | Manual subagent spawn |
| `descartes transcripts` | CORRECT | List transcripts |
| `descartes show <id>` | CORRECT | Show transcript |
| `descartes replay <id>` | CORRECT | Replay with timing |
| `descartes next` | CORRECT | Get next ready task |
| `descartes complete <id>` | CORRECT | Mark task done |
| `descartes waves` | CORRECT | Show execution waves |
| `descartes init` | CORRECT | Initialize .descartes |
| `descartes config` | CORRECT | Show configuration |
| `descartes harness` | CORRECT | Show active harness |
| `descartes interactive` | CORRECT | Start REPL session |
| `descartes skills` | CORRECT | Skill management |

### Swarm Command Flags (All Exist)

| Flag | Status |
|------|--------|
| `--scud-tag <tag>` | CORRECT |
| `--prd <path>` | CORRECT |
| `--num-tasks <n>` | CORRECT |
| `--tag <name>` | CORRECT |
| `--no-expand` | CORRECT |
| `--no-check-deps` | CORRECT |
| `--plan <path>` | CORRECT |
| `--spec-file <path>` | CORRECT |
| `--max-spec-tokens <n>` | CORRECT |
| `--verify <cmd>` | CORRECT |
| `--harness <name>` | CORRECT |
| `--model <name>` | CORRECT |
| `--round-size <n>` | CORRECT |
| `--no-validate` | CORRECT |
| `--dry-run` | CORRECT |
| `--working-dir <path>` | CORRECT |

### Configuration Sections (All Exist)

| Section | Status |
|---------|--------|
| `[harness]` | CORRECT |
| `[harness.claude_code]` | CORRECT |
| `[harness.opencode]` | CORRECT |
| `[harness.codex]` | CORRECT |
| `[categories.*]` | CORRECT |
| `[swarm]` | CORRECT |
| `[scud]` | CORRECT |
| `[transcripts]` | CORRECT |
| `[guidance]` | CORRECT (newly added) |

### Environment Variables (All Work)

| Variable | Status |
|----------|--------|
| `DESCARTES_HARNESS` | CORRECT |
| `DESCARTES_MODEL` | CORRECT |
| `DESCARTES_FAST_MODEL` | CORRECT |
| `DESCARTES_SMART_MODEL` | CORRECT |
| `DESCARTES_OPENCODE_MODEL` | CORRECT |
| `DESCARTES_CLAUDE_MODEL` | CORRECT |
| `DESCARTES_CODEX_MODEL` | CORRECT |

### Harness Implementations (All Exist)

| Harness | Status | Location |
|---------|--------|----------|
| ClaudeCode | CORRECT | `src/harness/claude_code.rs` |
| OpenCode | CORRECT | `src/harness/opencode.rs` |
| Codex | CORRECT | `src/harness/codex.rs` |

### Core Features (All Implemented)

| Feature | Status | Evidence |
|---------|--------|----------|
| Wave-based execution | CORRECT | `SwarmExecutor::compute_waves()` |
| Fresh context per task | CORRECT | New harness session per task |
| Backpressure validation | CORRECT | `run_validation()` method |
| Transcript recording | CORRECT | SCG format in `.descartes/transcripts/` |
| Dry-run mode | CORRECT | `dry_run()` method |
| Spec building (~5k tokens) | CORRECT | `build_prompt()` in `spec.rs` |
| User guidance injection | CORRECT | `GuidanceConfig` in `config.rs` |

---

## E2E Test Coverage

The test suite covers user stories US-23 through US-50:

| Module | User Stories | Tests |
|--------|--------------|-------|
| `single_agent` | US-23 to US-25 | Interactive sessions, task implementation |
| `swarm` | US-26 to US-28 | Wave execution, progress, categories |
| `context` | US-29 to US-31 | Fresh context, subagent injection |
| `harnesses` | US-32 to US-34 | All three harness implementations |
| `validation` | US-35 to US-36 | Backpressure validation pipeline |
| `transcript` | US-37 to US-38 | Recording and replay |
| `git` | US-41 to US-42 | AI-generated commits |
| `config` | US-43 to US-44 | Configuration overrides |
| `combined` | US-45 to US-50 | Full PRD-to-implementation workflows |

**Total**: 267 tests (118 unit + 132 E2E + 17 integration)

---

## Recommendations

### High Priority

1. **Remove `descartes run` from workflows.md** - Replace with actual commands
2. **Remove `--plan-only` workflow** - Either implement feature or remove documentation
3. **Fix `--last` flag reference** - Use `--today` or `--session` instead

### Medium Priority

4. **Clarify default harness** - Document that CLI default is `claude-code`, config default is `opencode`
5. **Add `orchestrator` to enum** - Or document that it uses Custom variant

### Low Priority

6. **Add `--last <n>` flag** - Would be useful feature for transcripts command
7. **Add `--plan-only` mode** - Would enable the documented plan-then-build workflow

---

## Files Reviewed

- `docs/README.md`
- `docs/getting-started.md`
- `docs/configuration.md`
- `docs/harnesses.md`
- `docs/swarm.md`
- `docs/workflows.md`
- `src/main.rs`
- `src/config.rs`
- `src/spec.rs`
- `src/swarm_executor.rs`
- `src/harness/*.rs`
- `src/agent/subagent.rs`
- `tests/e2e/*.rs`
- `tests/user_stories/*.rs`
- `TESTING.md`
