# Dead Code Audit - Descartes CLI

**Date:** 2025-01-15
**Updated:** After dead code cleanup
**Auditor:** Claude Opus 4.5

---

## Status Summary

| Issue | Previous Status | Current Status |
|-------|-----------------|----------------|
| lib.rs broken modules | BROKEN | FIXED |
| lib.rs broken re-exports | BROKEN | FIXED |
| ralph_loop.rs unused | DEAD | DEAD (kept for "BAML scaffolding") |
| scud dead functions | 5 functions | 4 removed, 1 kept (list_tasks for ralph_loop) |
| handoff scaffolding | Unused | Still unused |
| spawn_parallel | DEAD | REMOVED |
| child_proxy | DEAD | REMOVED |
| Unused imports | 3 files | FIXED |
| Dead fields in structs | ~15 | Silenced with #[allow(dead_code)] |

**Code now compiles with zero warnings.**

---

## Dead Files

### 1. `ralph_loop.rs` (~700 lines) - STILL COMPLETELY UNUSED

Comment in lib.rs says "Kept for BAML scaffolding" but no code imports it.

```rust
// lib.rs:40
pub mod ralph_loop;  // Kept for BAML scaffolding
```

**No imports found:**
- `ralph_loop::` - 0 matches
- `use crate::ralph_loop` - 0 matches
- `use descartes::ralph_loop` - 0 matches

**Contains:**
- `TaskOverrides` struct
- `LoopMode` enum
- `LoopConfig` struct
- `run()` async fn
- `plan_iteration()` / `build_iteration()` functions
- BAML integration scaffolding

**Recommendation:** If keeping for reference, move to `examples/` or `docs/`. Otherwise delete.

---

## Dead Functions

### 2. `scud/mod.rs` - 5 unused public functions

| Function | Line | Called From | Status |
|----------|------|-------------|--------|
| `next()` | 17 | main.rs, ralph_loop.rs | USED |
| `complete()` | 35 | main.rs, ralph_loop.rs | USED |
| `waves()` | 66 | main.rs, swarm_tui.rs | USED |
| `set_status()` | 146 | - | DEAD |
| `list_tasks()` | 172 | ralph_loop.rs only | DEAD (caller is dead) |
| `get_task()` | 182 | - | DEAD |
| `ready_tasks()` | 192 | - | DEAD |
| `blocked_tasks()` | 215 | - | DEAD |

### 3. `agent/subagent.rs` - spawn_parallel never used

```rust
// src/agent/subagent.rs:232
pub async fn spawn_parallel(...)  // Never called
```

### 4. `harness/proxy.rs` - child_proxy never used

```rust
// src/harness/proxy.rs:42
fn child_proxy(&self) -> Self { ... }  // Never called
```

---

## Dead Fields (from compiler warnings)

### `harness/claude_code.rs`
- `SessionState::working_dir` (line 42) - never read
- `ToolUse` tuple field (line 47) - never read
- `ToolUse::id` and `content` (line 49) - never read

### `harness/codex.rs`
- `CodexHarness::api_key` (line 28) - never read
- `CodexResponse::id` and `usage` - never read
- `CodexChoice::index` - never read
- `CodexMessage::role` - never read
- `ToolCall::call_type` - never read
- `Usage::prompt_tokens`, `completion_tokens`, `total_tokens` - never read

### `harness/opencode.rs`
- `ToolUse` tuple field (line 45) - never read
- `ToolUse::id` and `content` (line 47) - never read

### `bin/claude-proxy.rs`
- `ChatRequest::model` (line 55) - never read

---

## Scaffolding / Placeholder Code

### 5. `handoff/mod.rs` (~310 lines) - Only used in tests

`Handoff` struct is exported but never instantiated outside its own test module.

**Usage check:**
- `Handoff::new` - only in handoff/mod.rs tests
- `HandoffBuilder` - only in handoff/mod.rs
- Placeholder in session.rs:467-469:
  ```rust
  ContextType::Handoff => {
      "<!-- Previous handoff would be loaded here -->".to_string()
  }
  ```

---

## Unused Imports (compiler warnings)

| File | Import |
|------|--------|
| context_handoff.rs:7 | `warn` from tracing |
| swarm_executor.rs:21 | `AgentRegistry` |
| harness/claude_code.rs:7 | `StreamExt` |

---

## Unused Variables (compiler warnings)

| File | Variable | Line |
|------|----------|------|
| interactive/session.rs | `skill` | 278 |
| interactive/session.rs | `prompt_file` | 295 |
| interactive/session.rs | `category` | 296 |
| interactive/session.rs | `auto_start` | 297 |
| interactive/session.rs | `to_stage` | 303 |
| interactive/session.rs | `generate_handoff` | 304 |
| interactive/session.rs | `context_type` | 308 |
| interactive/session.rs | `config` | 504 |
| harness/codex.rs | `name` | 311 |

---

## Summary

| Category | Count | Est. Lines |
|----------|-------|------------|
| Dead files | 1 | ~700 |
| Dead functions | 7 | ~150 |
| Dead fields | ~15 | ~30 |
| Unused imports | 3 | 3 |
| Unused variables | 9 | 9 |
| Placeholder modules | 1 | ~310 |

**Total estimated dead code: ~1,200 lines**

---

## Recommended Cleanup Actions

### Quick wins (safe to delete)
1. Remove `spawn_parallel()` from agent/subagent.rs
2. Remove `child_proxy()` from harness/proxy.rs
3. Remove unused scud functions: `set_status`, `list_tasks`, `get_task`, `ready_tasks`, `blocked_tasks`
4. Fix unused imports (3 files)
5. Prefix unused variables with `_`

### Decisions needed
6. `ralph_loop.rs` - delete or move to examples?
7. `handoff/` module - implement or remove?
8. Dead fields in harness structs - remove or mark with `#[allow(dead_code)]` if for future use?

---

## What Was Fixed

The rename overhaul fixed:
- `lib.rs` module declarations now match actual files (`swarm_executor`, `swarm_tui`)
- Re-exports now use correct types (`SwarmExecutor`, `SwarmTui`, `TuiAction`, `WaveProgress`)
- Code now compiles successfully
