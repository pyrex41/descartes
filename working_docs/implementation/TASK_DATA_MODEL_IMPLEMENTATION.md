# Phase 3:4.1 - Task Data Model and SQLite Schema Implementation

**Date:** 2025-11-24
**Phase:** Phase 3 - Parallel Execution and Task Management
**Task:** 4.1 - Define Task Data Model and SQLite Schema
**Status:** ✅ Completed

## Overview

This document details the implementation of an enhanced Task data model with comprehensive fields for priority, complexity, and dependency management, along with the corresponding SQLite schema and database operations.

## Implementation Summary

### 1. Task Data Model (`/home/user/descartes/descartes/core/src/traits.rs`)

#### Enhanced Task Structure

```rust
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub complexity: TaskComplexity,
    pub assigned_to: Option<String>,
    pub dependencies: Vec<Uuid>, // IDs of tasks this task depends on
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<Value>,
}
```

#### New Enum Types

**TaskPriority**
- `Low` - Low priority tasks
- `Medium` - Standard priority (default)
- `High` - High priority tasks
- `Critical` - Critical/urgent tasks

Features:
- Implements `Default` (returns `Medium`)
- Implements `Display` for string conversion
- Implements `FromStr` for parsing
- Serializable/Deserializable with serde
- Comparable and Orderable

**TaskComplexity**
- `Trivial` - Less than 1 hour
- `Simple` - 1-4 hours
- `Moderate` - 1-2 days (default)
- `Complex` - 3-5 days
- `Epic` - More than 1 week

Features:
- Implements `Default` (returns `Moderate`)
- Implements `Display` for string conversion
- Implements `FromStr` for parsing
- Serializable/Deserializable with serde
- Comparable and Orderable

### 2. Database Schema

#### Main Tasks Table

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'medium',
    complexity TEXT NOT NULL DEFAULT 'moderate',
    assigned_to TEXT,
    dependencies TEXT DEFAULT '[]',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT,
    created_timestamp INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### Task Dependencies Junction Table

```sql
CREATE TABLE IF NOT EXISTS task_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id, depends_on_task_id)
);
```

### 3. Database Indexes

Performance indexes have been added for efficient querying:

```sql
-- Priority and complexity indexes
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_complexity ON tasks(complexity);

-- Composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_tasks_priority_status ON tasks(priority, status);
CREATE INDEX IF NOT EXISTS idx_tasks_complexity_status ON tasks(complexity, status);

-- Dependency table indexes
CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id);
```

### 4. Migration Files

**File:** `/home/user/descartes/descartes/core/migrations/005_enhance_task_model.sql`

Migration version: 5
Description: Add priority, complexity, and dependencies fields to tasks table

The migration includes:
- ALTER TABLE statements for backward compatibility
- CREATE TABLE for task_dependencies junction table
- Index creation for performance optimization
- Comments documenting valid values for enums

### 5. StateStore Implementation Updates

**File:** `/home/user/descartes/descartes/core/src/state_store.rs`

#### Updated Methods

**save_task()**
- Serializes dependencies as JSON array
- Saves task to main tasks table with all new fields
- Manages task_dependencies junction table
- Deletes old dependencies and inserts new ones atomically

**get_task()**
- Retrieves all task fields including priority, complexity, and dependencies
- Deserializes priority and complexity from strings using FromStr
- Deserializes dependencies from JSON array
- Handles defaults gracefully for backward compatibility

**get_tasks()**
- Returns all tasks with complete field set
- Orders by updated_at DESC for recent-first retrieval
- Applies same deserialization logic as get_task()

**apply_migrations()**
- Added migration #5 for task model enhancements
- Creates task_dependencies table
- Applies all indexes in a single migration

### 6. Test Updates

Updated test cases to include new fields:
- `test_save_and_load_task()` now validates priority and complexity
- Tests verify proper serialization/deserialization
- Tests ensure backward compatibility with defaults

## File Changes Summary

### Modified Files
1. `/home/user/descartes/descartes/core/src/traits.rs`
   - Added TaskPriority enum with implementations
   - Added TaskComplexity enum with implementations
   - Updated Task struct with new fields

2. `/home/user/descartes/descartes/core/src/state_store.rs`
   - Updated tasks table schema
   - Added migration #5
   - Updated save_task method
   - Updated get_task method
   - Updated get_tasks method
   - Updated test cases

### New Files
1. `/home/user/descartes/descartes/core/migrations/005_enhance_task_model.sql`
   - Standalone migration file for reference
   - Can be applied manually if needed

## Database Schema Design Decisions

### Priority Levels
The four-tier priority system (Low, Medium, High, Critical) provides:
- Clear prioritization without over-complication
- Natural ordering for task scheduling
- Alignment with common project management practices

### Complexity Estimates
The five-tier complexity system provides:
- Granular effort estimation
- Useful for resource allocation
- Helps identify epics that need breakdown

### Dependencies Storage
Dependencies are stored in two ways:
1. **JSON Array in tasks.dependencies** - For quick access and serialization
2. **task_dependencies Table** - For relational integrity and advanced queries

This dual approach provides:
- Fast reads (JSON array)
- Referential integrity (foreign keys)
- Cascade deletes (no orphaned dependencies)
- Ability to query dependency graphs

## Query Helpers and Indexes

### Common Query Patterns Optimized

1. **Find high-priority pending tasks:**
   ```sql
   SELECT * FROM tasks
   WHERE priority = 'high' AND status = 'todo'
   ORDER BY created_at;
   ```
   Uses: `idx_tasks_priority_status`

2. **Find complex in-progress tasks:**
   ```sql
   SELECT * FROM tasks
   WHERE complexity = 'complex' AND status = 'in_progress';
   ```
   Uses: `idx_tasks_complexity_status`

3. **Find all tasks depending on a specific task:**
   ```sql
   SELECT t.* FROM tasks t
   JOIN task_dependencies td ON t.id = td.task_id
   WHERE td.depends_on_task_id = ?;
   ```
   Uses: `idx_task_dependencies_depends_on`

4. **Find all dependencies for a task:**
   ```sql
   SELECT t.* FROM tasks t
   JOIN task_dependencies td ON t.id = td.depends_on_task_id
   WHERE td.task_id = ?;
   ```
   Uses: `idx_task_dependencies_task_id`

## Serialization/Deserialization

### Priority and Complexity
- Stored as lowercase strings in database
- Converted using FromStr/Display traits
- Defaults applied on parse failure (Medium/Moderate)
- Fully compatible with serde for JSON API responses

### Dependencies
- Stored as JSON array of UUIDs: `["uuid1", "uuid2", ...]`
- Deserialized into Vec<Uuid>
- Empty array (`[]`) for tasks with no dependencies
- Synced with task_dependencies table on save

## Testing Strategy

### Unit Tests
- Task creation with all fields
- Priority/Complexity enum parsing
- Default value handling
- Serialization round-trips

### Integration Tests
- Full save/load cycles
- Dependency management
- Query performance with indexes
- Migration application

### Future Testing
- Dependency cycle detection
- Cascade delete behavior
- Concurrent task updates
- Large-scale performance tests

## Migration Strategy

### Backward Compatibility
The migration uses ALTER TABLE with DEFAULT values to ensure:
- Existing tasks get default priority (medium)
- Existing tasks get default complexity (moderate)
- Existing tasks get empty dependencies array ([])
- No data loss during migration

### Rollback Strategy
If rollback is needed:
1. Drop task_dependencies table
2. Drop new indexes
3. Remove added columns (SQLite requires table recreation)

## Performance Considerations

### Indexes
All query patterns have supporting indexes:
- Single-column indexes for simple filters
- Composite indexes for common multi-column queries
- Foreign key indexes for join performance

### Dependencies
- Junction table enables efficient dependency queries
- JSON array enables fast dependency access without joins
- Trade-off: Small storage overhead for query flexibility

### Expected Performance
- Task retrieval: O(1) with primary key
- Priority/Status queries: O(log n) with composite index
- Dependency traversal: O(d) where d is dependency depth
- Bulk task queries: O(n log n) with indexed ordering

## Future Enhancements

### Potential Additions
1. **Task Tags/Labels** - For categorization beyond status
2. **Due Dates** - For time-based scheduling
3. **Estimated Duration** - More granular than complexity
4. **Actual Duration** - For tracking and improving estimates
5. **Parent Task** - For hierarchical task breakdown
6. **Assignee Team** - For multi-agent assignment
7. **Task History** - For audit trail and analytics

### Advanced Features
1. **Dependency Cycle Detection** - Prevent circular dependencies
2. **Critical Path Analysis** - Identify blocking tasks
3. **Resource Leveling** - Balance workload across agents
4. **Task Templates** - Reusable task patterns
5. **Bulk Operations** - Efficiently update multiple tasks

## Conclusion

The enhanced Task data model provides a solid foundation for task management in the Descartes orchestration system. The implementation includes:

✅ Comprehensive data model with priority, complexity, and dependencies
✅ Robust SQLite schema with proper indexes
✅ Efficient serialization/deserialization
✅ Migration support for schema evolution
✅ Backward compatibility
✅ Test coverage
✅ Performance optimization
✅ Clear documentation

This implementation satisfies all requirements for Phase 3:4.1 and provides a strong foundation for the parallel execution and task management features in Phase 3.
