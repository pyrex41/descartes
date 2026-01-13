# Advanced Features

*Time-travel, state machines, and beyond*

---

Beyond the core agent functionality, Descartes offers powerful advanced features for debugging, state management, and distributed execution. This guide covers the capabilities that make Descartes suitable for production workloads.

## Time-Travel Debugging

### The Concept

Every agent action is recorded. Time-travel lets you:
- **Replay** past execution
- **Inspect** any moment in history
- **Restore** to previous states
- **Audit** decision-making

### Event Sourcing

All agent activity is captured as events:

```rust
pub enum HistoryEventType {
    Thought,        // Agent reasoning
    Action,         // Operation performed
    ToolUse,        // External tool invocation
    StateChange,    // Status transition
    Communication,  // Message exchange
    Decision,       // Choice made
    Error,          // Failure occurred
    System,         // Lifecycle event
}
```

### Event Structure

```rust
pub struct AgentHistoryEvent {
    pub event_id: Uuid,
    pub agent_id: String,
    pub timestamp: i64,
    pub event_type: HistoryEventType,
    pub event_data: Value,              // Flexible JSON
    pub git_commit_hash: Option<String>, // Code state
    pub session_id: Option<String>,
    pub parent_event_id: Option<Uuid>,  // Causality
    pub tags: Vec<String>,
    pub metadata: Option<Value>,
}
```

### Using Time-Travel

**CLI:**
```bash
# List history events
descartes history a1b2c3

# Show specific event
descartes history a1b2c3 --event evt_123

# Restore to point in time
descartes restore a1b2c3 --to "2025-01-15T10:30:00Z"
```

**GUI:**
- Open the Debugger view
- Scrub the timeline
- Click events to inspect
- Use playback controls

---

## Brain and Body Restoration

Descartes separates agent state into "brain" (memory/context) and "body" (code/files).

### Brain Restore

Reconstruct agent mental state from events:

```rust
pub struct BrainRestore {
    /// Replay events to rebuild state
    pub fn restore_to(&self, timestamp: DateTime<Utc>) -> Result<AgentState>;

    /// Reconstruct thought history
    pub fn rebuild_thoughts(&self) -> Vec<Thought>;

    /// Restore decision tree
    pub fn rebuild_decisions(&self) -> DecisionTree;

    /// Recover conversation context
    pub fn rebuild_context(&self) -> ConversationContext;
}
```

### Body Restore

Git-based code state recovery:

```rust
pub struct BodyRestore {
    /// Safe checkout with backup
    pub fn restore_to_commit(&self, hash: &str) -> Result<()>;

    /// Stash uncommitted changes
    pub fn stash_current(&self) -> Result<StashId>;

    /// Rollback on error
    pub fn rollback(&self) -> Result<()>;

    /// Atomic operations
    pub fn atomic_restore(&self, hash: &str) -> Result<()>;
}
```

### Combined Restoration

```bash
# Restore both brain and body to specific point
descartes restore a1b2c3 --to evt_abc123 --include-code

# This will:
# 1. Backup current state
# 2. Checkout git commit from that event
# 3. Restore agent memory context
# 4. Resume session from that point
```

---

## State Machines

Agent lifecycle is managed by compile-time verified state machines.

### State Definitions

```rust
pub enum AgentStatus {
    Idle,
    Initializing,
    Running,
    Thinking,
    Paused,
    Completed,
    Failed,
    Terminated,
}
```

### Valid Transitions

```
Idle ──────────────────┬──────────────────▶ Terminated
                       │
                       ▼
Initializing ──────────┬──────────────────▶ Failed
                       │
                       ▼
Running ◀──────────────┬──────────────────▶ Paused
     │                 │                        │
     │                 ▼                        │
     ├────────────▶ Thinking ◀─────────────────┤
     │                 │                        │
     │                 ├──────────────────▶ Completed
     │                 │
     │                 ├──────────────────▶ Failed
     │                 │
     │                 └──────────────────▶ Terminated
     │
     └────────────────────────────────────▶ Terminated
```

### Compile-Time Verification

Using the `statig` crate, invalid transitions are caught at compile time:

```rust
#[derive(State)]
pub struct AgentStateMachine;

impl StateMachine for AgentStateMachine {
    type State = AgentStatus;
    type Event = AgentEvent;

    fn transition(state: &Self::State, event: &Self::Event) -> Option<Self::State> {
        match (state, event) {
            (Idle, Start) => Some(Initializing),
            (Running, Pause) => Some(Paused),
            (Paused, Resume) => Some(Running),
            // Invalid transitions return None
            (Completed, _) => None,
            // ...
        }
    }
}
```

---

## DAG (Directed Acyclic Graph) Execution

Task dependencies form a DAG for optimal parallel execution.

### Graph Structure

```rust
pub struct TaskDAG {
    pub nodes: HashMap<TaskId, TaskNode>,
    pub edges: Vec<(TaskId, TaskId)>,
}

pub struct TaskNode {
    pub id: TaskId,
    pub task: Task,
    pub status: TaskStatus,
    pub dependencies: Vec<TaskId>,
    pub position: Option<Position>,  // For visualization
    pub metadata: HashMap<String, Value>,
}
```

### Wave Computation

Tasks are grouped into waves for parallel execution:

```rust
pub fn compute_waves(dag: &TaskDAG) -> Vec<Wave> {
    // Topological sort
    // Group by dependency depth
    // Return execution waves
}
```

Example:
```
Wave 1: [A]           # No dependencies
Wave 2: [B, C]        # Both depend on A
Wave 3: [D, E]        # Depend on B or C
Wave 4: [F]           # Depends on D and E
```

### Cycle Detection

```rust
pub fn detect_cycles(dag: &TaskDAG) -> Result<(), CycleError> {
    // Tarjan's algorithm or DFS-based detection
    // Returns error with cycle path if found
}
```

### Serialization

DAGs can be exported to Swarm.toml format:

```toml
# swarm.toml

[[nodes]]
id = "task-a"
name = "Setup database"
dependencies = []

[[nodes]]
id = "task-b"
name = "Implement API"
dependencies = ["task-a"]

[[nodes]]
id = "task-c"
name = "Write tests"
dependencies = ["task-a"]
```

---

## Distributed Execution with ZeroMQ

For large workloads, agents can run across multiple machines.

### Architecture

```
┌─────────────────┐       ┌─────────────────┐
│  ZMQ Server     │       │   ZMQ Server    │
│  (Machine A)    │       │   (Machine B)   │
│   ├── Agent 1   │       │   ├── Agent 4   │
│   ├── Agent 2   │       │   └── Agent 5   │
│   └── Agent 3   │       │                 │
└────────┬────────┘       └────────┬────────┘
         │                         │
         └─────────┬───────────────┘
                   │
              ┌────▼────┐
              │ Central │
              │ Client  │
              └─────────┘
```

### ZMQ Server

```rust
pub struct ZmqServer {
    /// REP socket for request/response
    rep_socket: Socket,

    /// PUB socket for broadcasting logs
    pub_socket: Socket,

    /// Active agents
    agents: HashMap<Uuid, AgentHandle>,
}
```

### Remote Agent Spawning

```bash
# Start ZMQ server on remote machine
descartes daemon --zmq-bind tcp://0.0.0.0:5555

# Spawn agent on remote
descartes spawn \
  --task "Process batch data" \
  --remote tcp://machine-a:5555
```

### Log Streaming

Agents publish logs to PUB socket:

```rust
// Subscribe to all agent logs
zmq_client.subscribe("logs/*");

// Subscribe to specific agent
zmq_client.subscribe("logs/a1b2c3");
```

### Health Checks

Built-in health monitoring:

```bash
descartes remote health tcp://machine-a:5555
# Agent count: 3
# CPU usage: 45%
# Memory: 2.1 GB / 8 GB
# Status: Healthy
```

---

## Persistent Memory (Thoughts)

Agents can persist knowledge across sessions.

### Storage Structure

```
~/.descartes/thoughts/
├── research/
│   ├── 2025-01-15-auth-patterns.md
│   └── 2025-01-14-api-design.md
└── plans/
    ├── 2025-01-15-feature-x.md
    └── 2025-01-14-refactor.md
```

### Thought Metadata

```rust
pub struct ThoughtMetadata {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
}
```

### Project Symlinks

Projects can access global thoughts:

```
my-project/
└── thoughts -> ~/.descartes/thoughts/
```

### Usage

```bash
# List thoughts
descartes thoughts list

# Search thoughts
descartes thoughts search "authentication"

# Create thought
descartes thoughts create --title "API Design" --tags api,design
```

---

## Distributed File Locking

When multiple agents work on the same codebase:

### Lease Manager

```rust
pub struct LeaseManager {
    /// SQLite-backed persistence
    db: SqlitePool,

    /// Acquire lease on file
    pub async fn acquire(&self, path: &Path, agent_id: &str, ttl: Duration) -> Result<Lease>;

    /// Release lease
    pub async fn release(&self, lease: &Lease) -> Result<()>;

    /// Renew lease
    pub async fn renew(&self, lease: &Lease, ttl: Duration) -> Result<()>;

    /// Check if path is locked
    pub async fn is_locked(&self, path: &Path) -> bool;
}
```

### Lease Semantics

- **TTL-based** — Leases expire automatically
- **Agent-scoped** — Tied to specific agent
- **File-granular** — Lock individual files
- **Queryable** — Find who has locks

### Example

```rust
// Agent A acquires lease
let lease = lease_manager.acquire("src/main.rs", agent_id, Duration::minutes(5)).await?;

// Agent B tries to acquire - blocked
let result = lease_manager.acquire("src/main.rs", other_agent, Duration::minutes(5)).await;
assert!(result.is_err());

// Agent A releases
lease_manager.release(&lease).await?;

// Agent B can now acquire
let lease = lease_manager.acquire("src/main.rs", other_agent, Duration::minutes(5)).await?;
```

---

## Secrets and Encryption

Secure credential management for API keys and sensitive data.

### Encryption

- **Algorithm:** AES-256-GCM
- **Key Derivation:** Argon2id
- **Per-secret salts and nonces**
- **Authentication tags**

### Secret Store

```rust
pub struct SecretStore {
    /// Store a secret
    pub fn store(&self, name: &str, value: &[u8], secret_type: SecretType) -> Result<()>;

    /// Retrieve a secret
    pub fn get(&self, name: &str) -> Result<Secret>;

    /// Rotate a secret
    pub fn rotate(&self, name: &str, new_value: &[u8]) -> Result<()>;

    /// List secrets (metadata only)
    pub fn list(&self) -> Result<Vec<SecretMetadata>>;
}
```

### Access Control

```rust
pub enum AccessLevel {
    None,
    ViewMetadata,  // See name, not value
    Read,          // Decrypt and read
    Update,        // Modify value
    Delete,        // Remove
    Admin,         // Full control
}
```

### Audit Logging

All secret access is logged:

```rust
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub principal_id: String,
    pub action: AuditAction,
    pub secret_name: String,
    pub success: bool,
    pub source_ip: Option<String>,
}
```

---

## Lisp Development Integration

For Common Lisp developers, Descartes integrates with SBCL's Swank protocol.

### Swank Client

```rust
pub struct SwankClient {
    /// Connect to SBCL process
    pub fn connect(&self, port: u16) -> Result<()>;

    /// Evaluate Lisp expression
    pub fn eval(&self, expr: &str, package: Option<&str>) -> Result<Value>;

    /// Compile Lisp code
    pub fn compile(&self, code: &str) -> Result<CompileResult>;

    /// Inspect object
    pub fn inspect(&self, expr: &str) -> Result<InspectResult>;

    /// Invoke debugger restart
    pub fn restart(&self, restart_index: usize) -> Result<()>;
}
```

### Live Development

Agents with `lisp-developer` tool level can:

1. Evaluate expressions in running SBCL
2. Compile code with diagnostics
3. Inspect runtime objects
4. Handle errors with restarts

### GUI Integration

The Lisp Debugger panel appears on errors:

```
┌─────────────────────────────────────────────────────────────┐
│ Lisp Debugger                                               │
├─────────────────────────────────────────────────────────────┤
│ Condition: SIMPLE-ERROR                                     │
│ Message: "Division by zero"                                 │
│ Thread: 0 | Debug Level: 1                                  │
│                                                             │
│ Restarts:                                                   │
│   [0] ABORT - Return to REPL                                │
│   [1] CONTINUE - Use 0 as quotient                          │
│   [2] RETRY - Retry with different arguments                │
│                                                             │
│ Stack:                                                      │
│   0: (/ 10 0)                                               │
│   1: (CALCULATE-RESULT ...)                                 │
│   2: (PROCESS-DATA ...)                                     │
│                                                             │
│ [Invoke Restart 0] [Invoke Restart 1] [Invoke Restart 2]    │
└─────────────────────────────────────────────────────────────┘
```

---

## Expression Evaluation

Agents can evaluate expressions for dynamic behavior.

### Evaluator

```rust
pub struct ExpressionEvaluator {
    /// Evaluate expression with context
    pub fn eval(&self, expr: &str, context: &Context) -> Result<Value>;
}
```

### Supported Expressions

```
// Variable access
${task.status}
${agent.name}

// Conditionals
${if task.completed then "done" else "pending"}

// Comparisons
${task.priority > 5}

// String operations
${task.title | uppercase}
${task.description | truncate(100)}
```

### Use Cases

- Dynamic task descriptions
- Conditional workflows
- Template rendering
- Configuration interpolation

---

## Debugging Tools

### Debugger Integration

```rust
pub struct Debugger {
    /// Set breakpoint
    pub fn breakpoint(&self, condition: &str) -> BreakpointId;

    /// Inspect agent state
    pub fn inspect(&self, agent_id: &str) -> AgentSnapshot;

    /// Step through execution
    pub fn step(&self, agent_id: &str) -> StepResult;

    /// Continue execution
    pub fn continue_(&self, agent_id: &str) -> Result<()>;
}
```

### Conditional Breakpoints

```bash
# Break when agent reads specific file
descartes debug a1b2c3 --break "tool.name == 'read' && tool.args.path contains 'secret'"

# Break on error
descartes debug a1b2c3 --break "event.type == 'error'"
```

---

## Configuration Migration

Automatic config file upgrades:

```rust
pub struct ConfigMigration {
    /// Check if migration needed
    pub fn needs_migration(&self, config: &Config) -> bool;

    /// Migrate to latest version
    pub fn migrate(&self, config: Config) -> Result<Config>;

    /// Backup before migration
    pub fn backup(&self, path: &Path) -> Result<PathBuf>;
}
```

### Version History

```toml
# v1.0.0 - Initial format
# v1.1.0 - Added provider.grok
# v1.2.0 - Renamed api_key fields
# v2.0.0 - Restructured providers section
```

---

## Performance Tuning

### Configuration Options

```toml
[performance]
# Agent concurrency
max_concurrent_agents = 5

# Request pooling
connection_pool_size = 10
connection_timeout_secs = 30

# Caching
enable_response_cache = true
cache_ttl_secs = 300

# Streaming
stream_buffer_size = 65536
stream_chunk_size = 4096
```

### Metrics

```bash
# Enable Prometheus metrics
descartes daemon --metrics-port 9090

# Available metrics:
# descartes_agents_active
# descartes_requests_total
# descartes_request_duration_seconds
# descartes_tokens_used_total
```

---

## Next Steps

You've now explored the full depth of Descartes' capabilities:

- **[Getting Started →](02-getting-started.md)** — Begin your journey
- **[Flow Workflow →](07-flow-workflow.md)** — Production automation
- **[GUI Features →](09-gui-features.md)** — Visual monitoring

---

*From simple tasks to enterprise-scale orchestration—Descartes scales with you.*
