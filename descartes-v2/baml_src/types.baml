// Core types for Descartes agent orchestration

// Agent categories for subagent spawning
enum AgentCategory {
  Searcher @description("Find files, grep codebase, explore structure")
  Analyzer @description("Deep analysis, research, understand code")
  Builder @description("Write code, make changes, implement features")
  Validator @description("Run tests, verify correctness, check quality")
}

// Task status from SCUD
enum TaskStatus {
  Pending @description("Task not started")
  InProgress @description("Task currently being worked on")
  Blocked @description("Task blocked by dependency or issue")
  Done @description("Task completed successfully")
}

// Task complexity levels
enum Complexity {
  Low @description("Simple, straightforward task")
  Medium @description("Moderate complexity, some thought required")
  High @description("Complex task requiring careful planning")
}

// A task in the SCUD graph
class Task {
  id string @description("Unique task identifier like T1, T2.1")
  title string @description("Short task title")
  description string @description("Detailed task description")
  status TaskStatus
  complexity Complexity
  depends_on string[] @description("IDs of tasks this depends on")
}

// Current state of the task graph
class TaskGraphStatus {
  total_tasks int
  completed int
  in_progress int
  blocked int
  pending int
  next_ready Task? @description("Next task ready to work on")
  blockers string[] @description("Current blocking issues")
}

// Result from validation/testing
class ValidationResult {
  passed bool
  test_summary string?
  failures string[]
  warnings string[]
}

// Git status summary
class GitStatus {
  branch string
  modified_files int
  staged_files int
  untracked_files int
  has_conflicts bool
}

// Context available to the agent
class AgentContext {
  task_status TaskGraphStatus
  recent_output string @description("Output from last agent run")
  validation ValidationResult?
  git_status GitStatus?
  iteration int @description("Current loop iteration number")
  elapsed_minutes int @description("Time spent in current session")
}
