---
date: 2026-01-09T21:23:17Z
researcher: Claude Code (Opus 4.5)
git_commit: a6b0dc5a8cd609f9b7973c8c4f19e68cb6658caf
branch: master
repository: cap
topic: "SCUD and Descartes Interplay Analysis"
tags: [research, codebase, scud, descartes, architecture, sessions]
status: complete
last_updated: 2026-01-09
last_updated_by: Claude Code
---

# Research: SCUD and Descartes Interplay Analysis

**Date**: 2026-01-09T21:23:17Z
**Researcher**: Claude Code (Opus 4.5)
**Git Commit**: a6b0dc5a8cd609f9b7973c8c4f19e68cb6658caf
**Branch**: master
**Repository**: cap

## Research Question

User asked: "The interplay between SCUD and Descartes is challenging. I like using SCUD underneath, but the blog posts clearly seem to indicate that sessions for Descartes are logged in SCUD. I don't understand that, that doesn't make sense. Can we do a deep dive and make sure I understand the interplay and I like where this might be better off being separated?"

## Summary

The research reveals that **SCUD and Descartes share the `.scud/` directory** for pragmatic reasons, but this creates conceptual confusion about ownership. Here's the reality:

| Aspect | SCUD's Role | Descartes' Role |
|--------|-------------|-----------------|
| **Identity** | Task management CLI/library | AI orchestration framework |
| **Storage** | Creates `.scud/` and owns task files | Piggybacks on `.scud/` for sessions |
| **Sessions** | No session concept | Creates `.scud/sessions/` when `.scud/` exists |
| **Integration** | Provides types as Cargo dependency | Imports SCUD library, wraps in async |

**The confusing statement "sessions for Descartes are logged in SCUD"** means:
- When you're in a project with `.scud/` directory, Descartes writes its session transcripts to `.scud/sessions/`
- This is NOT SCUD logging sessions - it's Descartes choosing to colocate with SCUD data
- The decision logic: if `.scud/` exists, use `.scud/sessions/`; otherwise use `~/.descartes/sessions/`

**This design creates a semantic problem**: `.scud/` appears to be a SCUD directory, but Descartes writes non-SCUD data there.

## Detailed Findings

### 1. What SCUD Actually Is

SCUD is a **separate task management CLI** with its own repository. It provides:
- SCG file format for human-readable task definitions
- Wave-based dependency resolution
- Task status tracking (8 states: Pending, InProgress, Done, Review, Blocked, Deferred, Cancelled, Expanded)
- Priority and complexity tracking

Descartes imports SCUD as a Cargo dependency:
```toml
# descartes/Cargo.toml:44
scud = { version = "1.19", package = "scud-cli" }
```

### 2. Directory Ownership Analysis

#### Files SCUD CLI Creates
| File/Path | Purpose | Created By |
|-----------|---------|------------|
| `.scud/` | Root directory | `scud init` |
| `.scud/active-tag` | Current active epic/tag | SCUD CLI |
| `.scud/config.toml` | SCUD configuration | SCUD CLI |
| `.scud/tasks/tasks.scg` | SCG format task file | SCUD CLI |
| `.scud/tasks/tasks.json` | JSON format tasks (legacy) | SCUD CLI |
| `.scud/workflow-state.json` | Workflow state | SCUD CLI |
| `.scud/docs/` | Documentation directory | SCUD CLI |

#### Files Descartes Creates (IN `.scud/`!)
| File/Path | Purpose | Created By |
|-----------|---------|------------|
| `.scud/sessions/` | Session transcript storage | Descartes |
| `.scud/sessions/*.json` | Individual session transcripts | Descartes |
| `.scud/flow-state.json` | Flow workflow state | Descartes |
| `.scud/loop-state.json` | SCUD loop execution state | Descartes |
| `.scud/tune-state.json` | Task tuning state | Descartes |
| `.scud/qa-log.json` | QA monitoring log | Descartes |

### 3. The Session Location Decision Logic

From `descartes/core/src/session_transcript.rs:198-207`:

```rust
pub fn default_sessions_dir() -> PathBuf {
    let scud_sessions = PathBuf::from(".scud/sessions");
    if scud_sessions.parent().is_some_and(|p| p.exists()) {
        scud_sessions
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".descartes/sessions")
    }
}
```

**Translation**:
1. Check if `.scud/` directory exists in current project
2. If yes: store sessions in `.scud/sessions/`
3. If no: store sessions in `~/.descartes/sessions/`

**This is why the blog says "sessions logged in SCUD"** - Descartes opportunistically uses the SCUD directory when present.

### 4. Integration Architecture

```
                    ┌─────────────────────────────────────┐
                    │           User's Project            │
                    │                                     │
                    │  ┌───────────────────────────────┐  │
                    │  │         .scud/                │  │
                    │  │  ┌──────────┬───────────────┐ │  │
                    │  │  │ SCUD     │ Descartes     │ │  │
                    │  │  │ Owned    │ Piggybacks    │ │  │
                    │  │  ├──────────┼───────────────┤ │  │
                    │  │  │active-tag│ sessions/     │ │  │
                    │  │  │config.   │ flow-state.   │ │  │
                    │  │  │tasks/    │ loop-state.   │ │  │
                    │  │  │workflow- │ tune-state.   │ │  │
                    │  │  │state.json│ qa-log.json   │ │  │
                    │  │  └──────────┴───────────────┘ │  │
                    │  └───────────────────────────────┘  │
                    └─────────────────────────────────────┘
```

### 5. Why This Design Was Chosen

From `thoughts/shared/plans/2025-12-02-scud-descartes-unification.md`:

> "Unify Descartes and SCUD into a single, compatible task management system where:
> - SCUD's models and SCG format become the standard
> - Descartes builds 'on top' of SCUD conceptually
> - Users can use either tooling interchangeably"

The shared directory was intentional for **operational convenience**, not conceptual purity:
- One directory to backup/version control
- One place to find all project artifacts
- Seamless switching between `scud` and `descartes` CLI

### 6. The Conceptual Tension

The current design conflates two separate concerns:

| Concern | SCUD Responsibility | Descartes Responsibility |
|---------|--------------------|-----------------------|
| Task Definition | SCG files, task structure | Imports SCUD types |
| Task Execution | None (CLI tool only) | Iterative loops, agents |
| Session Logging | None | Full transcript system |
| Workflow State | workflow-state.json | flow-state.json, loop-state.json |

**Problem**: Descartes writes files that have nothing to do with SCUD into a directory named `.scud/`.

## Potential Separation Points

### Option A: Clean Directory Separation

```
.scud/                    # SCUD-only data
├── active-tag
├── config.toml
├── tasks/
│   └── tasks.scg
└── workflow-state.json

.descartes/               # Descartes-only data
├── sessions/
│   └── *.json
├── flow-state.json
├── loop-state.json
├── tune-state.json
└── qa-log.json
```

**Pros**:
- Clear ownership boundaries
- Semantically honest naming
- Can use SCUD without Descartes artifacts

**Cons**:
- Two directories to manage
- Breaks existing workflows
- More complex backup story

### Option B: Unified Under `.descartes/` With SCUD as Sub-component

```
.descartes/
├── tasks/                # SCUD data (symlinked or migrated)
│   └── tasks.scg
├── sessions/
├── flow-state.json
├── loop-state.json
└── config/
    └── scud.toml         # Former .scud/config.toml
```

**Pros**:
- Single directory ownership
- Descartes as the "host" framework

**Cons**:
- SCUD loses its identity
- Harder to use SCUD CLI standalone

### Option C: Keep Shared But Document Clearly

Keep current structure but add explicit documentation:

```
.scud/                    # Shared AI workspace
├── [SCUD] active-tag
├── [SCUD] config.toml
├── [SCUD] tasks/
├── [SCUD] workflow-state.json
├── [DESCARTES] sessions/
├── [DESCARTES] flow-state.json
└── [DESCARTES] loop-state.json
```

**Pros**:
- No code changes needed
- Works today

**Cons**:
- Still semantically confusing
- Directory name is misleading

### Option D: Rename `.scud/` to `.ai-workspace/` or `.cap/`

Rename the shared directory to something neutral:

```
.ai-workspace/            # Or .cap/, .agents/, etc.
├── scud/                 # SCUD-specific
│   ├── active-tag
│   ├── config.toml
│   └── tasks/
├── descartes/            # Descartes-specific
│   ├── sessions/
│   ├── flow-state.json
│   └── loop-state.json
└── shared/               # Truly shared
    └── workflow-state.json
```

**Pros**:
- Honest naming
- Clear sub-namespaces
- Room for future tools

**Cons**:
- Major migration effort
- Breaks all existing projects

## Current Code Touchpoints for Separation

If you wanted to implement separation, these are the key files:

| File | Purpose | Lines of Interest |
|------|---------|-------------------|
| `descartes/core/src/session_transcript.rs` | Session location logic | 198-207 |
| `descartes/core/src/scud_plugin.rs` | Path utilities | 24-40 |
| `descartes/core/src/flow_executor.rs` | Flow state paths | 326, 891-894 |
| `descartes/core/src/scud_loop.rs` | Loop state paths | 463, 739, 1369 |
| `descartes/core/src/session_manager.rs` | Workspace detection | 49-52 |
| `descartes/daemon/src/scg_task_event_emitter.rs` | File watching | 119-179 |

## Recommendation Summary

The confusion you're experiencing is valid - the current design sacrifices clarity for convenience. Options ranked by impact:

1. **Quick fix**: Update blog documentation to explain this explicitly
2. **Medium fix**: Add a `sessions_dir` configuration option to override default
3. **Full fix**: Implement Option A or D for clean separation

The question to answer: **Is "one directory for everything" worth the semantic confusion?**

For a production system used by others, I'd lean toward Option D (neutral shared directory name) as it's honest about what the directory contains. For personal use, Option C (document clearly) may be sufficient.

## Code References

- `descartes/core/src/session_transcript.rs:198-207` - Session directory decision
- `descartes/core/src/scud_plugin.rs:24-40` - SCUD path utilities
- `descartes/core/src/flow_executor.rs:326` - Flow state path
- `descartes/core/src/scud_loop.rs:463,739,1369` - Loop and tune state paths
- `descartes/core/src/traits.rs:536-687` - SCUD type imports and conversions
- `descartes/core/src/scg_task_storage.rs:19-299` - SCUD storage wrapper
- `thoughts/shared/plans/2025-12-02-scud-descartes-unification.md` - Original unification plan

## Architecture Documentation

The current architecture follows a "Descartes wraps SCUD" model:

```
┌─────────────────────────────────────────────────────┐
│                    Descartes                         │
│  ┌───────────────────────────────────────────────┐  │
│  │              ScgTaskStorage                    │  │
│  │   (Async wrapper with in-memory cache)        │  │
│  └────────────────────┬──────────────────────────┘  │
│                       │ imports                      │
│  ┌────────────────────▼──────────────────────────┐  │
│  │           scud-cli library                     │  │
│  │   - Storage (sync)                            │  │
│  │   - Task, Phase, Priority models              │  │
│  │   - SCG parser/serializer                     │  │
│  └───────────────────────────────────────────────┘  │
│                                                      │
│  ┌───────────────────────────────────────────────┐  │
│  │         Descartes-specific features            │  │
│  │   - Session transcripts                       │  │
│  │   - Flow workflow                             │  │
│  │   - Iterative loops (Ralph Wiggum)            │  │
│  │   - Tune the Guitar                           │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Related Research

- `thoughts/shared/plans/2025-12-02-scud-descartes-unification.md` - Original unification plan
- `thoughts/shared/research/2026-01-08-ralph-loop-scud-integration.md` - Ralph loop integration
- `descartes/docs/blog/05-session-management.md` - Session documentation
- `descartes/docs/blog/14-choosing-your-workflow.md` - Flow vs RW comparison

## Open Questions

1. **User preference**: Do you prefer clean separation (Option A/D) or keeping convenience (Option C)?
2. **Migration path**: If separating, should existing `.scud/sessions/` be migrated?
3. **Configuration**: Should session location be configurable per-project?
4. **SCUD CLI users**: Do standalone SCUD CLI users need to see Descartes artifacts?
