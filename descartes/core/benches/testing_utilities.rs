/// Performance Testing Utilities
/// Shared utilities for benchmark implementations
///
/// Provides:
/// - Timer utilities
/// - Statistics calculation
/// - Data generation
/// - Report formatting

use std::time::Instant;

/// High-precision timer for benchmarking
pub struct BenchmarkTimer {
    start: Instant,
    laps: Vec<f64>,
}

impl BenchmarkTimer {
    /// Create a new timer
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            laps: Vec::new(),
        }
    }

    /// Record a lap time in milliseconds
    pub fn lap(&mut self) -> f64 {
        let elapsed = self.start.elapsed().as_secs_f64() * 1000.0;
        self.laps.push(elapsed);
        elapsed
    }

    /// Reset the timer
    pub fn reset(&mut self) {
        self.start = Instant::now();
        self.laps.clear();
    }

    /// Get total elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    /// Get average lap time
    pub fn avg_lap_ms(&self) -> f64 {
        if self.laps.is_empty() {
            0.0
        } else {
            self.laps.iter().sum::<f64>() / self.laps.len() as f64
        }
    }

    /// Get min/max lap times
    pub fn min_max_laps(&self) -> (f64, f64) {
        if self.laps.is_empty() {
            (0.0, 0.0)
        } else {
            let min = self.laps.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = self.laps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        }
    }
}

impl Default for BenchmarkTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate payload data for benchmarking
pub fn generate_payload(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

/// Generate a sequence of payloads
pub fn generate_payloads(count: usize, size: usize) -> Vec<Vec<u8>> {
    (0..count).map(|_| generate_payload(size)).collect()
}

/// Calculate percentile from sorted values
pub fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let index = (values.len() as f64 * (percentile / 100.0)) as usize;
    values[index.min(values.len() - 1)]
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as usize, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Format throughput as human-readable string
pub fn format_throughput(msg_per_sec: f64) -> String {
    if msg_per_sec < 1_000.0 {
        format!("{:.0} msg/sec", msg_per_sec)
    } else if msg_per_sec < 1_000_000.0 {
        format!("{:.2}K msg/sec", msg_per_sec / 1_000.0)
    } else {
        format!("{:.2}M msg/sec", msg_per_sec / 1_000_000.0)
    }
}

/// Warmup function to stabilize performance
pub fn warmup(iterations: usize, payload_size: usize) {
    let payloads = generate_payloads(iterations, payload_size);
    for _ in payloads.iter() {
        // Simulate processing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer = BenchmarkTimer::new();
        assert_eq!(timer.laps.len(), 0);
    }

    #[test]
    fn test_timer_lap() {
        let mut timer = BenchmarkTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.lap();
        assert!(elapsed > 5.0);
        assert_eq!(timer.laps.len(), 1);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let p50 = calculate_percentile(&values, 50.0);
        assert!(p50 > 0.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_format_throughput() {
        assert_eq!(format_throughput(100.0), "100 msg/sec");
        assert_eq!(format_throughput(10_000.0), "10.00K msg/sec");
    }
}
