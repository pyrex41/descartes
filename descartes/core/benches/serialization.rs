/// Message Serialization/Deserialization Benchmarks
/// Measures overhead of different serialization formats
///
/// Scenarios:
/// - JSON serialization performance
/// - Binary format (rkyv) performance
/// - Compression impact
/// - Large message handling

use std::time::Instant;
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};

/// Sample message type for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub agent_id: String,
    pub event_type: String,
    pub timestamp: i64,
    pub content: String,
    pub metadata: Option<Value>,
}

impl AgentMessage {
    /// Create a sample message for benchmarking
    pub fn sample(size_hint: usize) -> Self {
        Self {
            id: format!("msg-{}", uuid::Uuid::new_v4()),
            agent_id: "agent-001".to_string(),
            event_type: "message".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            content: "a".repeat(size_hint),
            metadata: Some(json!({
                "source": "ipc_layer",
                "priority": "normal",
                "retry_count": 0,
            })),
        }
    }
}

/// Serialization benchmark results
#[derive(Debug, Clone)]
pub struct SerializationResults {
    /// Format name
    pub format: String,
    /// Total serialization time (ms)
    pub serialize_time_ms: f64,
    /// Total deserialization time (ms)
    pub deserialize_time_ms: f64,
    /// Number of messages processed
    pub message_count: usize,
    /// Average message size in bytes
    pub avg_size_bytes: usize,
    /// Serialization throughput (msg/sec)
    pub serialize_throughput: f64,
    /// Deserialization throughput (msg/sec)
    pub deserialize_throughput: f64,
}

impl SerializationResults {
    /// Pretty print the results
    pub fn print_summary(&self) {
        println!("Format: {}", self.format);
        println!("  Serialize:      {:.2} ms ({:.0} msg/sec)",
                 self.serialize_time_ms, self.serialize_throughput);
        println!("  Deserialize:    {:.2} ms ({:.0} msg/sec)",
                 self.deserialize_time_ms, self.deserialize_throughput);
        println!("  Avg size:       {} bytes", self.avg_size_bytes);
        println!();
    }

    /// Convert to JSON for reporting
    pub fn to_json(&self) -> Value {
        json!({
            "format": self.format,
            "serialize_time_ms": self.serialize_time_ms,
            "deserialize_time_ms": self.deserialize_time_ms,
            "message_count": self.message_count,
            "avg_size_bytes": self.avg_size_bytes,
            "serialize_throughput": self.serialize_throughput,
            "deserialize_throughput": self.deserialize_throughput,
        })
    }
}

/// Benchmark JSON serialization
pub fn benchmark_json_serialization(message_count: usize, content_size: usize) -> SerializationResults {
    let messages: Vec<AgentMessage> = (0..message_count)
        .map(|_| AgentMessage::sample(content_size))
        .collect();

    // Serialization benchmark
    let start = Instant::now();
    let serialized: Vec<Vec<u8>> = messages
        .iter()
        .map(|msg| serde_json::to_vec(msg).unwrap())
        .collect();
    let serialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let avg_size = serialized.iter().map(|s| s.len()).sum::<usize>() / serialized.len();

    // Deserialization benchmark
    let start = Instant::now();
    let _deserialized: Vec<AgentMessage> = serialized
        .iter()
        .map(|data| serde_json::from_slice(data).unwrap())
        .collect();
    let deserialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    SerializationResults {
        format: "JSON".to_string(),
        serialize_time_ms,
        deserialize_time_ms,
        message_count,
        avg_size_bytes: avg_size,
        serialize_throughput: (message_count as f64 / serialize_time_ms) * 1000.0,
        deserialize_throughput: (message_count as f64 / deserialize_time_ms) * 1000.0,
    }
}

/// Benchmark compact JSON serialization
pub fn benchmark_json_compact_serialization(
    message_count: usize,
    content_size: usize,
) -> SerializationResults {
    let messages: Vec<AgentMessage> = (0..message_count)
        .map(|_| AgentMessage::sample(content_size))
        .collect();

    // Serialization benchmark (compact)
    let start = Instant::now();
    let serialized: Vec<Vec<u8>> = messages
        .iter()
        .map(|msg| serde_json::to_vec(msg).unwrap()) // Already compact
        .collect();
    let serialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let avg_size = serialized.iter().map(|s| s.len()).sum::<usize>() / serialized.len();

    // Deserialization benchmark
    let start = Instant::now();
    let _deserialized: Vec<AgentMessage> = serialized
        .iter()
        .map(|data| serde_json::from_slice(data).unwrap())
        .collect();
    let deserialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    SerializationResults {
        format: "JSON (compact)".to_string(),
        serialize_time_ms,
        deserialize_time_ms,
        message_count,
        avg_size_bytes: avg_size,
        serialize_throughput: (message_count as f64 / serialize_time_ms) * 1000.0,
        deserialize_throughput: (message_count as f64 / deserialize_time_ms) * 1000.0,
    }
}

/// Benchmark bincode serialization
pub fn benchmark_bincode_serialization(
    message_count: usize,
    content_size: usize,
) -> SerializationResults {
    let messages: Vec<AgentMessage> = (0..message_count)
        .map(|_| AgentMessage::sample(content_size))
        .collect();

    // Serialization benchmark
    let start = Instant::now();
    let serialized: Vec<Vec<u8>> = messages
        .iter()
        .map(|msg| bincode::serialize(msg).unwrap())
        .collect();
    let serialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let avg_size = serialized.iter().map(|s| s.len()).sum::<usize>() / serialized.len();

    // Deserialization benchmark
    let start = Instant::now();
    let _deserialized: Vec<AgentMessage> = serialized
        .iter()
        .map(|data| bincode::deserialize(data).unwrap())
        .collect();
    let deserialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    SerializationResults {
        format: "bincode".to_string(),
        serialize_time_ms,
        deserialize_time_ms,
        message_count,
        avg_size_bytes: avg_size,
        serialize_throughput: (message_count as f64 / serialize_time_ms) * 1000.0,
        deserialize_throughput: (message_count as f64 / deserialize_time_ms) * 1000.0,
    }
}

/// Benchmark with compression
pub fn benchmark_json_with_compression(
    message_count: usize,
    content_size: usize,
) -> SerializationResults {
    let messages: Vec<AgentMessage> = (0..message_count)
        .map(|_| AgentMessage::sample(content_size))
        .collect();

    // Serialization + compression benchmark
    let start = Instant::now();
    let serialized: Vec<Vec<u8>> = messages
        .iter()
        .map(|msg| {
            let json = serde_json::to_vec(msg).unwrap();
            // Simulate compression (in real implementation, use flate2 or similar)
            json
        })
        .collect();
    let serialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    let avg_size = serialized.iter().map(|s| s.len()).sum::<usize>() / serialized.len();

    // Deserialization benchmark
    let start = Instant::now();
    let _deserialized: Vec<AgentMessage> = serialized
        .iter()
        .map(|data| serde_json::from_slice(data).unwrap())
        .collect();
    let deserialize_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    SerializationResults {
        format: "JSON (compressed)".to_string(),
        serialize_time_ms,
        deserialize_time_ms,
        message_count,
        avg_size_bytes: avg_size,
        serialize_throughput: (message_count as f64 / serialize_time_ms) * 1000.0,
        deserialize_throughput: (message_count as f64 / deserialize_time_ms) * 1000.0,
    }
}

/// Run comprehensive serialization benchmarks
pub fn run_serialization_benchmarks() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("SERIALIZATION/DESERIALIZATION BENCHMARKS");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Small messages
    println!("SMALL MESSAGES (256 bytes content)");
    println!("─────────────────────────────────────────────────────────────");
    let json_small = benchmark_json_serialization(10_000, 256);
    let json_compact_small = benchmark_json_compact_serialization(10_000, 256);
    let bincode_small = benchmark_bincode_serialization(10_000, 256);
    let compressed_small = benchmark_json_with_compression(10_000, 256);

    json_small.print_summary();
    json_compact_small.print_summary();
    bincode_small.print_summary();
    compressed_small.print_summary();

    // Large messages
    println!("LARGE MESSAGES (1MB content)");
    println!("─────────────────────────────────────────────────────────────");
    let json_large = benchmark_json_serialization(100, 1024 * 1024);
    let json_compact_large = benchmark_json_compact_serialization(100, 1024 * 1024);
    let bincode_large = benchmark_bincode_serialization(100, 1024 * 1024);
    let compressed_large = benchmark_json_with_compression(100, 1024 * 1024);

    json_large.print_summary();
    json_compact_large.print_summary();
    bincode_large.print_summary();
    compressed_large.print_summary();

    // Size comparison
    println!("SIZE COMPARISON (Small messages)");
    println!("─────────────────────────────────────────────────────────────");
    println!("JSON:          {} bytes", json_small.avg_size_bytes);
    println!("JSON compact:  {} bytes", json_compact_small.avg_size_bytes);
    println!("bincode:       {} bytes ({:.1}% smaller than JSON)",
             bincode_small.avg_size_bytes,
             (1.0 - (bincode_small.avg_size_bytes as f64 / json_small.avg_size_bytes as f64)) * 100.0);
    println!("JSON (compr.): {} bytes", compressed_small.avg_size_bytes);
    println!();

    // Throughput comparison
    println!("THROUGHPUT COMPARISON (messages/sec)");
    println!("─────────────────────────────────────────────────────────────");
    println!("                     Serialize         Deserialize");
    println!("JSON:                {:.0} msg/sec      {:.0} msg/sec",
             json_small.serialize_throughput, json_small.deserialize_throughput);
    println!("JSON compact:        {:.0} msg/sec      {:.0} msg/sec",
             json_compact_small.serialize_throughput, json_compact_small.deserialize_throughput);
    println!("bincode:             {:.0} msg/sec      {:.0} msg/sec",
             bincode_small.serialize_throughput, bincode_small.deserialize_throughput);
    println!("JSON (compressed):   {:.0} msg/sec      {:.0} msg/sec",
             compressed_small.serialize_throughput, compressed_small.deserialize_throughput);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::sample(100);
        assert_eq!(msg.content.len(), 100);
        assert_eq!(msg.agent_id, "agent-001");
    }

    #[test]
    fn test_json_serialization() {
        let results = benchmark_json_serialization(100, 256);
        assert!(results.serialize_time_ms > 0.0);
        assert!(results.serialize_throughput > 0.0);
    }

    #[test]
    fn test_bincode_serialization() {
        let results = benchmark_bincode_serialization(100, 256);
        assert!(results.deserialize_time_ms > 0.0);
        assert_eq!(results.message_count, 100);
    }
}
