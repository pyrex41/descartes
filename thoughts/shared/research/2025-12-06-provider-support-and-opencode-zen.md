---
date: 2025-12-06T22:10:30Z
researcher: reuben
git_commit: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
branch: master
repository: pyrex41/descartes
topic: "Provider Support and OpenCode Zen Integration"
tags: [research, codebase, providers, xai, grok, opencode-zen, pal-mcp]
status: complete
last_updated: 2025-12-06
last_updated_by: reuben
---

# Research: Provider Support and OpenCode Zen Integration

**Date**: 2025-12-06T22:10:30Z
**Researcher**: reuben
**Git Commit**: 41d09d4d1eff606d4c8c98a44bfc2296cbaef6d8
**Branch**: master
**Repository**: pyrex41/descartes

## Research Question

What providers are currently supported in the codebase? Is xAI supported? How to integrate with OpenCode Zen?

## Summary

The Descartes codebase supports **6 LLM providers** with varying implementation completeness. **xAI/Grok is fully supported and is actually the default primary provider**. OpenCode Zen refers to two distinct concepts: (1) a paid API gateway service, and (2) the PAL MCP Server for multi-AI orchestration.

## Detailed Findings

### Supported Providers

The codebase implements a unified provider abstraction system through the `ModelBackend` trait in three operational modes: API, Headless, and Local.

#### Fully Implemented Providers

| Provider | Factory Name | Default Endpoint | Environment Variable | Default Model |
|----------|-------------|------------------|---------------------|---------------|
| **OpenAI** | `openai` | `https://api.openai.com/v1` | `OPENAI_API_KEY` | `gpt-4-turbo` |
| **Anthropic** | `anthropic` | `https://api.anthropic.com/v1` | `ANTHROPIC_API_KEY` | `claude-3-5-sonnet-20241022` |
| **Grok (xAI)** | `grok` | `https://api.x.ai/v1` | `XAI_API_KEY` | `grok-4-1-fast-reasoning` |
| **Ollama** | `ollama` | `http://localhost:11434` | N/A | `llama2` |

#### Partially Implemented Providers

| Provider | Factory Name | Status |
|----------|-------------|--------|
| **Claude Code CLI** | `claude-code-cli` | Structure exists, `complete()` returns placeholder |
| **Generic Headless CLI** | `headless-cli` | Structure exists, `complete()` returns placeholder |

#### Configuration Only (No Implementation)

| Provider | Environment Variable | Default Model |
|----------|---------------------|---------------|
| **DeepSeek** | `DEEPSEEK_API_KEY` | `deepseek-chat` |
| **Groq** | `GROQ_API_KEY` | `mixtral-8x7b-32768` |

### xAI/Grok Support

**Status: Fully Implemented and Default Provider**

xAI is comprehensively supported across the codebase:

#### Provider Implementation (`providers.rs:272-404`)
```rust
pub struct GrokProvider {
    _mode: ModelProviderMode,
    client: Option<reqwest::Client>,
    available_models: Vec<String>,
}
```

#### Available Models
- `grok-4-1-fast-reasoning` (default)
- `grok-4-1-fast`
- `grok-4-1`
- `grok-3-latest`

#### Configuration (`config.rs:474-536`)
- **Enabled by default**: `true`
- **Environment Variable**: `XAI_API_KEY`
- **API Endpoint**: `https://api.x.ai/v1`
- **Console URL**: `https://console.x.ai`

#### Default Provider Status
In `config.rs:100-102`, Grok is set as the default primary provider:
```rust
fn default_primary_provider() -> String {
    "grok".to_string()
}
```

### OpenCode Zen

"OpenCode Zen" refers to two distinct but related projects:

#### 1. OpenCode Zen (AI Gateway Service)

**Source**: https://opencode.ai/zen

A paid API gateway service providing curated AI models optimized for coding agents.

**Key Features**:
- Curated models tested specifically for coding agents
- US-hosted infrastructure with zero-retention policies
- Transparent pricing (pay-as-you-go)

**Supported Models** (via gateway):
- OpenAI: GPT 5.1, GPT 5.1 Codex, GPT 5, GPT 5 Codex, GPT 5 Nano
- Anthropic: Claude Sonnet 4.5, Claude Haiku, Claude Opus
- Other: Gemini 3 Pro, GLM 4.6, Kimi K2, Qwen3 Coder, Grok Code

**API Endpoints**:
- OpenAI models: `https://opencode.ai/zen/v1/responses`
- Anthropic models: `https://opencode.ai/zen/v1/messages`

**Integration**: Access models via `opencode/<model-id>` format. Requires sign-in, billing setup, and API key from OpenCode.

#### 2. PAL MCP Server (formerly "Zen MCP")

**Source**: https://github.com/BeehiveInnovations/pal-mcp-server

An open-source MCP server for multi-AI orchestration.

**PAL = Provider Abstraction Layer**

**Key Features**:
- Enables Claude Code to collaborate with multiple AI models simultaneously
- Conversation threading across models
- CLI subagents ("clink"): Claude Code can spawn Codex/Gemini subagents
- Supports: Gemini, OpenAI, Anthropic, Grok, Azure, Ollama, OpenRouter

**Advanced Capabilities**:
- Multi-pass code reviews
- Systematic debugging with root cause analysis
- Automatic model selection
- Bypasses MCP's 25K token limit

**Installation**:
```bash
git clone https://github.com/BeehiveInnovations/pal-mcp-server.git
cd pal-mcp-server
```

**Note**: This project is currently configured in this codebase via MCP tools (see available `mcp__zen__*` tools).

## Code References

### Core Provider Files
- `descartes/core/src/providers.rs` - Main provider implementations
- `descartes/core/src/providers_test.rs` - Provider test suite
- `descartes/core/src/traits.rs:87-120` - ModelBackend trait definition
- `descartes/core/src/config.rs` - Provider configuration structs

### Grok/xAI Specific
- `descartes/core/src/providers.rs:272-404` - GrokProvider implementation
- `descartes/core/src/config.rs:474-536` - GrokConfig structure
- `descartes/core/src/config.rs:1494-1496` - XAI_API_KEY loading
- `descartes/cli/src/commands/spawn.rs:349-370` - CLI Grok integration
- `descartes/cli/src/commands/doctor.rs:214-216` - Health check

### Configuration
- `descartes/.descartes/config.toml.example` - Example configuration
- `descartes/core/src/config_loader.rs` - Configuration loading logic
- `descartes/core/src/secrets.rs` - API key secrets management

### Documentation
- `working_docs/implementation/PROVIDER_DESIGN.md` - Architecture documentation
- `working_docs/implementation/PROVIDER_EXAMPLES.md` - Usage examples

## Architecture Documentation

### Provider Modes (`traits.rs:76-85`)

Three operational modes:
1. **Api** - HTTP API with `endpoint` and `api_key`
2. **Headless** - Child process with `command` and `args`
3. **Local** - Local service with `endpoint` and `timeout_secs`

### ModelBackend Trait Methods
- `name()` - Provider identifier
- `mode()` - Current operational mode
- `initialize()` - Setup provider
- `health_check()` - Verify availability
- `complete()` - Synchronous completion
- `stream()` - Streaming responses (NOT YET IMPLEMENTED)
- `list_models()` - Available models
- `estimate_tokens()` - Token count estimation
- `shutdown()` - Graceful cleanup

### Environment Variables (Loaded in `config.rs:1475-1507`)
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `DEEPSEEK_API_KEY`
- `GROQ_API_KEY`
- `XAI_API_KEY`
- `DESCARTES_ENCRYPTION_KEY`
- `DESCARTES_SECRET_KEY`

## OpenCode Zen Integration Options

### Option A: Use OpenCode Zen as a Custom Provider

Add OpenCode Zen as a custom OpenAI-compatible provider:

```toml
[providers.custom.opencode_zen]
enabled = true
endpoint = "https://opencode.ai/zen/v1"
api_key = "<your-opencode-api-key>"
default_model = "opencode/gpt-5-codex"
```

The existing OpenAI provider implementation can be reused since OpenCode Zen uses OpenAI-compatible endpoints.

### Option B: Use PAL MCP Server

PAL MCP Server is already integrated in this environment (visible via `mcp__zen__*` tools). It provides:
- `mcp__zen__chat` - General chat with external models
- `mcp__zen__thinkdeep` - Multi-stage investigation
- `mcp__zen__codereview` - Systematic code review
- `mcp__zen__consensus` - Multi-model consensus building
- `mcp__zen__debug` - Root cause analysis
- And many more...

## Related Research

None found in `thoughts/shared/research/`.

## Open Questions

1. Should DeepSeek and Groq provider implementations be completed?
2. Should streaming be implemented for all providers?
3. Is OpenCode Zen gateway integration desired in addition to PAL MCP?
