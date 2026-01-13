# Integration Tests & Crates.io Deployment

## Part 1: Integration Test Plan

### Test Categories

#### 1. End-to-End Ralph Loop Tests (E2E)
Real integration tests that exercise the full flow with a mock harness.

```
tests/
├── ralph_integration.rs     # Existing unit-level tests
├── e2e/
│   ├── mod.rs
│   ├── mock_harness.rs      # Mock harness that returns canned responses
│   ├── ralph_e2e.rs         # Full Ralph loop tests
│   ├── backpressure_e2e.rs  # Backpressure validation tests
│   └── repair_loop_e2e.rs   # Repair agent tests
```

#### 2. Test Scenarios

##### Scenario 1: Happy Path - All Tasks Complete
```rust
#[tokio::test]
async fn test_ralph_completes_all_tasks() {
    // Setup: Create temp dir with SCUD project
    // - 3 tasks in 2 waves
    // - Mock harness returns success for each
    // Expected: All tasks marked done, waves complete in order
}
```

##### Scenario 2: Backpressure Blocks Progression
```rust
#[tokio::test]
async fn test_backpressure_failure_blocks_wave() {
    // Setup: 2 waves, backpressure fails after wave 1
    // Mock harness succeeds, but `cargo build` fails
    // Expected: Wave 2 never starts, tasks stay pending
}
```

##### Scenario 3: Repair Loop Fixes Build Errors
```rust
#[tokio::test]
async fn test_repair_loop_fixes_build() {
    // Setup: Wave 1 completes, backpressure fails
    // Mock repair agent "fixes" the error (touches a file)
    // Backpressure passes on retry
    // Expected: Wave 2 proceeds after repair
}
```

##### Scenario 4: Max Repair Retries Exceeded
```rust
#[tokio::test]
async fn test_repair_loop_max_retries() {
    // Setup: Backpressure always fails
    // Expected: After max_repair_rounds, state becomes BLOCKED
}
```

##### Scenario 5: Context Handoff Triggers at Threshold
```rust
#[tokio::test]
async fn test_context_handoff_at_60_percent() {
    // Setup: Mock harness reports 60% context usage
    // Expected: Agent terminated, summary generated, new agent spawned
}
```

##### Scenario 6: Dry Run Outputs Plan
```rust
#[tokio::test]
async fn test_dry_run_shows_waves_without_execution() {
    // Setup: Project with tasks
    // Expected: dry_run() outputs wave plan, no tasks change status
}
```

##### Scenario 7: Task Blocked by Agent
```rust
#[tokio::test]
async fn test_task_blocked_output_handled() {
    // Setup: Mock harness returns "TASK_BLOCKED: missing dependency"
    // Expected: Task marked blocked, wave continues with other tasks
}
```

### Mock Harness Design

```rust
// tests/e2e/mock_harness.rs

pub struct MockHarness {
    responses: HashMap<String, MockResponse>,
    call_log: Arc<Mutex<Vec<String>>>,
}

pub enum MockResponse {
    Success { output: String },
    Blocked { reason: String },
    ContextOverflow { usage_percent: u8 },
    Timeout,
}

impl MockHarness {
    pub fn new() -> Self { ... }

    /// Set response for a specific task ID
    pub fn when_task(&mut self, task_id: &str, response: MockResponse) { ... }

    /// Get the calls that were made
    pub fn calls(&self) -> Vec<String> { ... }
}

// Implement the Harness trait for MockHarness
#[async_trait]
impl Harness for MockHarness {
    async fn send(&mut self, prompt: &str) -> Result<HarnessResponse> {
        // Log the call
        // Return configured response
    }
}
```

### Mock Backpressure Design

```rust
// tests/e2e/mock_backpressure.rs

pub struct MockBackpressure {
    fail_count: AtomicUsize,
    fail_until: usize,
    error_output: String,
}

impl MockBackpressure {
    /// Fail N times, then succeed
    pub fn fail_then_succeed(n: usize, error: &str) -> Self { ... }

    /// Always fail
    pub fn always_fail(error: &str) -> Self { ... }

    /// Always succeed
    pub fn always_succeed() -> Self { ... }
}
```

### Test Fixtures

```rust
// tests/e2e/fixtures.rs

/// Create a minimal SCUD project in a temp directory
pub fn create_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create .scud/tasks/tasks.scg
    // Create .scud/config.toml
    // Create minimal Cargo.toml for backpressure

    dir
}

/// Create a failing Rust project (for repair tests)
pub fn create_broken_project() -> TempDir {
    let dir = create_test_project();

    // Add src/main.rs with a compile error
    fs::write(dir.path().join("src/main.rs"), "fn main() { undefined_var }").unwrap();

    dir
}
```

---

## Part 2: Crates.io Deployment

### Cargo.toml Updates for Descartes

```toml
[package]
name = "descartes"
version = "0.1.0"
edition = "2021"
description = "Visible subagent orchestration with Ralph-Wiggum loops"
license = "MIT"
repository = "https://github.com/pyrex41/descartes"
homepage = "https://github.com/pyrex41/descartes"
documentation = "https://docs.rs/descartes"
readme = "README.md"
keywords = ["ai", "agents", "orchestration", "llm", "claude"]
categories = ["command-line-utilities", "development-tools"]

# IMPORTANT: Change from path to version dependency for publishing
[dependencies]
# ... other deps ...
scud = { package = "scud-cli", version = "1.34" }  # NOT path!

[package.metadata.release]
pre-release-commit-message = "chore: release {{version}}"
tag-message = "v{{version}}"
```

### Publishing Checklist

1. **Version Sync**: Ensure SCUD is published first with the version Descartes depends on
2. **Dependency Change**: Switch from path to version dependency before publish
3. **README**: Ensure README.md exists in descartes/descartes/ (or root with include)
4. **Login**: `cargo login` with crates.io token

### Release Script

```bash
#!/bin/bash
# scripts/release.sh

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh <version>"
    exit 1
fi

# 1. Update version in Cargo.toml
sed -i '' "s/^version = .*/version = \"$VERSION\"/" descartes/Cargo.toml

# 2. Ensure scud dependency uses version, not path
if grep -q 'path = "../../scud' descartes/Cargo.toml; then
    echo "ERROR: scud dependency still uses path!"
    echo "Change to: scud = { package = \"scud-cli\", version = \"X.Y\" }"
    exit 1
fi

# 3. Build and test
cargo build --release -p descartes
cargo test -p descartes

# 4. Dry run publish
cargo publish -p descartes --dry-run

# 5. Actually publish (uncomment when ready)
# cargo publish -p descartes

# 6. Tag and push
git add descartes/Cargo.toml
git commit -m "chore: release descartes v$VERSION"
git tag "descartes-v$VERSION"
git push && git push --tags

echo "Published descartes v$VERSION"
```

### GitHub Actions CI/CD

```yaml
# .github/workflows/release-descartes.yml
name: Release Descartes

on:
  push:
    tags:
      - 'descartes-v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install protoc
        run: sudo apt-get install -y protobuf-compiler

      - name: Verify version matches tag
        run: |
          TAG_VERSION=${GITHUB_REF#refs/tags/descartes-v}
          CARGO_VERSION=$(grep '^version' descartes/descartes/Cargo.toml | sed 's/.*"\(.*\)"/\1/')
          if [ "$TAG_VERSION" != "$CARGO_VERSION" ]; then
            echo "Tag version ($TAG_VERSION) doesn't match Cargo.toml ($CARGO_VERSION)"
            exit 1
          fi

      - name: Publish to crates.io
        run: cargo publish -p descartes
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

---

## Part 3: Development vs Published Dependencies

### For Development (local path)
```toml
# Use when developing SCUD and Descartes together
[dependencies]
scud = { package = "scud-cli", path = "../../scud/scud-cli" }
```

### For Publishing (version)
```toml
# Use for crates.io publish
[dependencies]
scud = { package = "scud-cli", version = "1.34" }
```

### Feature Flag Approach (Optional)
```toml
[features]
default = []
local-scud = []

[dependencies]
scud = { package = "scud-cli", version = "1.34", optional = true }

[target.'cfg(feature = "local-scud")'.dependencies]
scud = { package = "scud-cli", path = "../../scud/scud-cli" }
```

Then use `cargo build --features local-scud` for development.

---

## Part 4: Running Integration Tests

```bash
# Run all integration tests
cargo test --test '*' -p descartes

# Run specific e2e test
cargo test --test e2e_ralph -p descartes

# Run with logging
RUST_LOG=debug cargo test --test e2e_ralph -p descartes -- --nocapture
```

### Test Environment Setup

Integration tests need:
1. Temp directory for SCUD project
2. Mock harness (no real API calls)
3. Controlled backpressure (mock cargo build)

Tests should NOT require:
- API keys
- Internet connection
- Real Claude/OpenCode processes
