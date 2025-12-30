# Deploying Descartes Orchestrator

## Prerequisites

1. Fly.io CLI installed: `brew install flyctl` or `curl -L https://fly.io/install.sh | sh`
2. Fly.io account and authentication: `fly auth login`
3. Required secrets configured (see below)

## Initial Setup

```bash
# Create the app (first time only)
cd descartes/daemon
fly apps create descartes-api

# Create persistent volume for SQLite
fly volumes create descartes_data --size 1 --region iad

# Set required secrets
fly secrets set ANTHROPIC_API_KEY=your-key-here
fly secrets set FLY_API_TOKEN=$(fly tokens create deploy -x 999999h)
fly secrets set JWT_SECRET=$(openssl rand -hex 32)
```

## Deployment

```bash
# Deploy the orchestrator
fly deploy

# Check status
fly status

# View logs
fly logs

# SSH into the machine (for debugging)
fly ssh console
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `FLY_API_TOKEN` | Yes | Token for spawning worker machines |
| `ANTHROPIC_API_KEY` | Yes | API key for Claude agent |
| `JWT_SECRET` | Yes | Secret for JWT token signing |
| `DATABASE_URL` | Auto | SQLite path (defaults to /data/descartes.db) |
| `RUST_LOG` | No | Log level (default: info) |

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `https://descartes-api.fly.dev/health` | Health check |
| `https://descartes-api.fly.dev/api/projects` | Project CRUD |
| `wss://descartes-api.fly.dev:8081/events` | WebSocket events |

## Scaling

```bash
# Scale to 2 machines
fly scale count 2

# Scale machine size
fly scale vm shared-cpu-2x
```

## Monitoring

- Dashboard: https://fly.io/apps/descartes-api
- Metrics: `fly dashboard`
- Logs: `fly logs -a descartes-api`
