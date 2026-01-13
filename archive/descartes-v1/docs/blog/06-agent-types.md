# Agent Types and Tool Levels

*Right-sizing capabilities for every task*

---

Not every task needs full system access. Descartes provides **tool levels** that scope agent capabilities appropriately—from read-only exploration to full orchestration with sub-agent spawning.

## The Tool Level Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                     ORCHESTRATOR                            │
│  read, write, edit, bash, spawn_session                     │
│  Full capabilities + delegation                             │
├─────────────────────────────────────────────────────────────┤
│                       MINIMAL                               │
│  read, write, edit, bash                                    │
│  Focused work, no delegation                                │
├─────────────────────────────────────────────────────────────┤
│                       PLANNER                               │
│  read, write (thoughts/ only), bash                         │
│  Planning and documentation                                 │
├─────────────────────────────────────────────────────────────┤
│                      RESEARCHER                             │
│  read, bash                                                 │
│  Analysis and exploration                                   │
├─────────────────────────────────────────────────────────────┤
│                       READONLY                              │
│  read, bash (read-only commands)                            │
│  Safe observation                                           │
└─────────────────────────────────────────────────────────────┘
```

> **Implementation Note:** Tool availability (which tools an agent receives) is code-enforced. Behavioral restrictions within tools (e.g., "bash read-only") are prompt-based guidance that relies on LLM compliance.

---

## Orchestrator Level

**The fully-capable agent for complex, multi-step tasks.**

### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `edit` | Surgical text replacement |
| `bash` | Execute any command |
| `spawn_session` | Delegate to sub-agents |

### When to Use

- Complex features requiring multiple sub-tasks
- Tasks that benefit from delegation
- Top-level workflow orchestration
- Tasks requiring broad system changes

### Example

```bash
descartes spawn \
  --task "Implement a new payment system with Stripe integration" \
  --tool-level orchestrator
```

The agent might:
1. Analyze the existing codebase
2. Spawn a sub-agent to write database models
3. Spawn another to implement API endpoints
4. Spawn a third to write frontend components
5. Integrate and test everything

### Sub-Agent Spawning

```json
{
  "name": "spawn_session",
  "arguments": {
    "task": "Write database models for payment transactions",
    "agent": "minimal",
    "output_file": ".scud/sessions/payment-models.json"
  }
}
```

**Key Rules:**
- Sub-agents get **minimal** level (no further spawning)
- Each sub-agent has its own transcript
- Parent tracks sub-agent completion

---

## Minimal Level

**Focused execution without delegation overhead.**

### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `edit` | Surgical text replacement |
| `bash` | Execute any command |

### When to Use

- Single-focus tasks
- Bug fixes
- Small feature additions
- When you want to prevent sub-agent complexity

### Example

```bash
descartes spawn \
  --task "Fix the race condition in the user service" \
  --tool-level minimal
```

### Why Not Orchestrator?

For simple tasks, orchestrator adds overhead:
- Context for `spawn_session` tool
- Potential for unnecessary delegation
- More complex transcripts

Minimal is leaner and more predictable.

---

## Planner Level

**For designing and documenting, not implementing.**

### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `write` | Create/overwrite files |
| `bash` | Execute commands |

> **Guidance:** The Planner's system prompt instructs it to write only to the `thoughts/` directory for plans and documentation. This is prompt-based guidance, not a code-enforced restriction.

### When to Use

- Creating implementation plans
- Designing architecture
- Writing technical specifications
- Research that produces documentation

### Example

```bash
descartes spawn \
  --task "Design the authentication system architecture" \
  --tool-level planner
```

### Output Location

Plans are written to:
```
thoughts/
├── shared/
│   ├── plans/
│   │   └── 2025-01-15-auth-design.md
│   └── research/
│       └── 2025-01-15-auth-patterns.md
```

### Why Restricted Write?

Planners should think, not implement. Restricting writes to `thoughts/` ensures:
- No accidental code changes
- Clear separation of planning and execution
- Reviewable design artifacts

---

## Researcher Level

**Pure analysis and exploration.**

### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `bash` | Execute read-only commands |

### When to Use

- Codebase analysis
- Security audits
- Understanding existing patterns
- Answering "how does X work?" questions

### Example

```bash
descartes spawn \
  --task "Analyze how error handling works across the application" \
  --tool-level researcher
```

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

---

## Read-Only Level

**Maximum safety for sensitive environments.**

### Tools Available

| Tool | Description |
|------|-------------|
| `read` | Read any file |
| `bash` | Extremely restricted (listing only) |

### When to Use

- Production environment exploration
- Auditing without risk
- When you need absolute safety
- Learning about unfamiliar codebases

### Example

```bash
descartes spawn \
  --task "Explore the production database schema" \
  --tool-level readonly
```

### Bash Guidance

The Read-Only system prompt strongly instructs the agent to use only safe commands:
- `ls`, `pwd` - Directory listing
- `cat`, `head`, `tail` - File reading
- `echo` - Basic output

> **Note:** This is prompt-based guidance. For environments requiring absolute safety, consider running Descartes with OS-level sandboxing or restricted user permissions.

---

## Lisp Developer Level

**Specialized for live Lisp development with SBCL.**

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

### When to Use

- Common Lisp development
- Interactive SBCL sessions
- Live debugging with Swank

### Example

```bash
descartes spawn \
  --task "Debug the memory leak in the image processor" \
  --tool-level lisp-developer
```

### Swank Integration

Descartes connects to SBCL's Swank server:

```lisp
;; Agent can evaluate
(swank_eval "(defun hello () 'world)")

;; Compile code
(swank_compile "(defun optimized-fn () ...)")

;; Inspect objects
(swank_inspect "*last-result*")
```

---

## Choosing the Right Level

### Decision Tree

```
Is this a complex multi-part task?
├─ Yes → Does it benefit from delegation?
│        ├─ Yes → ORCHESTRATOR
│        └─ No  → MINIMAL
└─ No  → Is it implementation or planning?
         ├─ Implementation → MINIMAL
         └─ Planning → Does it need to write plans?
                       ├─ Yes → PLANNER
                       └─ No  → RESEARCHER or READONLY
```

### Quick Reference

| Task Type | Recommended Level |
|-----------|------------------|
| Full feature implementation | Orchestrator |
| Bug fix | Minimal |
| Small enhancement | Minimal |
| Architecture design | Planner |
| Codebase exploration | Researcher |
| Security audit | Read-Only |
| Production debugging | Read-Only |
| Lisp development | Lisp Developer |

---

## Agent Definitions

Beyond tool levels, you can define custom agents with specific personalities and constraints.

### Definition Format

```markdown
---
name: security-reviewer
description: Reviews code for security vulnerabilities
model: claude-3-5-sonnet
tool_level: researcher
tags: [security, review]
---

You are a security-focused code reviewer. Analyze code for:
- OWASP Top 10 vulnerabilities
- Authentication weaknesses
- Data validation issues
- Injection risks

Report findings with severity levels and remediation suggestions.
```

### Using Custom Agents

```bash
descartes spawn \
  --task "Review the authentication module" \
  --agent ~/.descartes/agents/security-reviewer.md
```

### Built-in Agents

Descartes includes pre-built agents:

| Agent | Purpose |
|-------|---------|
| `codebase-analyzer` | Deep code analysis |
| `codebase-locator` | Find files and patterns |
| `researcher` | General research |
| `planner` | Implementation planning |
| `flow-orchestrator` | Workflow orchestration |

---

## Tool Level Downgrade

When orchestrators spawn sub-agents, capabilities are automatically reduced:

```
Orchestrator spawns → Minimal (cannot spawn further)
```

This prevents:
- Recursive agent explosions
- Uncontrolled delegation chains
- Resource exhaustion

### The One-Level Rule

```
Main Agent (Orchestrator)
├── Sub-Agent A (Minimal) ← Cannot spawn
├── Sub-Agent B (Minimal) ← Cannot spawn
└── Sub-Agent C (Minimal) ← Cannot spawn
```

---

## Security Considerations

### Principle of Least Privilege

Always use the minimum level needed:

```bash
# Bad: Using orchestrator for simple read
descartes spawn -t "What files are in src/?" --tool-level orchestrator

# Good: Using readonly for exploration
descartes spawn -t "What files are in src/?" --tool-level readonly
```

### Production Safety

For production environments:
```bash
# Safe exploration
descartes spawn -t "Check service health" --tool-level readonly
```

### Sensitive Codebases

For security-sensitive analysis:
```bash
# No modifications possible
descartes spawn -t "Audit auth module" --tool-level researcher
```

---

## Next Steps

- **[Flow Workflow →](07-flow-workflow.md)** — See agents in action
- **[Skills System →](08-skills-system.md)** — Extend agent capabilities
- **[Sub-Agent Tracking →](10-subagent-tracking.md)** — Monitor delegation

---

*The right tool for the right job—now you know how to choose.*
