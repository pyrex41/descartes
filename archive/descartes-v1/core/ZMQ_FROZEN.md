# FROZEN FEATURE

ZMQ distributed agent infrastructure is complete but not integrated.
Enables spawning agents on remote machines via ZeroMQ.

Status: Frozen as of 2026-01-09
Reason: Not needed currently, keep for future distributed deployment
Contact: reuben

## What This Module Does

The ZMQ infrastructure allows Descartes to distribute agent execution across
multiple machines using ZeroMQ for communication. This includes:

- Remote agent spawning and management
- Distributed task queues
- Cross-machine event streaming
- Scalable deployment patterns

## Files

Core implementation:
- `core/src/zmq_agent_runner.rs` - Remote agent execution
- `core/src/zmq_server.rs` - ZMQ server for accepting connections
- `core/src/zmq_client.rs` - Client for connecting to remote servers
- `core/src/zmq_communication.rs` - Message types and protocol

Examples and tests:
- `core/examples/zmq_server_example.rs`
- `core/examples/zmq_client_example.rs`
- `core/examples/zmq_deployment_poc.rs`
- `core/tests/zmq_integration_tests.rs`
- `core/tests/zmq_distributed_integration_tests.rs`
- `core/tests/zmq_server_integration_tests.rs`
- `core/benches/zmq_benchmarks.rs`

Integration with daemon/GUI:
- `daemon/src/zmq_publisher.rs` - Event publishing from daemon
- `gui/src/zmq_subscriber.rs` - Event subscription in GUI

## Why Frozen

The distributed deployment use case isn't needed for current workflows.
Single-machine agent orchestration handles typical development tasks well.

When scaling to multi-machine deployments becomes a priority, this
infrastructure is ready to be activated and integrated with the main
agent runner.
