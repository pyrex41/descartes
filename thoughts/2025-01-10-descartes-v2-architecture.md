# Descartes v2: Ralph-Wiggum-Inspired Agent Orchestration

**Date**: 2025-01-10
**Status**: Architecture Plan
**Goal**: Tight Rust binary for visible subagent orchestration with SCUD task management

---

## Core Philosophy

Synthesize three ideas:
1. **SCUD** - DAG-driven task management with token-efficient SCG format
2. **Ralph Wiggum** - Deterministic loops with planning/building modes
3. **Descartes** - Visible subagent execution with full transcripts

**Key differentiator**: You can see exactly what every subagent did.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Ralph Loop (outer)                           │
│  while :; do descartes run --mode=build ; done                  │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SCUD Task Graph                              │
│  .scud/scud.scg (token-efficient format)                        │
│                                                                 │
│  $ scud next   →  Returns ready task                            │
│  $ scud waves  →  Shows parallel execution potential            │
│  $ scud done   →  Mark complete                                 │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│              Subagents (1 level, full visibility)               │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                      │
│  │ searcher │  │ searcher │  │ analyzer │  ← parallel          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                      │
│       └─────────────┼─────────────┘                             │
│                     ▼                                           │
│              ┌──────────┐                                       │
│              │ builder  │  ← single, implements                 │
│              └────┬─────┘                                       │
│                   ▼                                             │
│              ┌──────────┐                                       │
│              │ validator│  ← backpressure gate                  │
│              └──────────┘                                       │
│                                                                 │
│  All transcripts → .descartes/transcripts/*.scg                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Dependencies | OK to use | Don't over-optimize for binary size |
| Transcript format | SCG | Token-efficient, matches SCUD |
| Subagent depth | 1 level only | Prevents explosion, simplifies |
| Model routing | Configurable | Agent categories define defaults |
| GUI | Deferred | Will use Rust Iced when ready |
| Harness support | Claude headless, OpenCode, Codex | Proxy pattern for all |

---

## Harness/Proxy Layer

Support multiple AI harnesses through a unified proxy:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Harness Abstraction                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │Claude Code  │  │  OpenCode   │  │   Codex     │             │
│  │ (headless)  │  │   (TUI)     │  │   (API)     │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          ▼                                      │
│                 ┌─────────────────┐                             │
│                 │  HarnessProxy   │                             │
│                 │                 │                             │
│                 │ - Intercepts    │                             │
│                 │   tool calls    │                             │
│                 │ - Detects       │                             │
│                 │   subagent      │                             │
│                 │   spawns        │                             │
│                 │ - Routes to     │                             │
│                 │   isolated      │                             │
│                 │   sessions      │                             │
│                 │ - Captures      │                             │
│                 │   transcripts   │                             │
│                 └─────────────────┘                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Harness Trait

```rust
#[async_trait]
pub trait Harness: Send + Sync {
    /// Name of the harness (for logging/config)
    fn name(&self) -> &str;

    /// Start a new session with the given prompt
    async fn start_session(&self, config: SessionConfig) -> Result<SessionHandle>;

    /// Send a message and get streaming response
    async fn send(&self, session: &SessionHandle, message: &str) -> Result<ResponseStream>;

    /// Detect if response contains subagent spawn request
    fn detect_subagent_spawn(&self, response: &ToolCall) -> Option<SubagentRequest>;

    /// Inject subagent result back into parent session
    async fn inject_result(&self, session: &SessionHandle, result: SubagentResult) -> Result<()>;
}
```

### Implementations

```rust
// Claude Code headless mode
pub struct ClaudeCodeHarness {
    api_key: String,
    model: String,
}

// OpenCode TUI (via IPC or API)
pub struct OpenCodeHarness {
    socket_path: PathBuf,
}

// Codex API
pub struct CodexHarness {
    api_base: Url,
    api_key: String,
}
```

---

## Agent Categories

Configurable agent types with default model routing:

```toml
# .descartes/config.toml

[categories.searcher]
description = "Fast parallel code search"
model = "sonnet"           # Default to cheaper/faster
tools = ["read", "bash"]   # Read-only
parallel = true            # Can run many at once

[categories.analyzer]
description = "Deep code analysis"
model = "sonnet"
tools = ["read"]
parallel = true

[categories.builder]
description = "Code implementation"
model = "opus"             # Stronger reasoning
tools = ["read", "write", "edit", "bash"]
parallel = false           # One at a time

[categories.validator]
description = "Test runner (backpressure)"
model = "sonnet"
tools = ["bash"]
parallel = false           # Gate - must complete
backpressure = true        # Blocks until pass

[categories.planner]
description = "Task planning and breakdown"
model = "opus"
tools = ["read", "bash"]
parallel = false
```

### Custom Categories

Users can define their own:

```toml
[categories.security_reviewer]
description = "Security-focused code review"
model = "opus"
tools = ["read"]
prompt_template = "prompts/security_review.md"
```

---

## SCG Transcript Format

Transcripts use SCG (SCUD Compact Graph) format for token efficiency:

```scg
@transcript
id: "session_2025-01-10_001"
harness: "claude-code"
model: "opus"
started: 2025-01-10T14:30:00Z
parent: null  # or parent session id for subagents

@messages
1:user "Find all rate limiting implementations"
2:assistant "I'll search for rate limiting code."
3:tool:bash "rg 'rate.?limit' --type rust"
4:tool_result:bash """
src/middleware/rate_limit.rs:15:pub struct RateLimiter {
src/middleware/rate_limit.rs:42:impl RateLimiter {
"""
5:assistant "Found RateLimiter in src/middleware/rate_limit.rs"

@subagents
sub1:searcher "find rate limit tests" -> session_2025-01-10_002

@metrics
tokens_in: 1240
tokens_out: 856
duration_ms: 3420
tools_called: 1
```

### Benefits
- 75% smaller than JSON
- Human readable
- Easy to grep
- Parseable for replay

---

## Crate Structure

```
descartes/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   │
│   ├── loop.rs              # Ralph loop runner
│   │   └── Modes: plan, build
│   │   └── Iteration tracking
│   │   └── Fresh context per loop
│   │
│   ├── harness/
│   │   ├── mod.rs           # Harness trait
│   │   ├── claude_code.rs   # Claude Code headless
│   │   ├── opencode.rs      # OpenCode integration
│   │   ├── codex.rs         # Codex API
│   │   └── proxy.rs         # Intercept & route logic
│   │
│   ├── agent/
│   │   ├── mod.rs           # Agent execution
│   │   ├── category.rs      # Agent categories/config
│   │   ├── subagent.rs      # Subagent spawning (1 level)
│   │   └── tools.rs         # Tool definitions (read/write/edit/bash)
│   │
│   ├── transcript/
│   │   ├── mod.rs           # Transcript types
│   │   ├── scg.rs           # SCG format parser/writer
│   │   └── replay.rs        # Transcript replay
│   │
│   ├── scud/
│   │   ├── mod.rs           # SCUD integration
│   │   ├── scg_tasks.rs     # Read/write task SCG
│   │   └── queries.rs       # next, ready, blocked, waves
│   │
│   └── config.rs            # Configuration loading
│
├── prompts/
│   ├── plan.md              # Planning mode prompt
│   └── build.md             # Building mode prompt
│
└── tests/
    ├── harness_tests.rs
    ├── transcript_tests.rs
    └── integration/
```

---

## CLI Interface

```bash
# Ralph loop
descartes loop                      # Infinite build loop
descartes loop --plan               # Planning mode
descartes loop --max 5              # Stop after 5 iterations

# Single iteration
descartes run                       # One build cycle
descartes plan                      # One planning cycle

# Subagent spawning (used internally, but exposed)
descartes spawn searcher "find auth implementations"
descartes spawn builder "implement rate limiting"
descartes spawn validator "run cargo test"

# Transcript inspection
descartes transcripts               # List all
descartes transcripts --today       # Today's sessions
descartes show <session-id>         # View transcript
descartes replay <session-id>       # Replay with timing

# SCUD integration
descartes next                      # What's ready?
descartes complete <task-id>        # Mark done
descartes waves                     # Visualize DAG

# Configuration
descartes init                      # Initialize .descartes/
descartes config                    # Show config
descartes harness                   # Show active harness
```

---

## Core Loop Implementation

```rust
pub async fn ralph_loop(mode: Mode, config: LoopConfig) -> Result<()> {
    let harness = create_harness(&config)?;
    let mut iteration = 0;

    loop {
        if config.max_iterations.map(|m| iteration >= m).unwrap_or(false) {
            info!("Reached max iterations: {}", iteration);
            break;
        }

        // Fresh context each iteration (Ralph principle)
        let transcript = Transcript::new()
            .harness(harness.name())
            .iteration(iteration);

        match mode {
            Mode::Plan => {
                plan_iteration(&harness, &transcript, &config).await?;
            }
            Mode::Build => {
                build_iteration(&harness, &transcript, &config).await?;
            }
        }

        // Save transcript in SCG format
        transcript.save_scg(&config.transcript_dir)?;

        iteration += 1;
    }

    Ok(())
}

async fn build_iteration(
    harness: &dyn Harness,
    transcript: &Transcript,
    config: &LoopConfig,
) -> Result<()> {
    // Get next task from SCUD
    let task = scud::next(&config.scud_path)?
        .ok_or(Error::NoTasksReady)?;

    info!("Working on task {}: {}", task.id, task.title);

    // Phase 1: Parallel searchers
    let search_results = join_all(vec![
        spawn_subagent(harness, Category::Searcher,
            format!("find implementations of {}", task.title), transcript),
        spawn_subagent(harness, Category::Searcher,
            format!("find tests for {}", task.title), transcript),
        spawn_subagent(harness, Category::Analyzer,
            format!("analyze specs relevant to {}", task.title), transcript),
    ]).await;

    // Phase 2: Single builder with context
    let build_context = SearchContext::from_results(&search_results);
    let builder = spawn_subagent(harness, Category::Builder,
        format!("implement: {}\n\nContext:\n{}", task.description, build_context),
        transcript,
    ).await?;

    // Phase 3: Validator (backpressure)
    let validation = spawn_subagent(harness, Category::Validator,
        "run cargo test",
        transcript,
    ).await?;

    if validation.passed() {
        scud::complete(&config.scud_path, &task.id)?;
        git_commit(&task.title)?;
        info!("Task {} completed and committed", task.id);
    } else {
        warn!("Task {} failed validation, will retry", task.id);
    }

    Ok(())
}
```

---

## Subagent Spawning (1 Level Only)

```rust
pub async fn spawn_subagent(
    harness: &dyn Harness,
    category: Category,
    prompt: String,
    parent_transcript: &Transcript,
) -> Result<SubagentResult> {
    let config = category.session_config();

    // Create isolated session
    let session = harness.start_session(config).await?;

    // Create child transcript linked to parent
    let transcript = Transcript::new()
        .harness(harness.name())
        .parent(parent_transcript.id())
        .category(category);

    // Send prompt and collect response
    let mut response = harness.send(&session, &prompt).await?;

    while let Some(chunk) = response.next().await {
        transcript.record(chunk);

        // Check for nested subagent attempts - BLOCK THEM
        if harness.detect_subagent_spawn(&chunk).is_some() {
            warn!("Subagent attempted to spawn nested subagent - blocked");
            // Inject error response instead of spawning
            harness.inject_result(&session, SubagentResult::blocked(
                "Subagents cannot spawn further subagents"
            )).await?;
        }
    }

    // Save child transcript
    transcript.save_scg(&config.transcript_dir)?;

    // Record in parent transcript
    parent_transcript.record_subagent(transcript.id(), &category, &prompt);

    Ok(transcript.into_result())
}
```

---

## Dependencies

```toml
[package]
name = "descartes"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1", features = ["derive"] }

# HTTP client (for API harnesses)
reqwest = { version = "0.12", features = ["json", "stream"] }

# SCG parsing (reuse from SCUD or implement)
# scud-core = { path = "../scud/core" }  # if embedding
nom = "7"  # for SCG parser

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1"
anyhow = "1"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
dirs = "5"

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
```

---

## Migration Path

1. **Phase 1**: Create new `descartes` crate alongside existing
2. **Phase 2**: Implement harness layer (Claude Code first)
3. **Phase 3**: Add SCUD integration
4. **Phase 4**: Port SCG transcript format
5. **Phase 5**: Build CLI
6. **Phase 6**: Add OpenCode/Codex harnesses
7. **Phase 7**: Deprecate old Descartes workspace

Old Descartes stays in `descartes/` subdirectory until v2 is ready.

---

## Open Questions

1. **SCUD embedding** - Link to scud-core crate or shell out to `scud` binary?
2. **Harness auto-detection** - Detect which harness is available at runtime?
3. **Prompt templates** - Embed in binary or load from disk?
4. **Config format** - TOML or keep consistent with SCUD?

---

## Success Criteria

- [ ] Single binary, `cargo install descartes` works
- [ ] Claude Code headless harness functional
- [ ] Subagents spawn with full transcript visibility
- [ ] 1-level subagent enforcement works
- [ ] SCG transcripts are 75%+ smaller than JSON
- [ ] `descartes loop` runs Ralph-style
- [ ] SCUD integration for task management
- [ ] < 5k LOC total
