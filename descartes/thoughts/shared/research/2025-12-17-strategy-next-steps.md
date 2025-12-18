---
date: 2025-12-17T20:19:17-06:00
researcher: Claude
git_commit: 5c9a99211d8db55b5f359646a5d15005cca697a6
branch: master
repository: backbone/descartes
topic: "Strategy Review: What's Next"
tags: [research, strategy, planning, roadmap]
status: complete
last_updated: 2025-12-17
last_updated_by: Claude
---

# Research: Strategy Review - What's Next

**Date**: 2025-12-17T20:19:17-06:00
**Researcher**: Claude
**Git Commit**: 5c9a99211d8db55b5f359646a5d15005cca697a6
**Branch**: master
**Repository**: backbone/descartes

## Research Question

Review existing research and plans to formulate a strategy for what's next.

## Summary

The Descartes codebase has completed significant foundational work. Here's the current status and strategic options for next steps.

### Completed Work

| Plan | Status | Summary |
|------|--------|---------|
| Plan 1: Fix Failing Tests | ✅ Complete | 6 test failures fixed |
| Plan 2: Critical Features | ✅ Complete | Expression eval, streaming, ZMQ commands |
| Plan 3: Warnings Cleanup | ✅ Complete | 165→10 clippy warnings |
| Deferred Features | ✅ Complete | WriteStdin/ReadStdout/ReadStderr, QueryOutput, Time Travel + Debugger |
| StreamLogs PUB/SUB | ✅ Complete | Real-time log streaming via ZMQ PUB/SUB |

### Outstanding Plans (Not Started)

| Plan | Status | Estimated Effort |
|------|--------|------------------|
| Codebase Cleanup (Remove low-value features) | ❌ Not started | Medium |
| SCUD Integration Fix (.json → .scg) | ❌ Not started | Low |

---

## Strategic Options

### Option A: Technical Debt Reduction (Cleanup Focus)

**Rationale**: Clean up before adding more features.

**Actions**:
1. Execute `2025-12-12-codebase-cleanup-scud-update.md`
   - Remove notification system (~1,078 lines)
   - Remove plugin system (~540 lines, removes `wasmtime` dependency)
   - Remove file browser stub (~1,700 lines)
   - Remove knowledge graph stub (~1,400 lines)
   - Fix SCUD .json → .scg file format

**Outcomes**:
- ~4,500 lines of unused code removed
- Binary size reduced (wasmtime removed)
- Simpler codebase to maintain
- SCUD CLI interoperability fixed

**Risk**: Low - these are isolated removals with no dependencies

---

### Option B: Feature Completion (Fill Gaps)

**Rationale**: Complete remaining half-baked features before cleanup.

**Actions from Half-Baked Analysis**:

| Feature | Current State | Effort to Complete |
|---------|---------------|-------------------|
| Daemon Metrics | Returns hardcoded 0.0 | Low |
| Git Stash Operations | Not implemented | Medium |
| CustomAction Command | Not implemented | Low |
| State Store Tests | 6 failing (created_at bug) | Low |

**New Feature Opportunities**:
- Log level filtering in StreamLogs
- Log persistence/replay
- Web interface for log viewing (from StreamLogs plan "not implementing" list)

---

### Option C: Stabilization (Test & Document)

**Rationale**: Solidify what exists before changing.

**Actions**:
1. Fix the 6 failing state_store_integration_tests
   - Root cause: `save_agent_state` missing `created_at` column
2. Add integration tests for new features:
   - StreamLogs PUB/SUB end-to-end test
   - Expression evaluator with complex DebugContext
   - Provider streaming with live API (optional)
3. Document the architecture
4. Update README with current capabilities

---

### Option D: User-Facing Features

**Rationale**: Make the system usable end-to-end.

**Actions**:
1. CLI improvements for agent interaction
2. GUI polish for core workflows
3. Configuration documentation
4. Sample agent definitions

---

## Recommended Strategy

**Sequence**: A → C → D

### Phase 1: Cleanup (Option A)
Execute the codebase cleanup plan to remove ~4,500 lines of dead code and fix SCUD integration. This:
- Reduces maintenance burden
- Simplifies codebase for future work
- Fixes a real bug (SCUD file format)

### Phase 2: Stabilization (Option C)
- Fix state_store tests (quick win)
- Add missing integration tests
- Document current architecture

### Phase 3: User-Facing (Option D)
- Focus on making the system usable
- CLI/GUI polish
- Sample configurations and agents

---

## Decision Points

Before proceeding, consider:

1. **SCUD Dependency**: Is SCUD CLI interoperability important?
   - If YES: Fix the .json → .scg issue (Phase 1)
   - If NO: Consider removing SCUD entirely (more aggressive cleanup)

2. **GUI Scope**: Should the GUI remain an IDE-like application?
   - If YES: Keep TaskBoard, DAGEditor, consider reviving file browser
   - If NO: Focus on Chat + Settings only

3. **Plugin System**: Is extensibility via WASM plugins on the roadmap?
   - If YES: Keep plugin system, start building plugins
   - If NO: Remove (saves large dependency)

---

## Immediate Action Items

If no decision needed, here are low-risk immediate actions:

1. **Fix state_store tests** (30 min)
   - Add `created_at` to INSERT in `save_agent_state`
   - Resolves 6 failing tests

2. **Update plan statuses** (10 min)
   - Mark StreamLogs plan as completed
   - Archive completed plans

3. **Run full test suite** (5 min)
   - Verify 387 tests still pass
   - No regressions from StreamLogs work

---

## Code References

### Completed Plans
- `thoughts/shared/plans/2025-12-15-fix-failing-tests.md` - Plan 1
- `thoughts/shared/plans/2025-12-15-critical-features.md` - Plan 2
- `thoughts/shared/plans/2025-12-15-warnings-dead-code-cleanup.md` - Plan 3
- `thoughts/shared/plans/2025-12-17-deferred-features-implementation.md`
- `thoughts/shared/plans/2025-12-17-streamlogs-pub-sub.md`

### Pending Plans
- `thoughts/shared/plans/2025-12-12-codebase-cleanup-scud-update.md`

### Research
- `thoughts/shared/research/2025-12-12-half-baked-features-analysis.md`
- `thoughts/shared/research/2025-12-15-plans-integration-review.md`

---

## Related Research

- [Half-Baked Features Analysis](2025-12-12-half-baked-features-analysis.md)
- [Plans Integration Review](2025-12-15-plans-integration-review.md)

---

## Open Questions

1. What is the priority order: cleanup vs features vs stability?
2. Is SCUD CLI interoperability a requirement?
3. Should the notification/plugin systems be kept for future use?
4. What user workflows need to work end-to-end first?
