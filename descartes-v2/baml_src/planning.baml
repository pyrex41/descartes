// Planning prompts - for creating implementation plans

enum TaskComplexity {
  Low @description("Simple, straightforward task")
  Medium @description("Moderate complexity")
  High @description("Complex, needs careful thought")
}

class Task {
  id string @description("Task ID like T1, T2, T2.1")
  title string @description("Short task title")
  description string @description("What needs to be done")
  complexity TaskComplexity
  depends_on string[] @description("IDs of tasks this depends on")
  acceptance_criteria string[] @description("How to know it's done")
}

class ImplementationPlan {
  goal string @description("What we're trying to accomplish")
  approach string @description("High-level approach")
  tasks Task[] @description("Ordered list of tasks")
  risks string[] @description("Potential issues to watch for")
  out_of_scope string[] @description("What we're NOT doing")
}

function CreatePlan(
  objective string @description("What needs to be built or changed"),
  research_context string @description("Findings from research phase"),
  constraints string? @description("Technical or time constraints")
) -> ImplementationPlan {
  client Claude
  prompt #"
    Create an implementation plan for the following objective.

    ## Objective
    {{ objective }}

    ## Research Context
    {{ research_context }}

    {% if constraints %}
    ## Constraints
    {{ constraints }}
    {% endif %}

    ## Guidelines
    - Break work into small, testable tasks
    - Each task should take 15-60 minutes
    - Use IDs like T1, T2, T2.1, T2.2 for subtasks
    - Identify dependencies between tasks
    - Be specific about acceptance criteria

    {{ ctx.output_format }}
  "#
}

class TaskBreakdown {
  subtasks Task[] @description("Subtasks for this task")
  estimated_minutes int @description("Total estimated time")
  blockers string[] @description("Things that might block progress")
}

function BreakdownTask(
  task Task @description("Task to break down"),
  context string @description("Relevant code or context")
) -> TaskBreakdown {
  client Default
  prompt #"
    Break down this task into smaller subtasks.

    ## Task
    ID: {{ task.id }}
    Title: {{ task.title }}
    Description: {{ task.description }}
    Complexity: {{ task.complexity }}

    ## Context
    {{ context }}

    {{ ctx.output_format }}
  "#
}
