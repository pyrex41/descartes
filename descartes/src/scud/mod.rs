//! SCUD integration for task management
//!
//! Uses the scud-cli crate directly for:
//! - Task loading/saving
//! - Dependency tracking
//! - Wave visualization (parallel execution potential)

use std::path::PathBuf;

// Re-export scud types for convenience
pub use scud::models::{Phase, Task, TaskStatus, Priority};
pub use scud::storage::Storage;

use crate::{Config, Error, Result};

/// Get the next ready task from SCUD
pub fn next(config: &Config) -> Result<Option<Task>> {
    let storage = create_storage(config)?;

    // Load active group/phase
    let phase = match storage.load_active_group() {
        Ok(p) => p,
        Err(e) => {
            // If no active group, try to load default tasks
            tracing::debug!("No active group: {}", e);
            return Ok(None);
        }
    };

    // Find next task with all dependencies met
    Ok(phase.find_next_task().cloned())
}

/// Mark a task as complete
pub fn complete(config: &Config, task_id: &str) -> Result<()> {
    let storage = create_storage(config)?;

    // Get active group tag
    let group_tag = storage.get_active_group()
        .map_err(|e| Error::Subagent(format!("Failed to get active group: {}", e)))?
        .ok_or_else(|| Error::Subagent("No active group set".to_string()))?;

    // Load the group
    let mut phase = storage.load_group(&group_tag)
        .map_err(|e| Error::Subagent(format!("Failed to load group: {}", e)))?;

    // Find and update the task
    if let Some(task) = phase.get_task_mut(task_id) {
        task.set_status(TaskStatus::Done);

        // Save the updated group
        storage.update_group(&group_tag, &phase)
            .map_err(|e| Error::Subagent(format!("Failed to save group: {}", e)))?;

        Ok(())
    } else {
        Err(Error::Subagent(format!("Task not found: {}", task_id)))
    }
}

/// Get task waves (parallel execution potential)
/// Returns groups of task IDs that can be executed in parallel
pub fn waves(config: &Config) -> Result<Vec<Vec<String>>> {
    let storage = create_storage(config)?;

    let phase = match storage.load_active_group() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };

    // Calculate waves using Kahn's algorithm
    calculate_waves(&phase)
}

/// Calculate execution waves from a phase
fn calculate_waves(phase: &Phase) -> Result<Vec<Vec<String>>> {
    use std::collections::{HashMap, HashSet};

    // Build dependency graph for pending/in-progress tasks only
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    let done_ids: HashSet<_> = phase.tasks.iter()
        .filter(|t| t.status == TaskStatus::Done)
        .map(|t| t.id.clone())
        .collect();

    for task in &phase.tasks {
        // Skip done tasks and subtasks
        if task.status == TaskStatus::Done || task.is_subtask() {
            continue;
        }

        // Initialize in-degree
        in_degree.entry(task.id.clone()).or_insert(0);

        // Count dependencies on non-done tasks
        for dep in &task.dependencies {
            if !done_ids.contains(dep) {
                *in_degree.entry(task.id.clone()).or_insert(0) += 1;
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(task.id.clone());
            }
        }
    }

    // Kahn's algorithm by waves
    let mut waves = Vec::new();
    let mut current_wave: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    while !current_wave.is_empty() {
        current_wave.sort(); // Deterministic ordering
        waves.push(current_wave.clone());

        let mut next_wave = Vec::new();
        for task_id in &current_wave {
            if let Some(deps) = dependents.get(task_id) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            next_wave.push(dep.clone());
                        }
                    }
                }
            }
        }

        current_wave = next_wave;
    }

    Ok(waves)
}

/// Set task status
pub fn set_status(config: &Config, task_id: &str, status: TaskStatus) -> Result<()> {
    let storage = create_storage(config)?;

    let group_tag = storage.get_active_group()
        .map_err(|e| Error::Subagent(format!("Failed to get active group: {}", e)))?
        .ok_or_else(|| Error::Subagent("No active group set".to_string()))?;

    let mut phase = storage.load_group(&group_tag)
        .map_err(|e| Error::Subagent(format!("Failed to load group: {}", e)))?;

    if let Some(task) = phase.get_task_mut(task_id) {
        task.set_status(status);

        storage.update_group(&group_tag, &phase)
            .map_err(|e| Error::Subagent(format!("Failed to save group: {}", e)))?;

        Ok(())
    } else {
        Err(Error::Subagent(format!("Task not found: {}", task_id)))
    }
}

/// Get all tasks from the active group
pub fn list_tasks(config: &Config) -> Result<Vec<Task>> {
    let storage = create_storage(config)?;

    match storage.load_active_group() {
        Ok(phase) => Ok(phase.tasks),
        Err(_) => Ok(Vec::new()),
    }
}

/// Get a specific task by ID
pub fn get_task(config: &Config, task_id: &str) -> Result<Option<Task>> {
    let storage = create_storage(config)?;

    match storage.load_active_group() {
        Ok(phase) => Ok(phase.get_task(task_id).cloned()),
        Err(_) => Ok(None),
    }
}

/// Get tasks ready to work on (pending with dependencies met)
pub fn ready_tasks(config: &Config) -> Result<Vec<Task>> {
    let storage = create_storage(config)?;

    let phase = match storage.load_active_group() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };

    let ready: Vec<Task> = phase.tasks.iter()
        .filter(|t| {
            t.status == TaskStatus::Pending &&
            t.has_dependencies_met(&phase.tasks) &&
            !t.is_subtask()
        })
        .cloned()
        .collect();

    Ok(ready)
}

/// Get blocked tasks (pending but dependencies not met)
pub fn blocked_tasks(config: &Config) -> Result<Vec<Task>> {
    let storage = create_storage(config)?;

    let phase = match storage.load_active_group() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };

    let blocked: Vec<Task> = phase.tasks.iter()
        .filter(|t| {
            t.status == TaskStatus::Pending &&
            !t.has_dependencies_met(&phase.tasks) &&
            !t.is_subtask()
        })
        .cloned()
        .collect();

    Ok(blocked)
}

/// Create a storage instance from config
fn create_storage(config: &Config) -> Result<Storage> {
    let project_root = if config.scud.task_file.is_absolute() {
        config.scud.task_file.parent()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
    } else {
        Some(std::env::current_dir()?)
    };

    Ok(Storage::new(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_waves_simple() {
        let mut phase = Phase::new("test".to_string());

        // Task 1 has no deps
        let task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());

        // Task 2 depends on 1
        let mut task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());
        task2.dependencies = vec!["1".to_string()];

        // Task 3 depends on 1
        let mut task3 = Task::new("3".to_string(), "Task 3".to_string(), "".to_string());
        task3.dependencies = vec!["1".to_string()];

        // Task 4 depends on 2 and 3
        let mut task4 = Task::new("4".to_string(), "Task 4".to_string(), "".to_string());
        task4.dependencies = vec!["2".to_string(), "3".to_string()];

        phase.add_task(task1);
        phase.add_task(task2);
        phase.add_task(task3);
        phase.add_task(task4);

        let waves = calculate_waves(&phase).unwrap();

        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["1"]);
        assert_eq!(waves[1], vec!["2", "3"]); // Can run in parallel
        assert_eq!(waves[2], vec!["4"]);
    }
}
