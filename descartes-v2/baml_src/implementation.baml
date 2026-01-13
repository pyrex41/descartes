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
  task_id: string,
  task_description: string,
  relevant_code: string,
  acceptance_criteria: string[],
  additional_context: string?
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

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

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
  code_diff: string,
  context: string,
  additional_context: string?
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

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    ## Review Checklist
    - Correctness: Does it do what it's supposed to?
    - Security: Any vulnerabilities?
    - Performance: Any obvious issues?
    - Readability: Is it clear?
    - Error handling: Are edge cases covered?

    {{ ctx.output_format }}
  "#
}
