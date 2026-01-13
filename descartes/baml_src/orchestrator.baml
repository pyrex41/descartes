// Orchestrator prompts - for Ralph Wiggum loop decisions

enum NextAction {
  Continue @description("Continue with next task")
  Replan @description("Need to revise the plan")
  Validate @description("Run validation before proceeding")
  Complete @description("All work is done")
  AskHuman @description("Need human input")
}

class LoopDecision {
  action NextAction
  reasoning string @description("Why this action")
  next_task_id string? @description("Task to work on next")
  message string? @description("Message for human if asking")
}

function DecideNextAction(
  completed_tasks: string[],
  current_task: string?,
  remaining_tasks: string[],
  blockers: string[],
  recent_output: string,
  additional_context: string?
) -> LoopDecision {
  client GPT4oMini
  prompt #"
    Decide what the agent loop should do next.

    ## Completed
    {% for task in completed_tasks %}
    - {{ task }}
    {% endfor %}

    {% if current_task %}
    ## In Progress
    {{ current_task }}
    {% endif %}

    ## Remaining
    {% for task in remaining_tasks %}
    - {{ task }}
    {% endfor %}

    {% if blockers | length > 0 %}
    ## Blockers
    {% for b in blockers %}
    - {{ b }}
    {% endfor %}
    {% endif %}

    ## Recent Output
    {{ recent_output }}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    ## Rules
    - If blockers need human clarification → AskHuman
    - If blockers are technical → Replan
    - If all tasks done → Validate then Complete
    - Otherwise → Continue with next ready task

    {{ ctx.output_format }}
  "#
}

class SubagentSelection {
  category "searcher" | "analyzer" | "builder" | "validator"
  prompt string @description("Prompt to send to subagent")
  timeout_hint string @description("Expected duration: short/medium/long")
}

function SelectSubagent(
  task_title: string,
  task_description: string,
  available_context: string,
  additional_context: string?
) -> SubagentSelection {
  client GPT4oMini
  prompt #"
    Select the right subagent for this task.

    ## Task
    {{ task_title }}
    {{ task_description }}

    ## Available Context
    {{ available_context }}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    ## Subagent Categories
    - searcher: Find files, grep codebase, explore
    - analyzer: Understand code, research, plan
    - builder: Write code, make changes
    - validator: Run tests, check quality

    Generate a focused prompt for the selected subagent.

    {{ ctx.output_format }}
  "#
}
