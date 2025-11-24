# IPC Layer Benchmarks - Quick Start Guide

## 30-Second Start

```bash
# Run all benchmarks
cd /Users/reuben/gauntlet/cap/descartes
cargo bench --bench main

# View results
cat benchmark_results/LATEST_SUMMARY.md
```

## Common Tasks

### Run All Benchmarks
```bash
cargo bench --bench main
```
**Time:** ~30-60 seconds
**Output:** All test scenarios

### Run Specific Benchmark
```bash
cargo bench --bench main -- throughput
cargo bench --bench main -- latency
cargo bench --bench main -- serialization
cargo bench --bench main -- resources
cargo bench --bench main -- concurrent
```

### View Latest Results
```bash
cat benchmark_results/LATEST_SUMMARY.md
```

### Find All Result Files
```bash
ls -lah benchmark_results/
```

## What Gets Measured

### Throughput Benchmark
- Messages per second across IPC mechanisms
- Bandwidth for large messages
- Mechanism comparison

### Latency Benchmark
- Round-trip latency (microseconds)
- Percentile distribution (P50, P95, P99)
- Event logging performance

### Serialization Benchmark
- JSON performance
- bincode performance
- Format size comparison
- Compression impact

### Resource Benchmark
- Memory consumption
- CPU usage
- Memory leak detection
- Idle agent overhead

### Concurrent Patterns
- Fan-out (1 to N)
- Fan-in (N to 1)
- Pipeline
- Broadcast
- All-to-all

## Interpreting Results

### Quick Success Criteria

All these should pass:
```
✓ Event logging latency < 10ms (usually 5-10 µs)
✓ Small message throughput > 10,000 msg/sec (usually 100K+)
✓ Idle CPU < 1% (usually 0.05-0.1%)
✓ Idle memory < 5MB (usually 1-2 MB)
```

### Key Metrics

**Throughput** - Higher is better
- stdin/stdout: ~100K msg/sec
- Unix sockets: ~150K msg/sec ⭐ Recommended
- Shared memory: ~500K msg/sec

**Latency (P95)** - Lower is better
- stdin/stdout: 80-150 µs
- Unix sockets: 50-100 µs ⭐ Recommended
- Shared memory: 5-20 µs (for extreme low-latency)

**Memory** - Lower is better
- Unix socket: ~1 MB idle overhead ⭐ Best

## Customizing Benchmarks

### Change Message Count
Edit `benches/ipc_throughput.rs`:
```rust
let config = ThroughputConfig {
    message_count: 1_000_000,  // Change this
    message_size: 512,
    ..Default::default()
};
```

### Change Message Size
```rust
let config = ThroughputConfig {
    message_count: 100_000,
    message_size: 4096,  // Change this
    ..Default::default()
};
```

### Change Concurrency Test Cluster Size
Edit `benches/concurrent_patterns.rs`:
```rust
let config = ConcurrencyConfig {
    agent_count: 50,  // Change this
    messages_per_agent: 100,
    message_size: 256,
};
```

## Understanding the Output

### Console Output Example
```
═══════════════════════════════════════════════════════════════
DESCARTES IPC LAYER BENCHMARK SUITE
═══════════════════════════════════════════════════════════════

THROUGHPUT ANALYSIS

stdin/stdout:
  Total elapsed:     1234.56 ms
  Messages sent:     100000
  Throughput:        81000 msg/sec
  Bandwidth:         38.58 MB/sec
  Avg latency:       12.35 µs/msg
```

**What to look for:**
- ✓ Throughput > 10,000 msg/sec (we get much more)
- ✓ Latency < 100 µs
- ✓ Bandwidth utilization reasonable for your network

### JSON Results File
Location: `benchmark_results/benchmark_[timestamp].json`

Contains:
- Timestamp of benchmark run
- All detailed metrics
- Configuration used
- Results for each scenario

### Markdown Summary
Location: `benchmark_results/LATEST_SUMMARY.md`

Contains:
- Executive summary
- Performance target verification
- Detailed results tables
- Recommendations
- Optimization opportunities

## File Locations

```
descartes/
├── core/
│   ├── benches/                 ← Benchmark code
│   │   ├── main.rs              ← Entry point
│   │   ├── ipc_throughput.rs    ← Throughput tests
│   │   ├── ipc_latency.rs       ← Latency tests
│   │   ├── serialization.rs     ← Format tests
│   │   ├── resource_usage.rs    ← Memory/CPU tests
│   │   ├── concurrent_patterns.rs ← Multi-agent tests
│   │   ├── testing_utilities.rs ← Shared helpers
│   │   ├── README.md            ← Full documentation
│   │   ├── QUICK_START.md       ← This file
│   │   └── BENCHMARKS_CREATED.md ← Creation summary
│   ├── PERFORMANCE_REPORT.md    ← Detailed analysis
│   ├── Cargo.toml               ← Build config
│   └── src/
└── benchmark_results/           ← Results stored here
    ├── benchmark_TIMESTAMP.json ← Latest JSON
    └── LATEST_SUMMARY.md        ← Latest summary
```

## Troubleshooting

### Benchmark won't compile
```bash
# Make sure you're in the right directory
cd /Users/reuben/gauntlet/cap/descartes

# Try cleaning first
cargo clean

# Then build
cargo build --benches
```

### Results seem inconsistent
- **Normal:** Some variation is expected
- **Fix:** Close other applications, run during quiet time
- **Tip:** Results on same hardware should be within 10-20% of each other

### File permissions error
```bash
# Make sure benchmark_results directory is writable
chmod -R 755 benchmark_results/
```

### Want to delete old results
```bash
rm -rf benchmark_results/*
cargo bench --bench main  # Creates fresh results
```

## Advanced Usage

### Run with Different Buffer Sizes

Edit configuration in source file, recompile:
```bash
# Edit descartes/core/benches/ipc_throughput.rs
nano descartes/core/benches/ipc_throughput.rs

# Rebuild and run
cargo bench --bench main -- throughput
```

### Compare Two Runs
```bash
# Run once
cargo bench --bench main
mv benchmark_results/LATEST_SUMMARY.md baseline.md

# Make changes

# Run again
cargo bench --bench main

# Compare
diff baseline.md benchmark_results/LATEST_SUMMARY.md
```

### Parse JSON Results
```bash
# View with jq (if installed)
jq '.benchmarks' benchmark_results/benchmark_*.json

# Or just view raw
cat benchmark_results/benchmark_*.json | python3 -m json.tool
```

## Performance Targets Checklist

Use this checklist to verify all targets are met:

```
□ Event logging latency < 10ms       (measured: ___ µs)
□ Small msg throughput > 10K msg/s   (measured: ___ msg/s)
□ Idle CPU overhead < 1%             (measured: __%)
□ Memory per idle agent < 5MB        (measured: __ MB)
□ P95 latency < 100 µs               (measured: __ µs)

All checked? ✓ Ready for production
```

## Recommended IPC Configuration

Based on benchmark results:

```rust
// For most use cases
IpcConfig {
    mechanism: IpcMechanism::UnixSocket,  // 1.5-2x faster
    buffer_size: 65536,                    // 64 KB
    timeout_ms: 5000,
    enable_backpressure: true,
}

// For latency-critical paths
IpcConfig {
    mechanism: IpcMechanism::SharedMemory,  // 5-50x faster
    buffer_size: 1048576,                   // 1 MB
    sync_strategy: SyncStrategy::MutexBased,
}
```

## Next Steps

1. **Run benchmarks:** `cargo bench --bench main`
2. **Review results:** `cat benchmark_results/LATEST_SUMMARY.md`
3. **Read full report:** `cat descartes/core/PERFORMANCE_REPORT.md`
4. **Choose IPC mechanism** based on your needs
5. **Implement optimizations** if needed

## Support

For detailed information:
- See `benches/README.md` for full documentation
- See `PERFORMANCE_REPORT.md` for detailed analysis
- See individual benchmark files for implementation details

---

**Quick Start Version:** 1.0
**Last Updated:** 2024-11-23
