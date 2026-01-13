// Research prompts - for exploring and understanding code

class ResearchFindings {
  summary string @description("High-level summary of findings")
  relevant_files string[] @description("Files related to the topic")
  key_insights string[] @description("Important observations")
  patterns string[] @description("Patterns or conventions discovered")
  open_questions string[] @description("Things that need clarification")
}

function ResearchTopic(
  topic: string,
  codebase_context: string,
  constraints: string?,
  additional_context: string?
) -> ResearchFindings {
  client Claude
  prompt #"
    Research the following topic in this codebase.

    ## Topic
    {{ topic }}

    ## Codebase Context
    {{ codebase_context }}

    {% if constraints %}
    ## Constraints
    {{ constraints }}
    {% endif %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    Analyze the code and provide your findings.

    {{ ctx.output_format }}
  "#
}

class CodeExplanation {
  purpose string @description("What this code does")
  how_it_works string @description("How it accomplishes its purpose")
  key_components string[] @description("Important functions/types/modules")
  dependencies string[] @description("What this code depends on")
  used_by string[] @description("What uses this code")
}

function ExplainCode(
  code: string,
  file_path: string,
  question: string?,
  additional_context: string?
) -> CodeExplanation {
  client Default
  prompt #"
    Explain the following code.

    ## File: {{ file_path }}
    ```
    {{ code }}
    ```

    {% if question %}
    ## Specific Question
    {{ question }}
    {% endif %}

    {% if additional_context %}
    ## Additional Context
    {{ additional_context }}
    {% endif %}

    {{ ctx.output_format }}
  "#
}
