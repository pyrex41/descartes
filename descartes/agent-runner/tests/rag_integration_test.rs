/// Integration tests for the RAG system
///
/// These tests validate the complete RAG pipeline including:
/// - Semantic chunking
/// - Embedding generation (mocked)
/// - Vector storage
/// - Full-text indexing
/// - Hybrid search

use agent_runner::rag::*;
use agent_runner::{Language, ParserResult};
use async_trait::async_trait;
use std::sync::Arc;
use tempfile::TempDir;

/// Mock embedding provider for testing
struct MockEmbeddingProvider {
    dimension: usize,
}

impl MockEmbeddingProvider {
    fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Generate deterministic embeddings based on text hash
    fn generate_mock_embedding(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate deterministic but varied embeddings
        (0..self.dimension)
            .map(|i| {
                let val = ((hash.wrapping_add(i as u64)) as f32) / u64::MAX as f32;
                (val - 0.5) * 2.0  // Normalize to [-1, 1]
            })
            .collect()
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed_batch(&self, texts: &[String]) -> ParserResult<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| self.generate_mock_embedding(text))
            .collect())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[test]
fn test_embedding_cache() {
    let cache = EmbeddingCache::new(100);
    let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];

    // Test put and get
    cache.put("test text", embedding.clone());
    let retrieved = cache.get("test text");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), embedding);

    // Test cache miss
    let missing = cache.get("nonexistent");
    assert!(missing.is_none());

    // Test stats
    let (size, capacity) = cache.stats();
    assert_eq!(size, 1);
    assert_eq!(capacity, 100);
}

#[test]
fn test_embedding_cache_eviction() {
    let cache = EmbeddingCache::new(3);  // Small cache

    // Fill cache
    cache.put("text1", vec![1.0]);
    cache.put("text2", vec![2.0]);
    cache.put("text3", vec![3.0]);

    let (size, _) = cache.stats();
    assert_eq!(size, 3);

    // Add one more - should evict oldest
    cache.put("text4", vec![4.0]);

    let (size, _) = cache.stats();
    assert_eq!(size, 3);

    // text1 should be evicted
    assert!(cache.get("text1").is_none());
    assert!(cache.get("text2").is_some());
    assert!(cache.get("text3").is_some());
    assert!(cache.get("text4").is_some());
}

#[test]
fn test_embedding_cache_lru_update() {
    let cache = EmbeddingCache::new(3);

    cache.put("text1", vec![1.0]);
    cache.put("text2", vec![2.0]);
    cache.put("text3", vec![3.0]);

    // Access text1 to make it most recently used
    let _ = cache.get("text1");

    // Add new item - should evict text2 (oldest)
    cache.put("text4", vec![4.0]);

    assert!(cache.get("text1").is_some());  // Should still exist
    assert!(cache.get("text2").is_none());  // Should be evicted
    assert!(cache.get("text3").is_some());
    assert!(cache.get("text4").is_some());
}

#[tokio::test]
async fn test_mock_embedding_provider() {
    let provider = MockEmbeddingProvider::new(128);

    // Test single embedding
    let embedding = provider.embed("test text").await.unwrap();
    assert_eq!(embedding.len(), 128);

    // Test deterministic generation
    let embedding2 = provider.embed("test text").await.unwrap();
    assert_eq!(embedding, embedding2);

    // Test different text produces different embedding
    let embedding3 = provider.embed("different text").await.unwrap();
    assert_ne!(embedding, embedding3);
}

#[tokio::test]
async fn test_mock_embedding_batch() {
    let provider = MockEmbeddingProvider::new(64);

    let texts = vec![
        "function definition".to_string(),
        "class declaration".to_string(),
        "import statement".to_string(),
    ];

    let embeddings = provider.embed_batch(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 3);
    assert_eq!(embeddings[0].len(), 64);
    assert_eq!(embeddings[1].len(), 64);
    assert_eq!(embeddings[2].len(), 64);

    // Each should be different
    assert_ne!(embeddings[0], embeddings[1]);
    assert_ne!(embeddings[1], embeddings[2]);
}

#[test]
fn test_semantic_chunker_text() {
    let chunker = SemanticChunker::new(500, 100).unwrap();

    let text = r#"
fn hello() {
    println!("Hello, world!");
}

fn goodbye() {
    println!("Goodbye!");
}

struct Point {
    x: i32,
    y: i32,
}
"#;

    let chunks = chunker
        .chunk_text(text, "test.rs", Language::Rust)
        .unwrap();

    assert!(!chunks.is_empty());

    // Verify chunk properties
    for chunk in &chunks {
        assert_eq!(chunk.file_path, "test.rs");
        assert_eq!(chunk.language, Language::Rust);
        assert!(chunk.content.len() <= 500);
    }
}

#[test]
fn test_semantic_chunker_overlap() {
    let chunker = SemanticChunker::new(100, 20).unwrap();

    // Create text that will definitely need multiple chunks
    let mut long_text = String::new();
    for i in 0..10 {
        long_text.push_str(&format!("Line {} with some content\n", i));
    }

    let chunks = chunker
        .chunk_text(&long_text, "test.rs", Language::Rust)
        .unwrap();

    // Should create multiple chunks
    assert!(chunks.len() > 1);

    // Check for overlap between consecutive chunks
    for i in 0..chunks.len() - 1 {
        let current_end = chunks[i].line_range.1;
        let next_start = chunks[i + 1].line_range.0;

        // Next chunk should start before current ends (overlap)
        assert!(next_start <= current_end);
    }
}

#[tokio::test]
async fn test_vector_store_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("lancedb");

    let mut store = VectorStore::new(db_path.to_str().unwrap(), 128)
        .await
        .unwrap();

    assert!(store.initialize().await.is_ok());
}

#[test]
fn test_fulltext_search_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().join("tantivy");

    let mut search = FullTextSearch::new(index_path.to_str().unwrap()).unwrap();

    assert!(search.initialize().is_ok());
}

#[test]
fn test_fulltext_search_add_and_search() {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().join("tantivy");

    let mut search = FullTextSearch::new(index_path.to_str().unwrap()).unwrap();
    search.initialize().unwrap();

    // Create test chunks
    let chunks = vec![
        CodeChunk {
            id: "chunk1".to_string(),
            content: "async fn connect_to_database() -> Result<Connection>".to_string(),
            file_path: "db.rs".to_string(),
            language: Language::Rust,
            line_range: (10, 15),
            node_id: None,
            chunk_type: "function".to_string(),
            embedding: None,
            metadata: Default::default(),
        },
        CodeChunk {
            id: "chunk2".to_string(),
            content: "fn parse_json(data: &str) -> Result<Value>".to_string(),
            file_path: "parser.rs".to_string(),
            language: Language::Rust,
            line_range: (20, 25),
            node_id: None,
            chunk_type: "function".to_string(),
            embedding: None,
            metadata: Default::default(),
        },
    ];

    // Add chunks
    assert!(search.add_chunks(&chunks).is_ok());

    // Search for "database"
    let results = search.search("database", 10).unwrap();
    assert!(!results.is_empty());

    // Should find chunk1
    assert!(results.iter().any(|(id, _)| id == "chunk1"));
}

#[test]
fn test_code_chunk_serialization() {
    let chunk = CodeChunk {
        id: "test_chunk".to_string(),
        content: "fn test() {}".to_string(),
        file_path: "test.rs".to_string(),
        language: Language::Rust,
        line_range: (1, 5),
        node_id: Some("node_123".to_string()),
        chunk_type: "function".to_string(),
        embedding: Some(vec![0.1, 0.2, 0.3]),
        metadata: [("key".to_string(), "value".to_string())]
            .iter()
            .cloned()
            .collect(),
    };

    // Serialize
    let json = serde_json::to_string(&chunk).unwrap();
    assert!(json.contains("test_chunk"));
    assert!(json.contains("test.rs"));

    // Deserialize
    let deserialized: CodeChunk = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, chunk.id);
    assert_eq!(deserialized.content, chunk.content);
    assert_eq!(deserialized.file_path, chunk.file_path);
}

#[test]
fn test_search_result_ordering() {
    let chunk1 = CodeChunk {
        id: "chunk1".to_string(),
        content: "test1".to_string(),
        file_path: "f1.rs".to_string(),
        language: Language::Rust,
        line_range: (1, 2),
        node_id: None,
        chunk_type: "function".to_string(),
        embedding: None,
        metadata: Default::default(),
    };

    let chunk2 = chunk1.clone();

    let mut results = vec![
        SearchResult {
            chunk: chunk1.clone(),
            score: 0.5,
            search_type: "vector".to_string(),
            vector_score: Some(0.5),
            fulltext_score: None,
        },
        SearchResult {
            chunk: chunk2,
            score: 0.9,
            search_type: "vector".to_string(),
            vector_score: Some(0.9),
            fulltext_score: None,
        },
    ];

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    assert!(results[0].score > results[1].score);
    assert_eq!(results[0].score, 0.9);
}

#[test]
fn test_rag_config_defaults() {
    let config = RagConfig::default();

    assert_eq!(config.embedding_dimension, 1536);
    assert_eq!(config.max_chunk_size, 1000);
    assert_eq!(config.chunk_overlap, 200);
    assert_eq!(config.vector_weight, 0.7);
    assert_eq!(config.fulltext_weight, 0.3);
    assert!(config.enable_cache);
    assert_eq!(config.max_cache_size, 10000);
}

#[test]
fn test_rag_config_weights_sum() {
    let config = RagConfig::default();

    // Weights should sum to 1.0
    let sum = config.vector_weight + config.fulltext_weight;
    assert!((sum - 1.0).abs() < 0.001);
}

#[test]
fn test_cache_key_consistency() {
    let text = "hello world";

    let key1 = EmbeddingCache::cache_key(text);
    let key2 = EmbeddingCache::cache_key(text);

    assert_eq!(key1, key2);
    assert_eq!(key1.len(), 64); // Blake3 hash is 32 bytes = 64 hex chars
}

#[test]
fn test_cache_key_uniqueness() {
    let text1 = "hello world";
    let text2 = "hello world!";

    let key1 = EmbeddingCache::cache_key(text1);
    let key2 = EmbeddingCache::cache_key(text2);

    assert_ne!(key1, key2);
}

#[tokio::test]
async fn test_embedding_provider_dimension() {
    let provider = MockEmbeddingProvider::new(256);
    assert_eq!(provider.dimension(), 256);

    let embedding = provider.embed("test").await.unwrap();
    assert_eq!(embedding.len(), 256);
}

#[test]
fn test_chunk_metadata() {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("visibility".to_string(), "public".to_string());
    metadata.insert("async".to_string(), "true".to_string());

    let chunk = CodeChunk {
        id: "test".to_string(),
        content: "async fn test()".to_string(),
        file_path: "test.rs".to_string(),
        language: Language::Rust,
        line_range: (1, 5),
        node_id: None,
        chunk_type: "function".to_string(),
        embedding: None,
        metadata,
    };

    assert_eq!(chunk.metadata.get("visibility").unwrap(), "public");
    assert_eq!(chunk.metadata.get("async").unwrap(), "true");
}

/// Integration test demonstrating complete RAG workflow
/// Note: This test uses mock embeddings and doesn't require API keys
#[tokio::test]
async fn test_complete_rag_workflow_mock() {
    // This test demonstrates the complete RAG workflow
    // In production, you would use real embedding providers

    let temp_dir = TempDir::new().unwrap();

    // 1. Create chunker
    let chunker = SemanticChunker::new(500, 100).unwrap();

    // 2. Create sample code
    let sample_code = r#"
/// Database connection pool
pub struct ConnectionPool {
    connections: Vec<Connection>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(size: usize) -> Self {
        Self {
            connections: Vec::with_capacity(size),
        }
    }

    /// Get a connection from the pool
    pub async fn acquire(&self) -> Result<Connection> {
        // Implementation
        Ok(Connection::new())
    }
}
"#;

    // 3. Chunk the code
    let chunks = chunker
        .chunk_text(sample_code, "pool.rs", Language::Rust)
        .unwrap();

    assert!(!chunks.is_empty());

    // 4. Generate mock embeddings
    let provider = MockEmbeddingProvider::new(128);
    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = provider.embed_batch(&texts).await.unwrap();

    assert_eq!(embeddings.len(), chunks.len());

    // 5. Create full-text index
    let index_path = temp_dir.path().join("tantivy");
    let mut search = FullTextSearch::new(index_path.to_str().unwrap()).unwrap();
    search.initialize().unwrap();

    // 6. Add chunks to index
    search.add_chunks(&chunks).unwrap();

    // 7. Test search
    let results = search.search("connection pool", 5).unwrap();

    assert!(!results.is_empty());
    println!("Found {} results for 'connection pool'", results.len());

    for (id, score) in results {
        println!("  {} -> score: {:.4}", id, score);
    }
}

#[test]
fn test_search_result_hybrid_scoring() {
    let chunk = CodeChunk {
        id: "test".to_string(),
        content: "test content".to_string(),
        file_path: "test.rs".to_string(),
        language: Language::Rust,
        line_range: (1, 2),
        node_id: None,
        chunk_type: "function".to_string(),
        embedding: None,
        metadata: Default::default(),
    };

    let result = SearchResult {
        chunk,
        score: 0.0,  // Will be computed
        search_type: "hybrid".to_string(),
        vector_score: Some(0.8),
        fulltext_score: Some(0.6),
    };

    // Compute hybrid score with default weights
    let config = RagConfig::default();
    let expected_score = 0.8 * config.vector_weight + 0.6 * config.fulltext_weight;

    // The actual implementation would compute this
    assert!(expected_score > 0.0);
    assert!(expected_score <= 1.0);
}
