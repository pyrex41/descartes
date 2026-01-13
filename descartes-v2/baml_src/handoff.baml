// Handoff prompts - for transitions between workflow stages

class StageHandoff {
  from_stage string @description("Stage we're leaving")
  to_stage string @description("Stage we're entering")
  summary string @description("What was accomplished")
  key_artifacts string[] @description("Important files or outputs")
  context_for_next string @description("What the next stage needs to know")
  open_items string[] @description("Unresolved questions or issues")
}

function GenerateHandoff(
  from_stage: string,
  to_stage: string,
  work_done: string,
  outputs: string,
  additional_context: string?
) -> StageHandoff {
  client Default
  prompt #"
    Generate a handoff from {{ from_stage }} to {{ to_stage }}.

    ## Work Completed
    {{ work_done }}

    ## Outputs
    {{ outputs }}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    Create a clear handoff that gives the next stage everything it needs.

    {{ ctx.output_format }}
  "#
}

class CommitMessage {
  type "feat" | "fix" | "docs" | "refactor" | "test" | "chore"
  scope string? @description("Component affected")
  subject string @description("Short description (50 chars)")
  body string? @description("Detailed explanation if needed")
  breaking bool @description("Is this a breaking change")
}

function GenerateCommitMessage(
  diff: string,
  context: string?,
  additional_context: string?
) -> CommitMessage {
  client GPT4oMini
  prompt #"
    Generate a conventional commit message for these changes.

    ## Changes
    ```diff
    {{ diff }}
    ```

    {% if context %}
    ## Context
    {{ context }}
    {% endif %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    Follow conventional commits format.
    Keep the subject under 50 characters.

    {{ ctx.output_format }}
  "#
}

class PRDescription {
  title string @description("PR title")
  summary string @description("What this PR does")
  changes string[] @description("List of changes")
  testing string @description("How it was tested")
  checklist string[] @description("Review checklist items")
}

function GeneratePRDescription(
  commits: string,
  diff_summary: string,
  related_issues: string?,
  additional_context: string?
) -> PRDescription {
  client Default
  prompt #"
    Generate a pull request description.

    ## Commits
    {{ commits }}

    ## Changes
    {{ diff_summary }}

    {% if related_issues %}
    ## Related Issues
    {{ related_issues }}
    {% endif %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    Create a clear PR description that helps reviewers.

    {{ ctx.output_format }}
  "#
}
