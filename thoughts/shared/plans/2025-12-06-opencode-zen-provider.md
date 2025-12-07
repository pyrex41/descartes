# OpenCode Zen Provider Implementation Plan

## Overview

Add OpenCode Zen as a dedicated provider to Descartes, providing access to curated AI models optimized for coding agents via the OpenCode Zen gateway at `https://opencode.ai/zen/v1`.

## Current State Analysis

The codebase supports 6 LLM providers through a unified `ModelBackend` trait:
- **Fully implemented**: OpenAI, Anthropic, Grok (xAI), Ollama
- **Partial**: Claude Code CLI, Headless CLI
- **Config only**: DeepSeek, Groq

OpenCode Zen uses an **OpenAI-compatible API**, meaning we can base the implementation on the existing `OpenAiProvider` with minimal changes.

### Key Discoveries:
- Provider implementations live in `descartes/core/src/providers.rs`
- Configuration structs are in `descartes/core/src/config.rs:70-98` (ProvidersConfig)
- CLI spawn logic is in `descartes/cli/src/commands/spawn.rs:349-385`
- Health checks are in `descartes/cli/src/commands/doctor.rs:214-220`
- Environment variables are loaded in `config.rs:1475-1507`

## Desired End State

After implementation:
1. Users can run `descartes spawn --provider opencode-zen --task "..."`
2. `OPENCODE_API_KEY` environment variable is recognized
3. `descartes doctor` shows OpenCode Zen status
4. Config file supports `[providers.opencode_zen]` section
5. Curated model list for coding agents is available

### Verification:
```bash
export OPENCODE_API_KEY=your-key
descartes doctor  # Should show OpenCode Zen status
descartes spawn --provider opencode-zen --task "Hello world"
```

## What We're NOT Doing

- Not implementing streaming (consistent with other providers)
- Not adding Anthropic-style `/messages` endpoint support (using OpenAI-compatible only)
- Not implementing `/v1/models` auto-discovery (not yet available from OpenCode Zen)
- Not adding team/billing features (out of scope for provider)

## Implementation Approach

Since OpenCode Zen is OpenAI-compatible, we'll:
1. Create a new `OpenCodeZenProvider` struct based on `OpenAiProvider`
2. Add corresponding configuration struct
3. Wire up CLI and health checks
4. Use hardcoded model list (API doesn't expose `/models` endpoint yet)

---

## Phase 1: Add Configuration Struct

### Overview
Add `OpenCodeZenConfig` to the configuration system.

### Changes Required:

#### 1. ProvidersConfig Struct
**File**: `descartes/core/src/config.rs`
**Location**: After line 93 (after `pub grok: GrokConfig`)

Add field to `ProvidersConfig`:
```rust
    /// OpenCode Zen provider settings
    #[serde(default)]
    pub opencode_zen: OpenCodeZenConfig,
```

#### 2. ProvidersConfig Default Implementation
**File**: `descartes/core/src/config.rs`
**Location**: In `impl Default for ProvidersConfig` (around line 104-116)

Add to the Default impl:
```rust
            opencode_zen: OpenCodeZenConfig::default(),
```

#### 3. OpenCodeZenConfig Struct
**File**: `descartes/core/src/config.rs`
**Location**: After `GrokConfig` (after line 536)

Add new config struct:
```rust
/// OpenCode Zen provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeZenConfig {
    /// Whether OpenCode Zen provider is enabled
    #[serde(default)]
    pub enabled: bool,

    /// API key (can be read from env: OPENCODE_API_KEY)
    #[serde(default)]
    pub api_key: Option<String>,

    /// API endpoint
    #[serde(default = "default_opencode_zen_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_opencode_zen_model")]
    pub model: String,

    /// Available models
    #[serde(default = "default_opencode_zen_models")]
    pub models: Vec<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for OpenCodeZenConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            endpoint: default_opencode_zen_endpoint(),
            model: default_opencode_zen_model(),
            models: default_opencode_zen_models(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_opencode_zen_endpoint() -> String {
    "https://opencode.ai/zen/v1".to_string()
}

fn default_opencode_zen_model() -> String {
    "opencode/qwen3-coder".to_string()
}

fn default_opencode_zen_models() -> Vec<String> {
    vec![
        // OpenAI models
        "opencode/gpt-5.1-codex".to_string(),
        "opencode/gpt-5-codex".to_string(),
        "opencode/gpt-5-nano".to_string(),
        // Anthropic models
        "opencode/claude-sonnet-4.5".to_string(),
        "opencode/claude-haiku-4.5".to_string(),
        "opencode/claude-opus-4".to_string(),
        // Google models
        "opencode/gemini-3-pro".to_string(),
        // OpenAI-compatible models (coding-focused)
        "opencode/qwen3-coder".to_string(),
        "opencode/kimi-k2".to_string(),
        "opencode/grok-code-fast-1".to_string(),
        "opencode/glm-4.6".to_string(),
    ]
}
```

#### 4. Environment Variable Loading
**File**: `descartes/core/src/config.rs`
**Location**: In `load_from_env` method (after line 1496)

Add after the `XAI_API_KEY` block:
```rust
        if let Ok(key) = std::env::var("OPENCODE_API_KEY") {
            self.config.providers.opencode_zen.api_key = Some(key);
            self.config.providers.opencode_zen.enabled = true;
        }
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cargo build -p descartes-core`
- [ ] Existing tests pass: `cargo test -p descartes-core`

#### Manual Verification:
- [ ] Config can be loaded with `[providers.opencode_zen]` section

---

## Phase 2: Add Provider Implementation

### Overview
Add `OpenCodeZenProvider` struct implementing `ModelBackend` trait.

### Changes Required:

#### 1. OpenCodeZenProvider Struct
**File**: `descartes/core/src/providers.rs`
**Location**: After `GrokProvider` implementation (after line 404)

Add new provider:
```rust
/// OpenCode Zen provider using HTTP API (OpenAI-compatible).
/// Provides access to curated AI models optimized for coding agents.
pub struct OpenCodeZenProvider {
    _mode: ModelProviderMode,
    client: Option<reqwest::Client>,
    available_models: Vec<String>,
}

impl OpenCodeZenProvider {
    /// Create a new OpenCode Zen provider.
    pub fn new(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://opencode.ai/zen/v1".to_string());
        Self {
            _mode: ModelProviderMode::Api { endpoint, api_key },
            client: None,
            available_models: vec![
                "opencode/gpt-5.1-codex".to_string(),
                "opencode/gpt-5-codex".to_string(),
                "opencode/gpt-5-nano".to_string(),
                "opencode/claude-sonnet-4.5".to_string(),
                "opencode/claude-haiku-4.5".to_string(),
                "opencode/claude-opus-4".to_string(),
                "opencode/gemini-3-pro".to_string(),
                "opencode/qwen3-coder".to_string(),
                "opencode/kimi-k2".to_string(),
                "opencode/grok-code-fast-1".to_string(),
                "opencode/glm-4.6".to_string(),
            ],
        }
    }
}

#[async_trait]
impl ModelBackend for OpenCodeZenProvider {
    fn name(&self) -> &str {
        "opencode-zen"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        self.client = Some(reqwest::Client::new());
        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        // OpenCode Zen doesn't have a /models endpoint yet, so just check client exists
        Ok(self.client.is_some())
    }

    async fn complete(&self, request: ModelRequest) -> AgentResult<ModelResponse> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| ProviderError::BackendError("Client not initialized".to_string()))?;

        if let ModelProviderMode::Api { endpoint, api_key } = &self._mode {
            let payload = json!({
                "model": request.model,
                "messages": request.messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
            });

            let response = client
                .post(&format!("{}/chat/completions", endpoint))
                .bearer_auth(api_key)
                .json(&payload)
                .send()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(ProviderError::ApiError(format!(
                    "OpenCode Zen API request failed with status {}: {}",
                    status, body
                ))
                .into());
            }

            let body = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            let content = body
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            Ok(ModelResponse {
                content,
                finish_reason: FinishReason::Stop,
                tokens_used: None,
                tool_calls: None,
            })
        } else {
            Err(ProviderError::BackendError("Invalid mode for OpenCode Zen provider".to_string()).into())
        }
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>>
    {
        Err(ProviderError::UnsupportedFeature("Streaming not yet implemented".to_string()).into())
    }

    async fn list_models(&self) -> AgentResult<Vec<String>> {
        Ok(self.available_models.clone())
    }

    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize> {
        // Simple heuristic: ~4 chars per token
        Ok((text.len() + 3) / 4)
    }

    async fn shutdown(&mut self) -> AgentResult<()> {
        self.client = None;
        Ok(())
    }
}
```

#### 2. ProviderFactory Update
**File**: `descartes/core/src/providers.rs`
**Location**: In `ProviderFactory::create` match block (after line 820, after "grok" case)

Add new match arm:
```rust
            "opencode-zen" => {
                let api_key = config
                    .get("api_key")
                    .ok_or_else(|| {
                        ProviderError::ConfigError("Missing 'api_key' for OpenCode Zen".to_string())
                    })?
                    .clone();
                let endpoint = config.get("endpoint").cloned();
                Ok(Box::new(OpenCodeZenProvider::new(api_key, endpoint)))
            }
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cargo build -p descartes-core`
- [ ] Unit tests pass: `cargo test -p descartes-core`

#### Manual Verification:
- [ ] Provider can be instantiated via factory

---

## Phase 3: CLI Integration

### Overview
Update spawn command to recognize and configure the OpenCode Zen provider.

### Changes Required:

#### 1. Provider Config Handling in spawn.rs
**File**: `descartes/cli/src/commands/spawn.rs`
**Location**: In the provider match block (after line 370, after "grok" case)

Add new match arm before the `_ =>` default case:
```rust
        "opencode-zen" => {
            match &config.providers.opencode_zen.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ OpenCode Zen API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export OPENCODE_API_KEY=...".cyan());
                    eprintln!();
                    eprintln!("  Get your key at: {}", "https://opencode.ai/zen".cyan());
                    eprintln!();
                    anyhow::bail!("OpenCode Zen API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.opencode_zen.endpoint.clone(),
            );
        }
```

#### 2. Update Available Providers List
**File**: `descartes/cli/src/commands/spawn.rs`
**Location**: In the `_ =>` default case error message (around line 375-382)

Update the available providers list to include opencode-zen:
```rust
            eprintln!("  Available providers:");
            eprintln!("    {} - Grok models (default)", "grok".cyan());
            eprintln!("    {} - Claude models", "anthropic".cyan());
            eprintln!("    {} - GPT models", "openai".cyan());
            eprintln!("    {} - Curated coding models", "opencode-zen".cyan());
            eprintln!("    {} - Local models", "ollama".cyan());
            eprintln!("    {} - DeepSeek models", "deepseek".cyan());
            eprintln!("    {} - Fast inference", "groq".cyan());
```

#### 3. Update get_model_for_provider Function
**File**: `descartes/cli/src/commands/spawn.rs`
**Location**: In `get_model_for_provider` match block (around line 402-409)

Add new match arm:
```rust
        "opencode-zen" => Ok(config.providers.opencode_zen.model.clone()),
```

### Success Criteria:

#### Automated Verification:
- [ ] CLI compiles: `cargo build -p descartes-cli`
- [ ] CLI tests pass: `cargo test -p descartes-cli`

#### Manual Verification:
- [ ] `descartes spawn --provider opencode-zen --task "test"` shows proper error when no API key set
- [ ] Unknown provider error message lists opencode-zen

---

## Phase 4: Health Check Integration

### Overview
Add OpenCode Zen to the doctor command's API key checks.

### Changes Required:

#### 1. Add to Providers List
**File**: `descartes/cli/src/commands/doctor.rs`
**Location**: In the providers array (around line 214-220)

Update the providers array:
```rust
    let providers = [
        ("XAI_API_KEY", "Grok (xAI)"),
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("OPENCODE_API_KEY", "OpenCode Zen"),
        ("DEEPSEEK_API_KEY", "DeepSeek"),
        ("GROQ_API_KEY", "Groq"),
    ];
```

### Success Criteria:

#### Automated Verification:
- [ ] CLI compiles: `cargo build -p descartes-cli`
- [ ] CLI tests pass: `cargo test -p descartes-cli`

#### Manual Verification:
- [ ] `descartes doctor` shows OpenCode Zen status
- [ ] Shows "not configured" when OPENCODE_API_KEY is not set
- [ ] Shows configured when OPENCODE_API_KEY is set

---

## Phase 5: Documentation Update

### Overview
Update the example configuration file to include OpenCode Zen.

### Changes Required:

#### 1. Update Config Example
**File**: `descartes/.descartes/config.toml.example`

Add OpenCode Zen section:
```toml
# OpenCode Zen - Curated AI models for coding
[providers.opencode_zen]
enabled = false
# api_key = "your-opencode-api-key"  # Or set OPENCODE_API_KEY env var
endpoint = "https://opencode.ai/zen/v1"
model = "opencode/qwen3-coder"
```

### Success Criteria:

#### Automated Verification:
- [ ] Example file is valid TOML

#### Manual Verification:
- [ ] Config can be loaded with the example values

---

## Testing Strategy

### Unit Tests:
- Test `OpenCodeZenConfig` default values
- Test `OpenCodeZenProvider` factory creation
- Test provider name returns "opencode-zen"

### Integration Tests:
- Test provider can be created via factory with valid config
- Test proper error when API key is missing

### Manual Testing Steps:
1. Set `OPENCODE_API_KEY` environment variable
2. Run `descartes doctor` - verify OpenCode Zen shows as configured
3. Run `descartes spawn --provider opencode-zen --task "Write hello world in Python"`
4. Verify response is received from OpenCode Zen API

## Performance Considerations

No special performance considerations - uses same HTTP client approach as other providers.

## Migration Notes

No migration needed - this is a new provider addition. Existing configurations will continue to work unchanged.

## References

- OpenCode Zen Documentation: https://opencode.ai/docs/zen/
- OpenCode Zen API: https://opencode.ai/zen/v1
- Related research: `thoughts/shared/research/2025-12-06-provider-support-and-opencode-zen.md`
- Existing OpenAI provider: `descartes/core/src/providers.rs:16-146`
- Grok provider (similar pattern): `descartes/core/src/providers.rs:272-404`
