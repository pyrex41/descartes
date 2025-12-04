/// JSON-RPC 2.0 server implementation
use crate::auth::{AuthContext, AuthManager};
use crate::errors::{DaemonError, DaemonResult};
use crate::handlers::RpcHandlers;
use crate::metrics::MetricsCollector;
use crate::types::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

/// JSON-RPC Server
pub struct JsonRpcServer {
    handlers: Arc<RpcHandlers>,
    auth: Option<Arc<AuthManager>>,
    metrics: Arc<MetricsCollector>,
}

impl JsonRpcServer {
    /// Create a new JSON-RPC server
    pub fn new(
        handlers: Arc<RpcHandlers>,
        auth: Option<Arc<AuthManager>>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        JsonRpcServer {
            handlers,
            auth,
            metrics,
        }
    }

    /// Process a JSON-RPC request
    pub async fn process_request(&self, request: RpcRequest) -> RpcResponse {
        let request_id = request.id.clone();
        let method = request.method.clone();
        let start = Instant::now();

        // Extract auth context
        let auth_context = match self.extract_auth_context(&request).await {
            Ok(ctx) => ctx,
            Err(e) => {
                error!("Authentication failed for {}: {}", method, e);
                return RpcResponse::error(-32001, "Authentication failed".to_string(), request_id);
            }
        };

        debug!("Processing RPC request: {} (id: {:?})", method, request_id);

        // Validate request
        if request.jsonrpc != "2.0" {
            return RpcResponse::error(-32600, "Invalid Request".to_string(), request_id);
        }

        // Process the method
        let result = match method.as_str() {
            "agent.spawn" => self.call_agent_spawn(request.params, auth_context).await,
            "agent.list" => self.call_agent_list(request.params, auth_context).await,
            "agent.kill" => self.call_agent_kill(request.params, auth_context).await,
            "agent.logs" => self.call_agent_logs(request.params, auth_context).await,
            "workflow.execute" => {
                self.call_workflow_execute(request.params, auth_context)
                    .await
            }
            "state.query" => self.call_state_query(request.params, auth_context).await,
            "system.health" => self.call_system_health(request.params, auth_context).await,
            "system.metrics" => self.call_system_metrics(request.params, auth_context).await,
            _ => Err(DaemonError::MethodNotFound(method.clone())),
        };

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        self.metrics.record_request(duration);

        // Build response
        match result {
            Ok(data) => {
                info!(
                    "RPC request successful: {} (duration: {:.3}s)",
                    method, duration
                );
                RpcResponse::success(data, request_id)
            }
            Err(e) => {
                error!("RPC request failed: {} - {}", method, e);
                self.metrics.record_error();
                let error = e.to_rpc_error();
                RpcResponse::error(
                    error["code"].as_i64().unwrap_or(-32603),
                    error["message"]
                        .as_str()
                        .unwrap_or("Internal error")
                        .to_string(),
                    request_id,
                )
            }
        }
    }

    /// Extract authentication context from request
    async fn extract_auth_context(&self, request: &RpcRequest) -> DaemonResult<AuthContext> {
        let auth_manager = match &self.auth {
            Some(manager) if manager.is_enabled() => manager,
            _ => return Ok(AuthContext::unauthenticated()),
        };

        let token = Self::resolve_auth_token(request)
            .ok_or_else(|| DaemonError::AuthError("Missing authentication token".to_string()))?;

        if let Ok(claims) = auth_manager.verify_token(&token) {
            return Ok(AuthContext::new(claims.sub, claims.scope));
        }

        if auth_manager.verify_api_key(&token).is_ok() {
            return Ok(AuthContext::new(
                "api-key".to_string(),
                vec!["*".to_string()],
            ));
        }

        Err(DaemonError::AuthError(
            "Invalid authentication token".to_string(),
        ))
    }

    fn resolve_auth_token(request: &RpcRequest) -> Option<String> {
        request
            .auth_token
            .clone()
            .or_else(|| Self::token_from_params(&request.params))
    }

    fn token_from_params(params: &Option<Value>) -> Option<String> {
        match params {
            Some(Value::Object(map)) => {
                if let Some(token) = map.get("auth_token").and_then(|v| v.as_str()) {
                    return Some(token.to_string());
                }

                if let Some(Value::Object(auth_obj)) = map.get("auth") {
                    if let Some(token) = auth_obj.get("token").and_then(|v| v.as_str()) {
                        return Some(token.to_string());
                    }
                }

                None
            }
            _ => None,
        }
    }

    // RPC method handlers

    async fn call_agent_spawn(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params =
            params.ok_or_else(|| DaemonError::InvalidRequest("Missing params".to_string()))?;
        self.handlers.handle_agent_spawn(params, auth).await
    }

    async fn call_agent_list(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params = params.unwrap_or_else(|| Value::Object(Default::default()));
        self.handlers.handle_agent_list(params, auth).await
    }

    async fn call_agent_kill(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params =
            params.ok_or_else(|| DaemonError::InvalidRequest("Missing params".to_string()))?;
        self.handlers.handle_agent_kill(params, auth).await
    }

    async fn call_agent_logs(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params =
            params.ok_or_else(|| DaemonError::InvalidRequest("Missing params".to_string()))?;
        self.handlers.handle_agent_logs(params, auth).await
    }

    async fn call_workflow_execute(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params =
            params.ok_or_else(|| DaemonError::InvalidRequest("Missing params".to_string()))?;
        self.handlers.handle_workflow_execute(params, auth).await
    }

    async fn call_state_query(
        &self,
        params: Option<Value>,
        auth: AuthContext,
    ) -> DaemonResult<Value> {
        let params = params.unwrap_or_else(|| Value::Object(Default::default()));
        self.handlers.handle_state_query(params, auth).await
    }

    async fn call_system_health(
        &self,
        _params: Option<Value>,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let response = HealthCheckResponse {
            status: "healthy".to_string(),
            version: crate::VERSION.to_string(),
            uptime_secs: self.metrics.server_start.elapsed().as_secs(),
            timestamp: chrono::Utc::now(),
        };

        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
    }

    async fn call_system_metrics(
        &self,
        _params: Option<Value>,
        _auth: AuthContext,
    ) -> DaemonResult<Value> {
        let response = self.metrics.get_metrics_response();
        Ok(serde_json::to_value(response)
            .map_err(|e| DaemonError::SerializationError(e.to_string()))?)
    }

    /// Handle batch requests (JSON-RPC 2.0)
    pub async fn process_batch(&self, requests: Vec<RpcRequest>) -> Vec<RpcResponse> {
        let mut responses = Vec::new();

        for request in requests {
            let response = self.process_request(request).await;
            responses.push(response);
        }

        responses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;

    #[tokio::test]
    async fn test_invalid_jsonrpc_version() {
        let handlers = Arc::new(RpcHandlers::new());
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let server = JsonRpcServer::new(handlers, None, metrics);

        let request = RpcRequest {
            jsonrpc: "1.0".to_string(),
            method: "agent.list".to_string(),
            params: None,
            id: Some(json!(1)),
            auth_token: None,
        };

        let response = server.process_request(request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32600);
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let handlers = Arc::new(RpcHandlers::new());
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let server = JsonRpcServer::new(handlers, None, metrics);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown.method".to_string(),
            params: None,
            id: Some(json!(1)),
            auth_token: None,
        };

        let response = server.process_request(request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_system_health() {
        let handlers = Arc::new(RpcHandlers::new());
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let server = JsonRpcServer::new(handlers, None, metrics);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "system.health".to_string(),
            params: None,
            id: Some(json!(1)),
            auth_token: None,
        };

        let response = server.process_request(request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_authentication_required_when_enabled() {
        let handlers = Arc::new(RpcHandlers::new());
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let auth_manager = Arc::new(
            AuthManager::new(AuthConfig {
                enabled: true,
                jwt_secret: "auth-secret".to_string(),
                token_expiry_secs: 3600,
                api_key: Some("api-test-key".to_string()),
            })
            .unwrap(),
        );
        let server = JsonRpcServer::new(handlers, Some(auth_manager), metrics);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "system.health".to_string(),
            params: None,
            id: Some(json!(1)),
            auth_token: None,
        };

        let response = server.process_request(request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[tokio::test]
    async fn test_authentication_with_valid_token() {
        let handlers = Arc::new(RpcHandlers::new());
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let auth_manager = Arc::new(
            AuthManager::new(AuthConfig {
                enabled: true,
                jwt_secret: "valid-secret".to_string(),
                token_expiry_secs: 3600,
                api_key: None,
            })
            .unwrap(),
        );
        let server = JsonRpcServer::new(handlers, Some(auth_manager.clone()), metrics);

        let token = auth_manager
            .generate_token("user-1", vec!["*".to_string()])
            .unwrap();

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "system.health".to_string(),
            params: None,
            id: Some(json!(1)),
            auth_token: Some(token.token),
        };

        let response = server.process_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
