mod concurrent_patterns;
/// Descartes Benchmark Suite
///
/// Comprehensive performance testing for:
/// - Serialization/deserialization overhead
/// - CPU and memory resource usage
/// - Concurrency and load scenarios
///
/// Usage: cargo bench --bench main -- [SCENARIO]
/// Where SCENARIO is one of: serialization, resources, concurrent, all
mod resource_usage;
mod serialization;
mod testing_utilities;

use serde_json::{json, Value};
use std::fs;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Main benchmark suite
pub struct BenchmarkSuite {
    configs: Vec<BenchmarkConfig>,
}

impl BenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new() -> Self {
        Self {
            configs: vec![
                BenchmarkConfig {
                    name: "serialization".to_string(),
                    description: "Serialization/deserialization overhead".to_string(),
                    enabled: true,
                },
                BenchmarkConfig {
                    name: "resources".to_string(),
                    description: "CPU and memory resource usage".to_string(),
                    enabled: true,
                },
                BenchmarkConfig {
                    name: "concurrent".to_string(),
                    description: "Multi-agent communication patterns".to_string(),
                    enabled: true,
                },
            ],
        }
    }

    /// Run all benchmarks
    pub fn run_all(&self) {
        println!("\n");
        print_header("DESCARTES BENCHMARK SUITE");

        let mut results = json!({
            "timestamp": chrono::Local::now().to_rfc3339(),
            "version": "0.1.0",
            "benchmarks": {},
        });

        for config in &self.configs {
            if config.enabled {
                println!("\n");
                match config.name.as_str() {
                    "serialization" => {
                        serialization::run_serialization_benchmarks();
                        results["benchmarks"]["serialization"] = json!({"status": "completed"});
                    }
                    "resources" => {
                        resource_usage::run_resource_benchmarks();
                        results["benchmarks"]["resources"] = json!({"status": "completed"});
                    }
                    "concurrent" => {
                        concurrent_patterns::benchmark_concurrent_patterns();
                        results["benchmarks"]["concurrent"] = json!({"status": "completed"});
                    }
                    _ => {}
                }
            }
        }

        generate_report(&results);
    }

    /// Run specific benchmark
    pub fn run(&self, scenario: &str) {
        println!("\n");
        print_header(&format!("BENCHMARK: {}", scenario));

        match scenario {
            "serialization" => {
                serialization::run_serialization_benchmarks();
            }
            "resources" => {
                resource_usage::run_resource_benchmarks();
            }
            "concurrent" => {
                concurrent_patterns::benchmark_concurrent_patterns();
            }
            "all" => {
                self.run_all();
                return;
            }
            _ => {
                eprintln!("Unknown scenario: {}", scenario);
                eprintln!("Available: serialization, resources, concurrent, all");
                std::process::exit(1);
            }
        }

        let results = json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "scenario": scenario,
        });

        generate_report(&results);
    }
}

/// Print formatted header
fn print_header(title: &str) {
    let width = 67;
    println!("{}", "═".repeat(width));
    println!("{:^width$}", title);
    println!("{}", "═".repeat(width));
}

/// Generate performance report
pub fn generate_report(results: &Value) {
    println!("\n");
    print_header("BENCHMARK REPORT GENERATION");

    let report_dir = "benchmark_results";
    fs::create_dir_all(report_dir).ok();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string());
    let report_file = format!("{}/benchmark_{}.json", report_dir, timestamp);

    // Write JSON report
    if let Ok(json_str) = serde_json::to_string_pretty(results) {
        fs::write(&report_file, json_str).ok();
        println!("JSON report saved: {}", report_file);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scenario = if args.len() > 1 { &args[1] } else { "all" };

    let suite = BenchmarkSuite::new();

    if scenario == "all" {
        suite.run_all();
    } else {
        suite.run(scenario);
    }

    println!("\n");
    println!("Benchmark completed successfully!");
    println!("Results saved to: benchmark_results/");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config() {
        let config = BenchmarkConfig {
            name: "test".to_string(),
            description: "Test config".to_string(),
            enabled: true,
        };
        assert_eq!(config.name, "test");
        assert!(config.enabled);
    }

    #[test]
    fn test_benchmark_suite_creation() {
        let suite = BenchmarkSuite::new();
        assert!(!suite.configs.is_empty());
    }
}
