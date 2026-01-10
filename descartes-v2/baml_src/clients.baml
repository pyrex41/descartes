// BAML Client Configurations for Descartes
// Defines LLM providers with retry and fallback strategies

// Retry policy for transient failures
retry_policy AgentRetry {
  max_retries 2
  strategy {
    type exponential_backoff
    delay_ms 1000
    multiplier 2.0
  }
}

// Fast, cheap model for simple decisions
client<llm> FastModel {
  provider openai
  retry_policy AgentRetry
  options {
    model "gpt-4o-mini"
    temperature 0.1
    api_key env.OPENAI_API_KEY
  }
}

// Claude for complex reasoning
client<llm> ClaudeModel {
  provider anthropic
  retry_policy AgentRetry
  options {
    model "claude-sonnet-4-20250514"
    max_tokens 4096
    api_key env.ANTHROPIC_API_KEY
  }
}

// High-capability model for planning
client<llm> PlannerModel {
  provider anthropic
  retry_policy AgentRetry
  options {
    model "claude-sonnet-4-20250514"
    max_tokens 8192
    api_key env.ANTHROPIC_API_KEY
  }
}

// Fallback chain: try fast first, then reliable
client<llm> AgentWithFallback {
  provider fallback
  options {
    strategy [
      FastModel
      ClaudeModel
    ]
  }
}

// Decision-specific client (fast model is sufficient for structured decisions)
client<llm> DecisionModel {
  provider openai
  retry_policy AgentRetry
  options {
    model "gpt-4o-mini"
    temperature 0.0
    api_key env.OPENAI_API_KEY
  }
}
