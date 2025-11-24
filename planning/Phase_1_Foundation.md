# Descartes Implementation Phase 1: Foundation
**Goal**: Establish the core Rust architecture, high-performance plumbing, and Unified LLM Abstraction.
**Timeline**: Weeks 1-4

---

## 1. Project Setup & Core Architecture
- [ ] **Initialize Rust Workspace**
    - Create `Cargo.toml` with workspace members: `core`, `cli`, `gui`, `agent-runner`.
    - Set up dependencies: `tokio`, `sqlx` (sqlite), `clap`, `tracing`, `serde`.
    - **Performance Deps**: Add `mimalloc` (allocator), `rkyv` (zero-copy serialization), `ipmpsc` (shared memory).
    - Configure CI/CD pipeline (GitHub Actions) for build and test.

- [ ] **Define Core Traits**
    - `AgentRunner`: The interface for spawning and managing agents.
    - `StateStore`: The interface for persistence (SQLite).
    - `ContextSyncer`: The interface for file/context access.
    - `ModelBackend`: **[NEW]** Unified trait for LLM providers (API, Headless, Local).

## 2. Process Management (The "Agent Runner")
- [ ] **Implement `LocalProcessRunner`**
    - Use `tokio::process::Command` to spawn child processes.
    - Implement `stdin` writing and `stdout/stderr` streaming.
    - Add signal handling (SIGINT/SIGTERM) for graceful shutdown.

- [ ] **Implement `ModelBackend` Providers**
    - **API**: Implement `OpenAI` and `Anthropic` HTTP clients.
    - **Headless**: Implement `ClaudeCodeAdapter` (spawning `claude` CLI).
    - **Local**: Implement `OllamaAdapter` (connecting to `localhost:11434`).

## 3. State & Persistence
- [ ] **SQLite Schema Design**
    - Create `events` table (Hot Path).
    - Create `tasks` table (Global Task Manager).
    - Create `sessions` table.
    - Set up `sqlx` migrations.

- [ ] **Implement `SqliteStore`**
    - Implement `StateStore` trait using `sqlx`.
    - Create the "Hot Path" logger for events.

## 4. High-Performance Plumbing
- [ ] **Git Layer (`gitoxide`)**
    - Integrate `gix` crate.
    - Implement basic `GitRepo` trait using `gix` for fast traversal.

- [ ] **IPC Layer**
    - Implement `SharedMemoryChannel` using `ipmpsc` + `rkyv`.
    - Benchmark vs Standard Pipes to verify zero-copy gains.

## 5. CLI Implementation
- [ ] **Basic Commands**
    - `descartes init`: Initialize `.descartes` directory and DB.
    - `descartes spawn`: Launch an agent (via `ModelBackend`).
    - `descartes logs`: Tail the event stream from the DB.

- [ ] **Pipe Support**
    - Implement reading from `stdin` in the CLI to support `cat file | descartes spawn`.

---

## Acceptance Criteria for Phase 1
1.  Can run `descartes init` to create a project.
2.  Can run `descartes spawn --provider anthropic "Hello"` (API Mode).
3.  Can run `descartes spawn --provider headless "Hello"` (Claude CLI Mode).
4.  Can pipe a file into an agent: `cat README.md | descartes spawn "Summarize this"`.
5.  All events are logged to the local SQLite DB.
6.  `gitoxide` is used for basic repo checks.
