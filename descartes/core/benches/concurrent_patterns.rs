use serde_json::{json, Value};
/// Concurrent Agent Communication Patterns
/// Benchmarks for multi-agent scenarios
///
/// Scenarios:
/// - Fan-out (one agent to many)
/// - Fan-in (many agents to one)
/// - Pipeline (sequential processing)
/// - Broadcast (one to all)
use std::time::Instant;

/// Configuration for concurrency benchmarks
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Number of agents involved
    pub agent_count: usize,
    /// Messages per agent
    pub messages_per_agent: usize,
    /// Message size in bytes
    pub message_size: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            agent_count: 10,
            messages_per_agent: 1_000,
            message_size: 256,
        }
    }
}

/// Results from a concurrency benchmark
#[derive(Debug, Clone)]
pub struct ConcurrencyResults {
    pub pattern: String,
    pub agent_count: usize,
    pub total_messages: usize,
    pub elapsed_ms: f64,
    pub throughput_msg_sec: f64,
    pub avg_latency_us: f64,
}

impl ConcurrencyResults {
    /// Pretty print the results
    pub fn print_summary(&self) {
        println!("Pattern: {}", self.pattern);
        println!("  Agents:        {}", self.agent_count);
        println!("  Total msgs:    {}", self.total_messages);
        println!("  Elapsed:       {:.2} ms", self.elapsed_ms);
        println!("  Throughput:    {:.0} msg/sec", self.throughput_msg_sec);
        println!("  Avg latency:   {:.2} µs", self.avg_latency_us);
        println!();
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        json!({
            "pattern": self.pattern,
            "agent_count": self.agent_count,
            "total_messages": self.total_messages,
            "elapsed_ms": self.elapsed_ms,
            "throughput_msg_sec": self.throughput_msg_sec,
            "avg_latency_us": self.avg_latency_us,
        })
    }
}

/// Benchmark fan-out pattern (1 to N)
pub fn benchmark_fan_out(config: ConcurrencyConfig) -> ConcurrencyResults {
    let start = Instant::now();

    // Simulate one agent sending to N other agents
    let total_messages = config.messages_per_agent * config.agent_count;
    let payload = vec![0u8; config.message_size];

    for _ in 0..config.messages_per_agent {
        for _ in 0..config.agent_count {
            // Simulate sending message
            let _ = payload.clone();
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = (total_messages as f64 / elapsed_ms) * 1000.0;
    let avg_latency = (elapsed_ms * 1000.0) / total_messages as f64;

    ConcurrencyResults {
        pattern: "Fan-out (1→N)".to_string(),
        agent_count: config.agent_count,
        total_messages,
        elapsed_ms,
        throughput_msg_sec: throughput,
        avg_latency_us: avg_latency,
    }
}

/// Benchmark fan-in pattern (N to 1)
pub fn benchmark_fan_in(config: ConcurrencyConfig) -> ConcurrencyResults {
    let start = Instant::now();

    // Simulate N agents sending to one agent
    let total_messages = config.messages_per_agent * config.agent_count;
    let payload = vec![0u8; config.message_size];

    for _ in 0..config.messages_per_agent {
        for _ in 0..config.agent_count {
            // Simulate receiving message
            let _ = payload.clone();
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = (total_messages as f64 / elapsed_ms) * 1000.0;
    let avg_latency = (elapsed_ms * 1000.0) / total_messages as f64;

    ConcurrencyResults {
        pattern: "Fan-in (N→1)".to_string(),
        agent_count: config.agent_count,
        total_messages,
        elapsed_ms,
        throughput_msg_sec: throughput,
        avg_latency_us: avg_latency,
    }
}

/// Benchmark pipeline pattern (sequential)
pub fn benchmark_pipeline(config: ConcurrencyConfig) -> ConcurrencyResults {
    let start = Instant::now();

    // Simulate pipeline: agent 0 -> agent 1 -> agent 2 ... -> agent N
    let total_messages = config.messages_per_agent;
    let payload = vec![0u8; config.message_size];

    for _ in 0..total_messages {
        for _ in 0..config.agent_count {
            // Simulate passing through pipeline stage
            let _ = payload.clone();
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total_operations = total_messages * config.agent_count;
    let throughput = (total_operations as f64 / elapsed_ms) * 1000.0;
    let avg_latency = (elapsed_ms * 1000.0) / total_operations as f64;

    ConcurrencyResults {
        pattern: "Pipeline (sequential)".to_string(),
        agent_count: config.agent_count,
        total_messages: total_operations,
        elapsed_ms,
        throughput_msg_sec: throughput,
        avg_latency_us: avg_latency,
    }
}

/// Benchmark broadcast pattern (1 to all)
pub fn benchmark_broadcast(config: ConcurrencyConfig) -> ConcurrencyResults {
    let start = Instant::now();

    // Simulate broadcast: one message to all agents
    let total_broadcasts = config.messages_per_agent;
    let payload = vec![0u8; config.message_size];

    for _ in 0..total_broadcasts {
        for _ in 0..config.agent_count {
            // Simulate sending broadcast
            let _ = payload.clone();
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total_messages = total_broadcasts * config.agent_count;
    let throughput = (total_messages as f64 / elapsed_ms) * 1000.0;
    let avg_latency = (elapsed_ms * 1000.0) / total_messages as f64;

    ConcurrencyResults {
        pattern: "Broadcast (1→all)".to_string(),
        agent_count: config.agent_count,
        total_messages,
        elapsed_ms,
        throughput_msg_sec: throughput,
        avg_latency_us: avg_latency,
    }
}

/// Benchmark all-to-all pattern
pub fn benchmark_all_to_all(config: ConcurrencyConfig) -> ConcurrencyResults {
    let start = Instant::now();

    let payload = vec![0u8; config.message_size];
    let messages_per_pair = config.messages_per_agent / config.agent_count.max(1);

    // Simulate all agents sending to all other agents
    for _ in 0..messages_per_pair {
        for _ in 0..config.agent_count {
            for _ in 0..config.agent_count {
                if _ != 0 {
                    // Don't send to self
                    let _ = payload.clone();
                }
            }
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let total_messages = config.agent_count * config.agent_count * messages_per_pair;
    let throughput = (total_messages as f64 / elapsed_ms) * 1000.0;
    let avg_latency = (elapsed_ms * 1000.0) / total_messages as f64;

    ConcurrencyResults {
        pattern: "All-to-all".to_string(),
        agent_count: config.agent_count,
        total_messages,
        elapsed_ms,
        throughput_msg_sec: throughput,
        avg_latency_us: avg_latency,
    }
}

/// Run all concurrency benchmarks
pub fn benchmark_concurrent_patterns() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("CONCURRENT AGENT COMMUNICATION PATTERNS");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Small cluster (10 agents)
    println!("SMALL CLUSTER (10 agents, 1K messages each)");
    println!("─────────────────────────────────────────────────────────────");
    let small_config = ConcurrencyConfig {
        agent_count: 10,
        messages_per_agent: 1_000,
        message_size: 256,
    };

    let fan_out = benchmark_fan_out(small_config.clone());
    let fan_in = benchmark_fan_in(small_config.clone());
    let pipeline = benchmark_pipeline(small_config.clone());
    let broadcast = benchmark_broadcast(small_config.clone());
    let all_to_all = benchmark_all_to_all(small_config.clone());

    fan_out.print_summary();
    fan_in.print_summary();
    pipeline.print_summary();
    broadcast.print_summary();
    all_to_all.print_summary();

    // Medium cluster (50 agents)
    println!("MEDIUM CLUSTER (50 agents, 100 messages each)");
    println!("─────────────────────────────────────────────────────────────");
    let medium_config = ConcurrencyConfig {
        agent_count: 50,
        messages_per_agent: 100,
        message_size: 256,
    };

    let fan_out_med = benchmark_fan_out(medium_config.clone());
    let fan_in_med = benchmark_fan_in(medium_config.clone());
    let pipeline_med = benchmark_pipeline(medium_config.clone());
    let broadcast_med = benchmark_broadcast(medium_config.clone());

    fan_out_med.print_summary();
    fan_in_med.print_summary();
    pipeline_med.print_summary();
    broadcast_med.print_summary();

    // Pattern comparison
    println!("PATTERN COMPARISON (10 agents, throughput in msg/sec)");
    println!("─────────────────────────────────────────────────────────────");
    println!("Fan-out:    {:.0}", fan_out.throughput_msg_sec);
    println!("Fan-in:     {:.0}", fan_in.throughput_msg_sec);
    println!("Pipeline:   {:.0}", pipeline.throughput_msg_sec);
    println!("Broadcast:  {:.0}", broadcast.throughput_msg_sec);
    println!("All-to-all: {:.0}", all_to_all.throughput_msg_sec);
    println!();

    // Scalability analysis
    println!("SCALABILITY (throughput ratio: medium/small)");
    println!("─────────────────────────────────────────────────────────────");
    let fan_out_ratio = fan_out_med.throughput_msg_sec / fan_out.throughput_msg_sec;
    let pipeline_ratio = pipeline_med.throughput_msg_sec / pipeline.throughput_msg_sec;

    println!("Fan-out scalability:  {:.2}x", fan_out_ratio);
    println!("Pipeline scalability: {:.2}x", pipeline_ratio);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrency_config() {
        let config = ConcurrencyConfig::default();
        assert_eq!(config.agent_count, 10);
    }

    #[test]
    fn test_fan_out_benchmark() {
        let config = ConcurrencyConfig {
            agent_count: 5,
            messages_per_agent: 100,
            message_size: 256,
        };
        let results = benchmark_fan_out(config);
        assert!(results.elapsed_ms > 0.0);
        assert_eq!(results.agent_count, 5);
    }

    #[test]
    fn test_pipeline_benchmark() {
        let config = ConcurrencyConfig {
            agent_count: 5,
            messages_per_agent: 100,
            message_size: 256,
        };
        let results = benchmark_pipeline(config);
        assert!(results.throughput_msg_sec > 0.0);
    }
}
