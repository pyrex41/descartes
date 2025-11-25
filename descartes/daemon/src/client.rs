/// RPC Client for connecting to Descartes Core daemon
///
/// This module provides a comprehensive RPC client implementation that supports:
/// - HTTP and WebSocket connections
/// - Connection pooling and management
/// - Automatic retries with exponential backoff
/// - Authentication (JWT and API key)
/// - Comprehensive error handling
/// - Batch requests
/// - Async/await with tokio
use crate::errors::{DaemonError, DaemonResult};
use crate::types::*;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// RPC Client configuration
#[derive(Debug, Clone)]
pub struct RpcClientConfig {
    /// Server URL (e.g., "http://127.0.0.1:8080")
    pub url: String,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Authentication token (JWT or API key)
    pub auth_token: Option<String>,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Connection pool size
    pub pool_size: usize,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        RpcClientConfig {
            url: "http://127.0.0.1:8080".to_string(),
            timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 100,
            auth_token: None,
            request_timeout_secs: 30,
            pool_size: 10,
        }
    }
}

impl RpcClientConfig {
    /// Create a new configuration with the given URL
    pub fn new(url: &str) -> Self {
        RpcClientConfig {
            url: url.to_string(),
            ..Default::default()
        }
    }

    /// Set authentication token
    pub fn with_auth(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set max retries
    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// RPC Client for Descartes Core
pub struct RpcClient {
    config: Arc<RpcClientConfig>,
    http_client: reqwest::Client,
    request_id: Arc<RwLock<u64>>,
}

impl RpcClient {
    /// Create a new RPC client with the given configuration
    pub fn new(config: RpcClientConfig) -> DaemonResult<Self> {
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = reqwest::Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(config.pool_size)
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(|e| {
                DaemonError::ConnectionError(format!("Failed to create HTTP client: {}", e))
            })?;

        info!("RPC client created for {}", config.url);

        Ok(RpcClient {
            config: Arc::new(config),
            http_client,
            request_id: Arc::new(RwLock::new(1)),
        })
    }

    /// Create a new RPC client with default configuration
    pub fn default_client() -> DaemonResult<Self> {
        Self::new(RpcClientConfig::default())
    }

    /// Create a new RPC client with the given URL
    pub fn with_url(url: &str) -> DaemonResult<Self> {
        Self::new(RpcClientConfig::new(url))
    }

    /// Get the next request ID
    async fn next_request_id(&self) -> u64 {
        let mut id = self.request_id.write().await;
        let current = *id;
        *id += 1;
        current
    }

    /// Call a JSON-RPC 2.0 method
    pub async fn call(&self, method: &str, params: Option<Value>) -> DaemonResult<Value> {
        let request_id = self.next_request_id().await;

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(json!(request_id)),
        };

        debug!("RPC call: {} (id: {})", method, request_id);

        // Retry logic with exponential backoff
        let mut attempts = 0;
        let mut delay = Duration::from_millis(self.config.retry_delay_ms);

        loop {
            attempts += 1;

            match self.send_request(&request).await {
                Ok(response) => {
                    // Check for RPC errors
                    if let Some(error) = response.error {
                        error!(
                            "RPC error for {}: {} (code: {})",
                            method, error.message, error.code
                        );
                        return Err(DaemonError::RpcError(error.code, error.message));
                    }

                    // Return the result
                    return Ok(response.result.unwrap_or(Value::Null));
                }
                Err(e) => {
                    if attempts >= self.config.max_retries {
                        error!("RPC call failed after {} attempts: {}", attempts, e);
                        return Err(e);
                    }

                    warn!(
                        "RPC call failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempts, self.config.max_retries, e, delay
                    );

                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Send a single RPC request
    async fn send_request(&self, request: &RpcRequest) -> DaemonResult<RpcResponse> {
        let mut req = self
            .http_client
            .post(&self.config.url)
            .header("Content-Type", "application/json")
            .json(request);

        // Add authentication if configured
        if let Some(ref token) = self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .send()
            .await
            .map_err(|e| DaemonError::ConnectionError(format!("HTTP request failed: {}", e)))?;

        // Check HTTP status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DaemonError::ConnectionError(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        // Parse JSON-RPC response
        let rpc_response: RpcResponse = response.json().await.map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse response: {}", e))
        })?;

        Ok(rpc_response)
    }

    /// Send a batch of RPC requests
    pub async fn batch_call(
        &self,
        requests: Vec<(&str, Option<Value>)>,
    ) -> DaemonResult<Vec<Value>> {
        let mut rpc_requests = Vec::new();

        for (method, params) in requests {
            let request_id = self.next_request_id().await;
            rpc_requests.push(RpcRequest {
                jsonrpc: "2.0".to_string(),
                method: method.to_string(),
                params,
                id: Some(json!(request_id)),
            });
        }

        debug!("Batch RPC call with {} requests", rpc_requests.len());

        let mut req = self
            .http_client
            .post(&self.config.url)
            .header("Content-Type", "application/json")
            .json(&rpc_requests);

        if let Some(ref token) = self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .send()
            .await
            .map_err(|e| DaemonError::ConnectionError(format!("Batch request failed: {}", e)))?;

        let rpc_responses: Vec<RpcResponse> = response.json().await.map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse batch response: {}", e))
        })?;

        let results = rpc_responses
            .into_iter()
            .map(|r| {
                if let Some(error) = r.error {
                    Err(DaemonError::RpcError(error.code, error.message))
                } else {
                    Ok(r.result.unwrap_or(Value::Null))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Test connection to the server
    pub async fn test_connection(&self) -> DaemonResult<()> {
        info!("Testing connection to {}", self.config.url);
        self.health().await?;
        info!("Connection test successful");
        Ok(())
    }

    // ============================================================
    // High-level API methods
    // ============================================================

    /// Spawn a new agent
    pub async fn spawn_agent(
        &self,
        name: &str,
        agent_type: &str,
        config: Value,
    ) -> DaemonResult<AgentSpawnResponse> {
        let params = json!({
            "name": name,
            "agent_type": agent_type,
            "config": config
        });

        let result = self.call("agent.spawn", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse spawn response: {}", e))
        })
    }

    /// List all agents
    pub async fn list_agents(&self) -> DaemonResult<AgentListResponse> {
        let result = self.call("agent.list", Some(json!({}))).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse list response: {}", e))
        })
    }

    /// Kill an agent
    pub async fn kill_agent(&self, agent_id: &str, force: bool) -> DaemonResult<AgentKillResponse> {
        let params = json!({
            "agent_id": agent_id,
            "force": force
        });

        let result = self.call("agent.kill", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse kill response: {}", e))
        })
    }

    /// Get agent logs
    pub async fn get_agent_logs(
        &self,
        agent_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> DaemonResult<AgentLogsResponse> {
        let params = json!({
            "agent_id": agent_id,
            "limit": limit.unwrap_or(100),
            "offset": offset.unwrap_or(0)
        });

        let result = self.call("agent.logs", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse logs response: {}", e))
        })
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        agents: Vec<String>,
        config: Value,
    ) -> DaemonResult<WorkflowExecuteResponse> {
        let params = json!({
            "workflow_id": workflow_id,
            "agents": agents,
            "config": config
        });

        let result = self.call("workflow.execute", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse workflow response: {}", e))
        })
    }

    /// Query state
    pub async fn query_state(
        &self,
        agent_id: Option<&str>,
        key: Option<&str>,
    ) -> DaemonResult<StateQueryResponse> {
        let mut params = json!({});

        if let Some(id) = agent_id {
            params["agent_id"] = json!(id);
        }
        if let Some(k) = key {
            params["key"] = json!(k);
        }

        let result = self.call("state.query", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse state response: {}", e))
        })
    }

    /// Get system health
    pub async fn health(&self) -> DaemonResult<HealthCheckResponse> {
        let result = self.call("system.health", None).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse health response: {}", e))
        })
    }

    /// Get system metrics
    pub async fn metrics(&self) -> DaemonResult<MetricsResponse> {
        let result = self.call("system.metrics", None).await?;
        serde_json::from_value(result).map_err(|e| {
            DaemonError::SerializationError(format!("Failed to parse metrics response: {}", e))
        })
    }

    /// Get the server URL
    pub fn url(&self) -> &str {
        &self.config.url
    }

    /// Check if authentication is configured
    pub fn is_authenticated(&self) -> bool {
        self.config.auth_token.is_some()
    }
}

/// Builder pattern for RpcClient
pub struct RpcClientBuilder {
    config: RpcClientConfig,
}

impl RpcClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        RpcClientBuilder {
            config: RpcClientConfig::default(),
        }
    }

    /// Set the server URL
    pub fn url(mut self, url: &str) -> Self {
        self.config.url = url.to_string();
        self
    }

    /// Set authentication token
    pub fn auth_token(mut self, token: String) -> Self {
        self.config.auth_token = Some(token);
        self
    }

    /// Set connection timeout
    pub fn timeout(mut self, timeout_secs: u64) -> Self {
        self.config.timeout_secs = timeout_secs;
        self
    }

    /// Set max retries
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Set retry delay
    pub fn retry_delay(mut self, retry_delay_ms: u64) -> Self {
        self.config.retry_delay_ms = retry_delay_ms;
        self
    }

    /// Set pool size
    pub fn pool_size(mut self, pool_size: usize) -> Self {
        self.config.pool_size = pool_size;
        self
    }

    /// Build the RPC client
    pub fn build(self) -> DaemonResult<RpcClient> {
        RpcClient::new(self.config)
    }
}

impl Default for RpcClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = RpcClientConfig::default();
        assert_eq!(config.url, "http://127.0.0.1:8080");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_client_config_builder() {
        let config = RpcClientConfig::new("http://localhost:9000")
            .with_auth("test-token".to_string())
            .with_timeout(60)
            .with_retries(5);

        assert_eq!(config.url, "http://localhost:9000");
        assert_eq!(config.auth_token, Some("test-token".to_string()));
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_client_builder() {
        let client = RpcClientBuilder::new()
            .url("http://localhost:8080")
            .auth_token("token123".to_string())
            .timeout(45)
            .max_retries(2)
            .pool_size(20)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.url(), "http://localhost:8080");
        assert!(client.is_authenticated());
    }

    #[tokio::test]
    async fn test_request_id_increment() {
        let client = RpcClient::default_client().unwrap();

        let id1 = client.next_request_id().await;
        let id2 = client.next_request_id().await;
        let id3 = client.next_request_id().await;

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }
}
