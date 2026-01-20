# Task 1: Verification Findings - Unused Files for Deletion

**Date**: 2026-01-20
**Task**: Verify unused files for safe deletion
**Status**: VERIFIED - Files can be deleted with coordinated cleanup

---

## Files Marked for Deletion

| File | Lines | Status |
|------|-------|--------|
| `src/swarm_executor.rs` | ~1,340 | HAS ACTIVE REFERENCES |
| `src/swarm_tui.rs` | ~520 | HAS ACTIVE REFERENCES |
| `src/context_handoff.rs` | ~472 | HAS ACTIVE REFERENCES |
| `src/handoff/mod.rs` | ~357 | HAS ACTIVE REFERENCES |

---

## Detailed Reference Analysis

### 1. `handoff/mod.rs` (handoff directory)

**References found:**
| Location | Reference | Action Required |
|----------|-----------|-----------------|
| `lib.rs:35` | `pub mod handoff;` | Remove module declaration |
| `lib.rs:52` | `pub use handoff::Handoff;` | Remove re-export |
| `interactive/commands.rs:85` | `Handoff` variant in `ContextType` enum | Remove or stub |
| `interactive/session.rs:467` | `ContextType::Handoff` pattern match | Remove or stub |

**Note**: The `Handoff` struct in this module is separate from `HandoffContext` in `context_handoff.rs`.

---

### 2. `swarm_executor.rs`

**References found:**
| Location | Reference | Action Required |
|----------|-----------|-----------------|
| `lib.rs:38` | `pub mod swarm_executor;` | Remove module declaration |
| `lib.rs:55` | `pub use swarm_executor::{SwarmExecutor, TaskResult};` | Remove re-export |
| `main.rs:590-604` | `SwarmExecutor::new(...)` in fallback path | Remove (Task 4) |
| `tests/swarm_integration.rs` | Multiple usages | Delete test file |
| `tests/user_stories/combined.rs:14,63,232,269` | `use descartes::swarm_executor::SwarmExecutor` | Update/delete test |
| `tests/user_stories/swarm.rs:9,22,46,75,101,208` | Multiple SwarmExecutor usages | Delete test file |
| `tests/e2e/swarm_e2e.rs` | 15+ usages | Delete test file |

---

### 3. `swarm_tui.rs`

**References found:**
| Location | Reference | Action Required |
|----------|-----------|-----------------|
| `lib.rs:39` | `pub mod swarm_tui;` | Remove module declaration |
| `lib.rs:56` | `pub use swarm_tui::{SwarmTui, TuiAction, WaveProgress};` | Remove re-export |
| `swarm_executor.rs:24` | `use crate::swarm_tui::{SwarmTui, TuiAction};` | N/A (executor deleted) |

**Note**: Only used by `swarm_executor.rs`, which is also being deleted.

---

### 4. `context_handoff.rs`

**References found:**
| Location | Reference | Action Required |
|----------|-----------|-----------------|
| `lib.rs:34` | `pub mod context_handoff;` | Remove module declaration |
| `lib.rs:49-50` | Re-exports of `estimate_tokens`, `summarize_agent_progress`, `ContextMonitor`, `HandoffContext` | Remove re-exports |
| `swarm_executor.rs:22` | Import for `ContextMonitor`, `HandoffContext` | N/A (executor deleted) |
| `tests/user_stories/context.rs:10` | `use descartes::context_handoff::{ContextMonitor, HandoffContext}` | Delete or update test |

---

## Test Files Requiring Action

| Test File | Action | Reason |
|-----------|--------|--------|
| `tests/swarm_integration.rs` | DELETE | Tests SwarmExecutor functionality |
| `tests/user_stories/swarm.rs` | DELETE | Tests SwarmExecutor functionality |
| `tests/e2e/swarm_e2e.rs` | DELETE | Tests SwarmExecutor functionality |
| `tests/user_stories/context.rs` | DELETE | Tests context_handoff functionality |
| `tests/user_stories/combined.rs` | UPDATE | May have other tests; remove SwarmExecutor portions |

---

## Verification Summary

**Can these files be safely deleted?** YES, with coordinated cleanup.

The deletion is safe because:
1. SCUD CLI now handles all orchestration functionality these files provided
2. The `--no-use-scud` fallback path is being removed (Task 4)
3. All references are tracked and cleanup tasks exist in the dependency chain

**Required coordination:**
- Task 2 (Remove deprecated orchestration files) must also update imports
- Task 3 (Update lib.rs exports) must remove all re-exports
- Task 4 (Update main.rs for SCUD-only delegation) must remove fallback path
- Test files must be deleted or updated as part of Task 2 or Task 5

---

## Recommendation

**PROCEED WITH DELETION** following the task sequence:
1. Task 1 (this task) - Document findings for confirmation
2. Task 2 - Delete files AND update lib.rs module declarations
3. Task 3 - Update lib.rs re-exports
4. Task 4 - Remove main.rs fallback path
5. Task 5 - Build and test to verify clean compilation

The existing task dependencies (2→1, 3→2, 4→3, 5→4) correctly sequence this work.
