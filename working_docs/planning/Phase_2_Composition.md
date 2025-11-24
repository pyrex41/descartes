# Descartes Implementation Phase 2: Composition & Intelligence
**Goal**: Enable multi-agent workflows, High-Performance RAG, and Declarative Control.
**Timeline**: Weeks 5-8

---

## 1. High-Performance RAG Layer ("Tri-Store")
- [ ] **Ingestion Pipeline (AST-Based)**
    - Integrate `tree-sitter` with grammars for Rust, Python, Markdown, Git Commit.
    - Implement "Semantic Chunking" strategy (AST nodes vs text splitting).
    - Use `rayon` for parallel indexing.

- [ ] **Data Stores**
    - **LanceDB**: Implement vector storage for embeddings.
    - **Tantivy**: Implement full-text search index.
    - **SQLite**: Extend schema for AST relationships and file dependencies.

## 2. Declarative Control Plane
- [ ] **State Machine Engine**
    - Integrate `rust-fsm` or `GraphFlow`.
    - Implement `Swarm.toml` parser for declarative state charts.
    - Implement compile-time verification for deadlocks.

- [ ] **Visualization**
    - Implement Mermaid diagram generator from `Swarm.toml`.

## 3. Advanced Context & Concurrency
- [ ] **File Locking (Leases)**
    - Implement TTL-based file locking in `ContextSyncer`.
    - Add `acquire_lease(file)` and `release_lease(file)` methods.

- [ ] **Optimistic Concurrency**
    - Implement hash-based checks on file writes.
    - Reject writes if file hash has changed since read.

## 4. Human-in-the-Loop & Personal Context
- [ ] **Approval Manager**
    - Implement `ApprovalQueue` in SQLite.
    - Implement `auto-deny` policies and timeouts.
    - Expose API for external UIs to approve/deny actions.

- [ ] **"Thoughts" System**
    - Implement `~/.descartes/thoughts` global storage.
    - Implement symlinking logic for project-specific notes.
    - Index thoughts in Tantivy for agent retrieval.

## 5. Global Task Manager
- [ ] **Task Database**
    - Finalize `tasks` schema in SQLite.
    - Implement CRUD operations in `StateStore`.

- [ ] **SCUD Integration**
    - Create `.descartes/config.toml` schema.
    - Implement "Shared Schema" logic so `scud` can read the DB.

## 6. Security & Notifications
- [ ] **Secrets Vault**
    - Implement local encrypted storage (AES-256).
    - Implement secret masking in logs.

- [ ] **Notification Router**
    - Create `NotificationRouter` trait.
    - Implement `TelegramAdapter` (Bot API).

---

## Acceptance Criteria for Phase 2
1.  Can ingest a repo and query it via RAG (LanceDB + Tantivy).
2.  Can define a swarm in `Swarm.toml` and run it deterministically.
3.  Can run two agents simultaneously without file collisions (Leases working).
4.  Can "think" in a personal note and have an agent reference it.
5.  Can receive a permission request via Telegram and approve it.
