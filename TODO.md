# Immediate Fixes

## Completed

- [x] **Agent lifecycle cleanup** – ensure `LocalProcessRunner` removes completed handles so `max_concurrent_agents` is honored; also reflect termination in the registry when signals are delivered. (`descartes/core/src/agent_runner.rs`)
- [x] **Stream bootstrap transitions** – allow auto-created agents in `AgentStreamParser` to reach terminal and paused states without violating transition rules. (`descartes/core/src/agent_stream_parser.rs`)
- [x] **RPC authentication** – wire `JsonRpcServer::extract_auth_context` to the configured `AuthManager` so requests are authenticated/authorized before handlers run. (`descartes/daemon/src/rpc.rs`)
- [x] **SQLite migrations** – split multi-statement migrations into per-statement executions so the schema actually matches the inline definitions. (`descartes/core/src/state_store.rs`)
- [x] **WASM plugin runtime** – execute plugins through a real WASM runtime with memory passing so CLI `plugins exec` runs user code instead of the stub. (`descartes/core/src/plugins`)
- [x] **Plugin discovery robustness** – stop swallowing `read_dir` errors and derive stable plugin names from filenames to avoid silent `Unknown` collisions. (`descartes/core/src/plugins/manager.rs`)

## Next Up

_(none)_
