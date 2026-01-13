-- Migration: Enhance Task Model with Priority, Complexity, and Dependencies
-- Version: 5
-- Description: Add priority, complexity, and dependencies fields to tasks table

-- Add new columns to tasks table
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';
ALTER TABLE tasks ADD COLUMN complexity TEXT NOT NULL DEFAULT 'moderate';
ALTER TABLE tasks ADD COLUMN dependencies TEXT DEFAULT '[]';

-- Create task_dependencies junction table for better dependency management
CREATE TABLE IF NOT EXISTS task_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id, depends_on_task_id)
);

-- Create indexes for new fields
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_complexity ON tasks(complexity);
CREATE INDEX IF NOT EXISTS idx_tasks_priority_status ON tasks(priority, status);
CREATE INDEX IF NOT EXISTS idx_tasks_complexity_status ON tasks(complexity, status);

-- Create indexes for task_dependencies table
CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id);

-- Create a view for tasks with dependency information
CREATE VIEW IF NOT EXISTS tasks_with_dependencies AS
SELECT
    t.*,
    GROUP_CONCAT(td.depends_on_task_id) as dependency_ids,
    COUNT(td.depends_on_task_id) as dependency_count
FROM tasks t
LEFT JOIN task_dependencies td ON t.id = td.task_id
GROUP BY t.id;

-- Add comments (SQLite doesn't support COMMENT, but we document here)
-- priority values: 'low', 'medium', 'high', 'critical'
-- complexity values: 'trivial', 'simple', 'moderate', 'complex', 'epic'
-- dependencies: JSON array of task IDs (UUIDs) that this task depends on
