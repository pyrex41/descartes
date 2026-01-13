// LLM Client Configurations
// Uses Claude Code headless mode via local proxy

// Retry policy for transient failures
retry_policy Retry2x {
  max_retries 2
  strategy {
    type exponential_backoff
    delay_ms 500
  }
}

// Claude Code via local proxy (primary)
// Start proxy with: cargo run --bin claude-proxy
client<llm> ClaudeCode {
  provider openai-generic
  retry_policy Retry2x
  options {
    base_url "http://localhost:8765/v1"
    model "claude-code"
    api_key "not-needed"
  }
}

// Direct Anthropic API (fallback if proxy unavailable)
client<llm> ClaudeDirect {
  provider anthropic
  retry_policy Retry2x
  options {
    model "claude-sonnet-4-20250514"
    max_tokens 8192
    api_key env.ANTHROPIC_API_KEY
  }
}

// Fast model for simple extractions (optional)
client<llm> GPT4oMini {
  provider openai
  retry_policy Retry2x
  options {
    model "gpt-4o-mini"
    temperature 0.1
    api_key env.OPENAI_API_KEY
  }
}

// Default: use Claude Code proxy
client<llm> Default {
  provider fallback
  options {
    strategy [ClaudeCode, ClaudeDirect]
  }
}

// Alias for complex tasks
client<llm> Claude {
  provider fallback
  options {
    strategy [ClaudeCode, ClaudeDirect]
  }
}
