// Research prompts - for exploring and understanding code

class ResearchFindings {
  summary string @description("High-level summary of findings")
  relevant_files string[] @description("Files related to the topic")
  key_insights string[] @description("Important observations")
  patterns string[] @description("Patterns or conventions discovered")
  open_questions string[] @description("Things that need clarification")
}

function ResearchTopic(
  topic string @description("What to research"),
  codebase_context string @description("Relevant code snippets or file listings"),
  constraints string? @description("Any constraints or focus areas"),
  additional_context string? @description("Extra context to include in prompt")
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
  code string @description("Code to explain"),
  file_path string @description("Path to the file"),
  question string? @description("Specific question about the code"),
  additional_context string? @description("Extra context to include in prompt")
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
