# Blog Documentation Corrections Implementation Plan

## Overview

Fix discrepancies between Descartes blog documentation and actual code implementation, as identified in the validation research (`thoughts/shared/research/2026-01-03-blog-documentation-validation.md`).

## Current State Analysis

The blog documentation is ~85% accurate but contains several discrepancies where documentation describes features that are not implemented, partially implemented, or implemented differently than documented.

### Key Discoveries:
- `descartes/core/src/tools/registry.rs:33-65` - Tool level mappings show LispDeveloper has only `read` and `bash`, not `write`/`edit`
- `descartes/core/src/tools/executors.rs:114-183` - No path restrictions enforced for Planner writes
- `descartes/core/src/tools/executors.rs:291-362` - No bash command restrictions enforced
- `descartes/cli/src/commands/workflow.rs:85-106` - Only `--prd`, `--tag`, `--resume`, `--dir`, `--adapter` flags exist

## Desired End State

All blog documentation accurately reflects the current implementation:
1. No references to non-existent CLI flags
2. Tool level descriptions match actual tool availability
3. Restrictions clearly marked as "prompt-based" vs "code-enforced"
4. External dependencies (SCUD) clearly distinguished from Descartes features

### Verification:
- Manual review of each corrected section against source code
- No broken internal links between documentation files

## What We're NOT Doing

- Implementing the missing features (that's a separate task)
- Changing the code to match documentation
- Updating non-blog documentation (e.g., API docs, code comments)
- Rewriting entire documents - only surgical corrections

## Implementation Approach

Make minimal, targeted edits to correct specific inaccuracies while preserving the overall structure and style of each document.

---

## Phase 1: Fix Flow Workflow (07-flow-workflow.md)

### Overview
Remove references to non-existent CLI flags and clarify SCUD as an external dependency.

### Changes Required:

#### 1.1 Remove Non-Existent CLI Flags

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 348-351

**Current** (incorrect):
```markdown
### Start from Specific Phase

```bash
# Skip to implementation (tasks already planned)
descartes workflow flow --prd requirements.md --phase implement
```
```

**Change to**:
```markdown
### Start from Specific Phase

> **Note:** The `--phase` flag is planned but not yet implemented. Currently, use `--resume` to continue from saved state.

```bash
# Resume from where you left off
descartes workflow flow --prd requirements.md --resume
```
```

#### 1.2 Remove --stop Flag Reference

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 534-543

**Current** (incorrect):
```markdown
### 2. Review Before Implement

```bash
# Run just planning phases
descartes workflow flow --prd requirements.md --phase plan --stop

# Review generated plans
ls thoughts/shared/plans/

# If satisfied, continue
descartes workflow flow --prd requirements.md --resume
```
```

**Change to**:
```markdown
### 2. Review Before Implement

```bash
# Start the workflow - it saves state after each phase
descartes workflow flow --prd requirements.md

# If you need to pause, use Ctrl+C - state is preserved
# Review generated plans
ls thoughts/shared/plans/

# Resume from saved state
descartes workflow flow --prd requirements.md --resume
```
```

#### 1.3 Fix Rollback Command

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 204-208

**Current** (incorrect):
```markdown
Checkpoints enable rollback:
```bash
# Rollback to after Wave 1
descartes workflow rollback --phase implement --wave 1
```
```

**Change to**:
```markdown
Checkpoints enable rollback via git:
```bash
# Rollback to after Wave 1 using git
git log --oneline  # Find the wave commit
git reset --hard <commit-hash>
```

> **Note:** A dedicated `--rollback` CLI flag is planned for future releases.
```

#### 1.4 Clarify SCUD as External Dependency

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 601-641

**Current** (misleading - implies SCUD commands are part of Descartes):
```markdown
## SCUD Integration

The Flow Workflow integrates deeply with SCUD (the task management system) for tracking and executing tasks.

### Task Management

Flow uses SCUD commands throughout execution:
- `scud parse-prd` - Generate tasks from PRD (Ingest phase)
...
```

**Change to**:
```markdown
## SCUD Integration

The Flow Workflow can integrate with SCUD, an external task management system, for enhanced tracking and execution.

> **Note:** SCUD is a separate tool with its own CLI. The commands below are SCUD commands, not Descartes commands. If you don't have SCUD installed, Flow uses its own internal task representation.

### Task Management (with SCUD)

When SCUD is available, Flow can use these SCUD commands:
- `scud parse-prd` - Generate tasks from PRD (Ingest phase)
...
```

#### 1.5 Clarify Wave Execution is Sequential

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 172-180

**Current** (incorrect):
```markdown
### What Happens

1. Loads task waves from Phase 2
2. For each wave, spawns implementation agents
3. Tasks in same wave run in parallel
4. Waits for wave completion before next wave
5. Commits changes after each wave
```

**Change to**:
```markdown
### What Happens

1. Loads task waves from Phase 2
2. For each wave, spawns implementation agents
3. Tasks execute sequentially within each wave
4. Waits for wave completion before next wave
5. Commits changes after each wave

> **Note:** Parallel execution within waves is planned for a future release. Currently, tasks execute one at a time.
```

#### 1.6 Fix Flow Config TOML Reference

**File**: `descartes/docs/blog/07-flow-workflow.md`
**Lines**: 355-385

**Current** (non-existent feature):
```markdown
## Configuration

### Flow Config File

```toml
# .scud/flow-config.toml
...
```
```

**Change to**:
```markdown
## Configuration

### Flow Configuration

Flow currently uses sensible defaults. Custom configuration via TOML is planned for a future release.

**Default Settings:**
- Phase timeout: 30 minutes
- Max retries per phase: 3
- Auto-commit after each wave: enabled

> **Planned:** A `.scud/flow-config.toml` file will allow customization of these settings.
```

### Success Criteria:

#### Automated Verification:
- [x] No broken markdown links: `grep -r '\[.*\](.*\.md)' descartes/docs/blog/07-flow-workflow.md`
- [x] File renders correctly: preview in markdown viewer

#### Manual Verification:
- [x] All CLI examples use only implemented flags (`--prd`, `--tag`, `--resume`, `--dir`, `--adapter`)
- [x] SCUD clearly marked as external/optional
- [x] No claims of parallel execution (notes explain it's planned for future)

---

## Phase 2: Fix Tool Levels (06-agent-types.md)

### Overview
Correct the Lisp Developer tools list and clarify that bash/write restrictions are prompt-based guidance, not code-enforced.

### Changes Required:

#### 2.1 Fix Lisp Developer Tools Table

**File**: `descartes/docs/blog/06-agent-types.md`
**Lines**: 262-271

**Current** (incorrect):
```markdown
### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `edit` | Surgical text replacement |
| `bash` | Execute commands |
| `swank_eval` | Evaluate Lisp expressions |
...
```

**Change to**:
```markdown
### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `bash` | Execute commands |
| `swank_eval` | Evaluate Lisp expressions |
| `swank_compile` | Compile Lisp code |
| `swank_inspect` | Inspect Lisp objects |
| `swank_restart` | Invoke debugger restarts |

> **Note:** The Lisp Developer level focuses on interactive REPL-based development. File modifications are done via Swank compilation rather than direct write/edit tools.
```

#### 2.2 Clarify Planner Write Restrictions

**File**: `descartes/docs/blog/06-agent-types.md`
**Lines**: 137-143

**Current** (implies code enforcement):
```markdown
### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Write to `thoughts/` directory only |
| `bash` | Execute commands (typically read-only) |
```

**Change to**:
```markdown
### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `bash` | Execute commands |

> **Guidance:** The Planner's system prompt instructs it to write only to the `thoughts/` directory for plans and documentation. This is prompt-based guidance, not a code-enforced restriction.
```

#### 2.3 Clarify Researcher Bash Restrictions

**File**: `descartes/docs/blog/06-agent-types.md`
**Lines**: 206-217

**Current** (implies code enforcement):
```markdown
### Bash Restrictions

Researcher-level bash is limited to read-only operations:
- `ls`, `find`, `grep`, `cat`
- `git log`, `git show`, `git diff`
- `npm list`, `cargo tree`

Mutations are blocked:
- No `rm`, `mv`, `cp`
- No `git commit`, `git push`
- No file writes via redirection
```

**Change to**:
```markdown
### Bash Guidance

The Researcher's system prompt instructs it to use read-only bash operations:
- `ls`, `find`, `grep`, `cat`
- `git log`, `git show`, `git diff`
- `npm list`, `cargo tree`

The prompt discourages mutations:
- `rm`, `mv`, `cp`
- `git commit`, `git push`
- File writes via redirection

> **Note:** These restrictions are prompt-based guidance. The LLM is instructed to avoid mutations, but no code-level enforcement exists. For guaranteed safety, use the Read-Only level.
```

#### 2.4 Clarify ReadOnly Bash Restrictions

**File**: `descartes/docs/blog/06-agent-types.md`
**Lines**: 246-253

**Current** (implies whitelist enforcement):
```markdown
### Bash Whitelist

Only these commands are allowed:
- `ls`
- `pwd`
- `echo` (for basic output)
- `cat` (read-only file access)
```

**Change to**:
```markdown
### Bash Guidance

The Read-Only system prompt strongly instructs the agent to use only safe commands:
- `ls`, `pwd` - Directory listing
- `cat`, `head`, `tail` - File reading
- `echo` - Basic output

> **Note:** This is prompt-based guidance. For environments requiring absolute safety, consider running Descartes with OS-level sandboxing or restricted user permissions.
```

#### 2.5 Add Clarification Note to Hierarchy Diagram

**File**: `descartes/docs/blog/06-agent-types.md`
**Lines**: 9-33

After the hierarchy diagram, add:

```markdown
> **Implementation Note:** Tool availability (which tools an agent receives) is code-enforced. Behavioral restrictions within tools (e.g., "bash read-only") are prompt-based guidance that relies on LLM compliance.
```

### Success Criteria:

#### Automated Verification:
- [x] Markdown renders correctly
- [x] No broken internal links

#### Manual Verification:
- [x] Lisp Developer tools match `registry.rs:56-63` (read, bash, swank_eval, swank_compile, swank_inspect, swank_restart)
- [x] All "restrictions" clearly marked as prompt-based or code-enforced
- [x] Reader understands security model (Implementation Note added after hierarchy diagram)

---

## Phase 3: Fix README Files

### Overview
Add DeepSeek/Groq disclaimer to match the blog, update tool levels table.

### Changes Required:

#### 3.1 Update Blog README Tool Levels Table

**File**: `descartes/docs/blog/README.md`
**Lines**: 99-108

**Current** (incomplete):
```markdown
### Tool Levels

| Level | Capabilities |
|-------|-------------|
| Orchestrator | Full access + sub-agent spawning |
| Minimal | 4 tools, no spawning |
| Planner | Read + write to thoughts/ only |
| Researcher | Read + bash (read-only) |
| Read-Only | Safe observation only |
```

**Change to**:
```markdown
### Tool Levels

| Level | Capabilities |
|-------|-------------|
| Orchestrator | Full access + sub-agent spawning |
| Minimal | 4 tools, no spawning |
| Planner | Read + write + bash (prompt guides thoughts/ usage) |
| Researcher | Read + bash (prompt guides read-only usage) |
| Read-Only | Read + bash (prompt guides safe commands) |
| Lisp Developer | Read + bash + Swank tools |

> **Note:** Behavioral restrictions are prompt-based guidance. Tool availability is code-enforced.
```

### Success Criteria:

#### Automated Verification:
- [x] Markdown renders correctly

#### Manual Verification:
- [x] Tool levels table matches implementation
- [x] Lisp Developer level included

---

## Phase 4: Fix Session Management (05-session-management.md)

### Overview
Standardize session directory references.

### Changes Required:

#### 4.1 Clarify Session Directory Locations

**File**: `descartes/docs/blog/05-session-management.md`
**Lines**: 146-150

**Current** (single location):
```markdown
### Location

```
.scud/sessions/
└── 2025-01-15-10-30-00-a1b2c3.json
```
```

**Change to**:
```markdown
### Location

Session transcripts are stored in project-local directories:

```
.scud/sessions/           # Flow workflow sessions
└── 2025-01-15-10-30-00-a1b2c3.json

.descartes/sessions/      # General agent sessions
└── 2025-01-15-10-30-00-d4e5f6.json
```

The location depends on how the session was spawned. Flow workflow sessions use `.scud/`, while direct `descartes spawn` sessions use `.descartes/`.
```

### Success Criteria:

#### Automated Verification:
- [x] Markdown renders correctly

#### Manual Verification:
- [x] Both directory locations documented (.scud/sessions/ and .descartes/sessions/)
- [x] Clear explanation of when each is used (Flow vs spawn)

---

## Testing Strategy

### Manual Testing Steps:
1. Read each modified section and verify it matches current code behavior
2. Verify no broken markdown links between documents
3. Render documents in a markdown viewer to check formatting
4. Cross-reference claims against source files noted in research document

## References

- Research document: `thoughts/shared/research/2026-01-03-blog-documentation-validation.md`
- Tool registry: `descartes/core/src/tools/registry.rs`
- Tool executors: `descartes/core/src/tools/executors.rs`
- Workflow CLI: `descartes/cli/src/commands/workflow.rs`
