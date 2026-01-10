// Ralph Wiggum Loop Decision Functions
// Core orchestration logic for the agent loop

// Decision: Continue building (more tasks remain)
class ContinueBuilding {
  action "continue" @alias("continue_building") @description("More tasks remain, continue to next task")
  next_task_id string? @description("Specific task ID to work on next, if known")
  approach string @description("Brief note on how to approach the next task")
}

// Decision: Request replanning (blockers or significant changes)
class RequestReplan {
  action "replan" @alias("request_replan") @description("Need to replan due to blockers or new information")
  reason string @description("Why replanning is needed")
  context string @description("Relevant context for the planner")
  preserve_completed bool @description("Keep completed tasks in the plan")
}

// Decision: Work is complete
class Complete {
  action "complete" @alias("mark_complete") @description("All tasks done and validation passed")
  summary string @description("Summary of what was accomplished")
  artifacts string[] @description("List of created/modified files")
}

// Decision: Need human input
class NeedHumanInput {
  action "human" @alias("need_human") @description("Ambiguity or decision requires human input")
  question string @description("Clear question for the human")
  options string[]? @description("Suggested options if applicable")
  blocking bool @description("Whether this blocks all progress")
}

// Decision: Spawn a specialized subagent
class SpawnSubagent {
  action "spawn" @alias("spawn_subagent") @description("Spawn a subagent for a specific task")
  category AgentCategory
  task_id string @description("Task ID this subagent will work on")
  prompt string @description("Detailed prompt for the subagent")
  timeout_seconds int? @description("Optional timeout")
}

// Decision: Run validation
class RunValidation {
  action "validate" @alias("run_validation") @description("Run tests/validation before proceeding")
  scope string @description("What to validate: 'all', 'changed', or specific path")
  continue_on_failure bool @description("Whether to continue if validation fails")
}

// The main decision function for the Ralph Wiggum loop
function DecideNextStep(context: AgentContext) -> ContinueBuilding | RequestReplan | Complete | NeedHumanInput | SpawnSubagent | RunValidation {
  client DecisionModel
  prompt #"
    You are the orchestrator in a Ralph Wiggum loop - a deterministic agent execution pattern.
    Your job is to decide what action to take next based on the current state.

    ## Current State

    ### Task Graph
    - Total tasks: {{ context.task_status.total_tasks }}
    - Completed: {{ context.task_status.completed }}
    - In Progress: {{ context.task_status.in_progress }}
    - Blocked: {{ context.task_status.blocked }}
    - Pending: {{ context.task_status.pending }}

    {% if context.task_status.next_ready %}
    ### Next Ready Task
    - ID: {{ context.task_status.next_ready.id }}
    - Title: {{ context.task_status.next_ready.title }}
    - Description: {{ context.task_status.next_ready.description }}
    - Complexity: {{ context.task_status.next_ready.complexity }}
    {% else %}
    ### No tasks currently ready
    {% endif %}

    {% if context.task_status.blockers | length > 0 %}
    ### Current Blockers
    {% for blocker in context.task_status.blockers %}
    - {{ blocker }}
    {% endfor %}
    {% endif %}

    ### Recent Output
    {{ context.recent_output }}

    {% if context.validation %}
    ### Validation Result
    - Passed: {{ context.validation.passed }}
    {% if context.validation.test_summary %}
    - Summary: {{ context.validation.test_summary }}
    {% endif %}
    {% if context.validation.failures | length > 0 %}
    - Failures:
    {% for f in context.validation.failures %}
      - {{ f }}
    {% endfor %}
    {% endif %}
    {% endif %}

    {% if context.git_status %}
    ### Git Status
    - Branch: {{ context.git_status.branch }}
    - Modified: {{ context.git_status.modified_files }} files
    - Staged: {{ context.git_status.staged_files }} files
    {% endif %}

    ### Session Info
    - Iteration: {{ context.iteration }}
    - Elapsed: {{ context.elapsed_minutes }} minutes

    ## Decision Rules

    1. If all tasks are done AND validation passed → Complete
    2. If there are blockers that need human clarification → NeedHumanInput
    3. If blockers require architectural changes → RequestReplan
    4. If there's a ready task → SpawnSubagent with appropriate category
    5. If significant work was done, run validation → RunValidation
    6. If tasks remain but none ready (dependencies) → ContinueBuilding with wait

    ## Response Format
    {{ ctx.output_format }}

    Decide the next action:
  "#
}

// Simpler decision for choosing agent category
function SelectAgentCategory(task: Task) -> AgentCategory {
  client FastModel
  prompt #"
    Given this task, select the most appropriate agent category.

    Task ID: {{ task.id }}
    Title: {{ task.title }}
    Description: {{ task.description }}
    Complexity: {{ task.complexity }}

    Categories:
    - Searcher: For finding files, exploring codebase, grep operations
    - Analyzer: For understanding code, research, deep analysis
    - Builder: For writing code, making changes, implementing
    - Validator: For running tests, checking quality, verification

    {{ ctx.output_format }}
  "#
}

// Generate a prompt for a subagent based on task
function GenerateSubagentPrompt(task: Task, context: string) -> string {
  client FastModel
  prompt #"
    Generate a clear, actionable prompt for a subagent to complete this task.

    Task ID: {{ task.id }}
    Title: {{ task.title }}
    Description: {{ task.description }}

    Context:
    {{ context }}

    The prompt should:
    1. Be specific and actionable
    2. Include relevant context
    3. Specify expected deliverables
    4. Note any constraints or requirements

    Generate only the prompt text, no meta-commentary:
  "#
}

// Test cases for the decision function
test TestContinueWithReadyTask {
  functions [DecideNextStep]
  args {
    context {
      task_status {
        total_tasks 5
        completed 2
        in_progress 0
        blocked 0
        pending 3
        next_ready {
          id "T3"
          title "Implement user authentication"
          description "Add JWT-based auth to the API"
          status Pending
          complexity Medium
          depends_on ["T1", "T2"]
        }
        blockers []
      }
      recent_output "Completed database schema migration"
      validation null
      git_status {
        branch "feature/auth"
        modified_files 3
        staged_files 2
        untracked_files 0
        has_conflicts false
      }
      iteration 3
      elapsed_minutes 15
    }
  }
  // Should spawn a Builder subagent for T3
}

test TestCompleteWhenDone {
  functions [DecideNextStep]
  args {
    context {
      task_status {
        total_tasks 3
        completed 3
        in_progress 0
        blocked 0
        pending 0
        next_ready null
        blockers []
      }
      recent_output "All tests passing"
      validation {
        passed true
        test_summary "15 tests passed"
        failures []
        warnings []
      }
      git_status null
      iteration 5
      elapsed_minutes 30
    }
  }
  // Should return Complete
}

test TestNeedHumanOnAmbiguity {
  functions [DecideNextStep]
  args {
    context {
      task_status {
        total_tasks 4
        completed 1
        in_progress 0
        blocked 1
        pending 2
        next_ready null
        blockers ["Unclear requirement: Should auth support OAuth or just JWT?"]
      }
      recent_output "Started auth implementation but hit ambiguity"
      validation null
      git_status null
      iteration 2
      elapsed_minutes 10
    }
  }
  // Should return NeedHumanInput
}
