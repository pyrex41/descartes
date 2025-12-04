//! Tool executors for Descartes agents.
//!
//! These handle the actual execution of tool calls returned by the LLM.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{debug, info};

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Whether the tool succeeded
    pub success: bool,
    /// Output or error message
    pub output: String,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Execute the `read` tool.
pub fn execute_read(args: &Value, working_dir: &Path) -> ToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: path".to_string(),
                metadata: None,
            }
        }
    };

    let path = resolve_path(path_str, working_dir);

    // Check if file exists
    if !path.exists() {
        return ToolResult {
            success: false,
            output: format!("File not found: {}", path.display()),
            metadata: None,
        };
    }

    // Check if it's a directory
    if path.is_dir() {
        return ToolResult {
            success: false,
            output: format!("{} is a directory, not a file", path.display()),
            metadata: None,
        };
    }

    // Read file content
    match fs::read_to_string(&path) {
        Ok(content) => {
            let offset = args
                .get("offset")
                .and_then(|v| v.as_i64())
                .map(|v| v.max(1) as usize)
                .unwrap_or(1);
            let limit = args
                .get("limit")
                .and_then(|v| v.as_i64())
                .map(|v| v as usize)
                .unwrap_or(2000);

            // Apply offset and limit
            let lines: Vec<&str> = content.lines().collect();
            let start = (offset - 1).min(lines.len());
            let end = (start + limit).min(lines.len());
            let selected: Vec<String> = lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>5} | {}", start + i + 1, line))
                .collect();

            ToolResult {
                success: true,
                output: selected.join("\n"),
                metadata: Some(
                    [
                        ("path".to_string(), path.display().to_string()),
                        ("lines".to_string(), (end - start).to_string()),
                        ("total_lines".to_string(), lines.len().to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to read file: {}", e),
            metadata: None,
        },
    }
}

/// Execute the `write` tool.
pub fn execute_write(args: &Value, working_dir: &Path) -> ToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: path".to_string(),
                metadata: None,
            }
        }
    };

    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: content".to_string(),
                metadata: None,
            }
        }
    };

    let path = resolve_path(path_str, working_dir);

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult {
                    success: false,
                    output: format!("Failed to create directory: {}", e),
                    metadata: None,
                };
            }
        }
    }

    // Write the file
    match fs::write(&path, content) {
        Ok(_) => {
            let lines = content.lines().count();
            let bytes = content.len();
            ToolResult {
                success: true,
                output: format!(
                    "Wrote {} lines ({} bytes) to {}",
                    lines,
                    bytes,
                    path.display()
                ),
                metadata: Some(
                    [
                        ("path".to_string(), path.display().to_string()),
                        ("lines".to_string(), lines.to_string()),
                        ("bytes".to_string(), bytes.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to write file: {}", e),
            metadata: None,
        },
    }
}

/// Execute the `edit` tool.
pub fn execute_edit(args: &Value, working_dir: &Path) -> ToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: path".to_string(),
                metadata: None,
            }
        }
    };

    let old_text = match args.get("old_text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: old_text".to_string(),
                metadata: None,
            }
        }
    };

    let new_text = match args.get("new_text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: new_text".to_string(),
                metadata: None,
            }
        }
    };

    let path = resolve_path(path_str, working_dir);

    // Read current content
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return ToolResult {
                success: false,
                output: format!("Failed to read file: {}", e),
                metadata: None,
            }
        }
    };

    // Check if old_text exists (exact match)
    if !content.contains(old_text) {
        return ToolResult {
            success: false,
            output: format!(
                "old_text not found in file. The text must match exactly including whitespace.\nSearched for:\n{}\n\nIn file: {}",
                old_text,
                path.display()
            ),
            metadata: None,
        };
    }

    // Count occurrences
    let occurrences = content.matches(old_text).count();
    if occurrences > 1 {
        return ToolResult {
            success: false,
            output: format!(
                "old_text found {} times in file. It must be unique. Add more context to make it unique.",
                occurrences
            ),
            metadata: None,
        };
    }

    // Perform replacement
    let new_content = content.replacen(old_text, new_text, 1);

    // Write back
    match fs::write(&path, &new_content) {
        Ok(_) => ToolResult {
            success: true,
            output: format!(
                "Edited {} - replaced {} chars with {} chars",
                path.display(),
                old_text.len(),
                new_text.len()
            ),
            metadata: Some(
                [
                    ("path".to_string(), path.display().to_string()),
                    ("old_len".to_string(), old_text.len().to_string()),
                    ("new_len".to_string(), new_text.len().to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        },
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to write file: {}", e),
            metadata: None,
        },
    }
}

/// Execute the `bash` tool.
pub fn execute_bash(args: &Value, working_dir: &Path) -> ToolResult {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: command".to_string(),
                metadata: None,
            }
        }
    };

    let _timeout_secs = args
        .get("timeout")
        .and_then(|v| v.as_i64())
        .map(|v| v as u64);

    debug!("Executing bash command: {}", command);

    // Use std Command for synchronous execution
    let mut cmd = Command::new("bash");
    cmd.arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n\n--- stderr ---\n");
                }
                result.push_str(&stderr);
            }

            if result.is_empty() {
                result = "(no output)".to_string();
            }

            ToolResult {
                success: output.status.success(),
                output: result,
                metadata: Some(
                    [
                        (
                            "exit_code".to_string(),
                            output.status.code().unwrap_or(-1).to_string(),
                        ),
                        ("command".to_string(), command.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to execute command: {}", e),
            metadata: None,
        },
    }
}

/// Execute the `spawn_session` tool.
/// This spawns a sub-session with --no-spawn to prevent recursive spawning.
pub fn execute_spawn_session(
    args: &Value,
    working_dir: &Path,
    descartes_bin: Option<&Path>,
) -> ToolResult {
    let task = match args.get("task").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return ToolResult {
                success: false,
                output: "Missing required parameter: task".to_string(),
                metadata: None,
            }
        }
    };

    let provider = args
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("anthropic");

    let output_file = args.get("output_file").and_then(|v| v.as_str());
    let _attachable = args
        .get("attachable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Find descartes binary
    let bin_path = descartes_bin
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("descartes"));

    info!(
        "Spawning sub-session with task: {} (provider: {})",
        task, provider
    );

    // Build command with --no-spawn to prevent recursive spawning
    let mut cmd = Command::new(&bin_path);
    cmd.arg("spawn")
        .arg("--task")
        .arg(task)
        .arg("--provider")
        .arg(provider)
        .arg("--no-spawn") // Critical: prevents sub-sessions from spawning their own sub-sessions
        .arg("--tool-level")
        .arg("minimal") // Sub-sessions get minimal tools
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(output) = output_file {
        cmd.arg("--transcript-dir").arg(output);
    }

    // Note: --attachable is handled separately by daemon integration (Phase 4)
    // For now, we run the sub-session inline

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let mut result = stdout.to_string();
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n\n--- stderr ---\n");
                }
                result.push_str(&stderr);
            }

            ToolResult {
                success: output.status.success(),
                output: result,
                metadata: Some(
                    [
                        ("task".to_string(), task.to_string()),
                        ("provider".to_string(), provider.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to spawn sub-session: {}", e),
            metadata: None,
        },
    }
}

/// Execute a tool by name.
pub fn execute_tool(
    name: &str,
    args: &Value,
    working_dir: &Path,
    descartes_bin: Option<&Path>,
) -> ToolResult {
    match name {
        "read" => execute_read(args, working_dir),
        "write" => execute_write(args, working_dir),
        "edit" => execute_edit(args, working_dir),
        "bash" => execute_bash(args, working_dir),
        "spawn_session" => execute_spawn_session(args, working_dir, descartes_bin),
        _ => ToolResult {
            success: false,
            output: format!("Unknown tool: {}", name),
            metadata: None,
        },
    }
}

/// Resolve a path relative to working directory.
fn resolve_path(path_str: &str, working_dir: &Path) -> PathBuf {
    let path = Path::new(path_str);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_execute_read_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let args = json!({
            "path": file_path.to_string_lossy()
        });

        let result = execute_read(&args, temp_dir.path());
        assert!(result.success);
        assert!(result.output.contains("line 1"));
        assert!(result.output.contains("line 2"));
    }

    #[test]
    fn test_execute_read_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let args = json!({
            "path": "nonexistent.txt"
        });

        let result = execute_read(&args, temp_dir.path());
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[test]
    fn test_execute_write_success() {
        let temp_dir = TempDir::new().unwrap();
        let args = json!({
            "path": "new_file.txt",
            "content": "Hello, World!"
        });

        let result = execute_write(&args, temp_dir.path());
        assert!(result.success);

        let content = fs::read_to_string(temp_dir.path().join("new_file.txt")).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_execute_write_creates_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let args = json!({
            "path": "nested/dir/file.txt",
            "content": "content"
        });

        let result = execute_write(&args, temp_dir.path());
        assert!(result.success);
        assert!(temp_dir.path().join("nested/dir/file.txt").exists());
    }

    #[test]
    fn test_execute_edit_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edit_test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let args = json!({
            "path": file_path.to_string_lossy(),
            "old_text": "World",
            "new_text": "Rust"
        });

        let result = execute_edit(&args, temp_dir.path());
        assert!(result.success);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[test]
    fn test_execute_edit_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edit_test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let args = json!({
            "path": file_path.to_string_lossy(),
            "old_text": "Nonexistent",
            "new_text": "Replacement"
        });

        let result = execute_edit(&args, temp_dir.path());
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[test]
    fn test_execute_edit_not_unique() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("edit_test.txt");
        fs::write(&file_path, "foo bar foo baz foo").unwrap();

        let args = json!({
            "path": file_path.to_string_lossy(),
            "old_text": "foo",
            "new_text": "qux"
        });

        let result = execute_edit(&args, temp_dir.path());
        assert!(!result.success);
        assert!(result.output.contains("3 times"));
    }

    #[test]
    fn test_execute_bash_success() {
        let temp_dir = TempDir::new().unwrap();
        let args = json!({
            "command": "echo 'hello world'"
        });

        let result = execute_bash(&args, temp_dir.path());
        assert!(result.success);
        assert!(result.output.contains("hello world"));
    }

    #[test]
    fn test_execute_bash_failure() {
        let temp_dir = TempDir::new().unwrap();
        let args = json!({
            "command": "exit 1"
        });

        let result = execute_bash(&args, temp_dir.path());
        assert!(!result.success);
    }

    #[test]
    fn test_execute_unknown_tool() {
        let temp_dir = TempDir::new().unwrap();
        let result = execute_tool("unknown_tool", &json!({}), temp_dir.path(), None);
        assert!(!result.success);
        assert!(result.output.contains("Unknown tool"));
    }

    #[test]
    fn test_resolve_path_absolute() {
        let working_dir = Path::new("/home/user/project");
        let result = resolve_path("/absolute/path/file.txt", working_dir);
        assert_eq!(result, PathBuf::from("/absolute/path/file.txt"));
    }

    #[test]
    fn test_resolve_path_relative() {
        let working_dir = Path::new("/home/user/project");
        let result = resolve_path("src/main.rs", working_dir);
        assert_eq!(result, PathBuf::from("/home/user/project/src/main.rs"));
    }
}
