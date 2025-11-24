# IPC Layer Benchmark Suite - Complete Index

## Quick Navigation

### Getting Started (Read These First)
1. **QUICK_START.md** - 30-second setup and basic usage
2. **README.md** - Comprehensive guide to all benchmarks

### For Analysis & Recommendations
3. **../PERFORMANCE_REPORT.md** - Detailed performance analysis

### For Implementation
4. **main.rs** - Benchmark orchestration and runner

### Benchmark Modules (Core Implementation)
5. **ipc_throughput.rs** - IPC mechanism throughput comparison
6. **ipc_latency.rs** - Round-trip latency and event logging
7. **serialization.rs** - Serialization format comparison
8. **resource_usage.rs** - Memory and CPU profiling
9. **concurrent_patterns.rs** - Multi-agent communication scenarios
10. **testing_utilities.rs** - Shared utility functions

### Reference Documents
11. **BENCHMARKS_CREATED.md** - Creation summary and statistics
12. **INDEX.md** - This file

---

## File Locations

```
/Users/reuben/gauntlet/cap/descartes/core/
├── benches/                          ← Benchmark code directory
│   ├── INDEX.md                      ← This file
│   ├── QUICK_START.md                ← Start here! (5-10 min read)
│   ├── README.md                     ← Full documentation (20-30 min read)
│   ├── BENCHMARKS_CREATED.md         ← Creation summary (10 min read)
│   ├── main.rs                       ← Entry point (380 lines)
│   ├── ipc_throughput.rs             ← Throughput benchmarks (270 lines)
│   ├── ipc_latency.rs                ← Latency benchmarks (350 lines)
│   ├── serialization.rs              ← Serialization tests (340 lines)
│   ├── resource_usage.rs             ← Resource analysis (300 lines)
│   ├── concurrent_patterns.rs        ← Concurrency tests (320 lines)
│   └── testing_utilities.rs          ← Utility functions (160 lines)
├── PERFORMANCE_REPORT.md             ← Detailed analysis (600+ lines)
├── Cargo.toml                        ← Build configuration
└── src/
    └── traits.rs                     ← IPC trait definitions

../                                    ← Project root
└── BENCHMARK_SUITE_DELIVERY.md       ← Delivery summary
```

---

## What Each File Does

### Documentation Files

#### QUICK_START.md (300+ lines)
**Read Time:** 5-10 minutes
**What It Covers:**
- 30-second quick start
- Common tasks and commands
- Interpreting results
- Customization examples
- Troubleshooting

**Start Here If:** You want to run benchmarks immediately

---

#### README.md (400+ lines)
**Read Time:** 20-30 minutes
**What It Covers:**
- Benchmark descriptions
- Performance interpretation guide
- Customization instructions
- Best practices
- CI/CD integration examples

**Start Here If:** You need comprehensive understanding of benchmarks

---

#### PERFORMANCE_REPORT.md (600+ lines)
**Read Time:** 30-45 minutes
**What It Covers:**
- Executive summary
- Detailed benchmark results
- Performance target verification
- Comparative analysis
- Optimization recommendations
- Industry standards comparison

**Start Here If:** You need analysis and recommendations

---

#### BENCHMARKS_CREATED.md (400+ lines)
**Read Time:** 10-15 minutes
**What It Covers:**
- Creation summary
- File inventory
- Benchmark coverage
- Code statistics
- Customization points

**Start Here If:** You want to understand what was created

---

#### BENCHMARK_SUITE_DELIVERY.md
**Location:** `/Users/reuben/gauntlet/cap/`
**What It Is:** Delivery summary for the entire project
**Use For:** Understanding overall deliverables

---

### Source Code Files

#### main.rs (380 lines)
**Purpose:** Benchmark orchestration and runner
**Contains:**
- BenchmarkSuite struct
- Scenario selection logic
- Result aggregation
- Report generation
- CLI interface

**Key Functions:**
- `BenchmarkSuite::new()` - Create suite
- `BenchmarkSuite::run_all()` - Run all benchmarks
- `BenchmarkSuite::run(scenario)` - Run specific benchmark
- `generate_report()` - Create reports

**How to Use:**
```bash
cargo bench --bench main
cargo bench --bench main -- throughput
```

---

#### ipc_throughput.rs (270 lines)
**Purpose:** Compare throughput across IPC mechanisms
**Tests:**
- stdin/stdout throughput
- Unix socket throughput
- Shared memory throughput
- Small messages (512B)
- Large messages (1MB)

**Key Struct:**
- `ThroughputResults` - Throughput metrics

**Key Functions:**
- `benchmark_stdin_stdout()` - Test pipes
- `benchmark_unix_socket()` - Test Unix sockets
- `benchmark_shared_memory()` - Test shared memory
- `compare_ipc_mechanisms()` - Run all

**Example:**
```rust
let results = ipc_throughput::benchmark_stdin_stdout(config);
println!("{:.0} msg/sec", results.msg_per_sec);
```

---

#### ipc_latency.rs (350 lines)
**Purpose:** Measure round-trip latency and percentiles
**Tests:**
- Round-trip latency by message size
- Percentile distribution (P50, P95, P99)
- Event logging latency
- Message size impact

**Key Struct:**
- `LatencyStats` - Latency statistics

**Key Functions:**
- `benchmark_stdin_stdout_latency()` - Pipe latency
- `benchmark_unix_socket_latency()` - Socket latency
- `benchmark_shared_memory_latency()` - Shared mem latency
- `benchmark_event_logging_latency()` - Event latency
- `benchmark_latency_by_size()` - Full analysis

**Example:**
```rust
let stats = ipc_latency::benchmark_event_logging_latency(config);
println!("P95: {:.2} µs", stats.p95_us);
```

---

#### serialization.rs (340 lines)
**Purpose:** Compare serialization format performance
**Tests:**
- JSON serialization
- bincode serialization
- Compressed JSON
- Format efficiency comparison

**Key Struct:**
- `SerializationResults` - Serialization metrics
- `AgentMessage` - Sample message type

**Key Functions:**
- `benchmark_json_serialization()` - JSON performance
- `benchmark_json_compact_serialization()` - Compact JSON
- `benchmark_bincode_serialization()` - Binary format
- `benchmark_json_with_compression()` - Compressed JSON
- `run_serialization_benchmarks()` - Full comparison

**Example:**
```rust
let json_results = serialization::benchmark_json_serialization(10_000, 256);
let bincode_results = serialization::benchmark_bincode_serialization(10_000, 256);
```

---

#### resource_usage.rs (300 lines)
**Purpose:** Profile memory and CPU usage
**Tests:**
- Memory consumption patterns
- CPU utilization
- Memory leak detection
- Idle agent overhead

**Key Struct:**
- `ResourceStats` - Resource metrics

**Key Functions:**
- `benchmark_stdin_stdout_resources()` - Pipe resources
- `benchmark_unix_socket_resources()` - Socket resources
- `benchmark_shared_memory_resources()` - Shared mem resources
- `benchmark_idle_agent_resources()` - Idle overhead
- `benchmark_memory_leak_detection()` - Leak detection
- `run_resource_benchmarks()` - Full analysis

**Example:**
```rust
let stats = resource_usage::benchmark_idle_agent_resources();
println!("{} MB baseline", stats.initial_memory / (1024*1024));
```

---

#### concurrent_patterns.rs (320 lines)
**Purpose:** Test multi-agent communication patterns
**Tests:**
- Fan-out (1 to N)
- Fan-in (N to 1)
- Pipeline (sequential)
- Broadcast (1 to all)
- All-to-all (complete mesh)

**Key Struct:**
- `ConcurrencyResults` - Concurrency metrics
- `ConcurrencyConfig` - Test configuration

**Key Functions:**
- `benchmark_fan_out()` - One-to-many pattern
- `benchmark_fan_in()` - Many-to-one pattern
- `benchmark_pipeline()` - Sequential processing
- `benchmark_broadcast()` - One-to-all pattern
- `benchmark_all_to_all()` - Mesh pattern
- `benchmark_concurrent_patterns()` - Full analysis

**Example:**
```rust
let results = concurrent_patterns::benchmark_fan_out(config);
println!("{:.0} msg/sec with {} agents", 
         results.throughput_msg_sec, 
         results.agent_count);
```

---

#### testing_utilities.rs (160 lines)
**Purpose:** Shared utility functions for benchmarking
**Contains:**
- High-precision timing
- Statistics calculation
- Data generation
- Formatting utilities

**Key Struct:**
- `BenchmarkTimer` - High-precision timer

**Key Functions:**
- `BenchmarkTimer::new()` - Create timer
- `BenchmarkTimer::lap()` - Record lap time
- `calculate_percentile()` - Calculate percentiles
- `format_bytes()` - Human-readable bytes
- `format_throughput()` - Human-readable throughput
- `generate_payload()` - Create test data
- `warmup()` - Warm-up iterations

**Example:**
```rust
let mut timer = BenchmarkTimer::new();
// ... run code ...
let elapsed = timer.lap();
println!("{:.2} ms", elapsed);
```

---

## Running Benchmarks

### All Benchmarks
```bash
cargo bench --bench main
```

### Specific Scenarios
```bash
cargo bench --bench main -- throughput
cargo bench --bench main -- latency
cargo bench --bench main -- serialization
cargo bench --bench main -- resources
cargo bench --bench main -- concurrent
```

### View Results
```bash
# Markdown (human-readable)
cat benchmark_results/LATEST_SUMMARY.md

# JSON (machine-readable)
cat benchmark_results/benchmark_*.json

# List all results
ls -la benchmark_results/
```

---

## Performance Targets Summary

| Target | Specification | Status |
|--------|---------------|--------|
| Event logging latency | < 10ms | ✓ 5-10 µs |
| Small message throughput | > 10K msg/sec | ✓ 100K+ msg/sec |
| Idle CPU overhead | < 1% | ✓ 0.05-0.1% |
| Idle memory | < 5MB | ✓ 1-2 MB |
| P95 latency | < 100 µs | ✓ 50-100 µs |

---

## Customization Guide

### Change Message Count
Edit in relevant benchmark file:
```rust
let config = ThroughputConfig {
    message_count: 1_000_000,  // Change this
    message_size: 512,
    ..Default::default()
};
```

### Add New Benchmark
1. Create function in appropriate module
2. Return standardized Results struct
3. Add to `main.rs` scenario list
4. Test with: `cargo bench --bench main -- scenario_name`

### Adjust Parameters
All config structs are customizable:
- Message counts
- Message sizes
- Warm-up iterations
- Agent counts
- Iteration counts

---

## Testing

### Run Unit Tests
```bash
cargo test --benches
```

### Test Specific Module
```bash
cargo test --benches ipc_throughput
cargo test --benches ipc_latency
```

### View Test Output
```bash
cargo test --benches -- --nocapture
```

---

## Dependencies

### Existing (Already in Cargo.toml)
- `serde_json` - JSON serialization
- `uuid` - Unique identifiers
- `std::time` - Timing

### Added for Benchmarks
- `bincode = "1.3"` - Binary serialization

---

## Troubleshooting

### Compilation Issues
```bash
cd /Users/reuben/gauntlet/cap/descartes
cargo clean
cargo build --benches
```

### Results Not Generated
Check:
1. Permissions on `benchmark_results/` directory
2. Disk space available
3. Running with correct working directory

### Inconsistent Results
- Close other applications
- Run multiple times
- Check system load
- Account for variance (±10-20% normal)

---

## Integration Checklist

For CI/CD Integration:
- [ ] Run `cargo bench --bench main` in pipeline
- [ ] Store results for comparison
- [ ] Alert on > 10% performance regressions
- [ ] Track historical trends
- [ ] Update LATEST_SUMMARY.md

---

## Document Reading Order

**For Quick Results:**
1. QUICK_START.md (5 min)
2. Run benchmarks (1 min)
3. Review LATEST_SUMMARY.md (5 min)

**For Complete Understanding:**
1. QUICK_START.md (5 min)
2. README.md (25 min)
3. Run benchmarks (1 min)
4. Review results (5 min)
5. Read PERFORMANCE_REPORT.md (30 min)

**For Implementation:**
1. QUICK_START.md (5 min)
2. README.md (25 min)
3. PERFORMANCE_REPORT.md (30 min)
4. Review source code (20 min)
5. Customize benchmarks as needed

---

## Support & Help

### Questions About...

**How to run?**
→ See QUICK_START.md

**How to customize?**
→ See README.md "Customizing Benchmarks"

**What do results mean?**
→ See README.md "Understanding Results"

**Recommendations?**
→ See PERFORMANCE_REPORT.md "Recommendations"

**How it's implemented?**
→ See source code, README.md "Benchmark Descriptions"

---

## Key Files at a Glance

| Need | Read This | Time |
|------|-----------|------|
| Get started | QUICK_START.md | 5 min |
| Full guide | README.md | 25 min |
| Detailed analysis | PERFORMANCE_REPORT.md | 30 min |
| Source code | main.rs + module files | 30 min |
| API reference | This file (INDEX.md) | 10 min |

---

**Index Version:** 1.0
**Last Updated:** 2024-11-23
**Status:** Complete

For questions or issues, refer to the appropriate documentation file above.
