# Descartes Webapp Implementation Plan

## Overview

This plan implements the **Guided Webapp** component from the Descartes Unified Platform PRD. Based on critical analysis, we're **skipping PRD Phases 1-2** (CLI tool extraction and plugin system) as the current architecture already provides those capabilities optimally.

**Scope**: Elm frontend + Extended Rust daemon + Fly.io worker infrastructure

## Current State Analysis

### What Already Exists (Leverage Points)

| Component | Location | Status |
|-----------|----------|--------|
| **RPC Server** | `descartes/daemon/src/rpc_server.rs` | Complete - JSON-RPC 2.0 over Unix socket |
| **HTTP Server** | `descartes/daemon/src/server.rs` | Partial - Hyper-based, needs REST endpoints |
| **WebSocket Handler** | `descartes/daemon/src/event_stream.rs` | Complete but not wired to server startup |
| **Event Bus** | `descartes/daemon/src/events.rs` | Complete - 1000-event broadcast channel |
| **Auth Module** | `descartes/daemon/src/auth.rs` | Complete - JWT-based (optional) |
| **OpenAPI Schema** | `descartes/daemon/src/openapi.rs` | Partial - needs update for new endpoints |
| **Agent Management** | `descartes/daemon/src/handlers.rs` | spawn, list, kill, logs |
| **Task Operations** | `descartes/daemon/src/rpc_server.rs:559-651` | list_tasks, approve |
| **SCUD Integration** | `descartes/core/src/scud_plugin.rs` | Complete |
| **Flow Executor** | `descartes/core/src/flow_executor.rs` | Complete |

### What Needs to Be Built

| Component | Priority | Effort |
|-----------|----------|--------|
| **Elm Frontend** | P0 | ~4 weeks |
| **Fly.io Machine Spawner** | P0 | ~1 week |
| **REST API Extensions** | P0 | ~2 weeks |
| **Worker Docker Image** | P0 | ~3 days |
| **Project CRUD** | P1 | ~1 week |
| **Cost Tracking** | P2 | ~1 week |
| **Guidance System** | P2 | ~2 weeks |

## Desired End State

After implementation:

1. **Users can access** `https://descartes.dev` and:
   - Create accounts via OAuth (GitHub/Google)
   - Create projects with PRD documents
   - Watch SCUD tasks parse and visualize as waves
   - Spawn cloud agents that execute tasks
   - Monitor agent progress in real-time via WebSocket
   - Approve/reject agent actions
   - See cost tracking per project

2. **Architecture**:
   ```
   Browser (Elm) → Fly.io Orchestrator (descartes-daemon) → Fly.io Workers (ephemeral)
                          ↓
                   PostgreSQL (state) + S3/Tigris (artifacts)
   ```

3. **Verification**:
   - `curl https://api.descartes.dev/health` returns 200
   - Elm app loads at `https://descartes.dev`
   - Can create project, parse PRD, spawn agent, see logs
   - Agent auto-destroys after task completion
   - All tests pass: `cargo test -p descartes-daemon`

## What We're NOT Doing

- ❌ CLI tool extraction (`dc-spawn`, `dc-parse`, `dc-waves`) - existing subcommands are sufficient
- ❌ Plugin system with hooks - existing slash commands work perfectly
- ❌ Desktop GUI changes - existing Iced app is complete
- ❌ Mobile app - web-first, responsive design
- ❌ Self-hosted worker option (Phase 5+ if needed)

## Implementation Approach

**Strategy**: Extend existing daemon rather than building separate orchestrator

1. Add REST endpoints to existing HTTP server
2. Wire WebSocket server into startup
3. Create Fly.io machine spawner as new daemon module
4. Build Elm frontend that consumes daemon API
5. Containerize daemon as orchestrator
6. Create minimal worker image

---

## Phase 1: Daemon REST API Extensions

### Overview
Extend the existing `descartes-daemon` with REST endpoints for webapp consumption. Unify the type systems between Unix socket RPC and HTTP handlers.

### Changes Required:

#### 1.1 Unified Type System

**File**: `descartes/daemon/src/types.rs`
**Changes**: Create shared request/response types for both transports

```rust
// Add after line 314

/// Project management types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub prd_content: Option<String>,
    pub scud_tag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub prd_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectResponse {
    pub project: Project,
}

/// Wave execution types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave {
    pub index: usize,
    pub tasks: Vec<String>,
    pub status: WaveStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaveStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWaveRequest {
    pub project_id: String,
    pub wave_index: usize,
}

/// Cost tracking types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub project_id: String,
    pub total_compute_seconds: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub breakdown: Vec<CostEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub agent_id: String,
    pub compute_seconds: u64,
    pub tokens: u64,
    pub cost_usd: f64,
}
```

#### 1.2 Project Store

**File**: `descartes/daemon/src/project_store.rs` (NEW)
**Changes**: SQLite-backed project storage

```rust
use sqlx::{SqlitePool, FromRow};
use crate::types::{Project, CreateProjectRequest};
use chrono::Utc;
use uuid::Uuid;

pub struct ProjectStore {
    pool: SqlitePool,
}

impl ProjectStore {
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        // Run migrations
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                owner_id TEXT NOT NULL,
                prd_content TEXT,
                scud_tag TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
        "#).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn create(&self, owner_id: &str, req: CreateProjectRequest) -> Result<Project, sqlx::Error> {
        let now = Utc::now();
        let project = Project {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            owner_id: owner_id.to_string(),
            prd_content: req.prd_content,
            scud_tag: None,
            created_at: now,
            updated_at: now,
        };

        sqlx::query(r#"
            INSERT INTO projects (id, name, owner_id, prd_content, scud_tag, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.owner_id)
        .bind(&project.prd_content)
        .bind(&project.scud_tag)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(project)
    }

    pub async fn list(&self, owner_id: &str) -> Result<Vec<Project>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE owner_id = ? ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Project>, sqlx::Error> {
        sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
```

#### 1.3 REST Endpoint Handlers

**File**: `descartes/daemon/src/handlers.rs`
**Changes**: Add project and wave handlers after line 389

```rust
// Project CRUD handlers

pub async fn handle_create_project(
    handlers: &RpcHandlers,
    auth: &AuthContext,
    req: CreateProjectRequest,
) -> Result<CreateProjectResponse, RpcError> {
    let project = handlers.project_store
        .create(&auth.user_id, req)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to create project: {}", e)))?;

    Ok(CreateProjectResponse { project })
}

pub async fn handle_list_projects(
    handlers: &RpcHandlers,
    auth: &AuthContext,
) -> Result<Vec<Project>, RpcError> {
    handlers.project_store
        .list(&auth.user_id)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to list projects: {}", e)))
}

pub async fn handle_get_project(
    handlers: &RpcHandlers,
    auth: &AuthContext,
    project_id: &str,
) -> Result<Project, RpcError> {
    handlers.project_store
        .get(project_id)
        .await
        .map_err(|e| RpcError::internal(format!("Failed to get project: {}", e)))?
        .ok_or_else(|| RpcError::not_found("Project not found"))
}

pub async fn handle_parse_prd(
    handlers: &RpcHandlers,
    auth: &AuthContext,
    project_id: &str,
) -> Result<Vec<Wave>, RpcError> {
    let project = handle_get_project(handlers, auth, project_id).await?;

    let prd_content = project.prd_content
        .ok_or_else(|| RpcError::bad_request("Project has no PRD content"))?;

    // Use SCUD CLI to parse PRD
    let output = tokio::process::Command::new("scud")
        .args(["parse", "--format", "json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| RpcError::internal(format!("Failed to spawn scud: {}", e)))?
        .wait_with_output()
        .await
        .map_err(|e| RpcError::internal(format!("scud failed: {}", e)))?;

    let waves: Vec<Wave> = serde_json::from_slice(&output.stdout)
        .map_err(|e| RpcError::internal(format!("Failed to parse scud output: {}", e)))?;

    Ok(waves)
}
```

#### 1.4 HTTP Router Updates

**File**: `descartes/daemon/src/server.rs`
**Changes**: Add REST routes to `handle_http_request` around line 287

```rust
// Update the method routing section (around line 287-300)

match (req.method(), req.uri().path()) {
    // Health check
    (&Method::GET, "/health") => {
        Ok(Response::new(Body::from("OK")))
    }

    // Project endpoints (REST-style)
    (&Method::GET, "/api/projects") => {
        let auth = extract_auth(&req)?;
        let projects = handle_list_projects(&handlers, &auth).await?;
        json_response(projects)
    }
    (&Method::POST, "/api/projects") => {
        let auth = extract_auth(&req)?;
        let body = hyper::body::to_bytes(req.into_body()).await?;
        let create_req: CreateProjectRequest = serde_json::from_slice(&body)?;
        let response = handle_create_project(&handlers, &auth, create_req).await?;
        json_response(response)
    }
    (&Method::GET, path) if path.starts_with("/api/projects/") => {
        let auth = extract_auth(&req)?;
        let project_id = path.strip_prefix("/api/projects/").unwrap();
        let project = handle_get_project(&handlers, &auth, project_id).await?;
        json_response(project)
    }
    (&Method::POST, path) if path.ends_with("/parse") => {
        let auth = extract_auth(&req)?;
        let project_id = path.strip_prefix("/api/projects/")
            .and_then(|p| p.strip_suffix("/parse"))
            .ok_or_else(|| RpcError::bad_request("Invalid path"))?;
        let waves = handle_parse_prd(&handlers, &auth, project_id).await?;
        json_response(waves)
    }

    // WebSocket upgrade for event streaming
    (&Method::GET, "/api/events") => {
        handle_websocket_upgrade(req, handlers.event_bus.clone()).await
    }

    // Existing RPC endpoint
    (&Method::POST, "/rpc") => {
        handle_rpc_request(req, handlers).await
    }

    _ => {
        Ok(Response::builder()
            .status(404)
            .body(Body::from("Not Found"))?)
    }
}
```

#### 1.5 WebSocket Server Integration

**File**: `descartes/daemon/src/server.rs`
**Changes**: Wire WebSocket server into `RpcServer::run()` around line 190

```rust
// Add to RpcServer::run() after HTTP server spawn

// Start WebSocket event server
let ws_event_bus = self.event_bus.clone();
let ws_addr: SocketAddr = format!("{}:{}",
    self.config.server.host,
    self.config.server.websocket_port.unwrap_or(8081)
).parse()?;

let ws_handle = tokio::spawn(async move {
    if let Err(e) = start_websocket_server(ws_addr, ws_event_bus).await {
        tracing::error!("WebSocket server error: {}", e);
    }
});

// ... existing code ...

// Wait for both servers
tokio::select! {
    result = http_handle => {
        tracing::info!("HTTP server stopped: {:?}", result);
    }
    result = ws_handle => {
        tracing::info!("WebSocket server stopped: {:?}", result);
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] All tests pass: `cargo test -p descartes-daemon`
- [ ] Clippy clean: `cargo clippy -p descartes-daemon`
- [ ] HTTP server starts: `curl http://localhost:8080/health` returns 200
- [ ] WebSocket connects: `websocat ws://localhost:8081/events`
- [ ] Project CRUD works via curl commands

#### Manual Verification:
- [ ] Create project via API, verify in SQLite
- [ ] Parse PRD, see waves returned
- [ ] WebSocket receives events when agent spawns

**Implementation Note**: Complete this phase before proceeding. All automated tests must pass.

---

## Phase 2: Fly.io Machine Spawner

### Overview
Create a Rust module that spawns ephemeral Fly.io Machines for agent execution.

### Changes Required:

#### 2.1 Fly.io API Client

**File**: `descartes/daemon/src/fly_machines.rs` (NEW)
**Changes**: HTTP client for Fly.io Machines API

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const FLY_API_BASE: &str = "https://api.machines.dev/v1";

#[derive(Clone)]
pub struct FlyMachinesClient {
    client: Client,
    api_token: String,
    app_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateMachineRequest {
    pub name: Option<String>,
    pub config: MachineConfig,
}

#[derive(Debug, Serialize)]
pub struct MachineConfig {
    pub image: String,
    pub auto_destroy: bool,
    pub env: std::collections::HashMap<String, String>,
    pub restart: RestartPolicy,
    pub guest: GuestConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<InitConfig>,
}

#[derive(Debug, Serialize)]
pub struct RestartPolicy {
    pub policy: String,
}

#[derive(Debug, Serialize)]
pub struct GuestConfig {
    pub cpu_kind: String,
    pub cpus: u32,
    pub memory_mb: u32,
}

#[derive(Debug, Serialize)]
pub struct InitConfig {
    pub cmd: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub state: String,
    pub instance_id: String,
    pub private_ip: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct MachineEvent {
    pub id: String,
    pub r#type: String,
    pub status: String,
    pub timestamp: i64,
}

impl FlyMachinesClient {
    pub fn new() -> Result<Self, String> {
        let api_token = env::var("FLY_API_TOKEN")
            .map_err(|_| "FLY_API_TOKEN not set")?;
        let app_name = env::var("FLY_WORKER_APP")
            .unwrap_or_else(|_| "descartes-workers".to_string());

        Ok(Self {
            client: Client::new(),
            api_token,
            app_name,
        })
    }

    pub async fn spawn_worker(
        &self,
        task_id: &str,
        project_id: &str,
        callback_url: &str,
    ) -> Result<Machine, reqwest::Error> {
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("TASK_ID".to_string(), task_id.to_string());
        env_vars.insert("PROJECT_ID".to_string(), project_id.to_string());
        env_vars.insert("CALLBACK_URL".to_string(), callback_url.to_string());
        env_vars.insert("ANTHROPIC_API_KEY".to_string(),
            env::var("ANTHROPIC_API_KEY").unwrap_or_default());

        let request = CreateMachineRequest {
            name: Some(format!("worker-{}", task_id)),
            config: MachineConfig {
                image: env::var("WORKER_IMAGE")
                    .unwrap_or_else(|_| "registry.fly.io/descartes-workers:latest".to_string()),
                auto_destroy: true,
                env: env_vars,
                restart: RestartPolicy {
                    policy: "no".to_string(),
                },
                guest: GuestConfig {
                    cpu_kind: "shared".to_string(),
                    cpus: 2,
                    memory_mb: 2048,
                },
                init: Some(InitConfig {
                    cmd: vec![
                        "/app/descartes-worker".to_string(),
                        "--task".to_string(),
                        task_id.to_string(),
                    ],
                }),
            },
        };

        let response = self.client
            .post(&format!("{}/apps/{}/machines", FLY_API_BASE, self.app_name))
            .bearer_auth(&self.api_token)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        response.json().await
    }

    pub async fn get_machine(&self, machine_id: &str) -> Result<Machine, reqwest::Error> {
        self.client
            .get(&format!("{}/apps/{}/machines/{}",
                FLY_API_BASE, self.app_name, machine_id))
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub async fn stop_machine(&self, machine_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .post(&format!("{}/apps/{}/machines/{}/stop",
                FLY_API_BASE, self.app_name, machine_id))
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn destroy_machine(&self, machine_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .delete(&format!("{}/apps/{}/machines/{}",
                FLY_API_BASE, self.app_name, machine_id))
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn list_machines(&self) -> Result<Vec<Machine>, reqwest::Error> {
        self.client
            .get(&format!("{}/apps/{}/machines", FLY_API_BASE, self.app_name))
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}
```

#### 2.2 Agent Spawn Handler Update

**File**: `descartes/daemon/src/handlers.rs`
**Changes**: Add cloud spawn option to existing spawn handler

```rust
// Add to handle_spawn_agent (around line 64-96)

#[derive(Debug, Deserialize)]
pub struct SpawnAgentRequest {
    pub name: String,
    pub agent_type: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub cloud: bool,  // NEW: spawn on Fly.io if true
}

pub async fn handle_spawn_agent(
    handlers: &RpcHandlers,
    auth: &AuthContext,
    req: SpawnAgentRequest,
) -> Result<SpawnAgentResponse, RpcError> {
    if req.cloud {
        // Spawn on Fly.io
        let fly_client = handlers.fly_client.as_ref()
            .ok_or_else(|| RpcError::internal("Fly.io not configured"))?;

        let task_id = req.config.get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::bad_request("task_id required for cloud spawn"))?;

        let project_id = req.config.get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::bad_request("project_id required for cloud spawn"))?;

        let callback_url = format!("{}/api/agents/callback",
            handlers.config.server.public_url);

        let machine = fly_client
            .spawn_worker(task_id, project_id, &callback_url)
            .await
            .map_err(|e| RpcError::internal(format!("Fly.io spawn failed: {}", e)))?;

        // Track machine in local state
        let agent_id = machine.id.clone();
        handlers.agents.insert(agent_id.clone(), AgentInfo {
            id: agent_id.clone(),
            name: req.name,
            status: AgentStatus::Running,
            fly_machine_id: Some(machine.id),
            created_at: chrono::Utc::now(),
            ..Default::default()
        });

        // Emit event
        handlers.event_bus.send(Event::Agent(AgentEvent::Spawned {
            agent_id: agent_id.clone(),
            cloud: true,
        }));

        Ok(SpawnAgentResponse { agent_id })
    } else {
        // Existing local spawn logic
        // ... (keep existing implementation)
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Unit tests for FlyMachinesClient: `cargo test fly_machines`
- [ ] Mock API tests pass
- [ ] Integration test with real Fly.io (optional, requires token)

#### Manual Verification:
- [ ] Spawn worker via API with `cloud: true`
- [ ] Machine appears in Fly.io dashboard
- [ ] Machine auto-destroys after task completion
- [ ] Logs stream back to orchestrator

**Implementation Note**: Requires `FLY_API_TOKEN` and `FLY_WORKER_APP` environment variables.

---

## Phase 3: Worker Docker Image

### Overview
Create a minimal Docker image for descartes-worker that runs on Fly.io Machines.

### Changes Required:

#### 3.1 Worker Binary

**File**: `descartes/worker/Cargo.toml` (NEW workspace member)
**Changes**: Minimal worker that executes single task

```toml
[package]
name = "descartes-worker"
version = "0.1.0"
edition = "2021"

[dependencies]
descartes-core = { path = "../core" }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }
```

**File**: `descartes/worker/src/main.rs` (NEW)

```rust
use clap::Parser;
use descartes_core::{
    spawn_agent, AgentConfig, ProviderFactory, ToolLevel,
};
use reqwest::Client;
use std::env;
use tracing::{info, error};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    task: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let task_id = args.task;
    let callback_url = env::var("CALLBACK_URL")?;
    let project_id = env::var("PROJECT_ID")?;

    info!("Worker starting for task: {}", task_id);

    // Notify orchestrator we're starting
    let client = Client::new();
    client.post(&format!("{}/started", callback_url))
        .json(&serde_json::json!({
            "task_id": task_id,
            "project_id": project_id,
            "status": "started"
        }))
        .send()
        .await?;

    // Get task details from SCUD
    let task_info = get_task_from_scud(&task_id).await?;

    // Configure minimal agent
    let config = AgentConfig {
        task: task_info.description,
        provider: env::var("PROVIDER").unwrap_or_else(|_| "anthropic".to_string()),
        model: env::var("MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
        tool_level: ToolLevel::Minimal,
        ..Default::default()
    };

    // Run agent
    let result = match spawn_agent(config).await {
        Ok(session) => {
            info!("Task completed successfully");
            serde_json::json!({
                "task_id": task_id,
                "status": "completed",
                "session_id": session.id,
            })
        }
        Err(e) => {
            error!("Task failed: {}", e);
            serde_json::json!({
                "task_id": task_id,
                "status": "failed",
                "error": e.to_string(),
            })
        }
    };

    // Report completion
    client.post(&format!("{}/completed", callback_url))
        .json(&result)
        .send()
        .await?;

    info!("Worker exiting");
    Ok(())
}

async fn get_task_from_scud(task_id: &str) -> anyhow::Result<TaskInfo> {
    // Use SCUD CLI or read from passed config
    let output = tokio::process::Command::new("scud")
        .args(["show", task_id, "--format", "json"])
        .output()
        .await?;

    let task: TaskInfo = serde_json::from_slice(&output.stdout)?;
    Ok(task)
}

#[derive(Debug, serde::Deserialize)]
struct TaskInfo {
    id: String,
    title: String,
    description: String,
}
```

#### 3.2 Dockerfile

**File**: `descartes/worker/Dockerfile` (NEW)

```dockerfile
# Build stage
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY worker ./worker

# Build worker binary
RUN cargo build --release -p descartes-worker

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install SCUD CLI
RUN curl -fsSL https://github.com/pyrex41/scud/releases/latest/download/scud-linux-amd64 \
    -o /usr/local/bin/scud && chmod +x /usr/local/bin/scud

WORKDIR /app

COPY --from=builder /app/target/release/descartes-worker /app/

ENV RUST_LOG=info

ENTRYPOINT ["/app/descartes-worker"]
```

#### 3.3 Fly.io App Configuration

**File**: `descartes/worker/fly.toml` (NEW)

```toml
app = "descartes-workers"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

# No services - workers are ephemeral and callback to orchestrator

[env]
  RUST_LOG = "info"
```

### Success Criteria:

#### Automated Verification:
- [ ] Docker builds: `docker build -t descartes-worker -f worker/Dockerfile .`
- [ ] Worker binary runs: `./descartes-worker --help`
- [ ] Image pushes to registry: `fly deploy --image-only`

#### Manual Verification:
- [ ] Spawn machine with image, see task execute
- [ ] Logs appear in Fly.io dashboard
- [ ] Machine auto-destroys after completion

---

## Phase 4: Elm Frontend

### Overview
Build the webapp frontend using Elm with elm-css and TailwindCSS (v3.4).

### Changes Required:

#### 4.1 Project Structure

**Directory**: `descartes/webapp/` (NEW)

```
webapp/
├── src/
│   ├── Main.elm
│   ├── Api.elm              # HTTP client
│   ├── Ports.elm            # WebSocket ports
│   ├── Route.elm            # URL routing
│   ├── Session.elm          # Auth state
│   ├── Page/
│   │   ├── Home.elm
│   │   ├── Dashboard.elm
│   │   ├── Project.elm
│   │   └── NotFound.elm
│   ├── View/
│   │   ├── Layout.elm
│   │   ├── TaskBoard.elm
│   │   ├── WaveViz.elm
│   │   └── AgentMonitor.elm
│   └── TW/                  # Generated Tailwind utilities
│       └── Utilities.elm
├── static/
│   └── index.html
├── elm.json
├── package.json
├── postcss.config.js
├── tailwind.config.js
└── tailwind.css
```

#### 4.2 Elm Entry Point

**File**: `descartes/webapp/src/Main.elm`

```elm
module Main exposing (main)

import Browser
import Browser.Navigation as Nav
import Html.Styled exposing (..)
import Html.Styled.Attributes exposing (css)
import Url exposing (Url)
import Css exposing (..)

import Route exposing (Route)
import Session exposing (Session)
import Page.Home as Home
import Page.Dashboard as Dashboard
import Page.Project as Project
import Api
import Ports


-- MAIN

main : Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }


-- MODEL

type alias Model =
    { key : Nav.Key
    , url : Url
    , session : Session
    , page : Page
    }

type Page
    = HomePage Home.Model
    | DashboardPage Dashboard.Model
    | ProjectPage Project.Model
    | NotFound

type alias Flags =
    { token : Maybe String
    }


-- INIT

init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        session =
            case flags.token of
                Just t -> Session.authenticated t
                Nothing -> Session.guest

        ( page, pageCmd ) =
            routeToPage session (Route.fromUrl url)
    in
    ( { key = key
      , url = url
      , session = session
      , page = page
      }
    , Cmd.batch
        [ pageCmd
        , Ports.connectWebSocket (Api.eventsUrl session)
        ]
    )


-- UPDATE

type Msg
    = UrlChanged Url
    | LinkClicked Browser.UrlRequest
    | GotHomeMsg Home.Msg
    | GotDashboardMsg Dashboard.Msg
    | GotProjectMsg Project.Msg
    | GotWebSocketMsg String
    | GotSessionMsg Session.Msg

update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model.page ) of
        ( UrlChanged url, _ ) ->
            let
                ( page, cmd ) = routeToPage model.session (Route.fromUrl url)
            in
            ( { model | url = url, page = page }, cmd )

        ( LinkClicked request, _ ) ->
            case request of
                Browser.Internal url ->
                    ( model, Nav.pushUrl model.key (Url.toString url) )
                Browser.External href ->
                    ( model, Nav.load href )

        ( GotDashboardMsg subMsg, DashboardPage subModel ) ->
            Dashboard.update subMsg subModel
                |> updateWith DashboardPage GotDashboardMsg model

        ( GotProjectMsg subMsg, ProjectPage subModel ) ->
            Project.update subMsg subModel
                |> updateWith ProjectPage GotProjectMsg model

        ( GotWebSocketMsg json, _ ) ->
            -- Handle real-time events
            case Api.decodeEvent json of
                Ok event ->
                    handleEvent event model
                Err _ ->
                    ( model, Cmd.none )

        _ ->
            ( model, Cmd.none )

handleEvent : Api.Event -> Model -> ( Model, Cmd Msg )
handleEvent event model =
    case model.page of
        ProjectPage subModel ->
            let
                ( newSubModel, cmd ) = Project.handleEvent event subModel
            in
            ( { model | page = ProjectPage newSubModel }
            , Cmd.map GotProjectMsg cmd
            )
        _ ->
            ( model, Cmd.none )


-- VIEW

view : Model -> Browser.Document Msg
view model =
    { title = "Descartes"
    , body =
        [ toUnstyled <|
            div [ css [ minHeight (vh 100), backgroundColor (hex "0f172a") ] ]
                [ viewPage model ]
        ]
    }

viewPage : Model -> Html Msg
viewPage model =
    case model.page of
        HomePage subModel ->
            Home.view subModel |> Html.Styled.map GotHomeMsg

        DashboardPage subModel ->
            Dashboard.view subModel |> Html.Styled.map GotDashboardMsg

        ProjectPage subModel ->
            Project.view subModel |> Html.Styled.map GotProjectMsg

        NotFound ->
            div [] [ text "Page not found" ]


-- SUBSCRIPTIONS

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ Ports.webSocketMessage GotWebSocketMsg
        , case model.page of
            ProjectPage subModel ->
                Project.subscriptions subModel |> Sub.map GotProjectMsg
            _ ->
                Sub.none
        ]


-- HELPERS

routeToPage : Session -> Maybe Route -> ( Page, Cmd Msg )
routeToPage session maybeRoute =
    case maybeRoute of
        Nothing ->
            ( NotFound, Cmd.none )

        Just Route.Home ->
            Home.init session
                |> Tuple.mapBoth HomePage (Cmd.map GotHomeMsg)

        Just Route.Dashboard ->
            Dashboard.init session
                |> Tuple.mapBoth DashboardPage (Cmd.map GotDashboardMsg)

        Just (Route.Project projectId) ->
            Project.init session projectId
                |> Tuple.mapBoth ProjectPage (Cmd.map GotProjectMsg)

updateWith : (subModel -> Page) -> (subMsg -> Msg) -> Model -> ( subModel, Cmd subMsg ) -> ( Model, Cmd Msg )
updateWith toPage toMsg model ( subModel, subCmd ) =
    ( { model | page = toPage subModel }
    , Cmd.map toMsg subCmd
    )
```

#### 4.3 WebSocket Ports

**File**: `descartes/webapp/src/Ports.elm`

```elm
port module Ports exposing
    ( connectWebSocket
    , disconnectWebSocket
    , sendWebSocketMessage
    , webSocketMessage
    , webSocketStatus
    )

-- Outgoing ports (Elm -> JS)
port connectWebSocket : String -> Cmd msg
port disconnectWebSocket : () -> Cmd msg
port sendWebSocketMessage : String -> Cmd msg

-- Incoming ports (JS -> Elm)
port webSocketMessage : (String -> msg) -> Sub msg
port webSocketStatus : (String -> msg) -> Sub msg
```

**File**: `descartes/webapp/static/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Descartes</title>
    <link href="/main.css" rel="stylesheet">
</head>
<body>
    <div id="app"></div>
    <script src="/main.js"></script>
    <script>
        // Initialize Elm
        const app = Elm.Main.init({
            node: document.getElementById('app'),
            flags: {
                token: localStorage.getItem('jwt')
            }
        });

        // WebSocket management
        let socket = null;

        app.ports.connectWebSocket.subscribe(function(url) {
            if (socket) socket.close();

            socket = new WebSocket(url);

            socket.onopen = function() {
                app.ports.webSocketStatus.send('connected');
            };

            socket.onmessage = function(event) {
                app.ports.webSocketMessage.send(event.data);
            };

            socket.onclose = function() {
                app.ports.webSocketStatus.send('disconnected');
                // Reconnect after 3 seconds
                setTimeout(() => app.ports.connectWebSocket.send(url), 3000);
            };

            socket.onerror = function() {
                app.ports.webSocketStatus.send('error');
            };
        });

        app.ports.disconnectWebSocket.subscribe(function() {
            if (socket) {
                socket.close();
                socket = null;
            }
        });

        app.ports.sendWebSocketMessage.subscribe(function(message) {
            if (socket && socket.readyState === WebSocket.OPEN) {
                socket.send(message);
            }
        });
    </script>
</body>
</html>
```

#### 4.4 TailwindCSS Configuration

**File**: `descartes/webapp/tailwind.config.js`

```javascript
module.exports = {
  content: ['./src/**/*.elm', './static/**/*.html'],
  theme: {
    extend: {
      colors: {
        'descartes': {
          50: '#f0f9ff',
          100: '#e0f2fe',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          900: '#0c4a6e',
        }
      }
    },
  },
  plugins: [],
}
```

**File**: `descartes/webapp/postcss.config.js`

```javascript
const postcssElmCssTailwind = require("postcss-elm-css-tailwind");

module.exports = {
  plugins: [
    require("tailwindcss")("./tailwind.config.js"),
    postcssElmCssTailwind({
      baseTailwindCSS: "./tailwind.css",
      rootOutputDir: "./src",
      rootModule: "TW"
    }),
  ],
};
```

**File**: `descartes/webapp/package.json`

```json
{
  "name": "descartes-webapp",
  "version": "0.1.0",
  "scripts": {
    "build:tw": "postcss tailwind.css -o static/main.css",
    "build:elm": "elm make src/Main.elm --optimize --output=static/main.js",
    "build": "npm run build:tw && npm run build:elm",
    "dev": "concurrently \"npm run watch:tw\" \"elm-live src/Main.elm --open -- --output=static/main.js\"",
    "watch:tw": "postcss tailwind.css -o static/main.css --watch"
  },
  "devDependencies": {
    "concurrently": "^8.2.0",
    "elm": "^0.19.1-6",
    "elm-live": "^4.0.2",
    "postcss": "^8.4.0",
    "postcss-cli": "^11.0.0",
    "postcss-elm-css-tailwind": "^0.11.0",
    "tailwindcss": "^3.4.0"
  }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Elm compiles: `cd webapp && elm make src/Main.elm`
- [ ] TailwindCSS generates: `npm run build:tw`
- [ ] Full build succeeds: `npm run build`
- [ ] No Elm compiler warnings

#### Manual Verification:
- [ ] App loads at `http://localhost:8000`
- [ ] Can navigate between pages
- [ ] WebSocket connects and receives events
- [ ] Create project flow works end-to-end

**Implementation Note**: This phase requires the most UI iteration. Complete core functionality first, then polish.

---

## Phase 5: Orchestrator Deployment

### Overview
Deploy descartes-daemon as the orchestrator on Fly.io.

### Changes Required:

#### 5.1 Orchestrator Dockerfile

**File**: `descartes/daemon/Dockerfile` (NEW)

```dockerfile
# Build stage
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY daemon ./daemon

RUN cargo build --release -p descartes-daemon

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/descartes-daemon /app/

ENV RUST_LOG=info
EXPOSE 8080 8081

CMD ["/app/descartes-daemon", "--host", "0.0.0.0"]
```

#### 5.2 Fly.io Configuration

**File**: `descartes/daemon/fly.toml` (NEW)

```toml
app = "descartes-api"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = false   # Always-on orchestrator
  auto_start_machines = true
  min_machines_running = 1

[[services]]
  protocol = "tcp"
  internal_port = 8081

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

[env]
  RUST_LOG = "info"
  FLY_WORKER_APP = "descartes-workers"

[[vm]]
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 1024
```

#### 5.3 Database Setup

**File**: `descartes/daemon/migrations/001_initial.sql` (NEW)

```sql
-- Projects table
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    prd_content TEXT,
    scud_tag TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_projects_owner ON projects(owner_id);

-- Agents table (for tracking cloud agents)
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
    fly_machine_id TEXT,
    status TEXT NOT NULL,
    task_id TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    cost_compute_seconds INTEGER DEFAULT 0,
    cost_tokens INTEGER DEFAULT 0
);

CREATE INDEX idx_agents_project ON agents(project_id);
CREATE INDEX idx_agents_status ON agents(status);

-- Cost tracking table
CREATE TABLE IF NOT EXISTS cost_entries (
    id TEXT PRIMARY KEY,
    agent_id TEXT REFERENCES agents(id),
    project_id TEXT REFERENCES projects(id),
    compute_seconds INTEGER NOT NULL,
    tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_cost_project ON cost_entries(project_id);
```

### Success Criteria:

#### Automated Verification:
- [ ] Docker builds: `docker build -t descartes-api -f daemon/Dockerfile .`
- [ ] Migrations run: `sqlite3 test.db < migrations/001_initial.sql`
- [ ] Deploy succeeds: `fly deploy`

#### Manual Verification:
- [ ] API accessible at `https://descartes-api.fly.dev/health`
- [ ] WebSocket connects from browser
- [ ] Can spawn worker machines
- [ ] Workers callback to orchestrator on completion

---

## Testing Strategy

### Unit Tests
- Rust: `cargo test` for all daemon modules
- Elm: `elm-test` for view and update logic

### Integration Tests
- API endpoint tests with mock Fly.io responses
- WebSocket event flow tests
- End-to-end project creation → task execution

### Manual Testing Steps
1. Create account via OAuth
2. Create project with PRD
3. Parse PRD, verify wave visualization
4. Spawn cloud agent
5. Watch real-time logs
6. Verify task completion
7. Check cost tracking

## Performance Considerations

- **Agent Spawn Time**: Target <500ms via Fly.io Machines
- **WebSocket Latency**: Keep event payload <1KB
- **Database**: SQLite fine for MVP; migrate to PostgreSQL at scale
- **Elm Bundle Size**: Use `--optimize` flag, expect ~100KB gzipped

## Migration Notes

- No existing webapp to migrate
- Daemon extensions are additive (backward compatible)
- Existing CLI workflows unaffected

## References

- Original PRD: `working_docs/planning/PRD_Unified_Platform.md`
- Daemon source: `descartes/daemon/`
- Fly.io Machines API: https://fly.io/docs/machines/api/
- elm-css: https://package.elm-lang.org/packages/rtfeldman/elm-css/latest/
- postcss-elm-css-tailwind: https://github.com/justinrassier/postcss-elm-css-tailwind

---

## Cost Estimates (Fly.io)

| Component | Cost | Monthly (500 users) |
|-----------|------|---------------------|
| Orchestrator (always-on) | $32/mo | $32 |
| Workers (shared-cpu, ~100 hrs/user) | $0.007/hr | $350 |
| Database (SQLite on volume) | $0.15/GB | $5 |
| Bandwidth | $0.02/GB | $50 |
| **Total** | | **~$437/mo** |

At 500 users: ~$0.87/user/month infrastructure cost
