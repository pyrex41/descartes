# Interactive Monitoring Views for Descartes

## Overview

Two critical real-time dashboards for managing AI-orchestrated development:
1. **Task Board View** - Interactive task management and status tracking
2. **Swarm Monitor View** - Live agent orchestration and control

---

## 1. Task Board View

### Design Inspiration
Based on `tm-view` but enhanced with real-time updates, filtering, and AI agent integration.

### Core Features

#### 1.1 Layout Options

```typescript
// Elm Model
type ViewMode 
    = KanbanBoard      -- Classic columns by status
    | TreeView         -- Hierarchical epic → tasks
    | GraphView        -- Dependency DAG visualization
    | TableView        -- Sortable/filterable grid
    | TimelineView     -- Gantt-style schedule
    | MatrixView       -- Complexity vs Priority grid

type alias TaskBoardModel = {
    viewMode : ViewMode,
    tasks : Dict String Task,
    epics : Dict String Epic,
    filters : FilterSet,
    selection : Maybe String,
    liveUpdates : WebSocket,
}
```

#### 1.2 Kanban Board Layout

```
┌────────────────────────────────────────────────────────────────┐
│  Epic: AUTH-SYSTEM  │  24 tasks  │  3 agents  │  2 humans     │
├────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │ BACKLOG  │  │   TODO   │  │   WIP    │  │   DONE   │      │
│  ├──────────┤  ├──────────┤  ├──────────┤  ├──────────┤      │
│  │ TASK-010 │  │ TASK-005 │  │ TASK-002 │  │ TASK-001 │      │
│  │ [8] API  │  │ [5] Auth │  │ [3] Login│  │ [2] Setup│      │
│  │ 🔒 ---   │  │ 🔓 Ready │  │ 🤖 Claude│  │ ✅ Bob   │      │
│  ├──────────┤  ├──────────┤  ├──────────┤  ├──────────┤      │
│  │ TASK-011 │  │ TASK-006 │  │ TASK-003 │  │ TASK-004 │      │
│  │ [13] DB  │  │ [3] Token│  │ [5] UI   │  │ [1] Config│     │
│  │ ⚠️ Blocked│  │ 👤 Alice │  │ 🤖 OCode │  │ ✅ AI    │      │
│  ├──────────┤  ├──────────┤  ├──────────┤  └──────────┘      │
│  │ TASK-012 │  │ TASK-007 │  │ TASK-009 │                    │
│  │ [21] !!!│  │ [8] Perms│  │ [3] Tests│                    │
│  │ 📊 Complex│  │ 🔓 Ready │  │ 🤖 Codex │                    │
│  └──────────┘  └──────────┘  └──────────┘                    │
│                                                                  │
│ Legend: [n]=complexity 🤖=AI 👤=Human 🔒=Locked ⚠️=Blocked      │
└────────────────────────────────────────────────────────────────┘
```

#### 1.3 Interactive Task Card

```elm
-- Each task card is interactive
type TaskCardMsg
    = Click TaskId
    | DoubleClick TaskId      -- Open details
    | RightClick TaskId        -- Context menu
    | DragStart TaskId
    | DragOver ColumnId
    | Drop TaskId ColumnId
    | HoverStart TaskId       -- Show preview
    | HoverEnd

-- Task card shows:
renderTaskCard : Task -> Html TaskCardMsg
renderTaskCard task =
    div [ 
        class "task-card",
        classList [
            ("locked", task.locked_by /= Nothing),
            ("ai-assigned", task.agent_type /= Nothing),
            ("high-complexity", task.complexity > 8),
            ("blocked", hasUnmetDependencies task)
        ],
        onClick (Click task.id),
        onDoubleClick (DoubleClick task.id),
        draggable True,
        onDragStart (DragStart task.id)
    ] [
        -- Complexity badge
        div [ class "complexity-badge" ] [ 
            text (fibonacciIcon task.complexity) 
        ],
        
        -- Task ID and Title
        div [ class "task-header" ] [
            span [ class "task-id" ] [ text task.id ],
            span [ class "task-title" ] [ text task.title ]
        ],
        
        -- Assignment indicator
        div [ class "assignment" ] [
            case (task.locked_by, task.agent_type) of
                (Just human, Nothing) -> 
                    userIcon human
                (Nothing, Just agent) -> 
                    agentIcon agent
                (Just human, Just agent) -> 
                    collaborationIcon human agent
                _ -> 
                    unassignedIcon
        ],
        
        -- Status indicators
        div [ class "status-row" ] [
            if task.approval_required then
                approvalBadge
            else 
                text "",
            
            if task.session_active then
                liveSessionIndicator task.session_id
            else
                text ""
        ]
    ]
```

#### 1.4 Tree View with Dependencies

```
Epic: AUTH-SYSTEM
├─⬤ TASK-001 [Setup] ✅
├─⬤ TASK-002 [Database Schema] ✅
│  ├─○ TASK-003 [User Table] 🤖 Claude (In Progress)
│  └─○ TASK-004 [Session Table] 👤 Bob (In Progress)
├─⬤ TASK-005 [API Framework] ✅
│  ├─○ TASK-006 [Auth Endpoints] 🔒 Blocked (needs TASK-003)
│  ├─○ TASK-007 [User CRUD] 🔓 Ready
│  └─○ TASK-008 [Session Management] 🔓 Ready
└─⬤ TASK-009 [Frontend] ⏳ Waiting
   ├─○ TASK-010 [Login Form] 🔒 Blocked
   ├─○ TASK-011 [Dashboard] 🔒 Blocked
   └─○ TASK-012 [Profile Page] 🔒 Blocked

[⬤ = Expanded, ○ = Subtask, ✅ = Done, 🤖 = AI, 👤 = Human]
```

#### 1.5 Real-Time Updates

```elm
-- WebSocket subscription for live updates
subscriptions : Model -> Sub Msg
subscriptions model =
    WebSocket.listen model.wsUrl 
        (\msg -> 
            case decodeTaskUpdate msg of
                Ok update -> TaskUpdate update
                Err _ -> NoOp
        )

-- Update handler
update : Msg -> Model -> (Model, Cmd Msg)
update msg model =
    case msg of
        TaskUpdate update ->
            case update.type of
                StatusChanged taskId newStatus ->
                    ( updateTaskStatus model taskId newStatus
                    , animateCard taskId
                    )
                
                AgentAssigned taskId agentType sessionId ->
                    ( assignAgent model taskId agentType sessionId
                    , showNotification ("Agent assigned: " ++ agentType)
                    )
                
                TaskCompleted taskId ->
                    ( markComplete model taskId
                    , Effects.batch [
                        playSound CompletionSound,
                        animateCompletion taskId,
                        checkDependentTasks taskId
                      ]
                    )
```

#### 1.6 Filtering and Search

```elm
type alias FilterSet = {
    status : Maybe TaskStatus,
    assignee : Maybe String,
    complexity : Maybe (Int, Int),  -- Range
    hasAgent : Maybe Bool,
    isBlocked : Maybe Bool,
    searchQuery : Maybe String,
    epic : Maybe String
}

-- Advanced filter UI
renderFilterPanel : FilterSet -> Html Msg
renderFilterPanel filters =
    div [ class "filter-panel" ] [
        -- Quick filters
        div [ class "quick-filters" ] [
            button [ onClick (SetFilter MyTasks) ] [ text "My Tasks" ],
            button [ onClick (SetFilter AIActive) ] [ text "AI Active" ],
            button [ onClick (SetFilter Blocked) ] [ text "Blocked" ],
            button [ onClick (SetFilter HighComplexity) ] [ text "Complex (>8)" ]
        ],
        
        -- Search bar
        input [ 
            type_ "search",
            placeholder "Search tasks...",
            onInput UpdateSearch
        ] [],
        
        -- Advanced filters dropdown
        details [ class "advanced-filters" ] [
            summary [] [ text "Advanced Filters" ],
            -- Filter form here
        ]
    ]
```

---

## 2. Swarm Monitor View

### Core Features

#### 2.1 Swarm Overview Dashboard

```
┌────────────────────────────────────────────────────────────────────┐
│                     SWARM CONTROL CENTER                           │
├────────────────────────────────────────────────────────────────────┤
│  Active Agents: 5  │  Tasks: 12/24  │  CPU: 45%  │  Memory: 3.2GB │
├────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    AGENT TOPOLOGY                            │  │
│  │                                                              │  │
│  │                      [Orchestrator]                          │  │
│  │                           │                                  │  │
│  │        ┌─────────────────┼─────────────────┐               │  │
│  │        │                 │                 │               │  │
│  │   [Claude-1]        [OpenCode-1]      [Codex-1]           │  │
│  │   AUTH-002          FRONTEND-001      TESTS-001           │  │
│  │   ●Running          ●Running          ⚠️Awaiting          │  │
│  │   CPU: 12%          CPU: 45%          Approval            │  │
│  │        │                 │                                  │  │
│  │        │            [OpenCode-2]                           │  │
│  │        │            FRONTEND-002                           │  │
│  │   [Claude-2]        ●Running                              │  │
│  │   AUTH-003          CPU: 23%                              │  │
│  │   ⏸️Paused                                                 │  │
│  │                                                              │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    AGENT DETAILS                            │  │
│  │  ┌──────────────────────────────────────────────────┐      │  │
│  │  │ Claude-1 (session-a1b2c3d4)                      │      │  │
│  │  ├──────────────────────────────────────────────────┤      │  │
│  │  │ Task: AUTH-002 - Implement JWT validation        │      │  │
│  │  │ Status: Running (15:32 elapsed)                  │      │  │
│  │  │ Progress: Writing auth_middleware.rs             │      │  │
│  │  │ Context: 45,231 tokens                          │      │  │
│  │  │ Approvals: 2 pending, 5 completed               │      │  │
│  │  │                                                  │      │  │
│  │  │ [⏸️Pause] [▶️Resume] [🔄Restart] [📎Attach]       │      │  │
│  │  │ [📋Checkpoint] [🔄Handoff] [❌Terminate]         │      │  │
│  │  └──────────────────────────────────────────────────┘      │  │
│  └─────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

#### 2.2 Agent Node Component

```elm
type alias AgentNode = {
    id : SessionId,
    tool : AgentTool,
    task : Maybe TaskId,
    status : AgentStatus,
    metrics : AgentMetrics,
    position : (Float, Float),  -- For drag and drop
    expanded : Bool
}

type AgentStatus 
    = Starting
    | Running Duration
    | Paused PauseReason
    | AwaitingApproval ApprovalRequest
    | Error String
    | Completed

type alias AgentMetrics = {
    cpu_percent : Float,
    memory_mb : Int,
    tokens_used : Int,
    approvals_pending : Int,
    files_modified : Int,
    commands_run : Int
}

-- Interactive agent node
renderAgentNode : AgentNode -> Html Msg
renderAgentNode agent =
    div [ 
        class "agent-node",
        classList [
            ("running", isRunning agent.status),
            ("paused", isPaused agent.status),
            ("error", isError agent.status),
            ("awaiting-approval", isAwaiting agent.status)
        ],
        style "left" (String.fromFloat agent.position.0 ++ "px"),
        style "top" (String.fromFloat agent.position.1 ++ "px"),
        onClick (SelectAgent agent.id),
        onDoubleClick (AttachToAgent agent.id),
        draggable True
    ] [
        -- Agent header
        div [ class "agent-header" ] [
            agentIcon agent.tool,
            text (agentToolName agent.tool),
            statusIndicator agent.status
        ],
        
        -- Task assignment
        case agent.task of
            Just taskId ->
                div [ class "agent-task" ] [
                    text taskId,
                    progressBar (taskProgress taskId)
                ]
            Nothing ->
                div [ class "agent-idle" ] [ text "Idle" ],
        
        -- Metrics
        if agent.expanded then
            renderMetrics agent.metrics
        else
            renderMetricsSummary agent.metrics,
        
        -- Quick actions
        div [ class "agent-actions" ] [
            button [ 
                onClick (PauseAgent agent.id),
                disabled (not (isRunning agent.status))
            ] [ text "⏸️" ],
            
            button [
                onClick (AttachToAgent agent.id)
            ] [ text "📎" ],
            
            button [
                onClick (ExpandAgent agent.id)
            ] [ text (if agent.expanded then "▼" else "▶") ]
        ]
    ]
```

#### 2.3 Live Session Stream

```elm
type alias SessionStream = {
    sessionId : SessionId,
    output : List OutputChunk,
    input : String,
    isAttached : Bool
}

type OutputChunk
    = Stdout String
    | Stderr String
    | Thinking String
    | ToolCall ToolCall
    | ApprovalRequest ApprovalRequest

-- Terminal-like view for attached session
renderSessionStream : SessionStream -> Html Msg
renderSessionStream stream =
    div [ class "session-terminal" ] [
        -- Output area
        div [ 
            class "terminal-output",
            id ("terminal-" ++ stream.sessionId)
        ] (
            List.map renderOutputChunk stream.output
        ),
        
        -- Input area (if attached)
        if stream.isAttached then
            textarea [
                class "terminal-input",
                value stream.input,
                onInput (UpdateInput stream.sessionId),
                onEnter (SendInput stream.sessionId)
            ] []
        else
            div [ class "terminal-readonly" ] [
                text "Read-only mode. Click 'Attach' to interact."
            ]
    ]

renderOutputChunk : OutputChunk -> Html Msg
renderOutputChunk chunk =
    case chunk of
        Stdout text ->
            pre [ class "stdout" ] [ text text ]
        
        Stderr text ->
            pre [ class "stderr" ] [ text text ]
        
        Thinking text ->
            details [ class "thinking" ] [
                summary [] [ text "🤔 Thinking..." ],
                pre [] [ text text ]
            ]
        
        ToolCall call ->
            div [ class "tool-call" ] [
                text ("🔧 " ++ call.tool ++ ": " ++ call.description)
            ]
        
        ApprovalRequest req ->
            div [ class "approval-request" ] [
                text ("⚠️ Approval needed: " ++ req.description),
                button [ onClick (Approve req.id) ] [ text "✅ Approve" ],
                button [ onClick (Deny req.id) ] [ text "❌ Deny" ]
            ]
```

#### 2.4 Swarm Control Panel

```elm
type SwarmControl
    = StartSwarm SwarmConfig
    | StopSwarm
    | ScaleAgents AgentTool Int
    | PauseAll
    | ResumeAll
    | SetApprovalMode ApprovalMode
    | SetResourceLimits ResourceLimits

renderControlPanel : SwarmState -> Html Msg
renderControlPanel swarm =
    div [ class "control-panel" ] [
        -- Global controls
        div [ class "global-controls" ] [
            button [ 
                onClick (if swarm.running then StopSwarm else StartSwarm defaultConfig),
                class (if swarm.running then "stop-button" else "start-button")
            ] [ 
                text (if swarm.running then "⏹️ Stop Swarm" else "▶️ Start Swarm") 
            ],
            
            button [ onClick PauseAll ] [ text "⏸️ Pause All" ],
            button [ onClick ResumeAll ] [ text "▶️ Resume All" ]
        ],
        
        -- Scaling controls
        div [ class "scaling-controls" ] [
            h3 [] [ text "Agent Scaling" ],
            
            agentScaler "Claude" swarm.claudeCount 
                (ScaleAgents Claude),
            agentScaler "OpenCode" swarm.openCodeCount 
                (ScaleAgents OpenCode),
            agentScaler "Codex" swarm.codexCount 
                (ScaleAgents Codex)
        ],
        
        -- Approval mode
        div [ class "approval-controls" ] [
            h3 [] [ text "Approval Mode" ],
            
            radio "approval-mode" [
                ("manual", "Manual - Approve each operation"),
                ("batch", "Batch - Group similar operations"),
                ("auto", "Auto - Approve safe operations"),
                ("autonomous", "Autonomous - No approvals")
            ] swarm.approvalMode SetApprovalMode
        ],
        
        -- Resource limits
        div [ class "resource-controls" ] [
            h3 [] [ text "Resource Limits" ],
            
            slider "CPU Limit (%)" 0 100 swarm.cpuLimit
                (\v -> SetResourceLimits { swarm.limits | cpu = v }),
                
            slider "Memory Limit (GB)" 1 32 swarm.memoryLimit
                (\v -> SetResourceLimits { swarm.limits | memory = v }),
                
            slider "Max Agents" 1 20 swarm.maxAgents
                (\v -> SetResourceLimits { swarm.limits | maxAgents = v })
        ]
    ]
```

#### 2.5 Agent Communication Visualization

```elm
-- Show communication between agents
type AgentMessage 
    = TaskHandoff TaskId FromAgent ToAgent
    | ContextShare Context FromAgent ToAgent
    | DependencyNotification TaskId FromAgent ToAgent
    | ApprovalForward ApprovalRequest FromAgent ToAgent

renderCommunication : List AgentMessage -> Html Msg
renderCommunication messages =
    svg [ class "communication-viz" ] (
        List.map renderMessage messages
    )

renderMessage : AgentMessage -> Svg Msg
renderMessage msg =
    case msg of
        TaskHandoff taskId from to ->
            g [] [
                -- Animated line between agents
                animatedLine from.position to.position "handoff",
                
                -- Message bubble
                text_ [
                    x (midpoint from.position to.position).x,
                    y (midpoint from.position to.position).y,
                    class "message-label"
                ] [ text ("📦 " ++ taskId) ]
            ]
        -- ... other message types
```

---

## 3. Implementation Architecture

### 3.1 Real-Time Data Flow

```rust
// Rust backend websocket handler
pub struct DashboardServer {
    task_updates: broadcast::Sender<TaskUpdate>,
    agent_updates: broadcast::Sender<AgentUpdate>,
    sessions: Arc<RwLock<HashMap<Uuid, SessionState>>>,
}

impl DashboardServer {
    pub async fn handle_connection(&self, ws: WebSocket) {
        let (tx, mut rx) = ws.split();
        
        // Subscribe to updates
        let mut task_rx = self.task_updates.subscribe();
        let mut agent_rx = self.agent_updates.subscribe();
        
        // Stream updates to client
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(task_update) = task_rx.recv() => {
                        let msg = serde_json::to_string(&task_update).unwrap();
                        tx.send(Message::Text(msg)).await.ok();
                    }
                    Ok(agent_update) = agent_rx.recv() => {
                        let msg = serde_json::to_string(&agent_update).unwrap();
                        tx.send(Message::Text(msg)).await.ok();
                    }
                }
            }
        });
    }
}
```

### 3.2 Terminal Attachment

```rust
// Attach to agent's PTY for interactive control
pub async fn attach_to_agent(
    session_id: Uuid,
    websocket: WebSocket,
) -> Result<()> {
    let session = get_session(session_id)?;
    let pty = session.get_pty()?;
    
    // Bidirectional streaming
    let (ws_tx, mut ws_rx) = websocket.split();
    let (pty_tx, mut pty_rx) = pty.split();
    
    // PTY -> WebSocket
    tokio::spawn(async move {
        while let Some(output) = pty_rx.next().await {
            ws_tx.send(Message::Text(output)).await.ok();
        }
    });
    
    // WebSocket -> PTY
    tokio::spawn(async move {
        while let Some(Ok(Message::Text(input))) = ws_rx.next().await {
            pty_tx.send(input.into_bytes()).await.ok();
        }
    });
    
    Ok(())
}
```

### 3.3 Elm Integration

```elm
-- Main dashboard app
type alias Model = {
    taskBoard : TaskBoardModel,
    swarmMonitor : SwarmMonitorModel,
    activeView : DashboardView,
    websocket : WebSocket.Connection
}

type DashboardView
    = TaskBoardView
    | SwarmMonitorView
    | SplitView  -- Both side by side

type Msg
    = TaskBoardMsg TaskBoard.Msg
    | SwarmMonitorMsg SwarmMonitor.Msg
    | SwitchView DashboardView
    | WebSocketMsg WebSocket.Message

update : Msg -> Model -> (Model, Cmd Msg)
update msg model =
    case msg of
        TaskBoardMsg subMsg ->
            let (newTaskBoard, cmd) = 
                TaskBoard.update subMsg model.taskBoard
            in
            ( { model | taskBoard = newTaskBoard }
            , Cmd.map TaskBoardMsg cmd
            )
        
        SwarmMonitorMsg subMsg ->
            let (newSwarmMonitor, cmd) = 
                SwarmMonitor.update subMsg model.swarmMonitor
            in
            ( { model | swarmMonitor = newSwarmMonitor }
            , Cmd.map SwarmMonitorMsg cmd
            )
        
        WebSocketMsg wsMsg ->
            handleWebSocketMessage wsMsg model
```

---

## 4. Interactive Features

### 4.1 Task Board Interactions

| Action | Trigger | Result |
|--------|---------|--------|
| View task details | Click card | Expand inline details |
| Edit task | Double-click | Open edit modal |
| Assign agent | Drag to agent | Start AI session |
| Change status | Drag between columns | Update task status |
| Show dependencies | Hover | Highlight connected tasks |
| Filter by epic | Click epic tag | Show only epic tasks |
| Quick assign | Right-click → Assign | Context menu |
| Bulk operations | Shift-select multiple | Batch actions |

### 4.2 Swarm Monitor Interactions

| Action | Trigger | Result |
|--------|---------|--------|
| View agent details | Click node | Expand details panel |
| Attach to session | Double-click/button | Open terminal view |
| Pause agent | Pause button | Suspend execution |
| Handoff task | Drag between agents | Transfer context |
| Scale agents | +/- buttons | Start/stop instances |
| View communication | Toggle layer | Show message flow |
| Resource monitor | Hover metrics | Show history graph |
| Emergency stop | Red button | Kill all agents |

---

## 5. Performance Optimizations

### 5.1 Virtual Scrolling for Large Task Lists

```elm
-- Only render visible tasks
virtualTaskList : List Task -> Html Msg
virtualTaskList tasks =
    Html.Lazy.lazy VirtualList.view {
        items = tasks,
        renderItem = renderTaskCard,
        itemHeight = 80,
        containerHeight = 600
    }
```

### 5.2 Throttled Updates

```rust
// Batch updates to prevent UI flooding
pub struct UpdateThrottler {
    pending: Vec<Update>,
    last_send: Instant,
    min_interval: Duration,
}

impl UpdateThrottler {
    pub async fn send_update(&mut self, update: Update) {
        self.pending.push(update);
        
        if self.last_send.elapsed() > self.min_interval {
            let batch = mem::take(&mut self.pending);
            self.broadcast(BatchUpdate(batch)).await;
            self.last_send = Instant::now();
        }
    }
}
```

---

## 6. Mobile/Responsive Design

```css
/* Responsive grid for different screen sizes */
.dashboard-container {
    display: grid;
    gap: 1rem;
}

/* Desktop: Side by side */
@media (min-width: 1200px) {
    .dashboard-container {
        grid-template-columns: 1fr 1fr;
    }
}

/* Tablet: Stacked with tabs */
@media (min-width: 768px) and (max-width: 1199px) {
    .dashboard-container {
        grid-template-columns: 1fr;
    }
}

/* Mobile: Simplified cards */
@media (max-width: 767px) {
    .task-card {
        simplified: true;
    }
    .agent-node {
        display: list-item;
    }
}
```

---

## 7. Example Workflows

### Workflow 1: Morning Standup

```bash
# Open dashboard
$ descartes dashboard

# View shows:
# - 3 tasks completed overnight by AI
# - 2 tasks awaiting approval
# - 1 agent error that needs intervention

# Click on error agent → Attach → Debug → Resume
# Batch approve the 2 pending operations
# Drag new tasks to AI agents for today's work
```

### Workflow 2: Complex Task Orchestration

```bash
# Large task needs multiple agents
# In Task Board: Right-click TASK-042 → "Orchestrate"

# System automatically:
# 1. Expands task into subtasks
# 2. Assigns specialized agents
# 3. Shows in Swarm Monitor:
#    - Claude doing architecture
#    - OpenCode implementing
#    - Codex writing tests
# 4. Coordinates handoffs between agents
```

### Workflow 3: Team Collaboration

```bash
# Alice sees Bob's task is blocked
# Click on blocked task → See missing dependency
# Complete dependency task with AI assist
# System automatically:
# - Unblocks Bob's task
# - Notifies Bob
# - Updates both dashboards in real-time
```

---

## 8. Integration with tm-view

The existing `tm-view` can be integrated as a lightweight alternative view:

```bash
# Use tm-view for quick CLI checks
$ tm-view --epic AUTH

# Output integrated into Descartes
$ descartes task-view --format tm-view

# Or embed tm-view as a widget
$ descartes dashboard --widget tm-view
```

---

## 9. Benefits

1. **Complete Visibility**: See all tasks and agents at a glance
2. **Interactive Control**: Pause, resume, attach to any agent
3. **Real-time Updates**: Live status changes as they happen
4. **Team Awareness**: See what humans and AIs are working on
5. **Quick Interventions**: Resolve blocks and errors immediately
6. **Resource Management**: Monitor and control resource usage
7. **Dependency Tracking**: Visualize and manage task relationships

---

## 10. Next Steps

### Phase 1: Basic Task Board (Week 1)
- Implement Kanban view
- Add drag-and-drop
- Connect to SCUD backend

### Phase 2: Swarm Monitor (Week 2)
- Create agent node components
- Add status updates via WebSocket
- Implement attach functionality

### Phase 3: Integration (Week 3)
- Connect both views
- Add real-time synchronization
- Implement control actions

### Phase 4: Polish (Week 4)
- Add animations
- Optimize performance
- Mobile responsiveness
- User preferences

This creates a powerful command center for AI-orchestrated development, giving developers complete visibility and control over both their tasks and their AI swarm.
