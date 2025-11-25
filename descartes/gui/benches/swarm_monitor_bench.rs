//! Performance Benchmarks for Swarm Monitor (Phase 3:5.5)
//!
//! This benchmark suite tests performance with various agent counts:
//! - 10, 50, 100, 500, 1000+ agents
//! - Animation frame rates
//! - Filtering and search performance
//! - Batch updates

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use descartes_core::{AgentProgress, AgentRuntimeState, AgentStatus};
use descartes_gui::swarm_monitor::{AgentEvent, SwarmMonitorState};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_agent(name: &str, task: &str) -> AgentRuntimeState {
    AgentRuntimeState::new(
        Uuid::new_v4(),
        name.to_string(),
        task.to_string(),
        "test-backend".to_string(),
    )
}

fn create_test_agents(count: usize) -> Vec<AgentRuntimeState> {
    (0..count)
        .map(|i| create_test_agent(&format!("agent-{}", i), &format!("task-{}", i)))
        .collect()
}

// ============================================================================
// AGENT ADDITION BENCHMARKS
// ============================================================================

fn bench_add_agents(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_agents");

    for agent_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                b.iter(|| {
                    let mut state = SwarmMonitorState::new();
                    let agents = create_test_agents(count);

                    for agent in agents {
                        state.update_agent(black_box(agent));
                    }

                    black_box(state)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// BATCH UPDATE BENCHMARKS
// ============================================================================

fn bench_batch_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_updates");

    for agent_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                b.iter(|| {
                    let mut state = SwarmMonitorState::new();
                    let agents = create_test_agents(count);

                    let agent_map: HashMap<Uuid, AgentRuntimeState> = agents
                        .into_iter()
                        .map(|agent| (agent.agent_id, agent))
                        .collect();

                    state.update_agents_batch(black_box(agent_map));

                    black_box(state)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// FILTERING BENCHMARKS
// ============================================================================

fn bench_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtering");

    for agent_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for (i, mut agent) in agents.into_iter().enumerate() {
                    let status = match i % 4 {
                        0 => AgentStatus::Running,
                        1 => AgentStatus::Thinking,
                        2 => AgentStatus::Completed,
                        _ => AgentStatus::Failed,
                    };
                    agent.transition_to(status, None).ok();
                    state.update_agent(agent);
                }

                // Benchmark filtering
                b.iter(|| {
                    let filtered = state.filtered_agents();
                    black_box(filtered)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// SEARCH BENCHMARKS
// ============================================================================

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    for agent_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for agent in agents {
                    state.update_agent(agent);
                }

                state.search_query = "agent-5".to_string();

                // Benchmark search
                b.iter(|| {
                    let filtered = state.filtered_agents();
                    black_box(filtered)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// ANIMATION TICK BENCHMARKS
// ============================================================================

fn bench_animation_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("animation_tick");

    for agent_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with thinking agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for mut agent in agents {
                    agent.transition_to(AgentStatus::Thinking, None).ok();
                    agent.update_thought("Processing...".to_string());
                    state.update_agent(agent);
                }

                // Benchmark animation tick
                b.iter(|| {
                    state.tick_animation();
                    black_box(&state)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// LIVE EVENT PROCESSING BENCHMARKS
// ============================================================================

fn bench_event_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_processing");

    for agent_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);
                let agent_ids: Vec<Uuid> = agents.iter().map(|a| a.agent_id).collect();

                for agent in agents {
                    state.update_agent(agent);
                }

                // Benchmark event processing
                b.iter(|| {
                    for agent_id in &agent_ids {
                        let event = AgentEvent::AgentStatusChanged {
                            agent_id: *agent_id,
                            status: AgentStatus::Running,
                        };
                        state.handle_agent_event(black_box(event));
                    }
                    black_box(&state)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// STATISTICS COMPUTATION BENCHMARKS
// ============================================================================

fn bench_compute_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("compute_statistics");

    for agent_count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for (i, mut agent) in agents.into_iter().enumerate() {
                    let status = match i % 4 {
                        0 => AgentStatus::Running,
                        1 => AgentStatus::Thinking,
                        2 => AgentStatus::Completed,
                        _ => AgentStatus::Failed,
                    };
                    agent.transition_to(status, None).ok();
                    if i % 2 == 0 {
                        agent.update_progress(AgentProgress::new(50.0));
                    }
                    state.update_agent(agent);
                }

                // Benchmark statistics computation
                b.iter(|| {
                    let stats = state.compute_statistics();
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// PERFORMANCE STATS BENCHMARKS
// ============================================================================

fn bench_performance_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_stats");

    for agent_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for mut agent in agents {
                    agent.transition_to(AgentStatus::Running, None).ok();
                    state.update_agent(agent);
                }

                // Tick a few times to populate frame time data
                for _ in 0..10 {
                    state.tick_animation();
                }

                // Benchmark performance stats computation
                b.iter(|| {
                    let stats = state.get_performance_stats();
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// GROUPING BENCHMARKS
// ============================================================================

fn bench_grouping(c: &mut Criterion) {
    let mut group = c.benchmark_group("grouping");

    for agent_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for (i, mut agent) in agents.into_iter().enumerate() {
                    let status = match i % 4 {
                        0 => AgentStatus::Running,
                        1 => AgentStatus::Thinking,
                        2 => AgentStatus::Completed,
                        _ => AgentStatus::Failed,
                    };
                    agent.transition_to(status, None).ok();
                    state.update_agent(agent);
                }

                // Benchmark grouping
                b.iter(|| {
                    let grouped = state.grouped_agents();
                    black_box(grouped)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// 60 FPS SIMULATION BENCHMARK
// ============================================================================

fn bench_60fps_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("60fps_simulation");

    for agent_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(agent_count),
            agent_count,
            |b, &count| {
                // Setup state with agents
                let mut state = SwarmMonitorState::new();
                let agents = create_test_agents(count);

                for mut agent in agents {
                    agent.transition_to(AgentStatus::Thinking, None).ok();
                    agent.update_thought("Processing...".to_string());
                    state.update_agent(agent);
                }

                // Benchmark 60 animation ticks (1 second at 60 FPS)
                b.iter(|| {
                    for _ in 0..60 {
                        state.tick_animation();
                    }
                    black_box(&state)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

criterion_group!(
    benches,
    bench_add_agents,
    bench_batch_updates,
    bench_filtering,
    bench_search,
    bench_animation_tick,
    bench_event_processing,
    bench_compute_statistics,
    bench_performance_stats,
    bench_grouping,
    bench_60fps_simulation,
);
criterion_main!(benches);
