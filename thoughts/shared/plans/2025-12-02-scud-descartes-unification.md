# Implementation Plan: SCUD-Descartes Unification

**Date**: 2025-12-02
**Status**: Phases 1-5 Complete
**Branch**: `feat/scud-unification`

## Overview

Unify Descartes and SCUD into a single, compatible task management system where:
- SCUD's models and SCG format become the standard
- Descartes builds "on top" of SCUD conceptually
- Users can use either tooling interchangeably (or neither - just edit .scg files)
- No hard dependencies between the systems

## Unified Data Model

### Task (Aligned)

```rust
pub struct Task {
    // Identity
    pub id: String,                    // Flexible: "auth:1.2" or UUID string

    // Core fields
    pub title: String,                 // Max 200 chars
    pub description: String,           // Max 5000 chars
    pub status: TaskStatus,            // 8 states (SCUD model)
    pub priority: Priority,            // 4 levels (adding Critical to SCUD)
    pub complexity: u32,               // Fibonacci: 0,1,2,3,5,8,13,21,34,55,89

    // Dependencies
    pub dependencies: Vec<String>,     // Task IDs this depends on

    // Hierarchy (subtasks)
    pub parent_id: Option<String>,
    pub subtasks: Vec<String>,

    // Metadata
    pub details: Option<String>,
    pub test_strategy: Option<String>,
    pub created_at: Option<String>,    // ISO8601
    pub updated_at: Option<String>,    // ISO8601

    // Parallel execution
    pub assigned_to: Option<String>,
    pub locked_by: Option<String>,
    pub locked_at: Option<String>,
}
```

### TaskStatus (8 states - SCUD model)

```rust
pub enum TaskStatus {
    Pending,      // P - Not started
    InProgress,   // I - Being worked on
    Done,         // D - Completed
    Review,       // R - Awaiting review
    Blocked,      // B - Blocked by dependency/issue
    Deferred,     // F - Postponed
    Cancelled,    // C - Won't do
    Expanded,     // X - Broken into subtasks
}
```

### Priority (4 levels - adding Critical to SCUD)

```rust
pub enum Priority {
    Low,       // L
    Medium,    // M (default)
    High,      // H
    Critical,  // C (new)
}
```

### Phase/Epic Container

```rust
pub struct Phase {
    pub name: String,
    pub tasks: Vec<Task>,
}
```

---

## Architecture

### Storage Layer

```
.scud/
├── tasks/
│   └── tasks.scg          # All phases, separated by ---
├── active-tag             # Current active phase name
├── config.toml            # SCUD configuration
└── history/               # Optional: event history (SQLite)
    └── events.db
```

### Component Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                         User Interface                          │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│   SCUD CLI   │ Descartes CLI│  Descartes   │   Direct Edit     │
│  (Rust)      │   (Rust)     │    GUI       │   (.scg files)    │
└──────┬───────┴──────┬───────┴──────┬───────┴─────────┬─────────┘
       │              │              │                 │
       ▼              ▼              ▼                 ▼
┌────────────────────────────────────────────────────────────────┐
│                    Shared Task Library                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ SCG Parser  │  │ Task Model  │  │  In-Memory Query Engine │ │
│  │ & Serializer│  │  (Unified)  │  │  (filter/sort/search)   │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│                        .scud/ Directory                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  tasks.scg  │  │ active-tag  │  │  config.toml            │ │
│  │  (primary)  │  │             │  │                         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: SCUD Model Updates (SCUD Repo) ✅ COMPLETE

**Goal**: Add Critical priority, ensure model is complete

**Status**: Already implemented! SCUD already has:
- [x] `Critical` priority level in `Priority` enum (`scud-cli/src/models/task.rs:62-68`)
- [x] `C` code mapping in SCG serializer (`scud-cli/src/formats/scg.rs:41-57`)
- [x] Library exports in `lib.rs`

No changes needed - SCUD is ready for integration.

---

### Phase 2: Add SCUD Dependency to Descartes ✅ COMPLETE

**Goal**: Add the existing `scud` library as a dependency to Descartes

**Status**: Complete! Added SCUD dependency and integration types.

**Changes completed**:
- [x] Add `scud` as a git dependency in `descartes/Cargo.toml` workspace deps
- [x] Add `scud` dependency to `descartes/core/Cargo.toml`
- [x] Re-export SCUD types in `traits.rs` with `Scud` prefix
- [x] Create bidirectional conversion functions (`task_to_scud`, `scud_to_task`)
- [x] Re-export SCUD types from `lib.rs`

**Files modified**:
- `descartes/Cargo.toml` - Added `scud = { git = "https://github.com/pyrex41/scud" }`
- `descartes/core/Cargo.toml` - Added `scud = { workspace = true }`
- `descartes/core/src/traits.rs` - Added SCUD type imports and conversion functions
- `descartes/core/src/lib.rs` - Re-exported SCUD types

**Approach taken**: Instead of replacing Descartes types wholesale, we:
1. Re-exported SCUD types alongside Descartes types (with `Scud` prefix)
2. Created conversion functions between the two task models
3. Kept existing SQLite task system working for backwards compatibility

This allows gradual migration without breaking existing functionality.

```
scud-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── models/
    │   ├── mod.rs
    │   ├── task.rs        # Task, TaskStatus, Priority
    │   └── phase.rs       # Phase, PhaseStats
    ├── formats/
    │   ├── mod.rs
    │   ├── scg.rs         # SCG parser/serializer
    │   └── json.rs        # JSON support (optional)
    ├── storage/
    │   ├── mod.rs
    │   ├── file.rs        # File-based storage with locking
    │   └── memory.rs      # In-memory storage for testing
    └── query/
        ├── mod.rs
        └── builder.rs     # In-memory query builder
```

**Key Components**:

#### 2.1 In-Memory Query Builder

```rust
// scud-core/src/query/builder.rs

pub struct TaskQueryBuilder {
    status_filter: Option<Vec<TaskStatus>>,
    priority_filter: Option<Vec<Priority>>,
    complexity_min: Option<u32>,
    complexity_max: Option<u32>,
    assigned_to: Option<String>,
    search_term: Option<String>,
    sort_by: SortField,
    sort_order: SortOrder,
    limit: Option<usize>,
    offset: usize,
}

impl TaskQueryBuilder {
    pub fn new() -> Self { ... }

    pub fn with_status(mut self, status: TaskStatus) -> Self { ... }
    pub fn with_priority(mut self, priority: Priority) -> Self { ... }
    pub fn with_complexity_range(mut self, min: u32, max: u32) -> Self { ... }
    pub fn assigned_to(mut self, assignee: &str) -> Self { ... }
    pub fn search(mut self, term: &str) -> Self { ... }
    pub fn sort_by(mut self, field: SortField) -> Self { ... }
    pub fn limit(mut self, n: usize) -> Self { ... }
    pub fn offset(mut self, n: usize) -> Self { ... }

    /// Execute query against in-memory task list
    pub fn execute(&self, tasks: &[Task]) -> Vec<&Task> {
        tasks.iter()
            .filter(|t| self.matches(t))
            .sorted_by(|a, b| self.compare(a, b))
            .skip(self.offset)
            .take(self.limit.unwrap_or(usize::MAX))
            .collect()
    }

    fn matches(&self, task: &Task) -> bool {
        // Status filter
        if let Some(statuses) = &self.status_filter {
            if !statuses.contains(&task.status) {
                return false;
            }
        }

        // Priority filter
        if let Some(priorities) = &self.priority_filter {
            if !priorities.contains(&task.priority) {
                return false;
            }
        }

        // Search term (title + description)
        if let Some(term) = &self.search_term {
            let term_lower = term.to_lowercase();
            let in_title = task.title.to_lowercase().contains(&term_lower);
            let in_desc = task.description.to_lowercase().contains(&term_lower);
            if !in_title && !in_desc {
                return false;
            }
        }

        // ... other filters
        true
    }
}
```

#### 2.2 Storage Trait

```rust
// scud-core/src/storage/mod.rs

#[async_trait]
pub trait TaskStorage: Send + Sync {
    /// Load all phases
    async fn load_phases(&self) -> Result<HashMap<String, Phase>>;

    /// Save all phases
    async fn save_phases(&self, phases: &HashMap<String, Phase>) -> Result<()>;

    /// Load single phase
    async fn load_phase(&self, name: &str) -> Result<Option<Phase>>;

    /// Save single phase (merge with existing)
    async fn save_phase(&self, phase: &Phase) -> Result<()>;

    /// Get active phase name
    async fn get_active_phase(&self) -> Result<Option<String>>;

    /// Set active phase
    async fn set_active_phase(&self, name: &str) -> Result<()>;
}
```

---

### Phase 3: Descartes Integration ✅ COMPLETE

**Goal**: Add SCG-based task storage alongside SQLite (gradual migration approach)

**Status**: Complete! Added `ScgTaskStorage` and `ScgTaskQueries` modules.

**Changes completed**:
- [x] Created `descartes/core/src/scg_task_storage.rs` with:
  - `ScgTaskStorage` - async wrapper around SCUD's synchronous Storage
  - `ScgTaskQueryBuilder` - in-memory query builder matching SQL query interface
  - `ScgTaskQueries` - query executor for SCG-based tasks
  - `ScgPhaseStats` - phase statistics struct
- [x] Updated `descartes/core/src/lib.rs` to export new SCG types
- [x] All unit tests passing (5 tests for query builder functionality)

**Approach taken**: Instead of replacing SQLite storage wholesale, we:
1. Created SCG-based storage as an alternative alongside SQLite
2. Async wrappers use `tokio::task::spawn_blocking` for SCUD's sync operations
3. In-memory query engine mirrors SQL query builder interface
4. Both storage backends can coexist during gradual migration

**Key implementation details**:
- `ScgTaskStorage` creates new SCUD Storage instances for each blocking operation (Storage doesn't implement Clone)
- Phase cache (`RwLock<HashMap<String, Phase>>`) provides fast in-memory access
- Query builder supports status, priority, complexity, assignee, and search filters
- Pagination (offset/limit) and sorting work on in-memory filtered results

**Original plan (for reference)**:

#### 3.1 Update `descartes/core/Cargo.toml`

```toml
[dependencies]
scud-core = { path = "../../scud-cli" }  # or git dependency
# Remove: sqlx for tasks (keep for events/history)
```

#### 3.2 Replace Task Types

**File**: `descartes/core/src/traits.rs`

```rust
// Re-export from scud-core
pub use scud_core::models::{Task, TaskStatus, Priority, Phase, PhaseStats};
pub use scud_core::query::TaskQueryBuilder;

// Keep Descartes-specific complexity enum as alias if needed
pub type TaskComplexity = u32;  // Direct Fibonacci value

// Helper to convert legacy complexity
pub fn complexity_to_fibonacci(c: LegacyTaskComplexity) -> u32 {
    match c {
        LegacyTaskComplexity::Trivial => 1,
        LegacyTaskComplexity::Simple => 2,
        LegacyTaskComplexity::Moderate => 3,
        LegacyTaskComplexity::Complex => 5,
        LegacyTaskComplexity::Epic => 13,
    }
}
```

#### 3.3 Update StateStore

**File**: `descartes/core/src/state_store.rs`

```rust
use scud_core::storage::{TaskStorage, ScgFileStorage};

pub struct DescartesStateStore {
    // Task storage via SCG files
    task_storage: ScgFileStorage,

    // Event storage still uses SQLite
    event_pool: SqlitePool,

    // In-memory cache for queries
    task_cache: RwLock<HashMap<String, Phase>>,
}

impl DescartesStateStore {
    pub async fn new(project_root: &Path) -> Result<Self> {
        let scud_dir = project_root.join(".scud");

        Ok(Self {
            task_storage: ScgFileStorage::new(scud_dir.clone()),
            event_pool: /* existing SQLite setup for events */,
            task_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Refresh cache from disk
    pub async fn refresh_tasks(&self) -> Result<()> {
        let phases = self.task_storage.load_phases().await?;
        *self.task_cache.write().await = phases;
        Ok(())
    }

    /// Query tasks using in-memory engine
    pub async fn query_tasks(&self) -> TaskQueryBuilder {
        TaskQueryBuilder::new()
    }

    /// Execute query against cached tasks
    pub async fn execute_query(&self, query: &TaskQueryBuilder) -> Vec<Task> {
        let cache = self.task_cache.read().await;
        let all_tasks: Vec<&Task> = cache.values()
            .flat_map(|p| p.tasks.iter())
            .collect();

        query.execute(&all_tasks)
            .into_iter()
            .cloned()
            .collect()
    }
}
```

#### 3.4 Update TaskQueries

**File**: `descartes/core/src/task_queries.rs`

Replace SQL-based queries with in-memory operations:

```rust
pub struct TaskQueries {
    store: Arc<DescartesStateStore>,
}

impl TaskQueries {
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        self.store.execute_query(
            &TaskQueryBuilder::new().with_status(status)
        ).await
    }

    pub async fn get_ready_tasks(&self) -> Result<Vec<Task>> {
        let cache = self.store.task_cache.read().await;
        let active = self.store.task_storage.get_active_phase().await?;

        if let Some(phase_name) = active {
            if let Some(phase) = cache.get(&phase_name) {
                return Ok(phase.find_next_tasks());  // Uses SCUD's dependency logic
            }
        }
        Ok(vec![])
    }

    pub async fn get_task_statistics(&self) -> Result<TaskStatistics> {
        let cache = self.store.task_cache.read().await;
        let active = self.store.task_storage.get_active_phase().await?;

        if let Some(phase_name) = active {
            if let Some(phase) = cache.get(&phase_name) {
                let stats = phase.get_stats();
                return Ok(TaskStatistics {
                    total: stats.total,
                    pending: stats.pending,
                    in_progress: stats.in_progress,
                    done: stats.done,
                    blocked: stats.blocked,
                });
            }
        }
        Ok(TaskStatistics::default())
    }
}
```

---

### Phase 4: CLI Alignment ✅ COMPLETE

**Goal**: Ensure both CLIs work with same data

**Status**: Complete! Added full task CLI commands to Descartes.

**Changes completed**:
- [x] Created `descartes/cli/src/commands/tasks.rs` with subcommands:
  - `list` - List tasks with status/priority/search filters and format options
  - `show` - Show detailed task info by ID
  - `next` - Show next actionable task (with `--id-only` option)
  - `stats` - Display phase statistics
  - `use` - Set active phase by tag
  - `phases` - List all available phases
- [x] Updated `descartes/cli/src/commands/mod.rs` to include tasks module
- [x] Updated `descartes/cli/src/main.rs` with Tasks subcommand
- [x] Build verified successful

**Command parity achieved**:
| SCUD Command | Descartes Command | Status |
|--------------|-------------------|--------|
| `scud list` | `descartes tasks list` | ✅ |
| `scud show <id>` | `descartes tasks show <id>` | ✅ |
| `scud next` | `descartes tasks next` | ✅ |
| `scud stats` | `descartes tasks stats` | ✅ |
| `scud use <tag>` | `descartes tasks use <tag>` | ✅ |
| `scud phases` | `descartes tasks phases` | ✅ |

**Original plan (for reference)**:

#### 4.1 Descartes CLI Task Commands

**File**: `descartes/cli/src/commands/tasks.rs`

```rust
/// List tasks in active phase
pub async fn list_tasks(store: &DescartesStateStore, filter: Option<TaskStatus>) -> Result<()> {
    store.refresh_tasks().await?;

    let query = match filter {
        Some(status) => TaskQueryBuilder::new().with_status(status),
        None => TaskQueryBuilder::new(),
    };

    let tasks = store.execute_query(&query).await?;

    for task in tasks {
        println!("{} | {} | {:?} | {}",
            task.id,
            task.title,
            task.status,
            task.complexity
        );
    }

    Ok(())
}
```

#### 4.2 Command Parity

| Command | SCUD CLI | Descartes CLI | Notes |
|---------|----------|---------------|-------|
| Init | `scud init` | `descartes init --with-tasks` | Both create `.scud/` |
| List | `scud list` | `descartes tasks list` | Same output |
| Show | `scud show <id>` | `descartes tasks show <id>` | Same output |
| Status | `scud set-status` | `descartes tasks status` | Same behavior |
| Next | `scud next` | `descartes tasks next` | Same logic |
| Stats | `scud stats` | `descartes tasks stats` | Same output |

---

### Phase 5: Event Integration ✅ COMPLETE

**Goal**: Descartes event system works with SCG-based tasks

**Status**: Complete! Created `ScgTaskEventEmitter` with file watching and event emission.

**Changes completed**:
- [x] Created `descartes/daemon/src/scg_task_event_emitter.rs` with:
  - `ScgTaskEventEmitter` - watches `.taskmaster/tasks/tasks.json` for changes
  - File system watching via `notify` crate (kqueue on macOS)
  - Change detection by comparing with cached task state
  - Event emission for Created, Updated (Progress), and Deleted (Cancelled) tasks
  - Configurable debouncing to handle rapid file saves
  - Thread-safe with `Arc<RwLock<...>>` patterns
- [x] Updated `descartes/daemon/src/lib.rs` to export new SCG event emitter
- [x] Added `notify = { version = "6.1", features = ["macos_kqueue"] }` to daemon Cargo.toml
- [x] Unit tests for emitter creation
- [x] Build verified successful

**Key features**:
- Watches directory for `tasks.json` file changes
- Debounces rapid file saves (configurable interval)
- Emits `TaskEvent` wrapped in `DescartesEvent` to EventBus
- Supports both verbose and quiet logging modes
- Clean shutdown handling with channel-based signaling

**Original plan (for reference)**:

**File**: `descartes/daemon/src/task_event_emitter.rs`

```rust
pub struct TaskEventEmitter {
    store: Arc<DescartesStateStore>,
    event_bus: Arc<EventBus>,

    // Cache previous state for change detection
    previous_state: RwLock<HashMap<String, Task>>,
}

impl TaskEventEmitter {
    /// Watch for file changes and emit events
    pub async fn watch_scg_file(&self) -> Result<()> {
        let watcher = notify::recommended_watcher(|res| {
            // On .scg file change, refresh and emit events
        })?;

        watcher.watch(
            self.store.task_storage.tasks_file(),
            RecursiveMode::NonRecursive
        )?;

        Ok(())
    }

    /// Detect changes and emit events
    async fn on_file_changed(&self) -> Result<()> {
        self.store.refresh_tasks().await?;

        let current = self.store.get_all_tasks().await?;
        let previous = self.previous_state.read().await;

        for task in &current {
            match previous.get(&task.id) {
                None => {
                    // New task
                    self.event_bus.emit(TaskChangeEvent::Created {
                        task_id: task.id.clone(),
                        task: Some(task.clone()),
                        timestamp: Utc::now().timestamp(),
                    }).await;
                }
                Some(prev) if prev.status != task.status => {
                    // Status changed
                    self.event_bus.emit(TaskChangeEvent::Updated {
                        task_id: task.id.clone(),
                        task: Some(task.clone()),
                        previous_status: Some(prev.status.as_str().to_string()),
                        new_status: task.status.as_str().to_string(),
                        timestamp: Utc::now().timestamp(),
                    }).await;
                }
                _ => {}
            }
        }

        // Update cache
        *self.previous_state.write().await = current
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        Ok(())
    }
}
```

---

## Success Criteria

### Automated Verification

```bash
# Both CLIs produce identical output
scud list > /tmp/scud_list.txt
descartes tasks list > /tmp/descartes_list.txt
diff /tmp/scud_list.txt /tmp/descartes_list.txt  # Should be empty

# SCG file is valid after Descartes operations
descartes tasks create "Test task" --complexity 5
scud show $(scud next --id-only)  # Should work

# Round-trip preservation
scud list --json > /tmp/before.json
descartes tasks sync  # Force refresh
scud list --json > /tmp/after.json
diff /tmp/before.json /tmp/after.json  # Should be empty
```

### Manual Verification

1. Create tasks with SCUD, view in Descartes GUI
2. Edit `.scg` file directly, both CLIs see changes
3. Run parallel agents (one SCUD, one Descartes), no conflicts
4. Git diff of `.scg` file shows readable changes

---

## What We're NOT Doing

1. **SQLite for task storage** - SCG files are primary
2. **Complex SQL queries** - In-memory query engine instead
3. **Separate task models** - Single unified model
4. **Backward compatibility** - No existing Descartes users
5. **MCP requirement** - SCUD MCP is optional enhancement

---

## File References

### SCUD (to modify)
- `scud-cli/src/models/task.rs:19585-19592` - Priority enum (add Critical)
- `scud-cli/src/formats/scg.rs:10149-10164` - Priority code mapping

### Descartes (to modify)
- `descartes/core/src/traits.rs:370-483` - Task model (replace)
- `descartes/core/src/task_queries.rs:1-987` - Query system (rewrite)
- `descartes/core/src/state_store.rs:511-563` - Task persistence (replace)
- `descartes/daemon/src/task_event_emitter.rs:82-396` - Event emission (adapt)

---

## SCUD Agent Prompt

The following prompt should be given to an agent working on the SCUD repository:

```markdown
# Task: Add Critical Priority Level to SCUD

## Context
We're unifying SCUD and Descartes task management systems. Descartes has 4 priority levels (Low, Medium, High, Critical) while SCUD has 3 (Low, Medium, High). We need to add Critical to SCUD for full compatibility.

## Changes Required

### 1. Update Priority Enum
**File**: `scud-cli/src/models/task.rs`

Find the Priority enum (around line 19585-19592) and add Critical:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Critical,  // NEW: Add this variant
}
```

### 2. Update SCG Priority Code Mapping
**File**: `scud-cli/src/formats/scg.rs`

Find `priority_to_code` function (around line 10149) and add Critical mapping:

```rust
fn priority_to_code(priority: &Priority) -> char {
    match priority {
        Priority::High => 'H',
        Priority::Medium => 'M',
        Priority::Low => 'L',
        Priority::Critical => 'C',  // NEW: Add this case
    }
}
```

Find `code_to_priority` function and add the reverse mapping:

```rust
fn code_to_priority(code: char) -> Priority {
    match code {
        'H' => Priority::High,
        'M' => Priority::Medium,
        'L' => Priority::Low,
        'C' => Priority::Critical,  // NEW: Add this case
        _ => Priority::Medium,  // Default
    }
}
```

### 3. Update MCP Tool Schema (if applicable)
**File**: `scud-mcp/src/tools/task.ts` (or similar)

If there's a priority enum in the MCP tool definitions, add "critical" to the allowed values.

### 4. Update Tests
Add test cases for Critical priority:
- Serialization/deserialization round-trip
- SCG format parsing with 'C' code
- Priority comparison/ordering (Critical > High > Medium > Low)

### 5. Update Documentation
- README.md - mention 4 priority levels
- Any help text in CLI that lists priority options

## Verification

```bash
# Test the change
cargo test -p scud

# Verify SCG round-trip
echo "1 | Test task | P | 5 | C" | scud parse-scg-line
scud show 1  # Should show priority: critical
```

## Notes
- Critical should be the highest priority (above High)
- Default priority remains Medium
- SCG code 'C' is used (not 'X' which is for Expanded status)
- This is for compatibility with Descartes - no breaking changes to existing SCUD functionality
```

---

## Timeline Estimate

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: SCUD Priority Update | Small | None |
| Phase 2: Shared Library | Medium | Phase 1 |
| Phase 3: Descartes Integration | Large | Phase 2 |
| Phase 4: CLI Alignment | Medium | Phase 3 |
| Phase 5: Event Integration | Medium | Phase 3 |

---

## Open Questions

None - all decisions made:
- Priority: 4 levels (adding Critical to SCUD)
- IDs: Strings (flexible format)
- Storage: SCG primary, SQLite for events only
- No migration needed (no existing users)
