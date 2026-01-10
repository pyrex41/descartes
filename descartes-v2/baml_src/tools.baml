// Tool definitions for subagent operations
// These represent actions that subagents can request

// SCUD task graph operations
class ScudNext {
  tool_name "scud_next" @description("Get the next ready task from SCUD")
}

class ScudComplete {
  tool_name "scud_complete" @description("Mark a task as complete")
  task_id string @description("Task ID to mark complete (e.g., T1, T2.1)")
  notes string? @description("Optional completion notes")
}

class ScudBlock {
  tool_name "scud_block" @description("Mark a task as blocked")
  task_id string
  reason string @description("Why the task is blocked")
}

class ScudWaves {
  tool_name "scud_waves" @description("Show parallel execution waves")
}

class ScudAdd {
  tool_name "scud_add" @description("Add a new task to the graph")
  title string
  description string
  depends_on string[] @description("Task IDs this depends on")
  complexity Complexity
}

// File operations
class ReadFile {
  tool_name "read_file" @description("Read contents of a file")
  path string @description("Path to the file")
  start_line int? @description("Optional start line")
  end_line int? @description("Optional end line")
}

class WriteFile {
  tool_name "write_file" @description("Write or create a file")
  path string
  content string
}

class EditFile {
  tool_name "edit_file" @description("Edit a specific section of a file")
  path string
  old_content string @description("Content to replace")
  new_content string @description("Replacement content")
}

// Search operations
class GrepSearch {
  tool_name "grep" @description("Search for pattern in files")
  pattern string @description("Regex pattern to search for")
  path string? @description("Optional path to limit search")
  file_type string? @description("Optional file extension filter")
}

class GlobSearch {
  tool_name "glob" @description("Find files matching a pattern")
  pattern string @description("Glob pattern like **/*.rs")
}

// Command execution
class RunCommand {
  tool_name "run_command" @description("Run a shell command")
  command string
  working_dir string?
  timeout_seconds int?
}

class RunTests {
  tool_name "run_tests" @description("Run the test suite")
  filter string? @description("Optional test filter pattern")
  verbose bool
}

// Git operations
class GitStatus {
  tool_name "git_status" @description("Get git status")
}

class GitDiff {
  tool_name "git_diff" @description("Show git diff")
  staged bool @description("Show staged changes only")
  path string? @description("Optional path filter")
}

class GitCommit {
  tool_name "git_commit" @description("Create a git commit")
  message string
  files string[]? @description("Specific files to commit, or all staged if empty")
}

// Union type for tool selection
function SelectTool(
  task_description: string,
  available_context: string
) -> ScudNext | ScudComplete | ScudBlock | ReadFile | WriteFile | EditFile | GrepSearch | GlobSearch | RunCommand | RunTests | GitStatus | GitDiff {
  client FastModel
  prompt #"
    Based on the task, select the most appropriate tool to use next.

    Task: {{ task_description }}

    Available Context:
    {{ available_context }}

    {{ ctx.output_format }}
  "#
}

// Select multiple tools for parallel execution
function SelectToolsParallel(
  task_description: string,
  max_tools: int
) -> (ScudNext | ReadFile | GrepSearch | GlobSearch | GitStatus | GitDiff)[] {
  client FastModel
  prompt #"
    Select up to {{ max_tools }} tools that can be executed in parallel for this task.
    Only select tools that don't depend on each other's output.

    Task: {{ task_description }}

    {{ ctx.output_format }}
  "#
}
