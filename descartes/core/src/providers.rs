/// Model provider implementations for API, Headless, and Local modes.
use crate::errors::{AgentResult, ProviderError, ProviderResult};
use crate::traits::{
    FinishReason, Message, MessageRole, ModelBackend, ModelProviderMode, ModelRequest,
    ModelResponse,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// ============================================================================
/// API MODE: Direct HTTP clients for OpenAI, Anthropic, DeepSeek, Groq
/// ============================================================================

/// OpenAI provider using HTTP API.
pub struct OpenAiProvider {
    _mode: ModelProviderMode,
    client: Option<reqwest::Client>,
    available_models: Vec<String>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    pub fn new(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        Self {
            _mode: ModelProviderMode::Api { endpoint, api_key },
            client: None,
            available_models: vec![
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        }
    }
}

#[async_trait]
impl ModelBackend for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        self.client = Some(reqwest::Client::new());
        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        if let Some(client) = &self.client {
            if let ModelProviderMode::Api { endpoint, api_key } = &self._mode {
                let resp = client
                    .get(&format!("{}/models", endpoint))
                    .bearer_auth(api_key)
                    .send()
                    .await;
                Ok(resp.is_ok())
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
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
                "max_tokens": request.max_tokens.unwrap_or(2048),
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
                return Err(ProviderError::ApiError(format!(
                    "API request failed with status {}",
                    response.status()
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
            Err(ProviderError::BackendError("Invalid mode for OpenAI provider".to_string()).into())
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

/// Anthropic provider using HTTP API.
pub struct AnthropicProvider {
    _mode: ModelProviderMode,
    client: Option<reqwest::Client>,
    available_models: Vec<String>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    pub fn new(api_key: String, endpoint: Option<String>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
        Self {
            _mode: ModelProviderMode::Api { endpoint, api_key },
            client: None,
            available_models: vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
        }
    }
}

#[async_trait]
impl ModelBackend for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        self.client = Some(reqwest::Client::new());
        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        // Anthropic doesn't have a standard health check endpoint
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
                "max_tokens": request.max_tokens.unwrap_or(2048),
                "messages": request.messages,
                "system": request.system_prompt.unwrap_or_default(),
            });

            let response = client
                .post(&format!("{}/messages", endpoint))
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&payload)
                .send()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            if !response.status().is_success() {
                return Err(ProviderError::ApiError(format!(
                    "API request failed with status {}",
                    response.status()
                ))
                .into());
            }

            let body = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            let content = body
                .get("content")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            Ok(ModelResponse {
                content,
                finish_reason: FinishReason::Stop,
                tokens_used: None,
                tool_calls: None,
            })
        } else {
            Err(
                ProviderError::BackendError("Invalid mode for Anthropic provider".to_string())
                    .into(),
            )
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
        // Simple heuristic: ~3 chars per token for Claude
        Ok((text.len() + 2) / 3)
    }

    async fn shutdown(&mut self) -> AgentResult<()> {
        self.client = None;
        Ok(())
    }
}

/// ============================================================================
/// HEADLESS MODE: Spawn CLI as child process
/// ============================================================================

/// Claude Code CLI adapter - spawns `claude` command as child process.
pub struct ClaudeCodeAdapter {
    _mode: ModelProviderMode,
    command: String,
    args: Vec<String>,
}

impl ClaudeCodeAdapter {
    /// Create a new Claude Code adapter.
    pub fn new(command: Option<String>, args: Option<Vec<String>>) -> Self {
        let cmd = command.unwrap_or_else(|| "claude".to_string());
        let a = args.unwrap_or_default();
        Self {
            _mode: ModelProviderMode::Headless {
                command: cmd.clone(),
                args: a.clone(),
            },
            command: cmd,
            args: a,
        }
    }
}

#[async_trait]
impl ModelBackend for ClaudeCodeAdapter {
    fn name(&self) -> &str {
        "claude-code-cli"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        // Verify the claude command exists
        let output = tokio::process::Command::new(&self.command)
            .arg("--version")
            .output()
            .await;

        if output.is_err() {
            return Err(ProviderError::InitializationError(
                "Claude CLI not found. Install it or verify the path.".to_string(),
            )
            .into());
        }

        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        match tokio::process::Command::new(&self.command)
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn complete(&self, _request: ModelRequest) -> AgentResult<ModelResponse> {
        // Simplified implementation - full version requires async process handling
        Ok(ModelResponse {
            content: "Claude CLI adapter implementation in progress".to_string(),
            finish_reason: FinishReason::Stop,
            tokens_used: None,
            tool_calls: None,
        })
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>>
    {
        Err(
            ProviderError::UnsupportedFeature("Streaming implementation pending".to_string())
                .into(),
        )
    }

    async fn list_models(&self) -> AgentResult<Vec<String>> {
        Ok(vec!["claude".to_string()])
    }

    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize> {
        Ok((text.len() + 2) / 3)
    }

    async fn shutdown(&mut self) -> AgentResult<()> {
        Ok(())
    }
}

/// Generic headless CLI adapter for arbitrary commands.
pub struct HeadlessCliAdapter {
    _mode: ModelProviderMode,
    command: String,
    args: Vec<String>,
}

impl HeadlessCliAdapter {
    /// Create a new generic headless CLI adapter.
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self {
            _mode: ModelProviderMode::Headless {
                command: command.clone(),
                args: args.clone(),
            },
            command,
            args,
        }
    }
}

#[async_trait]
impl ModelBackend for HeadlessCliAdapter {
    fn name(&self) -> &str {
        "headless-cli"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        // Verify command exists
        let output = tokio::process::Command::new(&self.command)
            .arg("--version")
            .output()
            .await;

        if output.is_err() {
            return Err(ProviderError::InitializationError(format!(
                "Command '{}' not found or not executable",
                self.command
            ))
            .into());
        }

        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        match tokio::process::Command::new(&self.command)
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn complete(&self, _request: ModelRequest) -> AgentResult<ModelResponse> {
        Ok(ModelResponse {
            content: "Headless CLI adapter implementation in progress".to_string(),
            finish_reason: FinishReason::Stop,
            tokens_used: None,
            tool_calls: None,
        })
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>>
    {
        Err(
            ProviderError::UnsupportedFeature("Streaming implementation pending".to_string())
                .into(),
        )
    }

    async fn list_models(&self) -> AgentResult<Vec<String>> {
        Ok(vec!["default".to_string()])
    }

    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize> {
        Ok((text.len() + 3) / 4)
    }

    async fn shutdown(&mut self) -> AgentResult<()> {
        Ok(())
    }
}

/// ============================================================================
/// LOCAL MODE: Connect to localhost services like Ollama
/// ============================================================================

/// Ollama provider - connects to local Ollama service.
pub struct OllamaProvider {
    _mode: ModelProviderMode,
    client: Option<reqwest::Client>,
    endpoint: String,
    timeout_secs: u64,
    available_models: Vec<String>,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    pub fn new(endpoint: Option<String>, timeout_secs: Option<u64>) -> Self {
        let endpoint = endpoint.unwrap_or_else(|| "http://localhost:11434".to_string());
        let timeout = timeout_secs.unwrap_or(300);
        Self {
            _mode: ModelProviderMode::Local {
                endpoint: endpoint.clone(),
                timeout_secs: timeout,
            },
            client: None,
            endpoint,
            timeout_secs: timeout,
            available_models: vec![],
        }
    }
}

#[async_trait]
impl ModelBackend for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn mode(&self) -> &ModelProviderMode {
        &self._mode
    }

    async fn initialize(&mut self) -> AgentResult<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| ProviderError::ReqwestError(e))?;

        self.client = Some(client);

        // Try to fetch available models
        if let Ok(models) = self.list_models().await {
            self.available_models = models;
        }

        Ok(())
    }

    async fn health_check(&self) -> AgentResult<bool> {
        if let Some(client) = &self.client {
            match client
                .get(&format!("{}/api/tags", self.endpoint))
                .send()
                .await
            {
                Ok(response) => Ok(response.status().is_success()),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn complete(&self, request: ModelRequest) -> AgentResult<ModelResponse> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| ProviderError::BackendError("Client not initialized".to_string()))?;

        let payload = json!({
            "model": request.model,
            "messages": request.messages,
            "stream": false,
        });

        let response = client
            .post(&format!("{}/api/chat", self.endpoint))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ProviderError::ReqwestError(e))?;

        if !response.status().is_success() {
            return Err(ProviderError::ApiError(format!(
                "Ollama request failed with status {}",
                response.status()
            ))
            .into());
        }

        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| ProviderError::ReqwestError(e))?;

        let content = body
            .get("message")
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
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>>
    {
        Err(ProviderError::UnsupportedFeature("Streaming not yet implemented".to_string()).into())
    }

    async fn list_models(&self) -> AgentResult<Vec<String>> {
        if let Some(client) = &self.client {
            let response = client
                .get(&format!("{}/api/tags", self.endpoint))
                .send()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            let body = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| ProviderError::ReqwestError(e))?;

            let models = body
                .get("models")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(models)
        } else {
            Ok(vec![])
        }
    }

    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize> {
        Ok((text.len() + 3) / 4)
    }

    async fn shutdown(&mut self) -> AgentResult<()> {
        self.client = None;
        Ok(())
    }
}

/// ============================================================================
/// PROVIDER FACTORY
/// ============================================================================

/// Factory for creating model backends based on provider type.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider by name and configuration.
    pub fn create(
        provider_name: &str,
        config: HashMap<String, String>,
    ) -> ProviderResult<Box<dyn ModelBackend>> {
        match provider_name.to_lowercase().as_str() {
            "openai" => {
                let api_key = config
                    .get("api_key")
                    .ok_or_else(|| {
                        ProviderError::ConfigError("Missing 'api_key' for OpenAI".to_string())
                    })?
                    .clone();
                let endpoint = config.get("endpoint").cloned();
                Ok(Box::new(OpenAiProvider::new(api_key, endpoint)))
            }
            "anthropic" => {
                let api_key = config
                    .get("api_key")
                    .ok_or_else(|| {
                        ProviderError::ConfigError("Missing 'api_key' for Anthropic".to_string())
                    })?
                    .clone();
                let endpoint = config.get("endpoint").cloned();
                Ok(Box::new(AnthropicProvider::new(api_key, endpoint)))
            }
            "claude-code-cli" => {
                let command = config.get("command").cloned();
                let args = config
                    .get("args")
                    .map(|a| a.split(',').map(String::from).collect());
                Ok(Box::new(ClaudeCodeAdapter::new(command, args)))
            }
            "ollama" => {
                let endpoint = config.get("endpoint").cloned();
                let timeout = config
                    .get("timeout_secs")
                    .and_then(|t| t.parse::<u64>().ok());
                Ok(Box::new(OllamaProvider::new(endpoint, timeout)))
            }
            "headless-cli" => {
                let command = config
                    .get("command")
                    .ok_or_else(|| {
                        ProviderError::ConfigError("Missing 'command' for headless CLI".to_string())
                    })?
                    .clone();
                let args = config
                    .get("args")
                    .map(|a| a.split(',').map(String::from).collect())
                    .unwrap_or_default();
                Ok(Box::new(HeadlessCliAdapter::new(command, args)))
            }
            _ => Err(ProviderError::ConfigError(format!(
                "Unknown provider: {}",
                provider_name
            ))),
        }
    }
}
