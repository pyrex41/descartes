//! ZMQ Performance Benchmarks
//!
//! Measures latency and throughput for ZMQ-based agent communication
//! using MessagePack serialization.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use descartes_core::zmq_agent_runner::{
    deserialize_zmq_message, serialize_zmq_message, validate_message_size, HealthCheckRequest,
    SpawnRequest, ZmqMessage,
};
use descartes_core::AgentConfig;
use std::collections::HashMap;
use uuid::Uuid;

/// Benchmark MessagePack serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("zmq_serialization");

    // Small message (health check)
    let health_msg = ZmqMessage::HealthCheckRequest(HealthCheckRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    // Medium message (spawn request)
    let spawn_msg = ZmqMessage::SpawnRequest(SpawnRequest {
        request_id: Uuid::new_v4().to_string(),
        config: AgentConfig {
            name: "test-agent".to_string(),
            model_backend: "anthropic".to_string(),
            task: "Hello, agent!".to_string(),
            context: None,
            system_prompt: None,
            environment: HashMap::new(),
        },
        timeout_secs: Some(300),
        metadata: None,
    });

    // Serialize benchmarks
    group.bench_function("health_check_serialize", |b| {
        b.iter(|| serialize_zmq_message(black_box(&health_msg)))
    });

    group.bench_function("spawn_request_serialize", |b| {
        b.iter(|| serialize_zmq_message(black_box(&spawn_msg)))
    });

    // Deserialize benchmarks
    let health_bytes = serialize_zmq_message(&health_msg).unwrap();
    let spawn_bytes = serialize_zmq_message(&spawn_msg).unwrap();

    group.bench_function("health_check_deserialize", |b| {
        b.iter(|| deserialize_zmq_message(black_box(&health_bytes)))
    });

    group.bench_function("spawn_request_deserialize", |b| {
        b.iter(|| deserialize_zmq_message(black_box(&spawn_bytes)))
    });

    group.finish();
}

/// Benchmark round-trip serialization
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("zmq_roundtrip");

    let msg = ZmqMessage::SpawnRequest(SpawnRequest {
        request_id: Uuid::new_v4().to_string(),
        config: AgentConfig {
            name: "roundtrip-test".to_string(),
            model_backend: "anthropic".to_string(),
            task: "Test prompt for roundtrip benchmark".to_string(),
            context: Some("Additional context for testing".to_string()),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            environment: HashMap::new(),
        },
        timeout_secs: Some(600),
        metadata: None,
    });

    group.bench_function("serialize_then_deserialize", |b| {
        b.iter(|| {
            let bytes = serialize_zmq_message(black_box(&msg)).unwrap();
            deserialize_zmq_message(black_box(&bytes))
        })
    });

    group.finish();
}

/// Benchmark message size validation
fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("zmq_validation");

    // Various message sizes
    for size in [100, 1000, 10000, 100000].iter() {
        group.bench_with_input(BenchmarkId::new("validate_size", size), size, |b, &size| {
            b.iter(|| validate_message_size(black_box(size)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_serialization, bench_roundtrip, bench_validation);
criterion_main!(benches);
