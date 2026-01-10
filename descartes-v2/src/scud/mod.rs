//! SCUD integration for task management
//!
//! SCUD provides DAG-driven task management with:
//! - Dependency tracking
//! - Wave visualization (parallel execution potential)
//! - SCG format for token-efficient storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

use crate::{Config, Error, Result};

/// A task from the SCUD task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: String,
    /// Task title
    pub title: String,
    /// Task description
    pub description: String,
    /// Current status
    pub status: TaskStatus,
    /// Dependencies (task IDs that must complete first)
    pub dependencies: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
    Deferred,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in-progress"),
            TaskStatus::Done => write!(f, "done"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Deferred => write!(f, "deferred"),
        }
    }
}

/// Get the next ready task from SCUD
pub fn next(config: &Config) -> Result<Option<Task>> {
    if config.scud.embedded {
        // Use embedded SCUD logic
        next_embedded(config)
    } else {
        // Shell out to scud binary
        next_shell(config)
    }
}

/// Get next task using embedded logic
fn next_embedded(config: &Config) -> Result<Option<Task>> {
    let tasks = load_tasks(config)?;

    // Find tasks that are pending and have all dependencies done
    let done_ids: std::collections::HashSet<_> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .map(|t| t.id.clone())
        .collect();

    for task in &tasks {
        if task.status != TaskStatus::Pending {
            continue;
        }

        let deps_satisfied = task.dependencies.iter().all(|d| done_ids.contains(d));

        if deps_satisfied {
            return Ok(Some(task.clone()));
        }
    }

    Ok(None)
}

/// Get next task by shelling out to scud binary
fn next_shell(config: &Config) -> Result<Option<Task>> {
    let binary = config
        .scud
        .binary
        .as_deref()
        .unwrap_or("scud");

    let output = Command::new(binary)
        .args(["next", "--json"])
        .current_dir(std::env::current_dir()?)
        .output()
        .map_err(|e| Error::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No tasks ready") {
            return Ok(None);
        }
        return Err(Error::Subagent(format!(
            "scud next failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let task: Task = serde_json::from_str(&stdout)
        .map_err(|e| Error::Json(e))?;

    Ok(Some(task))
}

/// Mark a task as complete
pub fn complete(config: &Config, task_id: &str) -> Result<()> {
    if config.scud.embedded {
        complete_embedded(config, task_id)
    } else {
        complete_shell(config, task_id)
    }
}

/// Mark complete using embedded logic
fn complete_embedded(config: &Config, task_id: &str) -> Result<()> {
    let mut tasks = load_tasks(config)?;

    if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
        task.status = TaskStatus::Done;
        save_tasks(config, &tasks)?;
        Ok(())
    } else {
        Err(Error::Subagent(format!("Task not found: {}", task_id)))
    }
}

/// Mark complete by shelling out
fn complete_shell(config: &Config, task_id: &str) -> Result<()> {
    let binary = config
        .scud
        .binary
        .as_deref()
        .unwrap_or("scud");

    let output = Command::new(binary)
        .args(["done", task_id])
        .current_dir(std::env::current_dir()?)
        .output()
        .map_err(|e| Error::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Subagent(format!(
            "scud done failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Get task waves (parallel execution potential)
pub fn waves(config: &Config) -> Result<Vec<Vec<String>>> {
    if config.scud.embedded {
        waves_embedded(config)
    } else {
        waves_shell(config)
    }
}

/// Calculate waves using embedded logic
fn waves_embedded(config: &Config) -> Result<Vec<Vec<String>>> {
    let tasks = load_tasks(config)?;

    // Build dependency graph
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for task in &tasks {
        if task.status == TaskStatus::Done {
            continue;
        }

        in_degree.entry(task.id.clone()).or_insert(0);

        for dep in &task.dependencies {
            // Only count dependencies on non-done tasks
            if tasks.iter().any(|t| t.id == *dep && t.status != TaskStatus::Done) {
                *in_degree.entry(task.id.clone()).or_insert(0) += 1;
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(task.id.clone());
            }
        }
    }

    // Kahn's algorithm for topological sort by waves
    let mut waves = Vec::new();
    let mut current_wave: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    while !current_wave.is_empty() {
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

/// Get waves by shelling out
fn waves_shell(config: &Config) -> Result<Vec<Vec<String>>> {
    let binary = config
        .scud
        .binary
        .as_deref()
        .unwrap_or("scud");

    let output = Command::new(binary)
        .args(["waves", "--json"])
        .current_dir(std::env::current_dir()?)
        .output()
        .map_err(|e| Error::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Subagent(format!(
            "scud waves failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let waves: Vec<Vec<String>> = serde_json::from_str(&stdout)
        .map_err(|e| Error::Json(e))?;

    Ok(waves)
}

/// Load tasks from the SCUD task file
fn load_tasks(config: &Config) -> Result<Vec<Task>> {
    let path = &config.scud.task_file;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)?;

    // Check if it's SCG or JSON format
    if path.extension().map(|e| e == "scg").unwrap_or(false) {
        parse_scg_tasks(&content)
    } else {
        let tasks: Vec<Task> = serde_json::from_str(&content)
            .map_err(|e| Error::Json(e))?;
        Ok(tasks)
    }
}

/// Save tasks to the SCUD task file
fn save_tasks(config: &Config, tasks: &[Task]) -> Result<()> {
    let path = &config.scud.task_file;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if path.extension().map(|e| e == "scg").unwrap_or(false) {
        let content = tasks_to_scg(tasks);
        std::fs::write(path, content)?;
    } else {
        let content = serde_json::to_string_pretty(tasks)
            .map_err(|e| Error::Json(e))?;
        std::fs::write(path, content)?;
    }

    Ok(())
}

/// Parse tasks from SCG format
fn parse_scg_tasks(content: &str) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    let mut current_task: Option<Task> = None;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Task line: ID:status "title" [deps]
        if let Some((id_status, rest)) = line.split_once(' ') {
            let parts: Vec<&str> = id_status.split(':').collect();
            if parts.len() >= 2 {
                // Save previous task
                if let Some(task) = current_task.take() {
                    tasks.push(task);
                }

                let id = parts[0].to_string();
                let status = match parts[1] {
                    "pending" => TaskStatus::Pending,
                    "in-progress" => TaskStatus::InProgress,
                    "done" => TaskStatus::Done,
                    "blocked" => TaskStatus::Blocked,
                    "deferred" => TaskStatus::Deferred,
                    _ => TaskStatus::Pending,
                };

                // Parse title and dependencies
                let (title, deps) = if let Some((t, d)) = rest.rsplit_once('[') {
                    let deps_str = d.trim_end_matches(']');
                    let deps: Vec<String> = deps_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    (t.trim().trim_matches('"').to_string(), deps)
                } else {
                    (rest.trim().trim_matches('"').to_string(), Vec::new())
                };

                current_task = Some(Task {
                    id,
                    title,
                    description: String::new(),
                    status,
                    dependencies: deps,
                    tags: Vec::new(),
                });
            }
        }
    }

    // Don't forget the last task
    if let Some(task) = current_task {
        tasks.push(task);
    }

    Ok(tasks)
}

/// Convert tasks to SCG format
fn tasks_to_scg(tasks: &[Task]) -> String {
    let mut out = String::new();
    out.push_str("@tasks\n");

    for task in tasks {
        let deps = if task.dependencies.is_empty() {
            String::new()
        } else {
            format!(" [{}]", task.dependencies.join(", "))
        };

        out.push_str(&format!(
            "{}:{} \"{}\"{}\n",
            task.id, task.status, task.title, deps
        ));

        if !task.description.is_empty() {
            out.push_str(&format!("  # {}\n", task.description));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scg_tasks() {
        let content = r#"
@tasks
1:pending "First task"
2:pending "Second task" [1]
3:done "Completed task"
"#;

        let tasks = parse_scg_tasks(content).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, "1");
        assert_eq!(tasks[0].title, "First task");
        assert_eq!(tasks[1].dependencies, vec!["1"]);
        assert_eq!(tasks[2].status, TaskStatus::Done);
    }

    #[test]
    fn test_waves_calculation() {
        // This would require setting up a mock config
    }
}
