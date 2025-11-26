/// HTTP and WebSocket server implementation
use crate::auth::AuthManager;
use crate::config::DaemonConfig;
use crate::errors::{DaemonError, DaemonResult};
use crate::handlers::RpcHandlers;
use crate::metrics::MetricsCollector;
use crate::pool::ConnectionPool;
use crate::rpc::JsonRpcServer;
use crate::types::*;
use hyper::header::AUTHORIZATION;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// RPC Server
#[derive(Clone)]
pub struct RpcServer {
    config: DaemonConfig,
    handlers: Arc<RpcHandlers>,
    auth: Option<Arc<AuthManager>>,
    metrics: Arc<MetricsCollector>,
    pool: Arc<ConnectionPool>,
    rpc: Arc<JsonRpcServer>,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new(config: DaemonConfig) -> DaemonResult<Self> {
        config.validate()?;

        let metrics = Arc::new(MetricsCollector::new()?);
        let handlers = Arc::new(RpcHandlers::new());
        let pool = Arc::new(ConnectionPool::new(config.pool.clone()));

        let auth = if config.auth.enabled {
            Some(Arc::new(AuthManager::new(config.auth.clone())?))
        } else {
            None
        };

        let rpc = Arc::new(JsonRpcServer::new(
            handlers.clone(),
            auth.clone(),
            metrics.clone(),
        ));

        Ok(RpcServer {
            config,
            handlers,
            auth,
            metrics,
            pool,
            rpc,
        })
    }

    /// Start the HTTP server
    pub async fn start_http(&self) -> DaemonResult<()> {
        let addr = format!(
            "{}:{}",
            self.config.server.http_addr, self.config.server.http_port
        );
        let addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| DaemonError::ServerError(format!("Invalid address: {}", e)))?;

        let rpc = self.rpc.clone();
        let metrics = self.metrics.clone();

        let make_svc = make_service_fn(move |_conn| {
            let rpc = rpc.clone();
            let metrics = metrics.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let rpc = rpc.clone();
                    let metrics = metrics.clone();
                    handle_http_request(req, rpc, metrics)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        info!("HTTP RPC server listening on http://{}", addr);

        server
            .await
            .map_err(|e| DaemonError::ServerError(format!("HTTP server error: {}", e)))
    }

    /// Start the metrics endpoint
    pub async fn start_metrics(&self) -> DaemonResult<()> {
        if !self.config.server.enable_metrics {
            return Ok(());
        }

        let addr = format!(
            "{}:{}",
            self.config.server.http_addr, self.config.server.metrics_port
        );
        let addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| DaemonError::ServerError(format!("Invalid address: {}", e)))?;

        let metrics = self.metrics.clone();

        let make_svc = make_service_fn(move |_conn| {
            let metrics = metrics.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |_req| {
                    let metrics = metrics.clone();
                    handle_metrics_request(metrics)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        info!("Metrics endpoint listening on http://{}", addr);

        server
            .await
            .map_err(|e| DaemonError::ServerError(format!("Metrics server error: {}", e)))
    }

    /// Run the server
    pub async fn run(&self) -> DaemonResult<()> {
        // Start HTTP server
        let server = self.clone();
        let http_handle = tokio::spawn(async move {
            if let Err(e) = server.start_http().await {
                error!("HTTP server error: {:?}", e);
            }
        });

        // Start metrics endpoint
        let server = self.clone();
        let metrics_handle = tokio::spawn(async move {
            if let Err(e) = server.start_metrics().await {
                error!("Metrics server error: {:?}", e);
            }
        });

        info!("RPC server started");

        // Wait for servers
        tokio::select! {
            _ = http_handle => {},
            _ = metrics_handle => {},
        }

        Ok(())
    }
}

/// Handle HTTP RPC requests
async fn handle_http_request(
    req: Request<Body>,
    rpc: Arc<JsonRpcServer>,
    metrics: Arc<MetricsCollector>,
) -> Result<Response<Body>, hyper::Error> {
    metrics.record_connection();

    match *req.method() {
        hyper::Method::POST => {
            let auth_header = req
                .headers()
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(|s| s.to_string());

            let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
            let header_token = auth_header
                .as_deref()
                .and_then(|raw| parse_bearer_token(raw).or_else(|| Some(raw.trim().to_string())));

            let result = match serde_json::from_slice::<RpcRequest>(&body_bytes) {
                Ok(mut request) => {
                    if request.auth_token.is_none() {
                        request.auth_token = header_token.clone();
                    }
                    let response = rpc.process_request(request).await;
                    serde_json::to_string(&response).unwrap_or_else(|_| {
                        json!({
                            "jsonrpc": "2.0",
                            "error": {
                                "code": -32603,
                                "message": "Internal server error"
                            },
                            "id": serde_json::Value::Null
                        })
                        .to_string()
                    })
                }
                Err(_) => match serde_json::from_slice::<Vec<RpcRequest>>(&body_bytes) {
                    Ok(mut requests) => {
                        for request in requests.iter_mut() {
                            if request.auth_token.is_none() {
                                request.auth_token = header_token.clone();
                            }
                        }
                        let responses = rpc.process_batch(requests).await;
                        serde_json::to_string(&responses).unwrap_or_else(|_| {
                            json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32603,
                                    "message": "Internal server error"
                                },
                                "id": serde_json::Value::Null
                            })
                            .to_string()
                        })
                    }
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e)
                        },
                        "id": serde_json::Value::Null
                    })
                    .to_string(),
                },
            };

            metrics.record_connection_closed();

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(result))
                .unwrap())
        }
        hyper::Method::GET => {
            metrics.record_connection_closed();

            let response = json!({
                "name": "Descartes RPC Server",
                "version": crate::VERSION,
                "methods": [
                    "agent.spawn",
                    "agent.list",
                    "agent.kill",
                    "agent.logs",
                    "workflow.execute",
                    "state.query",
                    "system.health",
                    "system.metrics"
                ]
            });

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(response.to_string()))
                .unwrap())
        }
        _ => {
            metrics.record_connection_closed();

            Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from("Method not allowed"))
                .unwrap())
        }
    }
}

fn parse_bearer_token(header: &str) -> Option<String> {
    let trimmed = header.trim();
    let mut parts = trimmed.splitn(2, ' ');
    let scheme = parts.next()?.to_ascii_lowercase();
    if scheme != "bearer" {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Handle metrics requests
async fn handle_metrics_request(
    metrics: Arc<MetricsCollector>,
) -> Result<Response<Body>, hyper::Error> {
    match metrics.gather_metrics() {
        Ok(body) => Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(Body::from(body))
            .unwrap()),
        Err(e) => {
            error!("Failed to gather metrics: {:?}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to gather metrics"))
                .unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_server_creation() {
        let config = DaemonConfig::default();
        let result = RpcServer::new(config);
        assert!(result.is_ok());
    }
}
