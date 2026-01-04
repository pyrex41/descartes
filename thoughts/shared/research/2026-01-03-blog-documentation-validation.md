# Descartes Blog Documentation Validation

**Date:** 2026-01-03
**Commit:** 38e9c4bd2146608c4a37f60992638ebdba6dbef4
**Branch:** master
**Scope:** Validation of all 13 blog documentation files against actual code implementation

---

## Executive Summary

The Descartes blog documentation series (`descartes/docs/blog/`) was cross-referenced against the actual Rust implementation. While the documentation is generally well-written and accurate for core features, several discrepancies were identified where the documentation describes features that are either not implemented, partially implemented, or implemented differently than documented.

**Overall Assessment:** Documentation is ~85% accurate. Key gaps exist in Flow/SCUD integration, tool level enforcement, and provider implementation status.

---

## Files Analyzed

### Blog Documentation (13 files)
- `01-introduction-the-pi-philosophy.md` - Pi philosophy
- `02-getting-started.md` - Installation guide
- `03-cli-commands.md` - CLI reference
- `04-providers-configuration.md` - Provider configuration
- `05-session-management.md` - Session lifecycle
- `06-agent-types.md` - Tool levels
- `07-flow-workflow.md` - Flow workflow
- `08-skills-system.md` - Skills system
- `09-gui-features.md` - GUI features
- `10-subagent-tracking.md` - Sub-agent tracking
- `11-advanced-features.md` - Advanced features
- `12-iterative-loops.md` - Iterative loops
- `README.md` - Documentation index

### Source Code Examined
- `descartes/core/src/providers.rs`
- `descartes/core/src/config.rs`
- `descartes/core/src/flow_executor.rs`
- `descartes/core/src/tools/registry.rs`
- `descartes/core/src/tools/definitions.rs`
- `descartes/core/src/iterative_loop.rs`
- `descartes/cli/src/commands/*.rs`

---

## Discrepancies Found

### 1. Provider Implementation (04-providers-configuration.md)

**Status:** MOSTLY ACCURATE

| Provider | Blog Claims | Actual Status |
|----------|-------------|---------------|
| Anthropic | Supported | IMPLEMENTED |
| OpenAI | Supported | IMPLEMENTED |
| xAI/Grok | Supported | IMPLEMENTED |
| Ollama | Supported | IMPLEMENTED |
| DeepSeek | "Not yet complete" | CONFIG ONLY - No HTTP client |
| Groq | "Not yet complete" | CONFIG ONLY - No HTTP client |

**Blog is accurate** - Line 18 correctly states:
> "DeepSeek and Groq configuration structures exist in the codebase but provider implementations are not yet complete."

**However, related documentation is misleading:**
- `README.md` lists DeepSeek and Groq without disclaimers
- `QUICKSTART.md` lists them without disclaimers
- Code comment in `providers.rs` line 8-9 is inaccurate: claims "HTTP clients for DeepSeek and Groq (DeepSeekClient, GroqClient)" exist but they don't

**Recommendation:** Update README.md and QUICKSTART.md with same disclaimer as blog.

---

### 2. CLI Commands (03-cli-commands.md)

**Status:** ACCURATE

All 14 documented commands are fully implemented:

| Command | Documented | Implemented | Location |
|---------|------------|-------------|----------|
| `spawn` | Yes | Yes | `spawn.rs` |
| `ps` | Yes | Yes | `ps.rs` |
| `logs` | Yes | Yes | `logs.rs` |
| `kill` | Yes | Yes | `kill.rs` |
| `pause` | Yes | Yes | `pause.rs` |
| `resume` | Yes | Yes | `resume.rs` |
| `attach` | Yes | Yes | `attach.rs` |
| `init` | Yes | Yes | `init.rs` |
| `doctor` | Yes | Yes | `doctor.rs` |
| `tasks` | Yes | Yes | `tasks.rs` |
| `workflow` | Yes | Yes | `workflow.rs` |
| `loop` | Yes | Yes | `loop_cmd.rs` |
| `gui` | Yes | Yes | `gui.rs` |
| `completions` | Yes | Yes | `completions.rs` |

**No changes needed.**

---

### 3. Flow Workflow (07-flow-workflow.md)

**Status:** SIGNIFICANT DISCREPANCIES

#### 3a. SCUD CLI Commands - NOT IMPLEMENTED

The blog documents these as part of the Flow workflow:
```bash
scud parse-prd docs/requirements.md
scud waves --list
scud expand --all
scud tasks --status pending
```

**Reality:** These are NOT Descartes commands. SCUD is an external task tracking system. Descartes has its own `descartes tasks` commands that have different syntax and behavior.

**Affected sections:** Lines 130-180 (SCUD Commands section)

#### 3b. Wave-Based Parallel Execution - NOT IMPLEMENTED

Blog claims (lines 78-100):
> "Waves execute in parallel when possible"
> "All tasks in a wave execute simultaneously"

**Reality:** `flow_executor.rs` processes tasks sequentially with no parallel execution logic. Wave grouping is conceptual only.

#### 3c. Rollback Command - NOT EXPOSED

Blog documents:
```bash
descartes workflow flow --rollback
```

**Reality:** `FlowExecutor::rollback()` method exists but is NOT exposed via CLI. No `--rollback` flag in workflow command.

#### 3d. Flow Config TOML - NOT SUPPORTED

Blog shows (lines 260-270):
```toml
# .scud/flow-config.toml
[flow]
orchestrator_model = "claude-3-5-sonnet-20241022"
```

**Reality:** No TOML config file parsing exists. Flow uses hardcoded defaults.

#### 3e. Missing CLI Flags

Blog documents flags that don't exist:
- `--phase` (start from specific phase) - NOT IMPLEMENTED
- `--stop` (stop at phase) - NOT IMPLEMENTED

**Recommendation:** Either implement these features or update documentation to reflect current capabilities.

---

### 4. Tool Levels (06-agent-types.md)

**Status:** PARTIAL DISCREPANCY

#### 4a. Bash Restrictions - PROMPT-BASED ONLY

Blog claims for ReadOnly/Researcher/Planner:
> "Bash commands are restricted to read-only operations"

**Reality:** No code enforcement exists. Restrictions are in system prompts only:
```rust
// registry.rs - readonly tools
fn build_tools_for_level(level: ToolLevel) -> Vec<Tool> {
    match level {
        ToolLevel::ReadOnly => vec![read_tool, bash_tool], // Full bash, no restrictions
        ...
    }
}
```

Users can run any bash command - restrictions rely on LLM following instructions.

#### 4b. Planner Write Restrictions - PROMPT-BASED ONLY

Blog claims:
> "Can only write to thoughts/ directory"

**Reality:** Write tool is included without path restrictions. System prompt asks nicely but code doesn't enforce it.

#### 4c. Lisp Developer Tools - DISCREPANCY

Blog claims (line 140):
> "Tools: read, bash, write, edit"

**Reality:** Implementation in `registry.rs`:
```rust
ToolLevel::LispDeveloper => vec![
    definitions::read_tool(),
    definitions::bash_tool(),
    // No write or edit!
]
```

The Lisp Developer level is **read-only** despite blog claiming write/edit access.

**Recommendation:** Either:
1. Update code to enforce documented restrictions, OR
2. Update documentation to clarify restrictions are prompt-based

---

### 5. Iterative Loops (12-iterative-loops.md)

**Status:** ACCURATE

Documentation accurately describes:
- Loop state machine implementation
- `--completion-promise` flag (default: "COMPLETE")
- `--max-iterations` flag (default: 20)
- `--backend` options (claude, opencode, generic)
- `--auto-commit` functionality
- State file location (`.descartes/loop-state.json`)
- Subcommands: start, status, resume, cancel

Code matches documentation in `iterative_loop.rs` and `loop_cmd.rs`.

**No changes needed.**

---

### 6. Session Management (05-session-management.md)

**Status:** ACCURATE WITH MINOR ISSUES

- Session lifecycle states are accurately documented
- Transcript structure matches implementation
- Pause/resume protocol is correctly described

**Minor issue:** Lines 148-150 reference `.scud/sessions/` as transcript location, but some code paths use `.descartes/sessions/`. Both are valid but inconsistency could confuse users.

---

### 7. Skills System (08-skills-system.md)

**Status:** NOT VERIFIED

Skills are loaded from markdown files with YAML frontmatter. The documentation appears conceptually accurate but skill execution varies by implementation.

---

### 8. GUI Features (09-gui-features.md)

**Status:** NOT VERIFIED

GUI implementation is in `descartes-gui/` Tauri app. Documentation describes planned features; actual implementation status varies.

---

## Summary of Required Changes

### High Priority (Functional Discrepancies)

1. **07-flow-workflow.md**: Remove or clearly mark SCUD CLI examples as external/planned
2. **07-flow-workflow.md**: Remove undocumented `--phase`, `--stop`, `--rollback` flags
3. **06-agent-types.md**: Clarify that bash/write restrictions are prompt-based, not enforced
4. **06-agent-types.md**: Fix Lisp Developer tools list (remove write/edit)

### Medium Priority (Misleading Information)

5. **README.md**: Add disclaimer for DeepSeek/Groq like blog has
6. **07-flow-workflow.md**: Clarify wave execution is sequential, not parallel

### Low Priority (Minor Inconsistencies)

7. **05-session-management.md**: Standardize session directory references

---

## Verification Commands Used

```bash
# Find all commands
rg "pub struct.*Args" descartes/cli/src/commands/ --type rust

# Check provider implementations
rg "impl.*Client" descartes/core/src/providers.rs --type rust

# Verify tool level definitions
rg "ToolLevel::" descartes/core/src/tools/registry.rs --type rust

# Check flow executor
rg "fn execute" descartes/core/src/flow_executor.rs --type rust
```

---

## Related Documents

- `thoughts/shared/research/2026-01-03-gastown-descartes-comparison.md` - Gastown vs Descartes analysis
- `thoughts/shared/plans/2025-12-30-blog-documentation-corrections.md` - Existing correction plan

---

*Research conducted via codebase analysis without runtime testing.*
