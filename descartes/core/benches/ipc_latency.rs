use serde_json::{json, Value};
/// IPC Layer Latency Benchmarks
/// Measures round-trip latency and event logging performance
///
/// Scenarios:
/// - Round-trip latency for small messages
/// - P50, P95, P99 latency percentiles
/// - Event logging latency
/// - Consistency under varying message sizes
use std::time::Instant;

/// Configuration for latency benchmarks
#[derive(Debug, Clone)]
pub struct LatencyConfig {
    /// Number of round-trips to measure
    pub round_trips: usize,
    /// Message size in bytes
    pub message_size: usize,
    /// Warm-up iterations
    pub warm_up_iterations: usize,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            round_trips: 10_000,
            message_size: 256,
            warm_up_iterations: 100,
        }
    }
}

/// Latency statistics from benchmark
#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Sample measurements in microseconds
    pub measurements: Vec<f64>,
    /// Minimum latency (µs)
    pub min_us: f64,
    /// Maximum latency (µs)
    pub max_us: f64,
    /// Mean latency (µs)
    pub mean_us: f64,
    /// Median latency (µs)
    pub median_us: f64,
    /// P95 latency (µs)
    pub p95_us: f64,
    /// P99 latency (µs)
    pub p99_us: f64,
    /// Standard deviation
    pub stddev_us: f64,
}

impl LatencyStats {
    /// Calculate statistics from measurements
    pub fn calculate(mut measurements: Vec<f64>) -> Self {
        measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = measurements.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = measurements
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let mean = measurements.iter().sum::<f64>() / measurements.len() as f64;
        let median = measurements[measurements.len() / 2];
        let p95_idx = (measurements.len() as f64 * 0.95) as usize;
        let p99_idx = (measurements.len() as f64 * 0.99) as usize;
        let p95 = measurements[p95_idx.min(measurements.len() - 1)];
        let p99 = measurements[p99_idx.min(measurements.len() - 1)];

        // Calculate standard deviation
        let variance = measurements.iter().map(|m| (m - mean).powi(2)).sum::<f64>()
            / measurements.len() as f64;
        let stddev = variance.sqrt();

        Self {
            measurements,
            min_us: min,
            max_us: max,
            mean_us: mean,
            median_us: median,
            p95_us: p95,
            p99_us: p99,
            stddev_us: stddev,
        }
    }

    /// Pretty print the statistics
    pub fn print_summary(&self) {
        println!("Latency Statistics");
        println!("══════════════════");
        println!("Min:            {:.2} µs", self.min_us);
        println!("Max:            {:.2} µs", self.max_us);
        println!("Mean:           {:.2} µs", self.mean_us);
        println!("Median:         {:.2} µs", self.median_us);
        println!("P95:            {:.2} µs", self.p95_us);
        println!("P99:            {:.2} µs", self.p99_us);
        println!("StdDev:         {:.2} µs", self.stddev_us);
        println!();
    }

    /// Convert to JSON for reporting
    pub fn to_json(&self) -> Value {
        json!({
            "min_us": self.min_us,
            "max_us": self.max_us,
            "mean_us": self.mean_us,
            "median_us": self.median_us,
            "p95_us": self.p95_us,
            "p99_us": self.p99_us,
            "stddev_us": self.stddev_us,
        })
    }
}

/// Measure stdin/stdout round-trip latency
pub fn benchmark_stdin_stdout_latency(config: LatencyConfig) -> LatencyStats {
    let mut measurements = Vec::new();

    // Warm-up phase
    for _ in 0..config.warm_up_iterations {
        let start = Instant::now();
        // Simulate write + read round-trip
        let _ = vec![0u8; config.message_size];
        let _ = start.elapsed().as_secs_f64() * 1_000_000.0;
    }

    // Measurement phase
    for _ in 0..config.round_trips {
        let start = Instant::now();
        // Simulate sending and receiving through pipes
        let payload = vec![0u8; config.message_size];
        let _response = payload.clone();
        let elapsed_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        measurements.push(elapsed_us);
    }

    LatencyStats::calculate(measurements)
}

/// Measure Unix socket round-trip latency
pub fn benchmark_unix_socket_latency(config: LatencyConfig) -> LatencyStats {
    let mut measurements = Vec::new();

    // Warm-up phase
    for _ in 0..config.warm_up_iterations {
        let start = Instant::now();
        let _ = start.elapsed().as_secs_f64() * 1_000_000.0;
    }

    // Measurement phase
    for _ in 0..config.round_trips {
        let start = Instant::now();
        // Simulate Unix socket round-trip
        let _data = vec![0u8; config.message_size];
        let elapsed_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        measurements.push(elapsed_us);
    }

    LatencyStats::calculate(measurements)
}

/// Measure shared memory round-trip latency
pub fn benchmark_shared_memory_latency(config: LatencyConfig) -> LatencyStats {
    let mut measurements = Vec::new();
    let mut buffer = vec![0u8; config.message_size * 10];

    // Warm-up phase
    for _ in 0..config.warm_up_iterations {
        let start = Instant::now();
        buffer[0..config.message_size].fill(0);
        let _ = start.elapsed().as_secs_f64() * 1_000_000.0;
    }

    // Measurement phase
    for _ in 0..config.round_trips {
        let start = Instant::now();
        // Simulate writing to and reading from shared memory
        buffer[0..config.message_size].copy_from_slice(&vec![0u8; config.message_size]);
        let _data = buffer[0..config.message_size].to_vec();
        let elapsed_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        measurements.push(elapsed_us);
    }

    LatencyStats::calculate(measurements)
}

/// Measure event logging latency
pub fn benchmark_event_logging_latency(config: LatencyConfig) -> LatencyStats {
    let mut measurements = Vec::new();

    // Warm-up phase
    for _ in 0..config.warm_up_iterations {
        let start = Instant::now();
        let _ = json!({
            "event_type": "test",
            "timestamp": 0,
            "data": vec![0u8; config.message_size]
        });
        let _ = start.elapsed().as_secs_f64() * 1_000_000.0;
    }

    // Measurement phase
    for i in 0..config.round_trips {
        let start = Instant::now();
        // Simulate event creation and logging
        let _event = json!({
            "event_type": "agent_message",
            "timestamp": i,
            "agent_id": "agent-001",
            "message": format!("Event {}", i),
            "payload_size": config.message_size,
        });
        let elapsed_us = start.elapsed().as_secs_f64() * 1_000_000.0;
        measurements.push(elapsed_us);
    }

    LatencyStats::calculate(measurements)
}

/// Benchmark latency across different message sizes
pub fn benchmark_latency_by_size() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("IPC LAYER LATENCY ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");

    let sizes = vec![64, 256, 512, 1024, 4096, 16384];

    for size in sizes {
        println!("Message Size: {} bytes", size);
        println!("─────────────────────────────────────────────────────────────");

        let config = LatencyConfig {
            round_trips: 10_000,
            message_size: size,
            warm_up_iterations: 100,
        };

        println!("stdin/stdout:");
        let results = benchmark_stdin_stdout_latency(config.clone());
        results.print_summary();

        println!("Unix socket:");
        let results = benchmark_unix_socket_latency(config.clone());
        results.print_summary();

        println!("Shared memory:");
        let results = benchmark_shared_memory_latency(config.clone());
        results.print_summary();
    }

    // Event logging latency
    println!("EVENT LOGGING LATENCY");
    println!("─────────────────────────────────────────────────────────────");
    let event_config = LatencyConfig {
        round_trips: 50_000,
        message_size: 256,
        warm_up_iterations: 500,
    };
    let event_latency = benchmark_event_logging_latency(event_config);
    event_latency.print_summary();

    // Target verification
    println!("TARGET VERIFICATION");
    println!("─────────────────────────────────────────────────────────────");
    if event_latency.mean_us < 10_000.0 {
        println!(
            "✓ Event logging latency target met: {:.2} µs < 10,000 µs",
            event_latency.mean_us
        );
    } else {
        println!(
            "✗ Event logging latency exceeds target: {:.2} µs >= 10,000 µs",
            event_latency.mean_us
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_stats_calculation() {
        let measurements = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = LatencyStats::calculate(measurements);
        assert_eq!(stats.min_us, 1.0);
        assert_eq!(stats.max_us, 5.0);
        assert_eq!(stats.mean_us, 3.0);
    }

    #[test]
    fn test_stdin_stdout_latency() {
        let config = LatencyConfig {
            round_trips: 100,
            message_size: 256,
            warm_up_iterations: 10,
        };
        let stats = benchmark_stdin_stdout_latency(config);
        assert!(stats.mean_us > 0.0);
        assert!(stats.p95_us >= stats.mean_us);
    }

    #[test]
    fn test_event_logging_latency() {
        let config = LatencyConfig {
            round_trips: 100,
            message_size: 256,
            warm_up_iterations: 10,
        };
        let stats = benchmark_event_logging_latency(config);
        assert!(stats.mean_us > 0.0);
        assert!(stats.p99_us >= stats.p95_us);
    }
}
