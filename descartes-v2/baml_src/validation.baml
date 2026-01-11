// Validation prompts - for testing and quality checks

class TestAnalysis {
  summary string @description("Overall test status")
  passed int @description("Number of passing tests")
  failed int @description("Number of failing tests")
  failures FailureAnalysis[] @description("Analysis of failures")
  coverage_notes string? @description("Notes on test coverage")
}

class FailureAnalysis {
  test_name string
  error_message string
  likely_cause string @description("What probably caused this")
  suggested_fix string @description("How to fix it")
}

function AnalyzeTestResults(
  test_output: string,
  code_context: string?,
  additional_context: string?
) -> TestAnalysis {
  client Default
  prompt #"
    Analyze these test results.

    ## Test Output
    ```
    {{ test_output }}
    ```

    {% if code_context %}
    ## Relevant Code
    {{ code_context }}
    {% endif %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    For each failure, explain what went wrong and how to fix it.

    {{ ctx.output_format }}
  "#
}

class ValidationResult {
  passed bool @description("Whether validation passed")
  checks CheckResult[] @description("Individual check results")
  blocking_issues string[] @description("Issues that must be fixed")
  recommendations string[] @description("Suggested improvements")
}

class CheckResult {
  name string @description("What was checked")
  passed bool
  details string?
}

function ValidateImplementation(
  task_description: string,
  changes_made: string,
  test_results: string?,
  acceptance_criteria: string[],
  additional_context: string?
) -> ValidationResult {
  client Default
  prompt #"
    Validate that this implementation is complete and correct.

    ## Task
    {{ task_description }}

    ## Changes Made
    {{ changes_made }}

    {% if test_results %}
    ## Test Results
    {{ test_results }}
    {% endif %}

    ## Acceptance Criteria
    {% for criterion in acceptance_criteria %}
    - {{ criterion }}
    {% endfor %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    Check each criterion and determine if the implementation is complete.

    {{ ctx.output_format }}
  "#
}
