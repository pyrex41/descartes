//! Claude Code OpenAI-compatible proxy server
//!
//! Wraps `claude -p` as an OpenAI-compatible HTTP endpoint for BAML.
//!
//! Usage:
//!   cargo run --bin claude-proxy
//!
//! Then configure BAML:
//!   client<llm> ClaudeCode {
//!     provider openai-generic
//!     options {
//!       base_url "http://localhost:8765/v1"
//!       model "claude-code"
//!     }
//!   }

use std::convert::Infallible;
use std::net::SocketAddr;
use std::process::Command;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<Message>,
    #[serde(default)]
    model: String,
}

#[derive(Debug, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize)]
struct Choice {
    index: u32,
    message: ResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct ResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();

    // Health check
    if path == "/health" || path == "/" {
        return Ok(Response::new(Full::new(Bytes::from("OK"))));
    }

    // Only handle POST to /v1/chat/completions
    if req.method() != Method::POST || !path.ends_with("/chat/completions") {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap());
    }

    // Read body
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("Error reading body: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!("Error: {}", e))))
                .unwrap());
        }
    };

    // Parse request
    let chat_req: ChatRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!("Invalid JSON: {}", e))))
                .unwrap());
        }
    };

    // Build prompt from messages
    let prompt: String = chat_req
        .messages
        .iter()
        .map(|m| match m.role.as_str() {
            "system" => format!("System: {}", m.content),
            "user" => format!("User: {}", m.content),
            "assistant" => format!("Assistant: {}", m.content),
            _ => m.content.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    eprintln!(
        "Calling Claude Code with prompt ({} chars)...",
        prompt.len()
    );

    // Call Claude Code headless (print mode, no interactive tools)
    let output = Command::new("claude")
        .args(["-p", &prompt, "--output-format", "text"])
        .output();

    let content = match output {
        Ok(out) => {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("Claude Code error: {}", stderr);
                format!("Error: {}", stderr)
            }
        }
        Err(e) => {
            eprintln!("Failed to run claude: {}", e);
            format!("Error running claude: {}", e)
        }
    };

    eprintln!("Got response ({} chars)", content.len());

    // Build response
    let response = ChatResponse {
        id: format!(
            "chatcmpl-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: "claude-code".to_string(),
        choices: vec![Choice {
            index: 0,
            message: ResponseMessage {
                role: "assistant".to_string(),
                content,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    };

    let json = serde_json::to_string(&response).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port: u16 = std::env::var("CLAUDE_PROXY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8765);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!("Claude Code proxy listening on http://{}", addr);
    println!("Configure BAML with:");
    println!("  base_url \"http://localhost:{}/v1\"", port);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
