# Descartes Provider Models - Design Documentation

## Overview

The Provider System is a unified abstraction layer for integrating multiple LLM backends into the Descartes orchestration platform. It provides a clean, extensible interface for working with:

- **API Providers**: Direct HTTP clients (OpenAI, Anthropic)
- **Headless CLI**: CLI tools spawned as child processes (Claude Code, OpenCode)
- **Local Services**: Localhost-based inference engines (Ollama, llama.cpp)

## Architecture

### Core Design Principle

The system uses Rust's trait system and async/await to provide a unified interface (`ModelBackend`) that abstracts over three distinct operational modes:

```
┌────────────────────────────────────────────────┐
│           ModelBackend Trait (Unified)          │
├────────────────────────────────────────────────┤
│                                                │
│  ┌─────────────┐  ┌─────────────┐  ┌────────┐ │
│  │   API Mode  │  │ Headless    │  │ Local  │ │
│  │             │  │ CLI Mode    │  │ Mode   │ │
│  └─────────────┘  └─────────────┘  └────────┘ │
│        │                 │             │       │
│        ▼                 ▼             ▼       │
│   ┌─────────┐       ┌────────┐   ┌────────┐   │
│   │ OpenAI  │       │Claude  │   │ Ollama │   │
│   │ Anthropic       │OpenCode    │        │   │
│   │ DeepSeek        │Custom CLI  │        │   │
│   │ Groq    │       │           │        │   │
│   └─────────┘       └────────┘   └────────┘   │
└────────────────────────────────────────────────┘
```

## Core Components

### 1. ModelBackend Trait

```rust
#[async_trait]
pub trait ModelBackend: Send + Sync {
    fn name(&self) -> &str;
    fn mode(&self) -> &ModelProviderMode;
    async fn initialize(&mut self) -> AgentResult<()>;
    async fn health_check(&self) -> AgentResult<bool>;
    async fn complete(&self, request: ModelRequest) -> AgentResult<ModelResponse>;
    async fn stream(&self, request: ModelRequest) -> AgentResult<...>;
    async fn list_models(&self) -> AgentResult<Vec<String>>;
    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize>;
    async fn shutdown(&mut self) -> AgentResult<()>;
}
```

**Key Features:**
- Fully async/await compatible via `async_trait`
- Support for both one-shot and streaming responses
- Token estimation for cost tracking and planning
- Health checks for reliability monitoring
- Graceful shutdown handling

### 2. ModelProviderMode

Defines the three operational modes:

```rust
pub enum ModelProviderMode {
    Api { endpoint: String, api_key: String },
    Headless { command: String, args: Vec<String> },
    Local { endpoint: String, timeout_secs: u64 },
}
```

### 3. Request/Response Types

**ModelRequest:**
- Messages (with role: user/assistant/system)
- Model identifier
- Optional: max_tokens, temperature, system_prompt, tools

**ModelResponse:**
- Generated content
- Finish reason (Stop/MaxTokens/ToolUse/Error)
- Optional: token count, tool calls

## Implementation Details

### API Mode Implementations

#### OpenAI Provider
- **Base URL**: `https://api.openai.com/v1` (configurable)
- **Authentication**: Bearer token via Authorization header
- **Models Supported**: gpt-4, gpt-4-turbo, gpt-3.5-turbo
- **Streaming**: Pending implementation

```rust
let provider = OpenAiProvider::new("sk-xxxxx".to_string(), None);
```

#### Anthropic Provider
- **Base URL**: `https://api.anthropic.com/v1` (configurable)
- **Authentication**: `x-api-key` header + `anthropic-version` header
- **Models Supported**: claude-3-opus, claude-3-sonnet, claude-3-haiku
- **Streaming**: Pending implementation

```rust
let provider = AnthropicProvider::new("sk-ant-xxxxx".to_string(), None);
```

### Headless Mode Implementations

#### ClaudeCodeAdapter
- Spawns `claude` CLI as child process
- Communicates via JSON streaming on stdin/stdout
- Uses `--output-format=stream-json` flag for structured responses
- Ideal for integrating Claude Code as an AI agent

```rust
let adapter = ClaudeCodeAdapter::new(
    Some("claude".to_string()),
    Some(vec!["--model=claude-3-opus".to_string()])
);
```

**Protocol:**
1. Spawn process with `--output-format=stream-json`
2. Send JSON-serialized `ModelRequest` to stdin
3. Read JSON-serialized `ModelResponse` from stdout
4. Parse ANSI color codes (if present) for structured output

#### Generic HeadlessCliAdapter
- Accepts any command + arguments
- Follows same JSON I/O protocol
- Extensible for custom tools (opencode, gh CLI, etc.)

```rust
let adapter = HeadlessCliAdapter::new(
    "opencode".to_string(),
    vec!["serve".to_string()]
);
```

### Local Mode Implementations

#### OllamaProvider
- **Endpoint**: `http://localhost:11434` (configurable)
- **Protocol**: REST API with `/api/chat` and `/api/tags` endpoints
- **Features**: Model enumeration, health checks, async timeouts
- **Authentication**: None (local service)

```rust
let provider = OllamaProvider::new(
    Some("http://localhost:11434".to_string()),
    Some(300) // 5 minute timeout
);
```

**Initialization Flow:**
1. Verify endpoint connectivity
2. Fetch available models via `/api/tags`
3. Cache model list for quick access

## Error Handling

Comprehensive error types with `thiserror`:

```rust
pub enum ProviderError {
    BackendError(String),
    InitializationError(String),
    ApiError(String),
    AuthenticationError(String),
    ProcessError(std::io::Error),
    JsonError(serde_json::Error),
    HttpError(String),
    ConfigError(String),
    RateLimited,
    ProviderUnavailable(String),
    UnsupportedFeature(String),
    StreamClosed,
    Timeout,
    InvalidResponse(String),
}
```

## Provider Factory

```rust
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(
        provider_name: &str,
        config: HashMap<String, String>,
    ) -> ProviderResult<Box<dyn ModelBackend>>
}
```

**Supported Providers:**
- `"openai"` - Requires `api_key`
- `"anthropic"` - Requires `api_key`
- `"claude-code-cli"` - Optional: `command`, `args`
- `"ollama"` - Optional: `endpoint`, `timeout_secs`
- `"headless-cli"` - Requires `command`, optional `args`

**Example Usage:**

```rust
// API Mode
let mut config = HashMap::new();
config.insert("api_key".to_string(), "sk-xxxxx".to_string());
let openai = ProviderFactory::create("openai", config)?;

// Headless Mode
let config = HashMap::new();
let claude = ProviderFactory::create("claude-code-cli", config)?;

// Local Mode
let ollama = ProviderFactory::create("ollama", config)?;
```

## Integration with Descartes

### In AgentRunner

Agents select a `ModelBackend` from the provider registry:

```rust
pub struct AgentConfig {
    pub model_backend: String, // e.g., "anthropic", "ollama"
    pub task: String,
    pub context: Option<String>,
    // ...
}
```

### Usage Pattern

```rust
// 1. Create provider
let mut provider = ProviderFactory::create("anthropic", config)?;
provider.initialize().await?;

// 2. Build request
let request = ModelRequest {
    messages: vec![
        Message { role: MessageRole::User, content: task }
    ],
    model: "claude-3-opus-20240229".to_string(),
    max_tokens: Some(2048),
    system_prompt: Some(context),
    ..Default::new()
};

// 3. Get response
let response = provider.complete(request).await?;

// 4. Shutdown
provider.shutdown().await?;
```

## Future Extensions

### Planned Features

1. **Streaming Responses**: Full implementation for all providers
   - Server-Sent Events (SSE) for API mode
   - Line-by-line streaming for headless mode
   - Chunked responses for local mode

2. **Tool/Function Calling**: Support for structured outputs
   - Tool definitions in requests
   - Tool call parsing in responses
   - Tool result handling in agent loops

3. **Batch Operations**: Efficient bulk processing
   - Batch API endpoints (OpenAI, Anthropic)
   - Parallel processing via rayon

4. **Caching Layer**: Token savings and performance
   - Prompt caching (Anthropic)
   - LRU cache for repeated prompts
   - Embeddings cache for semantic search

5. **Provider Routing**: Intelligent provider selection
   - Task complexity routing
   - Cost optimization
   - Model-specific feature support
   - Fallback chains (failover)

6. **Additional Providers**:
   - **DeepSeek** API
   - **Groq** API
   - **llama.cpp** local inference
   - **vLLM** distributed inference
   - **GitHub Models** API

## Configuration

### Environment Variables

```bash
# OpenAI
export OPENAI_API_KEY="sk-xxxx"
export OPENAI_ENDPOINT="https://api.openai.com/v1"

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-xxxx"
export ANTHROPIC_ENDPOINT="https://api.anthropic.com/v1"

# Local
export OLLAMA_ENDPOINT="http://localhost:11434"
export OLLAMA_TIMEOUT_SECS="300"
```

### Configuration Files

Future: `.descartes/providers.toml`

```toml
[providers.anthropic]
type = "api"
endpoint = "https://api.anthropic.com/v1"
api_key = "${ANTHROPIC_API_KEY}"

[providers.ollama]
type = "local"
endpoint = "http://localhost:11434"
timeout_secs = 300

[providers.claude_code]
type = "headless"
command = "claude"
args = ["--model=claude-3-opus"]
```

## Testing

Comprehensive test coverage:

```bash
# Run all tests
cargo test -p descartes-core

# Test specific provider
cargo test -p descartes-core openai_provider

# Test with logging
RUST_LOG=debug cargo test -p descartes-core --nocapture
```

### Test Structure

- Unit tests for each provider
- Factory pattern tests
- Error handling tests
- Configuration tests
- Mock provider implementations for integration tests

## Performance Characteristics

### API Mode
- **Latency**: ~100-500ms network round-trip
- **Throughput**: Rate-limited by provider (RPM/TPM)
- **Memory**: Minimal (requests/responses serialized)

### Headless Mode
- **Latency**: ~50ms process spawn + execution time
- **Throughput**: Single process bottleneck (sequential)
- **Memory**: Process overhead (~50-100MB)

### Local Mode
- **Latency**: ~10-100ms (network-based), ~1-10ms (memory-based)
- **Throughput**: Limited by local hardware
- **Memory**: Entire model in RAM

## Security Considerations

1. **API Keys**: Store in environment variables or secure vaults
   - Never commit to git
   - Use `.env` files (git-ignored)
   - Support OS credential stores

2. **Process Execution**: Validate command paths
   - Avoid shell interpretation
   - Use absolute paths when possible
   - Implement sandboxing for untrusted inputs

3. **Network**: Use HTTPS for all API connections
   - Verify SSL certificates
   - Support proxy configurations
   - Rate limiting to prevent abuse

4. **Logging**: Mask sensitive data
   - Strip API keys from logs
   - Redact authentication headers
   - Implement audit logging

## Migration Guide

### From Direct OpenAI Client

```rust
// Before
let client = openai::Client::new(api_key);
let response = client.chat().create(req).await?;

// After
let mut provider = ProviderFactory::create("openai", config)?;
provider.initialize().await?;
let response = provider.complete(request).await?;
```

### From Custom CLI Integration

```rust
// Before
let output = Command::new("my-cli").arg("prompt").output().await?;

// After
let adapter = HeadlessCliAdapter::new("my-cli".to_string(), vec![]);
adapter.initialize().await?;
let response = adapter.complete(request).await?;
```

## Contributing

When adding new providers:

1. Implement the `ModelBackend` trait
2. Add proper error handling
3. Include comprehensive tests
4. Document configuration requirements
5. Update `ProviderFactory::create()`
6. Add usage examples

## References

- **OpenAI API**: https://platform.openai.com/docs/api-reference
- **Anthropic API**: https://docs.anthropic.com/
- **Ollama**: https://ollama.ai/
- **Rust async-trait**: https://github.com/dtolnay/async-trait
