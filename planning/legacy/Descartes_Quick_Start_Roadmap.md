# Descartes Quick Start & Development Roadmap

## Project Overview

**Descartes** is a comprehensive AI orchestration platform that combines:
- SCUD's task management system
- Multi-agent swarm orchestration
- Session Bridge Protocol for CLI tool integration
- Interactive monitoring dashboards
- Human-in-the-loop approval controls

---

## Week 1: Foundation & Session Bridge PoC

### Day 1-2: Project Setup

```bash
# Create project structure
mkdir descartes && cd descartes
cargo new --bin descartes-daemon
cargo new --lib descartes-core
npm create vite@latest descartes-ui -- --template vanilla
cd descartes-ui && npm install -D elm

# Initialize git
git init
echo "target/" >> .gitignore
echo "node_modules/" >> .gitignore
echo ".taskmaster/" >> .gitignore
```

### Day 3-4: Session Bridge Core

```rust
// descartes-core/src/session/mod.rs
pub mod bridge;
pub mod discovery;
pub mod state;

// Start with Claude detection
use sysinfo::{System, SystemExt, ProcessExt};

pub fn find_claude_sessions() -> Vec<DetectedSession> {
    let mut system = System::new_all();
    system.refresh_processes();
    
    system.processes()
        .values()
        .filter(|p| p.name().contains("claude"))
        .map(|p| DetectedSession {
            pid: p.pid(),
            command: p.cmd().join(" "),
            working_dir: p.cwd().map(|p| p.to_path_buf()),
        })
        .collect()
}
```

### Day 5: Test Session Resume

```rust
// Test resuming a Claude session
#[tokio::test]
async fn test_claude_resume() {
    let sessions = find_claude_sessions();
    assert!(!sessions.is_empty());
    
    let session = &sessions[0];
    let context = export_context(session.pid)?;
    
    // Start new Claude with context
    let new_session = Command::new("claude")
        .stdin(Stdio::piped())
        .spawn()?;
    
    inject_context(&new_session, &context)?;
}
```

---

## Week 2: SCUD Integration

### Day 6-7: Import SCUD Models

```rust
// Copy SCUD's task model
// descartes-core/src/models/task.rs
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub complexity: u32,
    pub dependencies: Vec<String>,
    // Add Descartes fields
    pub session_id: Option<Uuid>,
    pub agent_type: Option<AgentType>,
}

// Import SCUD's Epic structure
pub struct Epic {
    pub tag: String,
    pub tasks: Vec<Task>,
}
```

### Day 8-9: Task CLI Commands

```bash
# Create SCUD-compatible CLI
cargo install clap

# Implement basic commands
descartes task list
descartes task show TASK-001
descartes task claim TASK-001 --name alice
descartes task assign TASK-001 --agent claude
```

### Day 10: Connect to AI

```rust
// Hook up task assignment to session creation
pub async fn assign_task_to_agent(
    task_id: &str, 
    agent: AgentType
) -> Result<Uuid> {
    let task = load_task(task_id)?;
    let context = create_task_context(&task)?;
    
    let session = match agent {
        AgentType::Claude => start_claude_session(context),
        AgentType::OpenCode => start_opencode_session(context),
        _ => unimplemented!()
    }?;
    
    update_task_session(task_id, session.id)?;
    Ok(session.id)
}
```

---

## Week 3: Basic UI with Task Board

### Day 11-12: Elm Setup

```elm
-- src/Main.elm
module Main exposing (main)

import Browser
import Html exposing (..)
import TaskBoard
import SwarmMonitor

type Model
    = TaskBoardView TaskBoard.Model
    | SwarmView SwarmMonitor.Model
    | SplitView TaskBoard.Model SwarmMonitor.Model

main =
    Browser.application
        { init = init
        , update = update
        , view = view
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }
```

### Day 13-14: Kanban Board

```elm
-- Implement draggable task cards
type Msg
    = DragStart TaskId
    | DragOver ColumnId
    | Drop TaskId ColumnId

view : Model -> Html Msg
view model =
    div [ class "kanban-board" ]
        (List.map (viewColumn model) 
            [ Backlog, Todo, InProgress, Review, Done ])

viewColumn : Model -> Status -> Html Msg
viewColumn model status =
    div 
        [ class "column"
        , onDragOver DragOver
        , onDrop (Drop status)
        ]
        [ h3 [] [ text (statusName status) ]
        , div [] (List.map viewCard (tasksWithStatus model status))
        ]
```

### Day 15: WebSocket Connection

```javascript
// Connect Elm to Rust backend
const app = Elm.Main.init();
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    app.ports.receiveUpdate.send(data);
};

app.ports.sendCommand.subscribe((cmd) => {
    ws.send(JSON.stringify(cmd));
});
```

---

## Week 4: Swarm Monitor

### Day 16-17: Agent Visualization

```elm
-- SVG-based agent topology
viewAgentNode : Agent -> Svg Msg
viewAgentNode agent =
    g [ transform (translate agent.x agent.y) ]
        [ circle 
            [ r 40
            , fill (agentColor agent.type)
            , onClick (SelectAgent agent.id)
            , onDoubleClick (AttachToAgent agent.id)
            ]
            []
        , text_ [ y 5 ] [ text agent.name ]
        ]
```

### Day 18-19: Terminal Attachment

```rust
// Attach to agent's PTY
pub async fn attach_terminal(
    session_id: Uuid,
    ws: WebSocket
) -> Result<()> {
    let session = get_session(session_id)?;
    let mut pty = session.get_pty()?;
    
    // Stream PTY output to WebSocket
    while let Some(output) = pty.read().await? {
        ws.send(Message::Text(output)).await?;
    }
    
    Ok(())
}
```

### Day 20: Control Actions

```elm
-- Agent control buttons
viewAgentControls : Agent -> Html Msg
viewAgentControls agent =
    div [ class "controls" ]
        [ button [ onClick (PauseAgent agent.id) ] [ text "⏸️" ]
        , button [ onClick (AttachToAgent agent.id) ] [ text "📎" ]
        , button [ onClick (RestartAgent agent.id) ] [ text "🔄" ]
        ]
```

---

## Week 5: Multi-Tool Support

### Day 21-22: OpenCode Adapter

```rust
pub struct OpenCodeAdapter;

impl AgentAdapter for OpenCodeAdapter {
    async fn start_session(&self, context: Context) -> Result<Session> {
        let mut cmd = Command::new("interpreter");
        cmd.arg("--safe");
        
        let child = cmd.spawn()?;
        inject_python_context(&child, &context)?;
        
        Ok(Session::new(child))
    }
}
```

### Day 23-24: Codex Adapter

```rust
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    async fn start_session(&self, context: Context) -> Result<Session> {
        // Codex implementation
    }
}
```

### Day 25: Test Handoffs

```bash
# Test session handoff
$ descartes session list
SESSION-1  claude     TASK-001  Running

$ descartes session handoff SESSION-1 --to opencode
✓ Context exported from Claude
✓ OpenCode session started
✓ Context injected
New session: SESSION-2
```

---

## Week 6: Approval System

### Day 26-27: Approval Interceptor

```rust
pub struct ApprovalInterceptor;

impl Interceptor for ApprovalInterceptor {
    async fn intercept(&self, operation: Operation) -> Result<Decision> {
        // Check if operation needs approval
        if operation.is_dangerous() {
            let request = ApprovalRequest::new(operation);
            
            // Send to UI
            broadcast_approval_request(request.clone()).await?;
            
            // Wait for decision
            wait_for_approval(request.id).await
        } else {
            Ok(Decision::Allow)
        }
    }
}
```

### Day 28-29: Approval UI

```elm
viewApprovalRequest : ApprovalRequest -> Html Msg
viewApprovalRequest request =
    div [ class "approval-card" ]
        [ h4 [] [ text "Approval Required" ]
        , p [] [ text request.description ]
        , pre [] [ text request.details ]
        , button 
            [ class "approve"
            , onClick (Approve request.id) 
            ] 
            [ text "✅ Approve" ]
        , button 
            [ class "deny"
            , onClick (Deny request.id) 
            ] 
            [ text "❌ Deny" ]
        ]
```

### Day 30: Integration Testing

```bash
# Full workflow test
$ descartes init
$ descartes parse-prd docs/requirements.md
$ descartes swarm start --auto-assign
$ descartes monitor

# Verify:
# - Tasks created from PRD
# - Agents assigned and running
# - Approvals working
# - Sessions resumable
```

---

## Month 2: Polish & Advanced Features

### Week 7-8: Performance & Reliability
- Add database persistence (SQLite)
- Implement checkpointing
- Add crash recovery
- Optimize WebSocket updates

### Week 9-10: Team Features
- Multi-user support
- Task claiming/locking
- Team dashboards
- Audit logging

### Week 11-12: Production Ready
- Documentation
- Docker deployment
- CI/CD pipeline
- Beta testing

---

## Directory Structure

```
descartes/
├── Cargo.toml                 # Workspace root
├── descartes-core/            # Core library
│   ├── src/
│   │   ├── models/           # Task, Epic, Session
│   │   ├── session/          # Session Bridge Protocol
│   │   ├── agents/           # Agent adapters
│   │   └── storage/          # Database layer
├── descartes-daemon/          # Backend service
│   ├── src/
│   │   ├── api/             # REST/WebSocket endpoints
│   │   ├── orchestrator/    # Swarm management
│   │   └── main.rs
├── descartes-cli/            # CLI interface
│   ├── src/
│   │   ├── commands/        # CLI commands
│   │   └── main.rs
├── descartes-ui/             # Frontend
│   ├── src/
│   │   ├── TaskBoard.elm
│   │   ├── SwarmMonitor.elm
│   │   └── Main.elm
│   ├── public/
│   └── package.json
├── .taskmaster/              # SCUD compatibility
│   └── tasks/
│       └── tasks.json
└── docs/
    ├── PRD.md
    └── API.md
```

---

## Development Commands

```bash
# Daily development
make dev              # Start all services in dev mode
make test            # Run test suite
make fmt             # Format code

# Task management
make task-list       # List all tasks
make task-claim ID   # Claim a task
make task-assign ID  # Assign to AI

# Session management  
make session-list    # List sessions
make session-attach  # Attach to session
make session-resume  # Resume session

# Monitoring
make monitor         # Open dashboard
make logs           # View daemon logs
```

---

## Key Milestones

### MVP (Week 4)
✅ Session resumption working
✅ Basic task management
✅ Single agent execution
✅ Simple web UI

### Alpha (Week 6)
✅ Multi-tool support
✅ Swarm orchestration
✅ Interactive dashboards
✅ Approval system

### Beta (Week 10)
⏳ Team features
⏳ Performance optimized
⏳ Documentation complete
⏳ Docker deployment

### 1.0 Release (Week 12)
⏳ Production ready
⏳ Comprehensive testing
⏳ User documentation
⏳ Community feedback incorporated

---

## Getting Help

### Resources
- SCUD source: Reference implementation for tasks
- HumanLayer: Reference for approval patterns  
- Elm guide: https://guide.elm-lang.org
- Tauri docs: https://tauri.app/v1/guides

### Community
- GitHub Discussions: Questions and ideas
- Discord: Real-time help
- Weekly demos: Thursdays 2pm PT

### Contributing
1. Pick a task from the board
2. Claim it with your name
3. Create a branch
4. Submit PR when done
5. Request review

---

## Success Metrics

### Technical
- [ ] 95% session recovery success rate
- [ ] <200ms UI response time
- [ ] Support 10+ concurrent agents
- [ ] <5% CPU overhead

### User Experience  
- [ ] Setup in <5 minutes
- [ ] First task completed <10 minutes
- [ ] Zero data loss on crashes
- [ ] Intuitive without documentation

### Adoption
- [ ] 50 beta users in month 1
- [ ] 500 users in month 3
- [ ] 5 contributor PRs
- [ ] 3 integration plugins

---

## Start Today!

```bash
# Clone the starter template
git clone https://github.com/yourusername/descartes-starter
cd descartes-starter

# Install dependencies
./scripts/setup.sh

# Start development
make dev

# Open browser to http://localhost:8080
# You're ready to orchestrate AI!
```

The journey from concept to working system is just 6 weeks. Start with the Session Bridge (immediate value), add SCUD's tasks (proven model), then layer on the swarm orchestration and interactive dashboards. Each week builds on the last, delivering value incrementally.

Remember: **"Cogito, ergo aedifico"** - I think, therefore I build!
