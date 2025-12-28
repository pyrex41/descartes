# Providers and Configuration

*Connect to any LLM, your way*

---

Descartes is provider-agnostic by design. Whether you prefer Claude's reasoning, GPT's breadth, or running models locally with Ollama, configuration is straightforward and consistent.

## Supported Providers

| Provider | Type | Models | Best For |
|----------|------|--------|----------|
| **Anthropic** | Cloud API | Claude 3.5 Sonnet, Opus, Haiku | Complex reasoning, code |
| **OpenAI** | Cloud API | GPT-4, GPT-4 Turbo, GPT-3.5 | General purpose |
| **xAI/Grok** | Cloud API | Grok 4.1, Grok 3 | Fast reasoning |
| **Ollama** | Local | Llama 2, CodeLlama, Mistral | Privacy, offline |
| **DeepSeek** | Cloud API | DeepSeek Coder | Code generation |
| **Groq** | Cloud API | Various | Ultra-fast inference |

---

## Configuration File

Descartes uses TOML for configuration. The primary config file lives at `~/.descartes/config.toml`.

### Minimal Configuration

```toml
[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
# API key from environment: ANTHROPIC_API_KEY
```

### Full Configuration

```toml
# Descartes Configuration
version = "1.0.0"

[providers]
# Which provider to use by default
primary = "anthropic"

# ─────────────────────────────────────────────────────────────
# Anthropic (Claude)
# ─────────────────────────────────────────────────────────────
[providers.anthropic]
enabled = true
api_key = ""  # Or use ANTHROPIC_API_KEY env var
endpoint = "https://api.anthropic.com/v1"
model = "claude-3-5-sonnet-20241022"
models = [
    "claude-3-5-sonnet-20241022",
    "claude-3-opus-20240229",
    "claude-3-haiku-20240307"
]
timeout_secs = 120
max_retries = 3
retry_backoff_ms = 1000
rate_limit_rpm = 60
temperature = 0.7
max_tokens = 4096

# ─────────────────────────────────────────────────────────────
# OpenAI (GPT)
# ─────────────────────────────────────────────────────────────
[providers.openai]
enabled = true
api_key = ""  # Or use OPENAI_API_KEY env var
endpoint = "https://api.openai.com/v1"
model = "gpt-4-turbo"
models = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
timeout_secs = 120
max_retries = 3
temperature = 0.7
max_tokens = 4096

# ─────────────────────────────────────────────────────────────
# xAI/Grok
# ─────────────────────────────────────────────────────────────
[providers.grok]
enabled = true
api_key = ""  # Or use XAI_API_KEY env var
endpoint = "https://api.x.ai/v1"
model = "grok-4-1-fast"
models = [
    "grok-4-1-fast-reasoning",
    "grok-4-1-fast",
    "grok-4-1",
    "grok-3-latest"
]
timeout_secs = 120
max_tokens = 4096

# ─────────────────────────────────────────────────────────────
# Ollama (Local)
# ─────────────────────────────────────────────────────────────
[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
model = "llama2"
timeout_secs = 300  # Longer for local inference
```

---

## Provider Deep Dives

### Anthropic (Claude)

Claude models excel at complex reasoning and code understanding.

```toml
[providers.anthropic]
enabled = true
model = "claude-3-5-sonnet-20241022"  # Best balance
# model = "claude-3-opus-20240229"    # Maximum capability
# model = "claude-3-haiku-20240307"   # Fast & cheap
```

**Authentication:**
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```

**API Headers Sent:**
- `x-api-key`: Your API key
- `anthropic-version`: `2023-06-01`
- `content-type`: `application/json`

### OpenAI (GPT)

Broad knowledge and strong instruction following.

```toml
[providers.openai]
enabled = true
model = "gpt-4-turbo"
# For cost optimization:
# model = "gpt-3.5-turbo"
```

**Authentication:**
```bash
export OPENAI_API_KEY="sk-..."
```

**Custom Endpoints:**

Use Azure OpenAI or compatible APIs:
```toml
[providers.openai]
endpoint = "https://your-resource.openai.azure.com/openai/deployments/your-model"
```

### xAI/Grok

Fast reasoning with real-time knowledge.

```toml
[providers.grok]
enabled = true
model = "grok-4-1-fast"
# For complex tasks:
# model = "grok-4-1-fast-reasoning"
```

**Authentication:**
```bash
export XAI_API_KEY="xai-..."
```

### Ollama (Local Models)

Run models locally for privacy and offline use.

**Setup Ollama:**
```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model
ollama pull llama2
ollama pull codellama

# Start the server (if not running)
ollama serve
```

**Configure Descartes:**
```toml
[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
model = "codellama"
timeout_secs = 300  # Local inference can be slow
```

**List Available Models:**
```bash
ollama list
```

---

## Environment Variables

API keys can be set via environment variables (recommended for security):

| Variable | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | Anthropic |
| `OPENAI_API_KEY` | OpenAI |
| `XAI_API_KEY` | xAI/Grok |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `GROQ_API_KEY` | Groq |

**Priority:**
1. Environment variable (highest)
2. Config file
3. Default (none)

---

## Custom Providers

Connect to any OpenAI-compatible API:

```toml
[providers.custom]
enabled = true
endpoint = "https://your-api.example.com/v1"
api_key = "your-key"
model = "your-model"
use_bearer_auth = true
timeout_secs = 120

[providers.custom.custom_headers]
X-Custom-Header = "value"
```

---

## Provider Selection

### At Spawn Time

Override the default provider per-task:

```bash
# Use OpenAI for this task
descartes spawn -t "Write tests" --provider openai

# Use local Ollama
descartes spawn -t "Review code" --provider ollama --model codellama
```

### In Workflows

Different phases can use different providers:

```toml
# .scud/flow-config.toml
[flow]
orchestrator_model = "claude-3-5-sonnet-20241022"
implementation_model = "gpt-4-turbo"
qa_model = "claude-3-haiku-20240307"
```

---

## Rate Limiting & Retries

Configure retry behavior for reliability:

```toml
[providers.anthropic]
max_retries = 3           # Retry failed requests
retry_backoff_ms = 1000   # Wait between retries (exponential)
rate_limit_rpm = 60       # Max requests per minute
timeout_secs = 120        # Request timeout
```

**Backoff Strategy:**
- 1st retry: 1 second
- 2nd retry: 2 seconds
- 3rd retry: 4 seconds

---

## Health Checks

Verify provider connectivity:

```bash
descartes doctor
```

**Output:**
```
Provider Health Checks:
✓ Anthropic: claude-3-5-sonnet-20241022 responding
✓ OpenAI: gpt-4-turbo responding
✗ Grok: API key not configured
✓ Ollama: localhost:11434 reachable, 3 models available
```

**Manual Check:**
```bash
# Test a specific provider
descartes spawn -t "Say hello" -p anthropic
```

---

## Secrets Management

Descartes includes encrypted secrets storage for sensitive credentials.

### Store a Secret

```bash
# Interactive
descartes secrets set anthropic-key

# From environment
descartes secrets set openai-key --from-env OPENAI_API_KEY
```

### Security Features

- **AES-256-GCM** encryption
- **Argon2id** key derivation
- **Per-secret salts** and nonces
- **Access audit logging**
- **Version history** for rotation

---

## Configuration Discovery

Descartes looks for configuration in this order:

1. `--config` flag (explicit path)
2. `.descartes/config.toml` (project directory)
3. `~/.descartes/config.toml` (home directory)
4. `DESCARTES_CONFIG` environment variable
5. Environment variables only (no file)

**Project-Specific Overrides:**

Create `.descartes/config.toml` in your project:

```toml
[providers]
primary = "ollama"  # Use local model for this project

[providers.ollama]
model = "codellama"
```

---

## Best Practices

### 1. Use Environment Variables for Keys

```bash
# ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
```

### 2. Match Model to Task

| Task Type | Recommended |
|-----------|-------------|
| Complex reasoning | Claude Opus, GPT-4 |
| Code generation | Claude Sonnet, CodeLlama |
| Fast iteration | Claude Haiku, GPT-3.5 |
| Privacy-sensitive | Ollama (local) |

### 3. Set Appropriate Timeouts

```toml
# Cloud APIs: 120s is usually enough
[providers.anthropic]
timeout_secs = 120

# Local models: allow more time
[providers.ollama]
timeout_secs = 300
```

### 4. Configure Retries

```toml
[providers.anthropic]
max_retries = 3
retry_backoff_ms = 1000
```

---

## Troubleshooting

### "API Key Invalid"

```bash
# Check environment
echo $ANTHROPIC_API_KEY

# Verify in config
cat ~/.descartes/config.toml | grep api_key
```

### "Connection Refused" (Ollama)

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama
ollama serve
```

### "Rate Limited"

Reduce `rate_limit_rpm` or add delays between requests:

```toml
[providers.anthropic]
rate_limit_rpm = 30  # Conservative limit
```

---

## Next Steps

- **[Session Management →](05-session-management.md)** — Understand session lifecycle
- **[Agent Types →](06-agent-types.md)** — Choose the right tool level
- **[Flow Workflow →](07-flow-workflow.md)** — Multi-phase automation

---

*With providers configured, you're ready to put your AI agents to work.*
