# ZMQ Backbone Cleanup Implementation Plan

## Overview

Complete the ZMQ Backbone Refactor by fixing broken imports, cleaning up dead test files, and verifying the migration. This is the final 15% of the refactor.

## Current State Analysis

The ZMQ Backbone Refactor is ~85% complete:
- ✅ `agent-runner` crate deleted (6,116 lines)
- ✅ `ipc.rs` deleted and replaced with `channel_bridge.rs`
- ✅ IPC benchmarks deleted
- ✅ ZMQ benchmarks created
- ✅ Database migration SQL written
- ⚠️ 2 test files have broken imports to deleted `agent-runner` crate
- ⚠️ Verification steps not completed

### Key Discoveries:
- Only 2 files actually have broken imports (not 11 as initially thought)
- `agent_runner` module exists in `descartes_core` (different from deleted `descartes_agent_runner` crate)
- Most "broken imports" were actually correct internal module references
- Knowledge graph tests are dead code that tests deleted functionality

## Desired End State

After this plan is complete:
1. `cargo build --workspace` succeeds with no errors
2. `cargo test --workspace` passes (excluding known pre-existing failures)
3. No references to deleted `descartes_agent_runner` crate
4. Database migration verified working
5. Documentation updated to reflect current architecture

### Verification Commands:
```bash
# All must succeed:
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
grep -r "descartes_agent_runner" descartes/ --include="*.rs" | wc -l  # Should be 0
```

## What We're NOT Doing

- Adding new tests (separate effort)
- Implementing missing test files from testing plan
- Migrating `.taskmaster` references in documentation (cosmetic)
- Performance benchmarking (separate effort)

---

## Phase 1: Fix Broken Test Files

### Overview
Remove or fix the 2 test files with broken imports to the deleted `descartes_agent_runner` crate.

### Changes Required:

#### 1.1 Delete Knowledge Graph Integration Tests
**File**: `descartes/gui/tests/knowledge_graph_integration_tests.rs`
**Action**: Delete entire file (701 lines)

This file tests knowledge graph functionality that was entirely removed with the `agent-runner` crate. The tests are dead code with no equivalent functionality in the current codebase.

```bash
rm descartes/gui/tests/knowledge_graph_integration_tests.rs
```

#### 1.2 Clean Up Context Browser Tests
**File**: `descartes/gui/tests/context_browser_features_tests.rs`
**Action**: Remove modules that import deleted crate, keep code_preview tests

The file has 4 modules:
- `mod code_preview_tests` (lines 12-234) - **KEEP** - tests `code_preview_panel` which exists
- `mod file_tree_tests` (lines 237-454) - **DELETE** - imports deleted crate
- `mod knowledge_graph_tests` (lines 456-711) - **DELETE** - imports deleted crate
- `mod integration_tests` (lines 713-742) - **DELETE** - placeholder tests

After cleanup, the file should only contain:
```rust
//! Context browser features tests
//!
//! Tests for the context browser interactive features including
//! code preview functionality.

mod code_preview_tests {
    // ... existing code_preview tests (lines 12-234)
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` succeeds
- [x] `cargo test -p descartes-gui --test context_browser_features_tests` passes (13 tests)
- [x] `grep -r "descartes_agent_runner" descartes/ --include="*.rs"` returns no results

#### Manual Verification:
- [ ] Confirm no other files reference `descartes_agent_runner`

**Implementation Note**: After completing this phase, run `cargo build --workspace` to verify. Pause for confirmation before proceeding.

---

## Phase 2: Verify Module Exports

### Overview
Verify that all ZMQ modules are correctly exported and the crate compiles cleanly.

### Changes Required:

#### 2.1 Verify lib.rs Exports
**File**: `descartes/core/src/lib.rs`
**Action**: Verify, no changes expected

Confirm these exports are present and correct:
- Lines 44-47: ZMQ module declarations
- Lines 81-82: agent_runner exports
- Lines 100-107: zmq_agent_runner exports
- Lines 126-127: channel_bridge exports

#### 2.2 Verify ZMQ Benchmark Imports
**File**: `descartes/core/benches/zmq_benchmarks.rs`
**Action**: Verify imports work

The benchmark uses:
```rust
use descartes_core::{
    deserialize_zmq_message, serialize_zmq_message, validate_message_size,
    HealthCheckRequest, SpawnRequest, ZmqMessage,
};
```

These should all be available via the lib.rs re-exports.

### Success Criteria:

#### Automated Verification:
- [x] `cargo build -p descartes-core` succeeds
- [x] `cargo bench --bench zmq_benchmarks --no-run` compiles successfully
- [x] `cargo test -p descartes-core --lib zmq_agent_runner` passes (6 tests)

#### Manual Verification:
- [ ] None required

---

## Phase 3: Verify Database Migration

### Overview
Test that the database migration works correctly with fresh database creation.

### Changes Required:

#### 3.1 Verify Migration File Exists
**File**: `descartes/core/migrations/100_zmq_backbone_simplify.sql`
**Action**: Verify file is present and correct

The migration should:
- DROP 28 semantic/RAG tables
- CREATE 3 core tables (agents, events, snapshots)
- CREATE 9 indexes

#### 3.2 Test Fresh Database Creation
**Action**: Manual test with daemon startup

```bash
# Remove any existing test database
rm -f /tmp/descartes-test.db

# Start daemon with fresh database (if supported)
# Or run migration directly via sqlx
```

### Success Criteria:

#### Automated Verification:
- [x] Migration file exists at correct path
- [x] Migration SQL is syntactically valid
- [x] Core tables created: `agents`, `events`, `snapshots`

#### Manual Verification:
- [ ] Fresh database can be created with daemon startup
- [ ] Indexes are created

**Implementation Note**: This phase may require starting the daemon or running migrations manually. Pause for human verification.

---

## Phase 4: Documentation Cleanup

### Overview
Remove obsolete IPC documentation and update architecture references.

### Changes Required:

#### 4.1 Delete Obsolete IPC Documentation
**Files to delete**:
- `descartes/core/IPC_INTEGRATION_GUIDE.md`
- `descartes/core/IPC_IMPLEMENTATION_SUMMARY.md`

These documents describe the deleted `ipc.rs` system which has been replaced by `channel_bridge.rs` and ZMQ.

```bash
rm descartes/core/IPC_INTEGRATION_GUIDE.md
rm descartes/core/IPC_IMPLEMENTATION_SUMMARY.md
```

### Success Criteria:

#### Automated Verification:
- [x] `ls descartes/core/IPC_*.md` returns no files

#### Manual Verification:
- [x] None required

---

## Phase 5: Final Verification

### Overview
Run comprehensive checks to ensure the refactor is complete.

### Changes Required:

#### 5.1 Full Build and Test
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

#### 5.2 Check for Orphaned References
```bash
# Should return 0
grep -r "descartes_agent_runner" descartes/ --include="*.rs" | wc -l

# Should return 0
grep -r "agent-runner" descartes/ --include="*.toml" | wc -l

# Should return 0 (excluding this plan file)
grep -r "ipc\.rs" descartes/ --include="*.rs" | wc -l
```

#### 5.3 Verify Binary Size Reduction (Optional)
```bash
cargo build --release
ls -lh target/release/descartes-daemon
ls -lh target/release/descartes-gui
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` passes (excluding pre-existing failures in dag_editor_tests, swarm_monitor_tests, etc.)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [x] No references to deleted crate in Rust files (grep returns 0)
- [x] No references to `agent-runner` in Cargo.toml files (grep returns 0)

#### Manual Verification:
- [ ] GUI launches: `cargo run --bin descartes-gui`
- [ ] Daemon starts: `cargo run --bin descartes-daemon`

---

## Testing Strategy

### Unit Tests:
- Existing unit tests should pass after cleanup
- No new tests added in this plan

### Integration Tests:
- `cargo test --workspace` covers integration tests
- Manual daemon + GUI startup verification

### Manual Testing Steps:
1. Run `cargo build --workspace` - should succeed
2. Run `cargo test --workspace` - note any failures
3. Start daemon and verify it accepts connections
4. Start GUI and verify it launches

## Performance Considerations

- No performance-critical changes in this cleanup
- Binary size should decrease after removing dead test code
- Compile time should improve slightly

## Migration Notes

No data migration required - this is code cleanup only.

## References

- Original PRD: `.scud/docs/prd/backbone.md`
- Implementation plan: `thoughts/shared/plans/2025-12-10-zmq-backbone-refactor.md`
- Research: `thoughts/shared/research/2025-12-10-planned-vs-implemented-features.md`
- Research: `thoughts/shared/research/2025-12-10-zmq-backbone-prd-context.md`
