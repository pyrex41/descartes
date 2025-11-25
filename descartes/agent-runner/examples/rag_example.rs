/// Example demonstrating the RAG (Retrieval-Augmented Generation) system
///
/// This example shows how to:
/// 1. Initialize the RAG system
/// 2. Index code files
/// 3. Perform vector, full-text, and hybrid searches
/// 4. Use embedding caching
/// 5. Access search results and statistics
use agent_runner::{RagConfig, RagSystem};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Descartes RAG System Example ===\n");

    // 1. Create RAG configuration
    let config = RagConfig {
        lance_db_path: "./example_data/lancedb".to_string(),
        tantivy_index_path: "./example_data/tantivy".to_string(),
        sqlite_db_path: "sqlite://./example_data/descartes.db".to_string(),
        embedding_model: "text-embedding-3-small".to_string(),
        embedding_provider: "openai".to_string(),
        api_key: std::env::var("OPENAI_API_KEY").ok(),
        embedding_dimension: 1536,
        max_chunk_size: 1000,
        chunk_overlap: 200,
        vector_top_k: 5,
        fulltext_top_k: 5,
        vector_weight: 0.7,
        fulltext_weight: 0.3,
        enable_cache: true,
        max_cache_size: 1000,
        embedding_batch_size: 32,
        max_concurrent_embeddings: 4,
    };

    println!("Configuration:");
    println!("  Embedding Provider: {}", config.embedding_provider);
    println!("  Embedding Model: {}", config.embedding_model);
    println!("  Embedding Dimension: {}", config.embedding_dimension);
    println!("  Max Chunk Size: {}", config.max_chunk_size);
    println!("  Cache Enabled: {}\n", config.enable_cache);

    // 2. Initialize RAG system
    println!("Initializing RAG system...");
    let mut rag = RagSystem::new(config).await?;
    println!("RAG system initialized successfully!\n");

    // 3. Index some example files
    println!("Indexing example files...");

    // In a real scenario, you would index actual code files
    // For this example, we'll show the API usage

    let example_files = vec!["../src/rag.rs", "../src/parser.rs", "../src/semantic.rs"];

    for file_path in &example_files {
        match rag.index_file(file_path).await {
            Ok(chunk_count) => {
                println!("  Indexed {} -> {} chunks", file_path, chunk_count);
            }
            Err(e) => {
                println!("  Failed to index {}: {}", file_path, e);
            }
        }
    }
    println!();

    // 4. Display statistics
    let stats = rag.stats();
    println!("RAG System Statistics:");
    println!("  Total Chunks: {}", stats.total_chunks);
    println!(
        "  Cache Size: {}/{}",
        stats.cache_size, stats.cache_capacity
    );
    println!("  Embedding Dimension: {}\n", stats.embedding_dimension);

    // 5. Perform searches
    if stats.total_chunks > 0 {
        println!("=== Search Examples ===\n");

        // Example query
        let query = "how to parse rust code";

        // Vector search
        println!("1. Vector Search:");
        println!("   Query: '{}'", query);
        match rag.vector_search(query, 3).await {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!(
                        "   [{}] Score: {:.4} | Type: {} | File: {}",
                        i + 1,
                        result.score,
                        result.chunk.chunk_type,
                        result.chunk.file_path
                    );
                    println!(
                        "       Preview: {}...",
                        result.chunk.content.chars().take(60).collect::<String>()
                    );
                }
            }
            Err(e) => println!("   Error: {}", e),
        }
        println!();

        // Full-text search
        println!("2. Full-Text Search:");
        println!("   Query: '{}'", query);
        match rag.fulltext_search(query, 3) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!(
                        "   [{}] Score: {:.4} | Type: {} | File: {}",
                        i + 1,
                        result.score,
                        result.chunk.chunk_type,
                        result.chunk.file_path
                    );
                }
            }
            Err(e) => println!("   Error: {}", e),
        }
        println!();

        // Hybrid search
        println!("3. Hybrid Search (Vector + Full-Text):");
        println!("   Query: '{}'", query);
        match rag.hybrid_search(query).await {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!(
                        "   [{}] Combined Score: {:.4} | Type: {} | File: {}",
                        i + 1,
                        result.score,
                        result.chunk.chunk_type,
                        result.chunk.file_path
                    );
                    if let Some(vs) = result.vector_score {
                        println!("       Vector Score: {:.4}", vs);
                    }
                    if let Some(fs) = result.fulltext_score {
                        println!("       Full-Text Score: {:.4}", fs);
                    }
                    println!(
                        "       Lines: {}-{}",
                        result.chunk.line_range.0, result.chunk.line_range.1
                    );
                }
            }
            Err(e) => println!("   Error: {}", e),
        }
        println!();
    }

    // 6. Demonstrate caching
    println!("=== Embedding Cache Demo ===");
    println!("Performing same query twice to demonstrate caching...");

    let test_query = "semantic code analysis";

    let start = std::time::Instant::now();
    let _ = rag.vector_search(test_query, 1).await;
    let first_duration = start.elapsed();

    let start = std::time::Instant::now();
    let _ = rag.vector_search(test_query, 1).await;
    let second_duration = start.elapsed();

    println!("  First query (no cache): {:?}", first_duration);
    println!("  Second query (cached): {:?}", second_duration);
    println!(
        "  Speedup: {:.2}x\n",
        first_duration.as_secs_f64() / second_duration.as_secs_f64()
    );

    // 7. Advanced features
    println!("=== Advanced Features ===\n");

    println!("Semantic Chunking:");
    println!("  - Chunks code at semantic boundaries (functions, classes, etc.)");
    println!("  - Preserves context and meaning");
    println!("  - Respects language structure using tree-sitter\n");

    println!("Hybrid Search:");
    println!("  - Combines vector similarity (semantic) with full-text (keywords)");
    println!("  - Configurable weights for each search type");
    println!("  - Best of both worlds for code retrieval\n");

    println!("Integration Points:");
    println!("  - Uses existing SemanticParser from parser.rs");
    println!("  - Stores metadata in SQLite (db_schema.rs)");
    println!("  - Compatible with OpenAI and Anthropic embedding APIs");
    println!("  - LanceDB for efficient vector storage");
    println!("  - Tantivy for full-text indexing\n");

    // Cleanup
    println!("Cleaning up...");
    rag.clear().await?;
    println!("Done!");

    Ok(())
}
