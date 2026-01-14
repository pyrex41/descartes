# claude-proxy

An OpenAI-compatible HTTP proxy that wraps `claude -p` for integration with tools expecting the OpenAI API.

## Purpose

Allows BAML, LangChain, or other OpenAI-compatible tools to use Claude Code as a backend without modifying their code.

## Usage

```bash
# Start the proxy server (default port 8765)
claude-proxy

# Or with custom port
CLAUDE_PROXY_PORT=9000 claude-proxy
```

The server runs on `localhost:8765` by default.

## API

### POST /v1/chat/completions

Accepts OpenAI chat completion requests:

```json
{
  "model": "claude-code",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello, how are you?"}
  ]
}
```

Returns a standard OpenAI chat completion response:

```json
{
  "id": "chatcmpl-1234567890",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "claude-code",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 0,
    "completion_tokens": 0,
    "total_tokens": 0
  }
}
```

### GET /health or GET /

Health check endpoint. Returns `OK`.

## Integration with BAML

In your `baml_src/clients.baml`:

```baml
client<llm> ClaudeCode {
  provider openai-generic
  options {
    base_url "http://localhost:8765/v1"
    model "claude-code"
  }
}
```

## How It Works

1. Receives OpenAI-format chat completion requests
2. Converts messages to a prompt format
3. Invokes `claude -p <prompt> --output-format text`
4. Wraps the response in OpenAI-compatible JSON

## Limitations

- **Non-streaming only**: Does not support streaming responses (`stream: true`)
- **No function calling**: Tool use / function calling is not supported
- **Subprocess per request**: Spawns a new `claude` process for each request
- **No session continuity**: Each request is independent (no conversation memory)
- **Token counts unavailable**: Returns 0 for token usage metrics

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CLAUDE_PROXY_PORT` | `8765` | Port to listen on |

## Building

```bash
cd descartes/descartes
cargo build --release --bin claude-proxy
```

The binary will be at `target/release/claude-proxy`.

## Requirements

- `claude` CLI must be installed and in PATH
- Claude Code must be authenticated (run `claude` once to authenticate)
