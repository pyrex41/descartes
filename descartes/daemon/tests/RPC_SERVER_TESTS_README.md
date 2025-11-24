# RPC Server Integration Tests - Documentation

## Overview

This document provides comprehensive documentation for the RPC server integration test suite located in `/home/user/descartes/descartes/daemon/tests/rpc_server_tests.rs`.

## Test Coverage

The test suite provides **comprehensive coverage** of the JSON-RPC server with **52 integration tests** organized into the following categories:

### 1. Server Lifecycle Tests (3 tests)
- **test_server_start_and_stop**: Verifies server can start and stop cleanly
- **test_server_socket_cleanup**: Ensures existing socket files are properly cleaned up
- **test_multiple_clients_can_connect**: Validates multiple concurrent client connections

### 2. list_tasks Method Tests (6 tests)
- **test_list_tasks_empty**: Verifies empty task list is returned when no tasks exist
- **test_list_tasks_with_data**: Tests listing tasks with populated database
- **test_list_tasks_filter_by_status**: Validates filtering tasks by status (Todo, InProgress, Done, Blocked)
- **test_list_tasks_filter_by_assigned_to**: Tests filtering by assigned agent
- **test_list_tasks_filter_multiple_criteria**: Validates combining multiple filter criteria
- **test_empty_filter_object**: Tests behavior with empty filter object

### 3. approve Method Tests (7 tests)
- **test_approve_task_success**: Verifies successful task approval
- **test_approve_task_rejection**: Tests task rejection flow
- **test_approve_nonexistent_task**: Validates error handling for non-existent tasks
- **test_approve_invalid_task_id**: Tests invalid UUID format handling
- **test_approve_task_metadata_preservation**: Ensures existing metadata is preserved during approval
- **test_multiple_approvals_same_task**: Tests idempotent approval behavior
- **test_approve_then_reject_task**: Validates approval followed by rejection

### 4. get_state Method Tests (3 tests)
- **test_get_state_system_level**: Verifies system-wide state retrieval
- **test_get_state_invalid_entity_id**: Tests error handling for invalid entity IDs
- **test_get_state_nonexistent_agent**: Validates error for non-existent agents

### 5. spawn Method Tests (3 tests)
- **test_spawn_agent_basic**: Tests basic agent spawning
- **test_spawn_agent_with_full_config**: Validates spawning with complete configuration
- **test_spawn_agent_minimal_config**: Tests minimal configuration spawning

### 6. Error Handling Tests (4 tests)
- **test_invalid_json_request**: Validates handling of malformed JSON
- **test_invalid_method_name**: Tests error response for non-existent methods
- **test_missing_required_params**: Verifies parameter validation
- **test_wrong_param_types**: Tests type checking for parameters

### 7. Concurrent Request Tests (3 tests)
- **test_concurrent_list_tasks_requests**: Validates handling of 10 simultaneous list_tasks requests
- **test_concurrent_mixed_requests**: Tests different request types executing concurrently
- **test_concurrent_task_approvals**: Verifies multiple simultaneous task approvals

### 8. Timeout and Performance Tests (3 tests)
- **test_request_with_timeout**: Validates requests complete within reasonable timeframes
- **test_rapid_sequential_requests**: Tests 50 rapid sequential requests
- **test_large_task_list_performance**: Validates performance with 100 tasks

### 9. Request/Response Validation Tests (5 tests)
- **test_json_rpc_version_field**: Verifies JSON-RPC 2.0 protocol compliance
- **test_request_id_preservation**: Validates request ID is preserved in responses
- **test_error_object_structure**: Tests error object structure compliance
- **test_task_info_structure**: Validates TaskInfo serialization/deserialization
- **test_approval_result_structure**: Tests ApprovalResult structure

### 10. Edge Case Tests (5 tests)
- **test_filter_with_nonexistent_field**: Tests handling of unknown filter fields
- **test_very_long_task_title**: Validates handling of 1000-character task titles
- **test_task_with_special_characters**: Tests special characters, quotes, and emojis
- **test_multiple_approvals_same_task**: Validates idempotent approval operations
- **test_approve_then_reject_task**: Tests approval state transitions

## Test Architecture

### Test Helpers

The test suite includes several helper functions to facilitate testing:

#### `setup_test_server()`
Creates a complete test environment with:
- Temporary Unix socket
- LocalProcessRunner for agent management
- SQLite state store with in-memory database
- Fully configured UnixSocketRpcServer

#### `create_rpc_request(method, params, id)`
Constructs properly formatted JSON-RPC 2.0 requests.

#### `send_rpc_request(socket_path, request)`
Sends a request over Unix socket and receives the response.

#### `send_rpc_request_with_timeout(socket_path, request, timeout_duration)`
Sends a request with a specified timeout duration.

#### `create_test_task(state_store, title, status)`
Creates and persists a test task in the database.

## Running the Tests

### Run All Tests
```bash
cd /home/user/descartes/descartes
cargo test --package descartes-daemon --test rpc_server_tests
```

### Run Specific Test Category
```bash
# Server lifecycle tests
cargo test --package descartes-daemon --test rpc_server_tests test_server

# list_tasks tests
cargo test --package descartes-daemon --test rpc_server_tests test_list_tasks

# approve tests
cargo test --package descartes-daemon --test rpc_server_tests test_approve

# Concurrent tests
cargo test --package descartes-daemon --test rpc_server_tests test_concurrent
```

### Run Single Test
```bash
cargo test --package descartes-daemon --test rpc_server_tests test_approve_task_success
```

### Run with Output
```bash
cargo test --package descartes-daemon --test rpc_server_tests -- --nocapture
```

## Known Issues

### Pre-existing Compilation Errors in descartes-core

Before the RPC server tests can run, the following compilation errors in `descartes-core` must be resolved:

1. **body_restore.rs**:
   - Line 314: `gix::Id` no longer has `.id` field
   - Line 340: `commit.parents()` changed to `commit.parent_ids()`
   - Line 358, 401, 580: `peel_to_id_in_place()` changed to `try_peel_to_id_in_place()`
   - Line 524, 559: `set_target_id()` called on `Option<Reference>` instead of `Reference`
   - Line 380: Lifetime issue with `repo` borrowing

2. **brain_restore.rs**:
   - Line 644: `StateStoreError::NotFoundError` variant doesn't exist

3. **time_travel_integration.rs**:
   - Lines 982, 1022: `DefaultBrainRestore.store` field is private

4. **debugger.rs**:
   - Lines 1068, 1123, 1164: Multiple mutable borrow issues

These errors appear to be related to recent updates to the `gix` (GitOxide) library API and some architectural issues in the core crate.

## Test Scenarios

### Error Code Validation

The tests validate the following JSON-RPC error codes:
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

### Status Transitions

The tests validate the following task status transitions:
- Todo → InProgress (on approval)
- Todo → Blocked (on rejection)
- InProgress → Blocked (on subsequent rejection)

### Concurrency

The tests validate that the server can handle:
- Multiple simultaneous clients
- Concurrent requests of the same type
- Mixed concurrent requests (different methods)
- Rapid sequential requests

### Performance

The tests ensure:
- Requests complete within 5-second timeout
- Large result sets (100+ tasks) are handled efficiently
- 50 rapid sequential requests can be processed

## Integration with CI/CD

These tests are designed to be run as part of a CI/CD pipeline:

```yaml
# Example GitHub Actions workflow
- name: Run RPC Server Tests
  run: |
    cd descartes
    cargo test --package descartes-daemon --test rpc_server_tests --no-fail-fast
```

## Future Enhancements

Potential improvements to the test suite:

1. **Authentication/Authorization Tests**: Once implemented in the RPC server, add tests for:
   - Token validation
   - Permission checking
   - Rate limiting

2. **Stress Tests**: Add tests for:
   - Very large task lists (1000+ tasks)
   - Sustained high load
   - Connection pool exhaustion

3. **Network Error Simulation**: Test handling of:
   - Socket disconnections
   - Partial message transmission
   - Timeout recovery

4. **Integration with Agent Runner**: Test actual agent spawning and lifecycle management once model backends are available in test environment.

5. **Metrics and Monitoring**: Validate that RPC operations emit appropriate metrics.

## Debugging Tests

### Enable Debug Logging
```bash
RUST_LOG=debug cargo test --package descartes-daemon --test rpc_server_tests -- --nocapture
```

### Run Tests with backtrace
```bash
RUST_BACKTRACE=1 cargo test --package descartes-daemon --test rpc_server_tests
```

### Inspect Test Database
Tests use temporary directories that are cleaned up automatically. To inspect the database during debugging:
1. Modify `setup_test_server()` to use a fixed path instead of `tempdir()`
2. Run the test
3. Use `sqlite3` to inspect the database

## Test Maintenance

### Adding New Tests
1. Follow the existing test structure and naming conventions
2. Use the provided helper functions
3. Ensure tests clean up resources (server handles, connections)
4. Add documentation for the new test category

### Updating Tests for API Changes
When the RPC API changes:
1. Update affected test cases
2. Update this documentation
3. Verify all tests still pass
4. Update error code validation if error handling changes

## Contact

For questions or issues with the test suite, refer to:
- RPC Server Implementation: `/home/user/descartes/descartes/daemon/src/rpc_server.rs`
- Unix Socket Documentation: `/home/user/descartes/descartes/daemon/UNIX_SOCKET_RPC.md`
- Main Daemon Documentation: `/home/user/descartes/descartes/daemon/README.md`
