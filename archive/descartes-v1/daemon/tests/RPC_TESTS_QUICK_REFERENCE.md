# RPC Server Tests - Quick Reference

## Quick Start

### Run All Tests
```bash
cd /home/user/descartes/descartes
cargo test --package descartes-daemon --test rpc_server_tests
```

### Run Specific Test
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_approve_task_success
```

## Test Categories (52 tests total)

### 🔌 Server Lifecycle (3 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_server
```
- Start/stop functionality
- Socket cleanup
- Multiple client connections

### 📋 list_tasks Method (6 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_list_tasks
```
- Empty/populated lists
- Status filtering
- Agent filtering
- Multiple filter criteria

### ✅ approve Method (7 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_approve
```
- Success/rejection
- Invalid task IDs
- Metadata preservation
- State transitions

### 📊 get_state Method (3 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_get_state
```
- System-level state
- Invalid entity IDs
- Non-existent agents

### 🚀 spawn Method (3 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_spawn
```
- Basic spawning
- Full/minimal configuration

### ⚠️ Error Handling (4 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_invalid
```
- Invalid JSON
- Unknown methods
- Missing/wrong parameters

### 🔄 Concurrent Requests (3 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_concurrent
```
- 10 simultaneous requests
- Mixed request types
- Concurrent approvals

### ⚡ Performance (3 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_rapid
```
- Timeout validation
- 50 sequential requests
- 100-task performance

### 📝 Validation (5 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_json_rpc
```
- Protocol compliance
- Request ID preservation
- Structure validation

### 🎯 Edge Cases (5 tests)
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_very
```
- Empty filters
- Long titles
- Special characters
- Multiple state transitions

## Common Commands

### Run with Output
```bash
cargo test --package descartes-daemon --test rpc_server_tests -- --nocapture
```

### Run with Debug Logging
```bash
RUST_LOG=debug cargo test --package descartes-daemon --test rpc_server_tests -- --nocapture
```

### Run with Backtrace
```bash
RUST_BACKTRACE=1 cargo test --package descartes-daemon --test rpc_server_tests
```

### Run Tests in Serial (not parallel)
```bash
cargo test --package descartes-daemon --test rpc_server_tests -- --test-threads=1
```

### List All Tests
```bash
cargo test --package descartes-daemon --test rpc_server_tests -- --list
```

## Test Results Interpretation

### Success
```
running 52 tests
test test_approve_task_success ... ok
...
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Failure Example
```
test test_approve_task_success ... FAILED

failures:
    test_approve_task_success

---- test_approve_task_success stdout ----
thread 'test_approve_task_success' panicked at 'assertion failed: ...'
```

## Common Issues & Solutions

### Issue: Tests Won't Compile
**Error**: `error: could not compile descartes-core`
**Solution**: Fix compilation errors in descartes-core (see PHASE3_1.4_RPC_SERVER_TESTS_IMPLEMENTATION.md)

### Issue: Socket Permission Denied
**Error**: `Permission denied (os error 13)`
**Solution**: Check socket directory permissions or run with appropriate permissions

### Issue: Socket Already in Use
**Error**: `Address already in use`
**Solution**: Tests use temporary directories, this shouldn't happen. If it does, clean up `/tmp/`

### Issue: Test Hangs
**Error**: Test doesn't complete
**Solution**:
```bash
# Kill hanging tests
pkill -9 cargo
# Check for zombie processes
ps aux | grep descartes
```

### Issue: Random Test Failures
**Error**: Tests pass sometimes, fail other times
**Solution**: Check for race conditions, ensure proper synchronization with:
```rust
tokio::time::sleep(Duration::from_millis(100)).await;
```

## Test File Structure

```
descartes/daemon/tests/
├── rpc_server_tests.rs          # Main test file (52 tests)
├── RPC_SERVER_TESTS_README.md   # Comprehensive documentation
└── RPC_TESTS_QUICK_REFERENCE.md # This file
```

## Key Helper Functions

### setup_test_server()
Creates complete test environment: server, socket, database
```rust
let (server, socket_path, state_store) = setup_test_server().await;
```

### create_rpc_request()
Builds JSON-RPC 2.0 request
```rust
let request = create_rpc_request("list_tasks", json!([null]), 1);
```

### send_rpc_request()
Sends request and receives response
```rust
let response = send_rpc_request(&socket_path, &request).await.unwrap();
```

### create_test_task()
Creates task in database
```rust
let task = create_test_task(&state_store, "Title", TaskStatus::Todo).await;
```

## Error Codes Reference

| Code | Meaning | Example Test |
|------|---------|--------------|
| -32700 | Parse error | test_invalid_json_request |
| -32601 | Method not found | test_invalid_method_name |
| -32602 | Invalid params | test_missing_required_params |
| -32603 | Internal error | test_approve_nonexistent_task |

## Task Status Transitions

```
Todo → InProgress  (approval)
Todo → Blocked     (rejection)
InProgress → Blocked  (rejection after approval)
```

## Testing Checklist

Before committing changes:
- [ ] All tests pass: `cargo test --package descartes-daemon --test rpc_server_tests`
- [ ] No warnings: `cargo clippy --package descartes-daemon`
- [ ] Formatted: `cargo fmt --package descartes-daemon`
- [ ] Documentation updated if API changed
- [ ] New tests added for new functionality

## Performance Targets

| Metric | Target | Test |
|--------|--------|------|
| Single request | < 100ms | test_request_with_timeout |
| 50 sequential | < 5s | test_rapid_sequential_requests |
| 100 tasks list | < 1s | test_large_task_list_performance |
| 10 concurrent | < 2s | test_concurrent_list_tasks_requests |

## CI/CD Integration

### GitHub Actions
```yaml
- name: Run RPC Server Tests
  run: |
    cd descartes
    cargo test --package descartes-daemon --test rpc_server_tests --no-fail-fast
```

### GitLab CI
```yaml
test:rpc-server:
  script:
    - cd descartes
    - cargo test --package descartes-daemon --test rpc_server_tests --no-fail-fast
```

## Coverage Analysis

### Generate Coverage Report
```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --package descartes-daemon --test rpc_server_tests --out Html
```

### View Coverage
```bash
open tarpaulin-report.html
```

## Debugging Individual Tests

### Add println! statements
```rust
println!("Response: {:?}", response);
```

### Run single test with output
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_approve_task_success -- --nocapture
```

### Use debugger (VS Code)
```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug RPC Test",
  "cargo": {
    "args": ["test", "--package", "descartes-daemon", "--test", "rpc_server_tests", "test_approve_task_success", "--no-run"],
  },
  "args": ["--nocapture"]
}
```

## Maintenance Schedule

### Weekly
- Run full test suite
- Check for flaky tests
- Review test coverage

### Monthly
- Update dependencies and rerun tests
- Review performance metrics
- Add tests for new edge cases

### Per Release
- Run tests with release build: `cargo test --release`
- Stress test with high load
- Verify all documentation is current

## Resources

- **Test Implementation**: `/home/user/descartes/descartes/daemon/tests/rpc_server_tests.rs`
- **Full Documentation**: `/home/user/descartes/descartes/daemon/tests/RPC_SERVER_TESTS_README.md`
- **Implementation Report**: `/home/user/descartes/PHASE3_1.4_RPC_SERVER_TESTS_IMPLEMENTATION.md`
- **RPC Server Code**: `/home/user/descartes/descartes/daemon/src/rpc_server.rs`
- **Unix Socket Docs**: `/home/user/descartes/descartes/daemon/UNIX_SOCKET_RPC.md`

## Quick Test Matrix

| Feature | Unit | Integration | Edge | Concurrent | Performance |
|---------|------|-------------|------|------------|-------------|
| list_tasks | ✅ | ✅ | ✅ | ✅ | ✅ |
| approve | ✅ | ✅ | ✅ | ✅ | ✅ |
| get_state | ✅ | ✅ | ✅ | ❌ | ❌ |
| spawn | ✅ | ⚠️ | ⚠️ | ❌ | ❌ |

Legend: ✅ Complete | ⚠️ Partial | ❌ Not tested

---

**Last Updated**: 2025-11-24
**Test Count**: 52
**Test File**: rpc_server_tests.rs
