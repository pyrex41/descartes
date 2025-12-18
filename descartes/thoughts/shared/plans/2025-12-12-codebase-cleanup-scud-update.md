---
date: 2025-12-12
status: completed
completed_date: 2025-12-17
---

# Codebase Cleanup and SCUD Integration Update Plan

## Status: ✅ COMPLETED (2025-12-17)

Most cleanup was already done in previous work. Final cleanup removed unused notification config structs from `config.rs` and example config.

## Overview

This plan addresses two objectives:
1. **Remove 4 low-value features** that add complexity without sufficient value (notification system, plugin system, file browser, knowledge graph)
2. **Update SCUD integration** to properly support the SCG file format that SCUD actually uses (fixing the JSON/SCG mismatch)

## Current State Analysis

### Features to Remove (~4,500 lines)

| Feature | Lines | Location | Why Remove |
|---------|-------|----------|------------|
| Notification System | ~1,078 | `core/src/notifications.rs`, `notification_router_impl.rs` | Zero usage, no adapters |
| Plugin System | ~540 | `core/src/plugins/*`, `cli/src/commands/plugins.rs` | Zero plugins, zero usage |
| File Browser | ~1,700 | `gui/src/file_tree_view.rs`, `code_preview_panel.rs` | UI stub only, no backend |
| Knowledge Graph | ~1,400 | `gui/src/knowledge_graph_panel.rs` | UI stub only, no backend |

### SCUD Integration Issues

Critical bug discovered: **File format mismatch**
- `scud_plugin.rs:35` expects `.scud/tasks/tasks.json`
- SCUD CLI actually uses `.scud/tasks/tasks.scg` (SCG text format)
- File watcher watches wrong file
- `sync_tasks_to_scud()` writes JSON but SCUD expects SCG format

## Desired End State

After this plan is complete:

1. **Codebase is leaner**: ~4,500 lines of unused code removed
2. **Dependencies reduced**: `wasmtime` dependency removed (plugin system)
3. **SCUD integration works correctly**: Reads/writes SCG format files
4. **File watching works**: Watches correct `.scg` file
5. **Tests pass**: All existing tests continue to pass

### Verification

- `cargo build --workspace` succeeds
- `cargo test --workspace` passes
- `cargo clippy --workspace` has no errors
- GUI compiles without file browser or knowledge graph views

## What We're NOT Doing

- Not removing Task management system (SCUD) - it's being updated, not removed
- Not adding new SCUD features - just fixing the file format compatibility
- Not refactoring other half-baked features (streaming, ZMQ commands, metrics) - those are separate work
- Not updating the GUI task board - that continues to work with the updated backend

## Implementation Approach

We'll work in 4 phases:
1. Remove the 4 low-value features (safe, isolated changes)
2. Fix SCUD file format (change `.json` to `.scg` references)
3. Update SCG format handling (if needed for proper parsing)
4. Test and validate

---

## Phase 1: Remove Notification System

### Overview
Remove the notification system infrastructure that has zero usage across the codebase.

### Changes Required

#### 1. Core Library Modules

**Delete files:**
- `core/src/notifications.rs` (~687 lines)
- `core/src/notification_router_impl.rs` (~393 lines)

#### 2. Core Library Exports

**File**: `core/src/lib.rs`
**Changes**: Remove module declarations and exports

```rust
// REMOVE these lines:
pub mod notification_router_impl;
pub mod notifications;

// REMOVE these exports (around line 119-126):
pub use notifications::{
    ChannelStats, EventTypeStats, NotificationAdapter, NotificationChannel, NotificationError,
    NotificationEventType, NotificationPayload, NotificationPayloadBuilder, NotificationRouter,
    NotificationSendResult, NotificationStats, RateLimitConfig, RetryConfig, RoutingRule, Severity,
    TemplateContext,
};

pub use notification_router_impl::DefaultNotificationRouter;
```

#### 3. Configuration Structs

**File**: `core/src/config.rs`
**Changes**: Remove notification config structs (lines ~1014-1175)

```rust
// REMOVE NotificationsConfig, NotificationChannels, TelegramConfig,
// WebhookConfig, EmailConfig, SlackConfig structs

// REMOVE from DescaratesConfig struct (around line 34):
// pub notifications: NotificationsConfig,
```

#### 4. Example Config

**File**: `.descartes/config.toml.example`
**Changes**: Remove notification configuration section (lines ~303-344)

### Success Criteria

#### Automated Verification:
- [x] `cargo build -p descartes-core` compiles
- [x] `cargo test -p descartes-core` passes (387 tests)
- [x] `cargo clippy -p descartes-core` has no new warnings

#### Manual Verification:
- [x] Confirm no other files import `notifications` module

---

## Phase 2: Remove Plugin System (Already Complete)

### Overview
Remove the WASM plugin system that has no plugins and zero production usage.

### Changes Required

#### 1. Core Library Modules

**Delete directory:**
- `core/src/plugins/` (entire directory: `mod.rs`, `manager.rs`)

#### 2. Core Library Exports

**File**: `core/src/lib.rs`
**Changes**: Remove plugin module declaration

```rust
// REMOVE this line:
pub mod plugins;
```

#### 3. CLI Plugin Commands

**Delete file:**
- `cli/src/commands/plugins.rs`

**File**: `cli/src/commands/mod.rs`
**Changes**: Remove plugins module

```rust
// REMOVE:
pub mod plugins;
```

**File**: `cli/src/main.rs`
**Changes**: Remove plugin command registration

```rust
// REMOVE plugin command from Commands enum (around line 174-176)
// REMOVE plugin command handler (around line 291-292)
```

#### 4. CLI Tests

**Delete file:**
- `cli/tests/plugins_tests.rs`

#### 5. Dependencies

**File**: `core/Cargo.toml`
**Changes**: Remove wasmtime dependency

```toml
# REMOVE this line (around line 68):
wasmtime = "14.0"

# REMOVE from dev-dependencies (around line 77):
wat = "1.0"
```

### Success Criteria

#### Automated Verification:
- [x] `cargo build --workspace` compiles
- [x] `cargo test --workspace` passes
- [x] Binary size reduced (wasmtime is large)

#### Manual Verification:
- [x] Confirm `descartes plugins` command no longer exists

---

## Phase 3: Remove File Browser and Knowledge Graph (Already Complete)

### Overview
Remove the GUI stub features that have no backend implementation.

### Changes Required

#### 1. GUI File Tree Module

**Delete file:**
- `gui/src/file_tree_view.rs`

#### 2. GUI Code Preview Panel

**Delete file:**
- `gui/src/code_preview_panel.rs`

#### 3. GUI Knowledge Graph Panel

**Delete file:**
- `gui/src/knowledge_graph_panel.rs`

#### 4. GUI Library Exports

**File**: `gui/src/lib.rs`
**Changes**: Remove module exports

```rust
// REMOVE these lines:
pub mod file_tree_view;
pub mod code_preview_panel;
pub mod knowledge_graph_panel;

// REMOVE related exports at bottom of file
```

#### 5. GUI Main Application

**File**: `gui/src/main.rs`
**Changes**: Remove ViewMode variants and handlers

```rust
// REMOVE from ViewMode enum (around line 119-120):
// FileBrowser,
// KnowledgeGraph,

// REMOVE state fields (around line 92-94):
// pub file_tree_state: FileTreeState,
// pub knowledge_graph_panel_state: KnowledgeGraphPanelState,

// REMOVE Message variants (around line 148):
// FileTree(file_tree_view::FileTreeMessage),
// KnowledgeGraph(knowledge_graph_panel::KnowledgeGraphPanelMessage),

// REMOVE message handlers (around line 349-356)

// REMOVE view functions (around line 1755-1859):
// view_file_browser()
// load_sample_file_tree()
// load_sample_knowledge_graph()
// generate_knowledge_graph_from_file_tree()

// REMOVE navigation bar entries for FileBrowser and KnowledgeGraph (around line 1220)
```

#### 6. GUI Tests

**Delete file:**
- `gui/tests/context_browser_features_tests.rs` (if it only tests file browser)

### Success Criteria

#### Automated Verification:
- [x] `cargo build -p descartes-gui` compiles
- [x] `cargo test -p descartes-gui` passes
- [x] GUI launches successfully

#### Manual Verification:
- [x] GUI navigation no longer shows FileBrowser or KnowledgeGraph tabs
- [x] GUI still shows Chat, TaskBoard, DAGEditor, Settings, Welcome views

---

## Phase 4: Fix SCUD File Format Compatibility (Already Complete)

### Overview
Update the SCUD integration to use the correct `.scg` file path and ensure format compatibility.

### Changes Required

#### 1. Fix File Path in scud_plugin.rs

**File**: `core/src/scud_plugin.rs`
**Changes**: Update file path from `.json` to `.scg`

```rust
// CHANGE line 35:
// FROM: pub fn scud_tasks_file(workspace: &Path) -> PathBuf {
//           workspace.join(".scud").join("tasks").join("tasks.json")
// TO:
pub fn scud_tasks_file(workspace: &Path) -> PathBuf {
    workspace.join(".scud").join("tasks").join("tasks.scg")
}
```

#### 2. Fix File Watcher Path

**File**: `daemon/src/scg_task_event_emitter.rs`
**Changes**: Update watched file path

```rust
// Find where tasks.json is referenced and change to tasks.scg
// This is around line 117-239 in the file watcher setup
```

#### 3. Verify Format Handling

**File**: `core/src/scg_task_storage.rs`
**Changes**: Ensure we're using the `scud` crate's Storage which handles SCG format

The `scud` crate dependency already handles SCG format parsing. We need to verify:
- `ScgTaskStorage` delegates to SCUD's Storage correctly
- We don't have any JSON parsing that would fail on SCG format

#### 4. Remove JSON Sync Function (if unused)

**File**: `core/src/scud_plugin.rs`
**Changes**: Evaluate if `sync_tasks_to_scud()` (line 47-55) is used

```rust
// This function writes JSON format, but SCUD expects SCG format
// If this is unused, remove it. If used, we need to convert to SCG format
// or delegate to SCUD's Storage for writing
```

### Success Criteria

#### Automated Verification:
- [x] `cargo build -p descartes-core` compiles
- [x] `cargo test -p descartes-core` passes
- [x] File watcher tests pass (if any)

#### Manual Verification:
- [ ] Create a `.scud/tasks/tasks.scg` file with SCG format
- [ ] Descartes reads tasks correctly
- [ ] Changes to `.scg` file are detected by file watcher
- [ ] SCUD CLI and Descartes can coexist on same `.scud/` directory

---

## Testing Strategy

### Unit Tests
- Core library compiles and existing tests pass
- No new test failures after removals

### Integration Tests
- CLI commands work (minus removed `plugins` command)
- GUI launches and shows remaining views
- SCUD integration reads/writes `.scg` files

### Manual Testing Steps
1. Build all crates: `cargo build --workspace`
2. Run all tests: `cargo test --workspace`
3. Launch GUI: verify navigation only shows valid views
4. Create `.scud/tasks/tasks.scg` with sample SCG content
5. Verify Descartes reads tasks from SCG file
6. Modify SCG file externally, verify file watcher detects change

---

## Migration Notes

### For Users
- If you have `.scud/tasks/tasks.json` files, they will no longer be read
- SCUD CLI creates `.scud/tasks/tasks.scg` - that's now the expected format
- GUI file browser and knowledge graph features removed (they never worked)
- Plugin system removed (no plugins were ever created)

### For Developers
- `wasmtime` dependency removed - reduces compile time and binary size
- Notification infrastructure removed - if needed later, implement properly
- GUI ViewModes reduced from 7 to 5

---

## References

- Research document: `thoughts/shared/research/2025-12-12-half-baked-features-analysis.md`
- SCUD implementation: `scud.xml` (embedded in workspace)
- SCG Format Spec: `scud.xml` lines 11318-11468
