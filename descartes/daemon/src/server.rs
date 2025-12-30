/// HTTP and WebSocket server implementation
use crate::auth::AuthManager;
use crate::config::DaemonConfig;
use crate::errors::{DaemonError, DaemonResult};
use crate::event_stream::start_websocket_server;
use crate::events::EventBus;
use crate::handlers::{
    handle_create_project, handle_delete_project, handle_get_project, handle_list_projects,
    handle_parse_prd, RpcHandlers,
};
use crate::metrics::MetricsCollector;
use crate::pool::ConnectionPool;
use crate::rpc::JsonRpcServer;
use crate::types::*;
use crate::chat_manager::ChatManager;
use crate::zmq_publisher::ZmqPublisher;
use hyper::header::AUTHORIZATION;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

/// RPC Server
#[derive(Clone)]
pub struct RpcServer {
    config: DaemonConfig,
    #[allow(dead_code)]
    handlers: Arc<RpcHandlers>,
    #[allow(dead_code)]
    auth: Option<Arc<AuthManager>>,
    metrics: Arc<MetricsCollector>,
    #[allow(dead_code)]
    pool: Arc<ConnectionPool>,
    rpc: Arc<JsonRpcServer>,
    /// ZMQ PUB socket for streaming chat output (initialized lazily in run())
    #[allow(dead_code)]
    publisher: Option<Arc<ZmqPublisher>>,
    /// Event bus for WebSocket event streaming
    event_bus: Arc<EventBus>,
}

impl RpcServer {
    /// Create a new RPC server
    pub fn new(config: DaemonConfig) -> DaemonResult<Self> {
        config.validate()?;

        let metrics = Arc::new(MetricsCollector::new()?);
        let handlers = Arc::new(RpcHandlers::new());
        let pool = Arc::new(ConnectionPool::new(config.pool.clone()));
        let event_bus = Arc::new(EventBus::new());

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
            publisher: None, // Initialized in run()
            event_bus,
        })
    }

    /// Get a reference to the ZMQ publisher (if running)
    pub fn publisher(&self) -> Option<Arc<ZmqPublisher>> {
        self.publisher.clone()
    }

    /// Get the server config
    pub fn config(&self) -> &DaemonConfig {
        &self.config
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
        let handlers = self.handlers.clone();

        let make_svc = make_service_fn(move |_conn| {
            let rpc = rpc.clone();
            let metrics = metrics.clone();
            let handlers = handlers.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let rpc = rpc.clone();
                    let metrics = metrics.clone();
                    let handlers = handlers.clone();
                    handle_http_request(req, rpc, metrics, handlers)
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
        // Initialize ZMQ publisher and chat manager
        let publisher = match ZmqPublisher::new(
            &self.config.server.pub_addr,
            self.config.server.pub_port,
        )
        .await
        {
            Ok(pub_socket) => {
                info!(
                    "ZMQ PUB socket listening on {}",
                    pub_socket.endpoint()
                );
                Some(Arc::new(pub_socket))
            }
            Err(e) => {
                error!("Failed to start ZMQ publisher: {}", e);
                None
            }
        };

        // Create chat manager if publisher is available
        if let Some(ref pub_socket) = publisher {
            let chat_manager = Arc::new(ChatManager::new(pub_socket.clone()));
            self.rpc.set_chat_manager(chat_manager).await;
            info!("Chat manager initialized");
        }

        // Update self with publisher (for access by handlers)
        let mut server = self.clone();
        server.publisher = publisher;

        // Start HTTP server
        let http_server = server.clone();
        let http_handle = tokio::spawn(async move {
            if let Err(e) = http_server.start_http().await {
                error!("HTTP server error: {:?}", e);
            }
        });

        // Start metrics endpoint
        let metrics_server = server.clone();
        let metrics_handle = tokio::spawn(async move {
            if let Err(e) = metrics_server.start_metrics().await {
                error!("Metrics server error: {:?}", e);
            }
        });

        // Start WebSocket server for event streaming
        let ws_event_bus = server.event_bus.clone();
        let ws_port = server.config.server.http_port + 1; // Use next port for WebSocket
        let ws_addr = format!("{}:{}", server.config.server.http_addr, ws_port);
        let ws_handle = tokio::spawn(async move {
            if let Err(e) = start_websocket_server(&ws_addr, ws_event_bus).await {
                error!("WebSocket server error: {}", e);
            }
        });

        info!("RPC server started");

        // Wait for servers
        tokio::select! {
            _ = http_handle => {},
            _ = metrics_handle => {},
            _ = ws_handle => {},
        }

        Ok(())
    }
}

/// Handle HTTP RPC requests
async fn handle_http_request(
    req: Request<Body>,
    rpc: Arc<JsonRpcServer>,
    metrics: Arc<MetricsCollector>,
    handlers: Arc<RpcHandlers>,
) -> Result<Response<Body>, hyper::Error> {
    metrics.record_connection();

    // Handle REST API routes
    match (req.method(), req.uri().path()) {
        // Health check
        (&Method::GET, "/health") => {
            metrics.record_connection_closed();
            return Ok(Response::new(Body::from("OK")));
        }

        // Project endpoints (REST-style)
        (&Method::GET, "/api/projects") => {
            metrics.record_connection_closed();
            let owner_id = extract_owner_id(&req).unwrap_or_else(|| "anonymous".to_string());
            if let Some(store) = handlers.project_store() {
                match handle_list_projects(store, &owner_id).await {
                    Ok(projects) => return json_response(projects),
                    Err(e) => return error_response(500, &e.message),
                }
            } else {
                return error_response(500, "Project store not configured");
            }
        }

        (&Method::POST, "/api/projects") => {
            metrics.record_connection_closed();
            let owner_id = extract_owner_id(&req).unwrap_or_else(|| "anonymous".to_string());
            if let Some(store) = handlers.project_store() {
                let body = hyper::body::to_bytes(req.into_body()).await?;
                match serde_json::from_slice::<CreateProjectRequest>(&body) {
                    Ok(create_req) => {
                        match handle_create_project(store, &owner_id, create_req).await {
                            Ok(response) => return json_response(response),
                            Err(e) => return error_response(500, &e.message),
                        }
                    }
                    Err(e) => return error_response(400, &format!("Invalid request: {}", e)),
                }
            } else {
                return error_response(500, "Project store not configured");
            }
        }

        (&Method::GET, path) if path.starts_with("/api/projects/") => {
            metrics.record_connection_closed();
            let project_id = path.trim_start_matches("/api/projects/");
            if let Some(store) = handlers.project_store() {
                match handle_get_project(store, project_id).await {
                    Ok(project) => return json_response(project),
                    Err(e) => return error_response(if e.code == -32001 { 404 } else { 500 }, &e.message),
                }
            } else {
                return error_response(500, "Project store not configured");
            }
        }

        (&Method::DELETE, path) if path.starts_with("/api/projects/") => {
            metrics.record_connection_closed();
            let project_id = path.trim_start_matches("/api/projects/");
            if let Some(store) = handlers.project_store() {
                match handle_delete_project(store, project_id).await {
                    Ok(deleted) => return json_response(serde_json::json!({"deleted": deleted})),
                    Err(e) => return error_response(500, &e.message),
                }
            } else {
                return error_response(500, "Project store not configured");
            }
        }

        (&Method::POST, path) if path.starts_with("/api/projects/") && path.ends_with("/parse-prd") => {
            metrics.record_connection_closed();
            let project_id = path.trim_start_matches("/api/projects/").trim_end_matches("/parse-prd");
            if let Some(store) = handlers.project_store() {
                match handle_parse_prd(store, project_id).await {
                    Ok(waves) => return json_response(waves),
                    Err(e) => return error_response(if e.code == -32001 { 404 } else { 500 }, &e.message),
                }
            } else {
                return error_response(500, "Project store not configured");
            }
        }

        _ => {
            // Fall through to JSON-RPC handling
        }
    }

    // JSON-RPC handling
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

/// Helper function to create JSON response
fn json_response<T: Serialize>(data: T) -> Result<Response<Body>, hyper::Error> {
    let body = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap())
}

/// Helper function to create error response
fn error_response(status: u16, message: &str) -> Result<Response<Body>, hyper::Error> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(format!(r#"{{"error":"{}"}}"#, message)))
        .unwrap())
}

/// Extract owner ID from request headers
fn extract_owner_id(req: &Request<Body>) -> Option<String> {
    // For MVP, extract from header or use anonymous
    req.headers()
        .get("X-User-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
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
