# The Descartes GUI

*Visual control and monitoring for your AI agents*

---

While Descartes shines on the command line, the native GUI provides powerful visualization and control capabilities. Built with the Iced framework, it offers real-time monitoring, visual workflow editing, and time-travel debugging.

## Launching the GUI

```bash
descartes gui
```

The GUI connects to the Descartes daemon, providing a unified view of all agent activity.

---

## The Interface

### Main Layout

```
┌────────────────────────────────────────────────────────────────┐
│ [◆] DESCARTES Agent Orchestration    [●] Connected [Disconnect]│
├────────┬───────────────────────────────────────────────────────┤
│        │                                                        │
│ ◆ Sess │   Main Content Area                                   │
│ ⌂ Dash │   (Changes based on selected view)                    │
│ ✉ Chat │                                                        │
│ ◎ Agents│                                                        │
│ ⏱ Debug│                                                        │
│        │                                                        │
├────────┴───────────────────────────────────────────────────────┤
│ Status: Connected to daemon successfully!                       │
└────────────────────────────────────────────────────────────────┘
```

### Navigation

| Icon | View | Purpose |
|------|------|---------|
| ◆ | Sessions | Workspace/session selection |
| ⌂ | Dashboard | Overview and statistics |
| ✉ | Chat | Interactive conversation |
| ◎ | Agents | Real-time agent monitoring |
| ⏱ | Debugger | Time-travel debugging |

---

## Dashboard View

The dashboard provides at-a-glance status:

### Stat Cards

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ 3 Active    │  │ 12 Tasks    │  │ 47 Events   │
│   Agents    │  │   Pending   │  │   Today     │
└─────────────┘  └─────────────┘  └─────────────┘
```

### Connection Status

```
┌─────────────────────────────────────────────┐
│ Connection Status                           │
├─────────────────────────────────────────────┤
│ Status: Connected ●                         │
│ Endpoint: http://localhost:8080             │
│ WebSocket: ws://localhost:8080/events       │
└─────────────────────────────────────────────┘
```

### Recent Events

Real-time feed of agent activity:

```
┌─────────────────────────────────────────────┐
│ Recent Events                               │
├─────────────────────────────────────────────┤
│ ⚡ Agent a1b2c3 started task                │
│ 🔧 Tool call: read src/main.rs              │
│ 💭 Agent thinking: "Analyzing structure..." │
│ ✓ Task completed successfully               │
│ 🚀 New agent spawned: d4e5f6                │
└─────────────────────────────────────────────┘
```

---

## Chat View

Interactive conversation with streaming support.

### Features

- **Full-session integration** with daemon backend
- **Real-time streaming** via ZeroMQ
- **Thinking block visualization** (purple/blue styling)
- **Sub-agent tracking** with badges

### Interface

```
┌─────────────────────────────────────────────────────────────┐
│                        Chat Session                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ You: Implement JWT authentication for the API               │
│                                                              │
│ ┌─ Claude ──────────────────────────────────────────────┐   │
│ │ I'll help you implement JWT authentication. Let me    │   │
│ │ first analyze your current codebase structure...      │   │
│ │                                                        │   │
│ │ 🔧 read src/api/auth.ts                               │   │
│ │ 🔧 read src/middleware/index.ts                       │   │
│ │                                                        │   │
│ │ 💭 Thinking: The current auth uses sessions...        │   │
│ └────────────────────────────────────────────────────────┘   │
│                                                              │
│ ┌─ Sub-Agent: explore-abc ──────────────────────────────┐   │
│ │ Type: Explore                                          │   │
│ │ Task: "Search for JWT patterns in codebase"           │   │
│ │ Status: Running                                        │   │
│ └────────────────────────────────────────────────────────┘   │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ [Enter message...]                              [Send] [⚡]  │
└─────────────────────────────────────────────────────────────┘
```

### Sub-Agent Display

When agents spawn sub-agents, they appear with:
- **Agent ID** and type badge
- **Task/prompt** preview
- **Status** indicator
- **Spawned timestamp**

---

## Agents View (Swarm Monitor)

Real-time monitoring of all running agents.

### Agent Cards

```
┌─────────────────────────────────────────────────────────────┐
│ Agent: a1b2c3                                    [Active ●] │
├─────────────────────────────────────────────────────────────┤
│ Task: Implement JWT authentication                          │
│ Provider: anthropic | Model: claude-3-5-sonnet              │
│ Started: 5 minutes ago                                      │
│                                                             │
│ Progress: ████████████░░░░░░░░ 60%                         │
│                                                             │
│ Current: Analyzing middleware structure...                  │
│ 💭 Thinking: "I need to check the existing auth..."        │
│                                                             │
│ Metrics:                                                    │
│   CPU: 12%  |  Memory: 245 MB  |  Tokens: 15,234           │
└─────────────────────────────────────────────────────────────┘
```

### Features

- **Live status updates** at 60 FPS
- **Thinking state animation** (pulsing indicator)
- **Performance metrics** (CPU, memory)
- **Progress tracking** with visual bars
- **Status filtering** (Active, Idle, Paused, Error)
- **Search** by agent name/task
- **Grouping** by type or status

### Status Colors

| Status | Color | Indicator |
|--------|-------|-----------|
| Active | Cyan | ● |
| Thinking | Purple | 💭 (animated) |
| Idle | Gray | ○ |
| Paused | Yellow | ◐ |
| Error | Red | ✕ |
| Completed | Green | ✓ |

---

## DAG Editor

Visual workflow designer for task dependencies.

### Canvas Interface

```
┌─────────────────────────────────────────────────────────────┐
│ Tools: [Select] [Add Node] [Add Edge] [Delete] [Pan]        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│         ┌─────────┐                                          │
│         │ TASK-01 │                                          │
│         │ Setup   │                                          │
│         └────┬────┘                                          │
│              │                                               │
│      ┌───────┴───────┐                                       │
│      ▼               ▼                                       │
│ ┌─────────┐    ┌─────────┐                                   │
│ │ TASK-02 │    │ TASK-03 │                                   │
│ │ API     │    │ Frontend│                                   │
│ └────┬────┘    └────┬────┘                                   │
│      │              │                                        │
│      └──────┬───────┘                                        │
│             ▼                                                │
│       ┌─────────┐                                            │
│       │ TASK-04 │                                            │
│       │ Testing │                                            │
│       └─────────┘                                            │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ Zoom: 100% | Nodes: 4 | Edges: 4                            │
└─────────────────────────────────────────────────────────────┘
```

### Features

- **Drag-and-drop nodes** with smooth positioning
- **Edge creation** by dragging from node to node
- **Cycle detection** prevents invalid graphs
- **Multi-select** with Ctrl+click or box select
- **Pan and zoom** (mouse wheel, Space+drag)
- **Snap to grid** for alignment
- **Undo/Redo** with full history

### Node Properties

Click a node to edit:

```
┌─────────────────────────────────┐
│ Node Properties                 │
├─────────────────────────────────┤
│ ID: TASK-02                     │
│ Title: [Implement API        ]  │
│ Description: [Create REST...  ] │
│ Status: ○ Pending ● Running     │
│ Tags: [api, backend]            │
│                    [Save] [Del] │
└─────────────────────────────────┘
```

### Edge Types

| Type | Style | Purpose |
|------|-------|---------|
| Dependency | Solid → | Must complete before |
| Data Flow | Dashed → | Passes data |
| Trigger | Dotted → | Triggers on event |
| Soft | Light → | Optional dependency |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Arrow keys | Navigate nodes |
| Ctrl+A | Select all |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Delete | Remove selected |
| Space+Drag | Pan canvas |
| +/- | Zoom in/out |

---

## Time-Travel Debugger

Replay and inspect agent execution history.

### Timeline View

```
┌─────────────────────────────────────────────────────────────┐
│ Time Travel Debugger                  [▶ Play] [Speed: 1x]  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Timeline:                                                    │
│ ─────●─────●─────●─────●─────●─────◆─────●─────●─────       │
│      │     │     │     │     │     │     │     │            │
│      💭    ⚡    🔧    💭    🔧    ▶     ⚡    ✓            │
│                                                              │
│ Event Details:                                               │
├─────────────────────────────────────────────────────────────┤
│ Event Type: Tool Use                                         │
│ Timestamp: 2025-01-15T10:32:15Z                             │
│ Tool: read                                                   │
│ Arguments: {"path": "src/auth.ts"}                          │
│                                                              │
│ Result:                                                      │
│ ┌───────────────────────────────────────────────────────┐   │
│ │ // auth.ts                                             │   │
│ │ export function authenticate(req, res, next) {         │   │
│ │   const token = req.headers.authorization;             │   │
│ │   ...                                                  │   │
│ │ }                                                      │   │
│ └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Event Types

| Icon | Type | Description |
|------|------|-------------|
| 💭 | Thought | Agent reasoning |
| ⚡ | Action | Agent taking action |
| 🔧 | Tool Use | Tool invocation |
| 📝 | State Change | Status transition |
| 💬 | Communication | Message sent/received |
| ❌ | Error | Failure occurred |
| 🚀 | System | Lifecycle event |

### Playback Controls

```
[◀◀] [◀] [▶/❚❚] [▶] [▶▶]   Speed: [0.5x] [1x] [2x] [5x]   [🔁 Loop]
```

- **Step backward/forward** through events
- **Play/Pause** automatic playback
- **Speed control** for fast review
- **Loop** for repeated viewing

### Snapshot Navigation

Jump to specific moments:

```
Snapshots:
├─ Start of session
├─ After file read
├─ Before edit
├─ After edit (current ▶)
├─ Tool error
└─ Completion
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| ← / → | Previous/next event |
| Space | Play/pause |
| +/- | Zoom timeline |
| 1-4 | Speed presets |
| L | Toggle loop |

---

## Sessions View

Manage workspaces and sessions.

### Session List

```
┌─────────────────────────────────────────────────────────────┐
│ Sessions                                      [+ New Session]│
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ 📁 my-project                                    [Active ●]  │
│    Path: /home/user/my-project                               │
│    Last accessed: 5 minutes ago                              │
│    Active agents: 2                                          │
│                                                              │
│ 📁 api-service                                   [Inactive]  │
│    Path: /home/user/api-service                              │
│    Last accessed: 2 hours ago                                │
│                                                              │
│ 📁 frontend-app                                  [Archived]  │
│    Path: /home/user/frontend-app                             │
│    Archived: 2025-01-10                                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Features

- **Create new sessions** for projects
- **Switch between** active sessions
- **Archive** old sessions
- **Search and filter** by name/status
- **View session history** and transcripts

---

## Theme

The GUI uses a "space-age hacker" aesthetic:

### Color Palette

| Element | Color | Hex |
|---------|-------|-----|
| Background | Deep black | #050508 |
| Surface | Dark gray | #0e0f13 |
| Primary | Neon cyan | #00e6e6 |
| Success | Neon green | #00ff80 |
| Warning | Amber | #ffcc00 |
| Error | Neon red | #ff334d |
| Text | Terminal green-white | #d9f2e6 |

### Typography

**Font:** JetBrains Mono (monospace throughout)

### Design Elements

- Sharp corners (2px border radius)
- Subtle borders with cyan tint
- Glow effects on interactive elements
- Terminal-inspired layout

---

## Connecting to Daemon

The GUI communicates with the Descartes daemon via:

### HTTP JSON-RPC

For request/response operations:
- Agent control (pause, resume, kill)
- Session management
- Configuration updates

### WebSocket

For real-time events:
- Agent status changes
- New agent spawns
- Error notifications

### ZeroMQ

For high-throughput streaming:
- Chat output streaming
- Log streaming
- Metrics updates

### Connection Status

The header shows connection state:

```
[●] Connected      # All systems operational
[◐] Reconnecting   # Lost connection, retrying
[○] Disconnected   # Not connected
```

---

## Keyboard Navigation

Global shortcuts:

| Shortcut | Action |
|----------|--------|
| Ctrl+1 | Go to Sessions |
| Ctrl+2 | Go to Dashboard |
| Ctrl+3 | Go to Chat |
| Ctrl+4 | Go to Agents |
| Ctrl+5 | Go to Debugger |
| Ctrl+R | Refresh data |
| Ctrl+Q | Quit |

---

## Next Steps

- **[Sub-Agent Tracking →](10-subagent-tracking.md)** — Monitor agent hierarchies
- **[Advanced Features →](11-advanced-features.md)** — Time-travel and restoration

---

*See your AI agents at work with the power of visual monitoring.*
