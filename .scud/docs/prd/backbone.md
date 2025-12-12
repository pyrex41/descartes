This makes perfect sense. By keeping ZeroMQ, you are effectively betting on a **Service-Oriented Architecture (SOA)** from day one, rather than a monolithic internal Rust application. This aligns perfectly with the goal of management and observability: if every agent is just a ZMQ client (whether local or remote), the Daemon becomes a standardized "Traffic Controller."

Here is the updated prompt to guide an AI or engineering team to execute this specific **ZMQ-First, RAG-Lite** refactor.

***

### The Prompt: "ZMQ Backbone" Refactor

**Context:**
We are hardening the `Descartes` architecture. We have decided that the current codebase is over-engineered in specific areas (RAG, Semantic Analysis, Internal Message Bus complexity) but correctly positioned in others (ZMQ Networking, Git-based State Restore, GUI Observability).

**Strategic Pivot:**
1.  **ZMQ is the Backbone:** We will standardise on ZeroMQ for *all* agent communication (local and remote).
2.  **Kill the Bloat:** We are removing the heavy "semantic analysis" stack (`lancedb`, `tantivy`, `tree-sitter`). The agent does not need to be a compiler; it just needs to execute tools.
3.  **Flatten the DB:** We are moving away from complex relational schemas to a high-speed **Event Sourcing** model stored in SQLite.

**Objective:**
Produce a refactoring plan that strips `agent-runner` and `core` down to a lightweight, ZMQ-driven orchestration engine.

**Detailed Requirements:**

**1. The "Delete" List (Aggressive Pruning)**
Identify dependencies and files to remove to speed up compilation and reduce binary size.
*   **Remove RAG:** Delete `agent-runner/src/rag.rs`, `semantic.rs`, and `knowledge_graph.rs`. Remove `lancedb` and `tantivy` dependencies.
*   **Simplify Internal IPC:** We have a complex internal bus (`ipc.rs`) with Dead Letter Queues and Backpressure. **Replace this.** If ZMQ handles the transport, the internal bus should just be simple Tokio channels bridging the ZMQ socket to the State Store.
*   **Remove Semantic DB:** Drop the 13+ tables related to AST nodes (`semantic_nodes`, `file_dependencies`).

**2. The Database Refactor (Event Sourcing)**
Design a minimal SQLite schema that focuses entirely on **Observability** and **Time Travel**.
We need essentially 3 tables:
*   `agents`: (UUID, status, ZMQ address/metadata).
*   `events`: The append-only log of everything (Thoughts, Tool Outputs, Errors). This is the source of truth for the GUI.
*   `snapshots`: For the Time Travel feature (Git Commit Hash + Last Event ID).

**3. The ZMQ-First Architecture**
Describe the data flow where ZMQ is the primary interface.
*   *Control Plane:* The Daemon binds a ROUTER socket.
*   *Data Plane:* Agents (even local ones) connect via DEALER sockets.
*   *Flow:* Agent stdout/stderr -> ZMQ Message -> Daemon -> SQLite (Event Log) -> GUI (via WebSocket).
*   *Benefits:* This makes moving an agent to a different server trivial later (just change the IP), as the architecture doesn't care if it's local or remote.

**4. The "Brain & Body" Simplification**
Clarify how the "Time Travel" feature works without the complex Knowledge Graph.
*   *Body:* Standard Git commits (keep `body_restore.rs`).
*   *Brain:* Replaying the `events` table.
*   Confirm that we do *not* need to restore complex AST states, only the event log history.

**Deliverable:**
A concise architectural specification and a list of file modifications/deletions to achieve this "ZMQ-First" lightweight state.

***

### Why this works for your goals:

1.  **ZMQ Flexibility:** By removing the complex internal `ipc.rs` and relying on ZMQ, you remove the distinction between "local thread" and "remote process." Everything is just a socket.
2.  **Focus on Management:** By stripping the RAG/AST code, the application stops trying to "understand" the code and focuses entirely on **managing the agents** that are writing the code.
3.  **Performance:** Removing `lancedb` and `tree-sitter` will likely drop compile times by 70% and runtime memory usage significantly, making the "Swarm" feel snappy in the GUI.
