# Implementation Plan: Blog Documentation Corrections

## Overview

Correct inaccuracies in the Descartes blog documentation series based on codebase verification. The blog posts contain several discrepancies between documented features and actual implementation that could confuse users.

## Current State Analysis

The blog series (12 posts) was created to document Descartes features comprehensively. However, several inaccuracies have been identified:

### Key Discoveries:

1. **CLI Flags (Blog 03)** - Multiple flags documented that don't exist in the code
   - File: `/Users/reuben/gauntlet/cap/descartes/cli/src/main.rs:42-191`

2. **Provider Support (Blog 04)** - DeepSeek and Groq listed as supported but have no implementations
   - File: `/Users/reuben/gauntlet/cap/descartes/core/src/providers.rs:1126-1196`

3. **Session States (Blog 05)** - Conflates SessionStatus (6 states) with AgentStatus (8 states)
   - Files: `/Users/reuben/gauntlet/cap/descartes/core/src/session.rs:82-96` and `/Users/reuben/gauntlet/cap/descartes/core/src/agent_state.rs:73-98`

4. **State Machine Claims (Blog 11)** - Claims statig crate usage but it's not in dependencies
   - Manual state machine at: `/Users/reuben/gauntlet/cap/descartes/core/src/state_machine.rs`

5. **GUI Features (Blog 09)** - Several features are placeholders or not connected
   - File: `/Users/reuben/gauntlet/cap/descartes/gui/src/main.rs`

6. **Flow Workflow (Blog 07)** - Wave execution relies on external tools, not fully implemented
   - Implementation stubs in flow executor

## Desired End State

All blog posts accurately reflect the current implementation state. Users following the documentation will not encounter unexpected errors or missing features. Where features are partially implemented, the documentation clearly states limitations.

### Verification:
- Users can copy CLI examples and have them work
- Provider list matches working implementations only
- Status enums match actual code definitions
- Feature claims match implementation reality

## What We're NOT Doing

- Adding new features to match documentation (documentation should match code, not vice versa)
- Removing posts about future/planned features (just marking them clearly as "planned")
- Changing the overall structure of the blog series
- Adding implementation details that don't exist

---

## Phase 1: Fix CLI Commands Reference (03-cli-commands.md)

### Overview
Correct the CLI flag documentation to match actual clap definitions.

### Changes Required:

#### 1. spawn Command Options Table (lines 44-57)
**File**: `docs/blog/03-cli-commands.md`

**Current (incorrect):**
```markdown
| `--output` | `-o` | Custom transcript output path | Auto-generated |
| `--agent` | `-a` | Agent definition file | None |
| `--context` | `-c` | Additional context file | None |
| `--attachable` | | Create attach socket for TUI | `false` |
```

**Correct to:**
```markdown
| `--system` | `-s` | System prompt/context | None |
| `--transcript-dir` | | Custom transcript directory | `.scud/sessions` |
```

Remove: `--output`, `--agent`, `--context`, `--attachable` (these don't exist)

#### 2. spawn Examples (lines 59-81)
Remove examples using non-existent flags:
- Remove: `--agent ~/.descartes/agents/architect.md`
- Remove: `--attachable`

#### 3. logs Command Options Table (lines 146-152)
**Current (incorrect):**
```markdown
| `--tail` | | Show last N entries |
```

**Correct to:**
```markdown
| `--limit` | `-l` | Number of entries to show | `100` |
```

Note: `--tail` does not exist, only `--limit`

#### 4. logs Examples (lines 154-167)
**Current (incorrect):**
```bash
descartes logs a1b2c3 --tail 10
```

**Correct to:**
```bash
descartes logs a1b2c3 --limit 10
```

#### 5. kill Command Options Table (lines 181-187)
**Current (incorrect):**
```markdown
| `--all` | Kill all running agents |
```

**Remove this row** - `--all` flag does not exist

#### 6. kill Examples (lines 189-199)
**Remove:**
```bash
# Kill all agents
descartes kill --all
```

#### 7. resume Command Options Table (lines 241-246)
**Current (incorrect):**
```markdown
| `--attach` | Resume and attach TUI immediately |
```

**Remove this row** - `--attach` flag does not exist

#### 8. resume Examples (lines 248-255)
**Remove:**
```bash
# Resume with TUI attachment
descartes resume a1b2c3 --attach
```

#### 9. attach Command Options Table (lines 269-276)
**Current (incorrect):**
```markdown
| `--tui` | TUI type: `claude`, `opencode`, or `custom` |
| `--command` | Custom command for TUI |
```

**Correct to:**
```markdown
| `--client` | `-c` | Client type: `claude-code`, `opencode`, or `custom` | `claude-code` |
| `--launch` | `-l` | Launch the TUI client after obtaining credentials | `false` |
| `--json` | | Output JSON for scripting | `false` |
```

#### 10. attach Examples (lines 278-287)
**Current (incorrect):**
```bash
descartes attach a1b2c3 --tui claude
descartes attach a1b2c3 --tui opencode
descartes attach a1b2c3 --tui custom --command "my-tui --session"
```

**Correct to:**
```bash
descartes attach a1b2c3 --client claude-code
descartes attach a1b2c3 --client opencode --launch
descartes attach a1b2c3 --json  # Get credentials as JSON
```

### Success Criteria:

#### Automated Verification:
- [ ] File parses as valid markdown
- [ ] No references to `--output`, `--agent`, `--context`, `--attachable`, `--tail`, `--all` (for kill), `--attach` (for resume), `--tui`

#### Manual Verification:
- [ ] Run `descartes spawn --help` and verify documented flags match
- [ ] Run `descartes logs --help` and verify documented flags match
- [ ] Run `descartes kill --help` and verify documented flags match
- [ ] Run `descartes attach --help` and verify documented flags match

---

## Phase 2: Fix Providers Documentation (04-providers-configuration.md)

### Overview
Remove DeepSeek and Groq from supported providers list and clarify actual support.

### Changes Required:

#### 1. Supported Providers Table (lines 10-18)
**File**: `docs/blog/04-providers-configuration.md`

**Remove these rows:**
```markdown
| **DeepSeek** | Cloud API | DeepSeek Coder | Code generation |
| **Groq** | Cloud API | Various | Ultra-fast inference |
```

**Add note after table:**
```markdown
> **Note:** DeepSeek and Groq configuration structures exist but provider implementations are not yet complete. These will be supported in a future release.
```

#### 2. Environment Variables Section (lines 209-220)
**Remove:**
```markdown
| `DEEPSEEK_API_KEY` | DeepSeek |
| `GROQ_API_KEY` | Groq |
```

#### 3. Remove from CLI Environment Variables (lines 665-672)
**Remove references to:**
```markdown
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `GROQ_API_KEY` | Groq API key |
```

### Success Criteria:

#### Automated Verification:
- [ ] Grep for "DeepSeek" returns only the "future release" note
- [ ] Grep for "Groq" returns only the "future release" note (not "Grok")

#### Manual Verification:
- [ ] Verify that listed providers (Anthropic, OpenAI, Grok, Ollama) all work with `descartes spawn`

---

## Phase 3: Fix Session Management Documentation (05-session-management.md)

### Overview
Correct the session status enumeration to accurately reflect SessionStatus vs AgentStatus.

### Changes Required:

#### 1. Session Lifecycle Diagram (lines 36-51)
**File**: `docs/blog/05-session-management.md`

**Replace with accurate SessionStatus flow:**
```markdown
### Session Lifecycle

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Inactive │────▶│ Starting │────▶│  Active  │
└──────────┘     └──────────┘     └────┬─────┘
                                       │
                      ┌────────────────┼────────────────┐
                      ▼                ▼                ▼
                ┌──────────┐    ┌──────────┐     ┌──────────┐
                │ Stopping │    │ Archived │     │  Error   │
                └──────────┘    └──────────┘     └──────────┘
```

### SessionStatus Values

| Status | Description |
|--------|-------------|
| **Inactive** | Session exists but daemon not running |
| **Starting** | Daemon is starting up |
| **Active** | Daemon is running and connected |
| **Stopping** | Daemon is stopping |
| **Archived** | Session has been archived |
| **Error** | Session has errors |
```

#### 2. Add Separate AgentStatus Section (after line 66)
**Add new section:**
```markdown
### AgentStatus Values

When monitoring individual agents within a session, agents have their own status:

| Status | Description |
|--------|-------------|
| **Idle** | Agent created but not started |
| **Initializing** | Agent loading context and environment |
| **Running** | Agent actively executing tasks |
| **Thinking** | Agent processing/generating response |
| **Paused** | Agent suspended, can be resumed |
| **Completed** | Agent finished successfully |
| **Failed** | Agent encountered unrecoverable error |
| **Terminated** | Agent was manually killed |

> **Note:** SessionStatus tracks the daemon/workspace state, while AgentStatus tracks individual agent lifecycle within that session.
```

#### 3. ps Status Values Table (lines 123-131)
**Update to clarify these are AgentStatus values:**
```markdown
### Agent Status Values (from `descartes ps`)

| Status | Description |
|--------|-------------|
| `idle` | Created but not started |
| `initializing` | Loading context |
| `running` | Actively executing |
| `thinking` | Processing/generating response |
| `paused` | Suspended, can be resumed |
| `completed` | Finished successfully |
| `failed` | Encountered error |
| `terminated` | Manually killed |
```

### Success Criteria:

#### Automated Verification:
- [ ] SessionStatus section lists exactly 6 states
- [ ] AgentStatus section lists exactly 8 states
- [ ] Clear distinction between Session and Agent status

#### Manual Verification:
- [ ] Compare with `/Users/reuben/gauntlet/cap/descartes/core/src/session.rs:82-96`
- [ ] Compare with `/Users/reuben/gauntlet/cap/descartes/core/src/agent_state.rs:73-98`

---

## Phase 4: Fix Advanced Features Documentation (11-advanced-features.md)

### Overview
Correct state machine claims - statig is NOT used, state transitions are validated at runtime.

### Changes Required:

#### 1. State Machines Section (lines 133-199)
**File**: `docs/blog/11-advanced-features.md`

**Replace the "Compile-Time Verification" subsection (lines 176-199):**

**Current (incorrect):**
```markdown
### Compile-Time Verification

Using the `statig` crate, invalid transitions are caught at compile time:

```rust
#[derive(State)]
pub struct AgentStateMachine;
...
```

**Correct to:**
```markdown
### Runtime Transition Validation

State transitions are validated at runtime using the `can_transition_to()` method:

```rust
impl AgentStatus {
    pub fn can_transition_to(&self, target: AgentStatus) -> bool {
        match (self, target) {
            // From Idle
            (AgentStatus::Idle, AgentStatus::Initializing) => true,
            (AgentStatus::Idle, AgentStatus::Terminated) => true,

            // From Running
            (AgentStatus::Running, AgentStatus::Thinking) => true,
            (AgentStatus::Running, AgentStatus::Paused) => true,
            (AgentStatus::Running, AgentStatus::Completed) => true,
            (AgentStatus::Running, AgentStatus::Failed) => true,
            (AgentStatus::Running, AgentStatus::Terminated) => true,

            // Terminal states - no transitions out
            (AgentStatus::Completed, _) => false,
            (AgentStatus::Failed, _) => false,
            (AgentStatus::Terminated, _) => false,

            // Self transitions allowed
            (a, b) if a == &b => true,

            _ => false,
        }
    }
}
```

> **Note:** While documentation may reference the `statig` crate for compile-time state machine verification, the current implementation uses runtime validation. Invalid transitions return `false` and are rejected by the state manager.
```

### Success Criteria:

#### Automated Verification:
- [ ] No claims of "compile-time verified" state machines
- [ ] `statig` mentioned only in context of "not currently used"

#### Manual Verification:
- [ ] Verify statig is not in Cargo.toml dependencies
- [ ] Verify runtime validation matches documented code

---

## Phase 5: Fix GUI Features Documentation (09-gui-features.md)

### Overview
Add accuracy notes about implementation status of GUI features.

### Changes Required:

#### 1. Agents View Section (lines 145-189)
**File**: `docs/blog/09-gui-features.md`

**Add implementation note after the Agent Cards example (after line 167):**
```markdown
> **Implementation Note:** The Swarm Monitor displays session-level status and daemon connection information. Real-time CPU, memory, and token metrics shown in examples are planned features not yet implemented. Current implementation shows:
> - Agent status and task
> - Progress percentage
> - Thinking state animation
> - Basic controls (pause/resume/kill)
```

**Update the Metrics line in the example (line 165-166):**
```markdown
│ Progress: ████████████░░░░░░░░ 60%                         │
│                                                             │
│ Current: Analyzing middleware structure...                  │
```

Remove the metrics line:
```markdown
│   CPU: 12%  |  Memory: 245 MB  |  Tokens: 15,234           │
```

#### 2. DAG Editor Section (lines 192-275)
**Add implementation note at the start (after line 192):**
```markdown
> **Implementation Note:** The DAG Editor provides visualization and editing capabilities for task dependency graphs. Workflow execution from the DAG Editor is not yet implemented - use `descartes workflow flow` CLI commands to execute workflows.
```

#### 3. WebSocket/Streaming Section (lines 426-459)
**Update the "Connecting to Daemon" section (lines 426-459):**
```markdown
### ZeroMQ (Primary Streaming)

For high-throughput streaming (fully implemented):
- Chat output streaming
- Log streaming

### WebSocket

For real-time events (infrastructure exists, not fully connected):
- Agent status changes
- New agent spawns
- Error notifications

> **Implementation Note:** ZeroMQ streaming for chat is fully operational. WebSocket event streaming infrastructure exists but is not yet fully connected to the UI event loop.
```

### Success Criteria:

#### Automated Verification:
- [ ] Implementation notes added to Agents View, DAG Editor, and WebSocket sections
- [ ] CPU/Memory/Token metrics line removed from example

#### Manual Verification:
- [ ] Launch `descartes gui` and verify documented features vs actual

---

## Phase 6: Fix Flow Workflow Documentation (07-flow-workflow.md)

### Overview
Clarify that wave execution relies on SCUD CLI and parallel task execution is not fully implemented.

### Changes Required:

#### 1. Phase 4: Implement Section (lines 169-214)
**File**: `docs/blog/07-flow-workflow.md`

**Add implementation note after "Wave Execution" (after line 194):**
```markdown
> **Implementation Note:** Wave computation uses the `scud waves` command from the SCUD task management system. The wave groupings shown are computed by analyzing task dependencies. Currently, tasks within a wave are executed sequentially by the orchestrator agent; true parallel execution within waves is planned for a future release.
```

#### 2. SCUD Integration Section (lines 603-661)
**Update to clarify implementation status:**

After line 612, add:
```markdown
> **Note:** The Flow workflow currently executes tasks through a single orchestrator agent that processes waves sequentially. The `scud waves` command computes optimal groupings, but spawning multiple sub-agents for parallel wave execution is not yet implemented.
```

### Success Criteria:

#### Automated Verification:
- [ ] Implementation notes added about sequential vs parallel execution
- [ ] SCUD dependency clearly stated

#### Manual Verification:
- [ ] Verify `scud waves` command exists and works

---

## Testing Strategy

### Automated Tests:
1. Markdown linting on all modified files
2. Grep verification that incorrect flags/features are removed
3. Link checking within blog series

### Manual Testing Steps:
1. Run each CLI command with `--help` and compare to documentation
2. Test spawning agents with each documented provider
3. Launch GUI and verify feature presence vs documentation claims
4. Run `descartes doctor` to verify system status

---

## Summary of Changes by File

| File | Changes |
|------|---------|
| `03-cli-commands.md` | Fix 10 incorrect CLI flags across spawn, logs, kill, resume, attach |
| `04-providers-configuration.md` | Remove DeepSeek/Groq from supported list, add future note |
| `05-session-management.md` | Separate SessionStatus (6) from AgentStatus (8), fix diagram |
| `07-flow-workflow.md` | Add notes about SCUD dependency and sequential execution |
| `09-gui-features.md` | Add implementation notes for metrics, DAG execution, WebSocket |
| `11-advanced-features.md` | Fix state machine claims (runtime, not compile-time) |

---

## References

- CLI definitions: `/Users/reuben/gauntlet/cap/descartes/cli/src/main.rs:42-191`
- Provider factory: `/Users/reuben/gauntlet/cap/descartes/core/src/providers.rs:1126-1196`
- SessionStatus enum: `/Users/reuben/gauntlet/cap/descartes/core/src/session.rs:82-96`
- AgentStatus enum: `/Users/reuben/gauntlet/cap/descartes/core/src/agent_state.rs:73-98`
- State machine: `/Users/reuben/gauntlet/cap/descartes/core/src/state_machine.rs`
- GUI main: `/Users/reuben/gauntlet/cap/descartes/gui/src/main.rs`
- Swarm monitor: `/Users/reuben/gauntlet/cap/descartes/gui/src/swarm_monitor.rs`
