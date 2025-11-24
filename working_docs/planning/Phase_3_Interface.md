# Descartes Implementation Phase 3: Interface & Swarms
**Goal**: Deliver the Native GUI, Debugger, and massive parallel agent swarms.
**Timeline**: Weeks 9-12

---

## 1. Daemon & RPC Layer
- [ ] **JSON-RPC Server**
    - Implement `jsonrpsee` server over Unix Socket.
    - Expose `spawn`, `list_tasks`, `approve`, `get_state` methods.
    - Ensure `scud` CLI and `descartes` GUI use the same RPC API.

- [ ] **ZMQ Transport (Future Proofing)**
    - Implement `ZmqAgentRunner` trait.
    - Allow spawning agents on remote machines via ZMQ.

## 2. Iced GUI (V1 - Dashboard)
- [ ] **Application Shell**
    - Set up `iced` application structure.
    - Implement Event Bus subscription (listening to Core events via RPC).

- [ ] **Task Board View**
    - Visualize the Global Task Manager (Kanban/List).
    - Real-time updates from SQLite events.

- [ ] **Swarm Monitor**
    - Visualize active agents and their status.
    - Show "Thinking" state via JSON Stream parsing.

## 3. Execution Control (Debugger)
- [ ] **Debugger UI**
    - Implement "Step-by-Step" mode in Iced.
    - Add "Pause", "Resume", "Step" buttons.
    - Visualize the current "Thought" and "Context" in a split view.

- [ ] **Time Travel (Rewind)**
    - Implement `restore_brain` (load events) and `restore_body` (git checkout).
    - Add slider UI to scroll back through agent history.

## 4. Iced GUI (V2 - Interactive)
- [ ] **Visual DAG Editor**
    - Implement node-based graph editor in Iced.
    - Allow drag-and-drop task dependencies.
    - Auto-generate `Swarm.toml` from the visual graph.

- [ ] **Interactive Context Browser**
    - Visual file tree with "Knowledge Graph" overlays.

---

## Acceptance Criteria for Phase 3
1.  Can launch the GUI and see tasks moving in real-time.
2.  Can pause an agent, inspect its thought, and resume it.
3.  Can "Time Travel" back to a previous state and resume working.
4.  Can deploy a swarm to a remote machine via ZMQ (Proof of Concept).
5.  Can visualize the entire project workflow as a DAG.
