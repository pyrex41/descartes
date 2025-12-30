# Descartes Worker

Ephemeral worker binary that executes SCUD tasks on Fly.io Machines.

## Overview

Workers are spawned on-demand by the orchestrator (descartes-daemon) and automatically destroyed after task completion. Each worker:

1. Receives task configuration via environment variables
2. Notifies the orchestrator when starting
3. Executes the assigned task
4. Reports completion/failure back to orchestrator
5. Exits (machine auto-destroys)

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `TASK_ID` | Yes | SCUD task identifier to execute |
| `PROJECT_ID` | Yes | Project context for the task |
| `CALLBACK_URL` | Yes | Orchestrator URL for status updates |
| `ANTHROPIC_API_KEY` | No | API key for Claude agent (if needed) |
| `RUST_LOG` | No | Log level (default: info) |

## Building

```bash
# Build Docker image
docker build -t descartes-worker -f Dockerfile ..

# Push to Fly.io registry
fly auth docker
docker tag descartes-worker registry.fly.io/descartes-workers:latest
docker push registry.fly.io/descartes-workers:latest
```

## Deployment

Workers are NOT deployed with `fly deploy`. Instead:

1. Build and push the image once
2. The orchestrator spawns workers via Fly.io Machines API
3. Workers auto-destroy after completion

## Local Testing

```bash
# Run locally with test environment
TASK_ID=test-123 \
PROJECT_ID=proj-456 \
CALLBACK_URL=http://localhost:8080/api/agents/callback \
cargo run -p descartes-worker -- --task test-123
```

## Architecture

```
┌─────────────────┐     spawn      ┌─────────────────┐
│   Orchestrator  │ ────────────▶  │     Worker      │
│ (descartes-api) │                │ (ephemeral VM)  │
└────────┬────────┘                └────────┬────────┘
         │                                  │
         │ ◀────── callback: started ───────│
         │                                  │
         │         (task execution)         │
         │                                  │
         │ ◀────── callback: completed ─────│
         │                                  │
         │                            auto-destroy
```
