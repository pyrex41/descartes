# Comparison: Ash Framework (Elixir) vs. Descartes (Rust/Iced)

This document compares the `AshPlan.md` (Elixir/Phoenix/Ash) with the current `Descartes_Master_Plan.md` (Rust/Iced/SQLite).

## 1. Core Philosophy & Architecture

| Feature | Ash Framework Plan (Elixir) | Descartes Plan (Rust) |
| :--- | :--- | :--- |
| **Paradigm** | **Centralized Platform**: A server-side application that manages everything. | **Unix Toolchain**: Composable, local-first CLI tools and native GUI. |
| **Runtime** | **BEAM VM**: Agents are lightweight `GenServer` processes. | **Native Binary**: Agents are OS processes (`std::process`) or Containers. |
| **Communication** | **Internal Messaging**: Erlang message passing & Phoenix PubSub. | **Standard Streams**: `stdin`, `stdout`, `stderr` pipes & IPC. |
| **State** | **PostgreSQL**: Centralized DB for all state and vector data. | **SQLite**: Local file-based DB per project/session. |
| **UI** | **Web (Phoenix LiveView)**: HTML/WebSocket-based browser interface. | **Native (Iced)**: GPU-accelerated desktop application. |

### Key Takeaway
*   **Ash Plan** builds a **SaaS-like product** where the intelligence lives on a server.
*   **Rust Plan** builds a **Developer Tool** (like `git` or `cargo`) that lives on your machine and integrates with your existing workflow.

---

## 2. Component Mapping

How concepts from the Ash plan translate to the Rust plan:

### 2.1 The "Agent"
*   **Ash**: A `GenServer` process managed by a DynamicSupervisor. It holds state in memory and persists to Postgres.
*   **Rust**: An OS Process spawned via `AgentRunner`. It communicates via JSON streams over `stdout`.
    *   *Benefit (Rust)*: True isolation, can run any binary (Python script, binary, etc.), easier to kill/manage via OS tools.
    *   *Benefit (Ash)*: Extremely lightweight, can spawn thousands cheaply.

### 2.2 Orchestration
*   **Ash**: "Workflow Engine" defined as Ash Resources (`Workflow`, `Task`). State transitions handled by Ash State Machine.
*   **Rust**: "Unix Pipes" for simple flows (`spawn | spawn`) and a "Global Task Manager" (SQLite) for complex swarms.
    *   *Rust Innovation*: The "Unix Pipe" model allows ad-hoc orchestration without defining a formal workflow first.

### 2.3 User Interface
*   **Ash**: **Phoenix LiveView Dashboard**.
    *   *Pros*: Accessible from anywhere, real-time updates, no install.
    *   *Cons*: Browser latency, less integration with local system (files, terminals).
*   **Rust**: **Iced GUI**.
    *   *Pros*: Native performance, system tray integration, can embed real terminal emulators, works offline.
    *   *Cons*: Requires local installation.

### 2.4 Knowledge Graph
*   **Ash**: **Postgres + pgvector**. Centralized, powerful, standard SQL.
*   **Rust**: **SQLite + FTS5 + Local Embeddings**.
    *   *Rust Approach*: "Cold Path" background workers process events and update local SQLite indices. Keeps data private and local.

---

## 3. Feature Comparison

### 3.1 Project Management
*   **Ash**: Create "Projects" in the database. Multi-tenant by default (Organizations/Users).
*   **Rust**: "Projects" are just directories with a `.descartes` folder (like `.git`). Single-user focus (initially).

### 3.2 Context Management
*   **Ash**: Agents read from DB or S3. Code must be ingested/synced to the server.
*   **Rust**: **Direct File Access**. Agents read directly from the local filesystem (with "Leases" for safety). No syncing delay.

### 3.3 IDE Integration
*   **Ash**: Likely requires a VS Code extension to talk to the API.
*   **Rust**: **LSP (Language Server Protocol)**. The binary *is* a language server. It integrates natively into VS Code/Neovim without custom plugins.

---

## 4. Analysis of OpenSwarm (AgentNet)

We analyzed `openswarm.xml`, which contains `Taskmaster` (CLI) and `AgentNet` (Elixir Backend).

### 4.1 Key Features of OpenSwarm
1.  **Execution Control (`Agentnet.ExecutionControl`)**:
    *   **Pause/Resume**: Can pause the entire swarm execution.
    *   **Step-by-Step**: Can execute one LLM call at a time ("Debugger" mode).
    *   **Replay**: Can replay execution states from ETS storage.
    *   *Relevance*: This is the "razor sharp control" the user wants.

2.  **Swarm Protocols (`Agentnet.Swarm.Coordinator`)**:
    *   **Voting/Tournaments**: Implements `hybrid_tournament_reviewer` strategy where agents vote on proposals.
    *   **Temperature Planning**: Automatically varies temperature across swarm members (`linear`, `random`).

3.  **Context Isolation (`Taskmaster`)**:
    *   **Tags**: Uses "Tags" in `tasks.json` to isolate tasks for features/branches.
    *   **Workflow**: Explicitly maps Tags to Git Branches (Pattern 1).

### 4.2 Integration into Descartes (Rust)
To achieve the "Right Path", Descartes should adopt the best parts of OpenSwarm:

1.  **Adopt "Debugger Mode"**:
    *   The **Rust AgentRunner** should support a `debug` flag that pauses before every LLM call.
    *   The **Iced GUI** should have "Step", "Continue", and "Rewind" buttons, similar to a GDB/LLDB interface for agents.

2.  **Adopt Swarm Strategies**:
    *   Implement the **Voting/Tournament** logic as a standard library in Rust, not just hardcoded strategies.
    *   Allow users to compose these strategies via pipes: `spawn --count 10 | tournament --top 3 | reviewer`.

3.  **Refine Worktree Isolation**:
    *   OpenSwarm's "Tags" are a lightweight version of Descartes' **Worktree Manager**.
    *   Descartes should ensure that *every* agent process is explicitly bound to a specific Git Worktree (and thus a specific Context Slice).

---

## 5. Summary & Recommendation

The move to **Rust/Iced** represents a shift from building a **Platform** to building a **Tool**.

**Recommendation**: Proceed with **Descartes (Rust/Iced)** but explicitly add the **Execution Control** (Debugger) features from OpenSwarm.

*   **Why Rust?** It gives the "Razor Sharp" control over memory, processes, and local state that a managed VM (BEAM) abstracts away.
*   **Why Iced?** It allows building the "Debugger UI" natively, which is crucial for the "Step-by-Step" control the user admired in OpenSwarm.
*   **The Missing Piece**: Ensure the `StateStore` (SQLite) captures the granular "Pre-Call/Post-Call" states that OpenSwarm stores in ETS, enabling the "Time Travel" feature.
