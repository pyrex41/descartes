# Descartes Testing Guide

This document explains how to run and write tests for the Descartes AI orchestration system.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Test Organization](#test-organization)
3. [Running Tests](#running-tests)
4. [Mock Infrastructure](#mock-infrastructure)
5. [Feature Flags](#feature-flags)
6. [Writing New Tests](#writing-new-tests)
7. [Continuous Integration](#continuous-integration)

---

## Quick Start

```bash
cd descartes

# Run all tests (mock mode - no API calls)
cargo test

# Run E2E tests specifically
cargo test --test e2e_tests

# Run user story tests
cargo test user_stories

# Run with verbose output
cargo test -- --nocapture
```

---

## Test Organization

```
tests/
├── e2e_tests.rs                 # Main E2E test entry point
├── swarm_integration.rs         # Swarm integration tests
├── e2e/
│   ├── mod.rs                   # E2E module
│   ├── fixtures.rs              # TestProject helper for temp SCUD projects
│   ├── mock_harness.rs          # MockHarness implementing Harness trait
│   └── swarm_e2e.rs             # Swarm-specific E2E tests
└── user_stories/
    ├── mod.rs                   # User story module
    ├── single_agent.rs          # US-23 to US-25: Single-agent workflow
    ├── swarm.rs                 # US-26 to US-28: Swarm execution
    ├── context.rs               # US-29 to US-31: Context management
    ├── harnesses.rs             # US-32 to US-34: Harness implementations
    ├── validation.rs            # US-35 to US-36: Validation pipeline
    ├── transcript.rs            # US-37 to US-38: Transcript system
    ├── git.rs                   # US-41 to US-42: Git automation
    ├── config.rs                # US-43 to US-44: Configuration overrides
    └── combined.rs              # US-45 to US-50: Combined SCUD+Descartes workflows
```

### User Story Coverage

| Module | User Stories | Description |
|--------|--------------|-------------|
| `single_agent` | US-23 to US-25 | Interactive sessions, task implementation, planning |
| `swarm` | US-26 to US-28 | Wave-based execution, progress visualization, agent categories |
| `context` | US-29 to US-31 | Fresh context pattern, subagent injection, depth enforcement |
| `harnesses` | US-32 to US-34 | Claude Code, Codex, OpenCode harness implementations |
| `validation` | US-35 to US-36 | Backpressure validation, code review for fast builds |
| `transcript` | US-37 to US-38 | Transcript recording, session replay |
| `git` | US-41 to US-42 | AI-generated commits, automatic task completion |
| `config` | US-43 to US-44 | Category overrides, task-level overrides |
| `combined` | US-45 to US-50 | Full PRD-to-implementation workflows |

---

## Running Tests

### All Tests (Mock Mode)

```bash
cargo test
```

### Specific Test Categories

```bash
# E2E tests
cargo test --test e2e_tests

# Swarm integration
cargo test --test swarm_integration

# Specific user story module
cargo test user_stories::single_agent
cargo test user_stories::swarm
cargo test user_stories::context
cargo test user_stories::harnesses
cargo test user_stories::validation
cargo test user_stories::transcript
cargo test user_stories::git
cargo test user_stories::config
cargo test user_stories::combined
```

### Single Test

```bash
cargo test test_us23_interactive_starts_session
cargo test test_us26_swarm_executes_waves
```

### Async Tests

Many Descartes tests are async (use `#[tokio::test]`):

```bash
# Run async tests with verbose output
cargo test -- --nocapture

# Filter async tests
cargo test async
```

---

## Mock Infrastructure

### MockHarness (`tests/e2e/mock_harness.rs`)

The `MockHarness` implements the `Harness` trait for testing without real LLM calls:

```rust
use crate::e2e::mock_harness::{MockHarness, MockResponse};

#[tokio::test]
async fn test_agent_execution() {
    // Create mock harness with success response
    let harness = MockHarness::new()
        .with_response(MockResponse::Success {
            output: "Task completed".to_string(),
        });

    // Use harness in test
    let result = harness.execute("implement feature").await;
    assert!(result.is_ok());
}
```

#### Response Types

```rust
pub enum MockResponse {
    Success { output: String },
    Blocked { reason: String },
    ContextOverflow { usage_percent: u8 },
    Timeout,
    SubagentSpawn { category: String, task: String },
    ValidationFailed { errors: Vec<String> },
    ReviewRequired { changes: String },
}
```

#### Configuration

```rust
// Set context limit for overflow testing
let harness = MockHarness::new()
    .with_context_limit(100_000);

// Enable/disable subagent blocking
let harness = MockHarness::new()
    .with_subagent_blocking(true);

// Queue multiple responses
let harness = MockHarness::new()
    .with_responses(vec![
        MockResponse::Success { output: "Step 1".into() },
        MockResponse::Success { output: "Step 2".into() },
    ]);
```

### TestProject (`tests/e2e/fixtures.rs`)

Creates temporary SCUD projects for testing:

```rust
use crate::e2e::fixtures::TestProject;

#[tokio::test]
async fn test_with_scud_project() {
    // Create temp project with git repo
    let project = TestProject::new().await?;

    // Add tasks
    project.add_task("1", "Implement feature A").await?;
    project.add_task("2", "Implement feature B").await?;
    project.add_dependency("2", "1").await?;

    // Set task status
    project.set_status("1", "done").await?;

    // Get next ready task
    let next = project.next_task().await?;
    assert_eq!(next.id, "2");

    // Project cleaned up on drop
}
```

#### Complex Fixtures

```rust
// Create project with multi-wave dependency graph
let project = TestProject::with_diamond_graph().await?;

// Create project with multiple phases
let project = TestProject::with_phases(vec!["phase1", "phase2"]).await?;

// Create project with cross-phase dependencies
let project = TestProject::with_cross_phase_deps().await?;
```

---

## Feature Flags

### Default (Mock Mode)

```bash
cargo test
```

All external dependencies are mocked:
- No LLM API calls
- No real terminal spawning
- Fast, deterministic tests

### Real LLM Integration

```bash
export ANTHROPIC_API_KEY=your_key_here  # For Claude
export XAI_API_KEY=your_key_here        # For xAI
export OPENAI_API_KEY=your_key_here     # For OpenAI

cargo test --features real-llm
```

Tests marked with `#[cfg(feature = "real-llm")]` will make actual API calls.

### Real Terminal Integration

```bash
# Requires tmux installed
cargo test --features real-terminal
```

Tests marked with `#[cfg(feature = "real-terminal")]` will spawn real terminal sessions.

### All Features

```bash
cargo test --features "real-llm real-terminal"
```

---

## Writing New Tests

### Basic Test Structure

```rust
use crate::e2e::fixtures::TestProject;
use crate::e2e::mock_harness::MockHarness;

#[tokio::test]
async fn test_us_example_description() {
    // Arrange: Set up test environment
    let project = TestProject::new().await.unwrap();
    let harness = MockHarness::new()
        .with_response(MockResponse::Success { output: "done".into() });

    // Act: Execute the operation
    let result = some_operation(&project, &harness).await;

    // Assert: Verify the outcome
    assert!(result.is_ok());
    assert_eq!(project.task_status("1").await.unwrap(), "done");
}
```

### Testing Async Operations

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Testing Error Conditions

```rust
#[tokio::test]
async fn test_handles_blocked_response() {
    let harness = MockHarness::new()
        .with_response(MockResponse::Blocked {
            reason: "Permission denied".into(),
        });

    let result = execute_with_harness(&harness).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("blocked"));
}
```

### Testing with Feature Flags

```rust
#[tokio::test]
#[cfg(feature = "real-llm")]
async fn test_real_claude_api() {
    // Only runs with --features real-llm
    let harness = ClaudeCodeHarness::new()?;
    let result = harness.execute("Hello, world").await?;
    assert!(!result.output.is_empty());
}
```

### Best Practices

1. **Use descriptive test names**: `test_us26_swarm_respects_dependencies`
2. **One assertion per concept**: Test one behavior per test function
3. **Clean up resources**: Use `TestProject` for automatic cleanup
4. **Mock external dependencies**: Use `MockHarness` for deterministic tests
5. **Document expected behavior**: Add comments explaining what the test verifies

---

## Continuous Integration

### GitHub Actions

Tests run automatically on:
- Every push to `main`/`master`
- Every pull request

### CI Test Commands

```bash
# Standard CI checks
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt -- --check
cargo test

# Build verification
cargo build --release
```

### Weekly Integration Tests

Real LLM integration tests run weekly with API keys from secrets:

```yaml
# In .github/workflows/test.yml
integration:
  if: github.event_name == 'schedule'
  steps:
    - name: Run integration tests
      env:
        ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
      run: cargo test --features real-llm
```

---

## Current Test Statistics

- **267 tests** total (118 unit + 132 E2E + 17 integration)
- **User story coverage**: US-23 to US-50
- **Mock-based by default**: Fast, deterministic CI
- **Feature flags**: `real-llm`, `real-terminal` for integration testing

### Running the Full Suite

```bash
# Quick check (mock mode)
cargo test

# Full verification
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
cargo test
cargo build --release
```

---

## Troubleshooting

### Tests Fail with "Connection Refused"

Ensure you're running in mock mode (no feature flags) or have valid API keys set.

### Async Test Hangs

Check for missing `.await` calls or deadlocks in async code.

### Temp Directory Cleanup

`TestProject` cleans up automatically. If tests crash, check for leftover temp directories:

```bash
ls /tmp | grep descartes-test
```
