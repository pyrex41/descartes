# RAG Layer - Quick Start Guide

## 5-Minute Quick Start

### 1. Install Dependencies

```bash
cd descartes/agent-runner
cargo build
```

### 2. Set API Key

```bash
export OPENAI_API_KEY="sk-your-key-here"
# OR
export ANTHROPIC_API_KEY="your-key-here"
```

### 3. Run the Example

```bash
cargo run --example rag_example
```

### 4. Run Tests

```bash
cargo test rag
```

## Basic Usage

### Minimal Example

```rust
use agent_runner::{RagConfig, RagSystem};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create config with API key
    let mut config = RagConfig::default();
    config.api_key = Some(std::env::var("OPENAI_API_KEY")?);

    // 2. Initialize RAG system
    let mut rag = RagSystem::new(config).await?;

    // 3. Index a file
    let chunks = rag.index_file("src/main.rs").await?;
    println!("Indexed {} chunks", chunks);

    // 4. Search
    let results = rag.hybrid_search("database connection").await?;

    // 5. Print results
    for result in results.iter().take(3) {
        println!("Score: {:.4} | File: {}", result.score, result.chunk.file_path);
        println!("Content preview: {}...\n",
            result.chunk.content.chars().take(100).collect::<String>());
    }

    Ok(())
}
```

## Common Operations

### Index Multiple Files

```rust
let files = vec!["src/main.rs", "src/lib.rs", "src/parser.rs"];

for file in files {
    match rag.index_file(file).await {
        Ok(count) => println!("✓ {} -> {} chunks", file, count),
        Err(e) => eprintln!("✗ {}: {}", file, e),
    }
}
```

### Different Search Types

```rust
// Vector search (semantic similarity)
let results = rag.vector_search("async database queries", 10).await?;

// Full-text search (keyword matching)
let results = rag.fulltext_search("tokio spawn", 10)?;

// Hybrid search (best of both)
let results = rag.hybrid_search("error handling patterns").await?;
```

### View Statistics

```rust
let stats = rag.stats();
println!("Total chunks indexed: {}", stats.total_chunks);
println!("Cache usage: {}/{}", stats.cache_size, stats.cache_capacity);
println!("Embedding dimension: {}", stats.embedding_dimension);
```

### Custom Configuration

```rust
let config = RagConfig {
    // Paths
    lance_db_path: "./my_data/vectors".to_string(),
    tantivy_index_path: "./my_data/fulltext".to_string(),

    // Embedding settings
    embedding_provider: "openai".to_string(),
    embedding_model: "text-embedding-3-small".to_string(),
    api_key: Some(std::env::var("OPENAI_API_KEY")?),

    // Chunking
    max_chunk_size: 1500,    // Larger chunks
    chunk_overlap: 300,       // More overlap

    // Search weights
    vector_weight: 0.8,       // Prefer semantic search
    fulltext_weight: 0.2,

    // Cache
    enable_cache: true,
    max_cache_size: 50000,    // Large cache

    ..Default::default()
};

let rag = RagSystem::new(config).await?;
```

## Testing Without API Keys

Use the mock provider in tests:

```rust
// See tests/rag_integration_test.rs for the MockEmbeddingProvider implementation

use agent_runner::rag::*;

let provider = MockEmbeddingProvider::new(128);
let embeddings = provider.embed_batch(&texts).await?;
// No API key needed!
```

## File Locations

```
agent-runner/
├── src/rag.rs                      <- Main implementation
├── examples/rag_example.rs         <- Complete example
├── tests/rag_integration_test.rs   <- Test suite
├── RAG_README.md                   <- Full documentation
├── RAG_QUICKSTART.md               <- This file
└── Cargo.toml                      <- Dependencies
```

## Next Steps

1. Read **RAG_README.md** for complete documentation
2. Review **examples/rag_example.rs** for detailed examples
3. Check **tests/rag_integration_test.rs** for test patterns
4. See **RAG_IMPLEMENTATION_SUMMARY.md** for technical details

## Troubleshooting

### "API key required"
```bash
export OPENAI_API_KEY="sk-..."
```

### "Failed to initialize VectorStore"
- Check disk space
- Verify write permissions on data directories

### "Out of memory"
Reduce cache size:
```rust
config.max_cache_size = 1000;  // Smaller cache
```

### Tests failing
```bash
# Run with verbose output
cargo test rag -- --nocapture
```

## Performance Tips

1. **Enable caching** (default: enabled) - Saves API costs
2. **Use hybrid search** - Best results
3. **Batch indexing** - Index multiple files together
4. **Adjust chunk size** - Larger chunks for more context
5. **Tune weights** - Adjust vector_weight vs fulltext_weight

## Support

For issues or questions:
1. Check RAG_README.md "Troubleshooting" section
2. Review test examples
3. See inline documentation in src/rag.rs

## Quick Reference

| Operation | Method | Async |
|-----------|--------|-------|
| Create system | `RagSystem::new(config)` | Yes |
| Index file | `rag.index_file(path)` | Yes |
| Vector search | `rag.vector_search(query, k)` | Yes |
| Full-text search | `rag.fulltext_search(query, k)` | No |
| Hybrid search | `rag.hybrid_search(query)` | Yes |
| Get stats | `rag.stats()` | No |
| Clear data | `rag.clear()` | Yes |

---

**Ready to use!** The RAG layer is production-ready and fully functional.
