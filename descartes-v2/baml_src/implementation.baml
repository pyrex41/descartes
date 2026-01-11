// Implementation prompts - for writing and modifying code

class CodeChange {
  file_path string @description("File to modify or create")
  action "create" | "modify" | "delete" @description("What to do")
  description string @description("What this change does")
  code string? @description("New code content or diff")
}

class ImplementationResult {
  summary string @description("What was done")
  changes CodeChange[] @description("Files changed")
  tests_to_run string[] @description("Tests that should be run")
  follow_up string[] @description("Things to do next")
}

function ImplementTask(
  task_id string @description("Task ID being implemented"),
  task_description string @description("What to implement"),
  relevant_code string @description("Existing code context"),
  acceptance_criteria string[] @description("How to verify success")
) -> ImplementationResult {
  client Claude
  prompt #"
    Implement the following task.

    ## Task {{ task_id }}
    {{ task_description }}

    ## Relevant Code
    {{ relevant_code }}

    ## Acceptance Criteria
    {% for criterion in acceptance_criteria %}
    - {{ criterion }}
    {% endfor %}

    ## Guidelines
    - Follow existing code patterns
    - Keep changes minimal and focused
    - Include error handling
    - Write clear code, not clever code

    {{ ctx.output_format }}
  "#
}

class CodeReviewFeedback {
  approved bool @description("Whether the code is ready")
  issues Issue[] @description("Problems found")
  suggestions string[] @description("Optional improvements")
}

class Issue {
  severity "critical" | "warning" | "nitpick"
  file_path string
  line int?
  description string
  suggestion string?
}

function ReviewCode(
  code_diff string @description("Git diff of changes"),
  context string @description("What the code is trying to do")
) -> CodeReviewFeedback {
  client Default
  prompt #"
    Review the following code changes.

    ## Context
    {{ context }}

    ## Changes
    ```diff
    {{ code_diff }}
    ```

    ## Review Checklist
    - Correctness: Does it do what it's supposed to?
    - Security: Any vulnerabilities?
    - Performance: Any obvious issues?
    - Readability: Is it clear?
    - Error handling: Are edge cases covered?

    {{ ctx.output_format }}
  "#
}
