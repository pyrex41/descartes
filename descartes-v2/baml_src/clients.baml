// LLM Client Configurations

// Retry policy for transient failures
retry_policy Retry2x {
  max_retries 2
  strategy {
    type exponential_backoff
    delay_ms 500
  }
}

// Claude for complex tasks
client<llm> Claude {
  provider anthropic
  retry_policy Retry2x
  options {
    model "claude-sonnet-4-20250514"
    max_tokens 8192
    api_key env.ANTHROPIC_API_KEY
  }
}

// Fast model for simple extractions
client<llm> GPT4oMini {
  provider openai
  retry_policy Retry2x
  options {
    model "gpt-4o-mini"
    temperature 0.1
    api_key env.OPENAI_API_KEY
  }
}

// Fallback: try fast first, then reliable
client<llm> Default {
  provider fallback
  options {
    strategy [GPT4oMini, Claude]
  }
}
