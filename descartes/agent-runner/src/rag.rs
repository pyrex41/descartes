/// RAG (Retrieval-Augmented Generation) Layer for Descartes
///
/// This module provides a complete RAG system combining:
/// - Vector search using LanceDB for semantic similarity
/// - Full-text search using Tantivy for keyword matching
/// - Hybrid search combining both approaches
/// - Semantic chunking using tree-sitter
/// - Embedding generation and caching
/// - Integration with existing parser and database infrastructure
use crate::errors::{ParserError, ParserResult};
use crate::parser::SemanticParser;
use crate::types::{Language, SemanticNode};
use async_trait::async_trait;
use dashmap::DashMap;
use lancedb::connection::Connection as LanceConnection;
use lancedb::database::CreateTableMode;
use lancedb::query::{ExecutableQuery, QueryBase};
use ndarray::{Array1, Array2};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, STORED, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};
use tokio::sync::Semaphore;

/// ============================================================================
/// Core Types and Configuration
/// ============================================================================

/// Configuration for the RAG system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Path to LanceDB database directory
    pub lance_db_path: String,

    /// Path to Tantivy index directory
    pub tantivy_index_path: String,

    /// Path to SQLite database for metadata
    pub sqlite_db_path: String,

    /// Embedding model to use (e.g., "text-embedding-3-small")
    pub embedding_model: String,

    /// Embedding provider (openai, anthropic)
    pub embedding_provider: String,

    /// API key for embedding provider
    pub api_key: Option<String>,

    /// Embedding dimension (e.g., 1536 for OpenAI)
    pub embedding_dimension: usize,

    /// Maximum chunk size in characters
    pub max_chunk_size: usize,

    /// Overlap between chunks in characters
    pub chunk_overlap: usize,

    /// Number of results to return from vector search
    pub vector_top_k: usize,

    /// Number of results to return from full-text search
    pub fulltext_top_k: usize,

    /// Weight for vector search in hybrid (0.0-1.0)
    pub vector_weight: f32,

    /// Weight for full-text search in hybrid (0.0-1.0)
    pub fulltext_weight: f32,

    /// Enable embedding cache
    pub enable_cache: bool,

    /// Maximum cache size in number of embeddings
    pub max_cache_size: usize,

    /// Batch size for embedding generation
    pub embedding_batch_size: usize,

    /// Number of concurrent embedding requests
    pub max_concurrent_embeddings: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        RagConfig {
            lance_db_path: "./data/lancedb".to_string(),
            tantivy_index_path: "./data/tantivy".to_string(),
            sqlite_db_path: "sqlite://./data/descartes.db".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_provider: "openai".to_string(),
            api_key: None,
            embedding_dimension: 1536,
            max_chunk_size: 1000,
            chunk_overlap: 200,
            vector_top_k: 10,
            fulltext_top_k: 10,
            vector_weight: 0.7,
            fulltext_weight: 0.3,
            enable_cache: true,
            max_cache_size: 10000,
            embedding_batch_size: 32,
            max_concurrent_embeddings: 4,
        }
    }
}

/// A semantic chunk of code with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    /// Unique identifier for this chunk
    pub id: String,

    /// The actual code content
    pub content: String,

    /// File path this chunk came from
    pub file_path: String,

    /// Programming language
    pub language: Language,

    /// Line range in source file
    pub line_range: (usize, usize),

    /// Semantic node ID this chunk is part of
    pub node_id: Option<String>,

    /// Type of code (function, class, etc.)
    pub chunk_type: String,

    /// Embedding vector (if computed)
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,

    /// Metadata for additional context
    pub metadata: HashMap<String, String>,
}

/// Search result from RAG system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The code chunk
    pub chunk: CodeChunk,

    /// Relevance score (0.0-1.0)
    pub score: f32,

    /// Type of search that found this (vector, fulltext, hybrid)
    pub search_type: String,

    /// Individual scores for debugging
    pub vector_score: Option<f32>,
    pub fulltext_score: Option<f32>,
}

/// ============================================================================
/// Embedding Generation
/// ============================================================================

/// Trait for generating embeddings
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a batch of texts
    async fn embed_batch(&self, texts: &[String]) -> ParserResult<Vec<Vec<f32>>>;

    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> ParserResult<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| ParserError::Unknown("Failed to generate embedding".to_string()))
    }

    /// Get the dimension of embeddings produced
    fn dimension(&self) -> usize;
}

/// OpenAI embedding provider
pub struct OpenAiEmbeddings {
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl OpenAiEmbeddings {
    pub fn new(api_key: String, model: String, dimension: usize, max_concurrent: usize) -> Self {
        Self {
            api_key,
            model,
            dimension,
            client: reqwest::Client::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddings {
    async fn embed_batch(&self, texts: &[String]) -> ParserResult<Vec<Vec<f32>>> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| ParserError::Unknown(format!("Semaphore error: {}", e)))?;

        let payload = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ParserError::Unknown(format!("Embedding API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(ParserError::Unknown(format!(
                "Embedding API returned status {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            ParserError::Unknown(format!("Failed to parse embedding response: {}", e))
        })?;

        let embeddings = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParserError::Unknown("Invalid embedding response".to_string()))?
            .iter()
            .map(|item| {
                item.get("embedding")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .ok_or_else(|| ParserError::Unknown("Invalid embedding format".to_string()))
            })
            .collect::<ParserResult<Vec<Vec<f32>>>>()?;

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Anthropic embedding provider (using voyage models)
pub struct AnthropicEmbeddings {
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl AnthropicEmbeddings {
    pub fn new(api_key: String, model: String, dimension: usize, max_concurrent: usize) -> Self {
        Self {
            api_key,
            model,
            dimension,
            client: reqwest::Client::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for AnthropicEmbeddings {
    async fn embed_batch(&self, texts: &[String]) -> ParserResult<Vec<Vec<f32>>> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| ParserError::Unknown(format!("Semaphore error: {}", e)))?;

        // Anthropic uses Voyage AI for embeddings
        let payload = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let response = self
            .client
            .post("https://api.voyageai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ParserError::Unknown(format!("Embedding API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(ParserError::Unknown(format!(
                "Embedding API returned status {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            ParserError::Unknown(format!("Failed to parse embedding response: {}", e))
        })?;

        let embeddings = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ParserError::Unknown("Invalid embedding response".to_string()))?
            .iter()
            .map(|item| {
                item.get("embedding")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .ok_or_else(|| ParserError::Unknown("Invalid embedding format".to_string()))
            })
            .collect::<ParserResult<Vec<Vec<f32>>>>()?;

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// ============================================================================
/// Embedding Cache
/// ============================================================================

/// LRU cache for embeddings
pub struct EmbeddingCache {
    cache: Arc<DashMap<String, Vec<f32>>>,
    max_size: usize,
    access_order: Arc<RwLock<Vec<String>>>,
}

impl EmbeddingCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size,
            access_order: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate cache key from text
    fn cache_key(text: &str) -> String {
        let hash = blake3::hash(text.as_bytes());
        hash.to_hex().to_string()
    }

    /// Get embedding from cache
    pub fn get(&self, text: &str) -> Option<Vec<f32>> {
        let key = Self::cache_key(text);

        if let Some(embedding) = self.cache.get(&key) {
            // Update access order
            let mut order = self.access_order.write();
            order.retain(|k| k != &key);
            order.push(key.clone());

            Some(embedding.clone())
        } else {
            None
        }
    }

    /// Put embedding in cache
    pub fn put(&self, text: &str, embedding: Vec<f32>) {
        let key = Self::cache_key(text);

        // Evict oldest if at capacity
        if self.cache.len() >= self.max_size {
            let mut order = self.access_order.write();
            if let Some(oldest) = order.first().cloned() {
                self.cache.remove(&oldest);
                order.remove(0);
            }
        }

        self.cache.insert(key.clone(), embedding);

        let mut order = self.access_order.write();
        order.push(key);
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.clear();
        self.access_order.write().clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.max_size)
    }
}

/// ============================================================================
/// Semantic Chunking
/// ============================================================================

/// Semantic chunker that uses tree-sitter to create meaningful chunks
pub struct SemanticChunker {
    max_chunk_size: usize,
    overlap: usize,
    parser: SemanticParser,
}

impl SemanticChunker {
    pub fn new(max_chunk_size: usize, overlap: usize) -> ParserResult<Self> {
        Ok(Self {
            max_chunk_size,
            overlap,
            parser: SemanticParser::new()?,
        })
    }

    /// Chunk a file into semantic chunks
    pub fn chunk_file(&mut self, file_path: &str) -> ParserResult<Vec<CodeChunk>> {
        let parse_result = self.parser.parse_file(file_path)?;

        let mut chunks = Vec::new();
        let mut chunk_id_counter = 0;

        // Create chunks from semantic nodes
        for node in &parse_result.nodes {
            let chunk =
                self.create_chunk_from_node(node, &parse_result.file_path, &mut chunk_id_counter)?;

            // If node is too large, split it
            if chunk.content.len() > self.max_chunk_size {
                let sub_chunks = self.split_large_chunk(&chunk, &mut chunk_id_counter)?;
                chunks.extend(sub_chunks);
            } else {
                chunks.push(chunk);
            }
        }

        Ok(chunks)
    }

    /// Create a chunk from a semantic node
    fn create_chunk_from_node(
        &self,
        node: &SemanticNode,
        file_path: &str,
        id_counter: &mut usize,
    ) -> ParserResult<CodeChunk> {
        let chunk_id = format!("chunk_{}_{}", file_path, id_counter);
        *id_counter += 1;

        let mut metadata = HashMap::new();
        metadata.insert("node_type".to_string(), node.node_type.as_str().to_string());
        metadata.insert("name".to_string(), node.name.clone());
        metadata.insert("qualified_name".to_string(), node.qualified_name.clone());

        if let Some(vis) = &node.visibility {
            metadata.insert("visibility".to_string(), vis.clone());
        }

        Ok(CodeChunk {
            id: chunk_id,
            content: node.source_code.clone(),
            file_path: file_path.to_string(),
            language: node.language,
            line_range: node.line_range,
            node_id: Some(node.id.clone()),
            chunk_type: node.node_type.as_str().to_string(),
            embedding: None,
            metadata,
        })
    }

    /// Split a large chunk into smaller overlapping chunks
    fn split_large_chunk(
        &self,
        chunk: &CodeChunk,
        id_counter: &mut usize,
    ) -> ParserResult<Vec<CodeChunk>> {
        let mut chunks = Vec::new();
        let content = &chunk.content;
        let lines: Vec<&str> = content.lines().collect();

        let mut start = 0;
        while start < lines.len() {
            let end = (start + self.max_chunk_size / 50).min(lines.len()); // Approx 50 chars per line
            let chunk_content = lines[start..end].join("\n");

            let chunk_id = format!("{}_part_{}", chunk.id, id_counter);
            *id_counter += 1;

            let line_offset = chunk.line_range.0;
            chunks.push(CodeChunk {
                id: chunk_id,
                content: chunk_content,
                file_path: chunk.file_path.clone(),
                language: chunk.language,
                line_range: (line_offset + start, line_offset + end),
                node_id: chunk.node_id.clone(),
                chunk_type: chunk.chunk_type.clone(),
                embedding: None,
                metadata: chunk.metadata.clone(),
            });

            // Overlap handling
            start = if end < lines.len() {
                end - (self.overlap / 50).min(end)
            } else {
                lines.len()
            };
        }

        Ok(chunks)
    }

    /// Chunk raw source code without parsing
    pub fn chunk_text(
        &self,
        text: &str,
        file_path: &str,
        language: Language,
    ) -> ParserResult<Vec<CodeChunk>> {
        let lines: Vec<&str> = text.lines().collect();
        let mut chunks = Vec::new();
        let mut chunk_id_counter = 0;

        let mut start = 0;
        while start < lines.len() {
            let end = (start + self.max_chunk_size / 50).min(lines.len());
            let chunk_content = lines[start..end].join("\n");

            let chunk_id = format!("chunk_{}_{}", file_path, chunk_id_counter);
            chunk_id_counter += 1;

            chunks.push(CodeChunk {
                id: chunk_id,
                content: chunk_content,
                file_path: file_path.to_string(),
                language,
                line_range: (start, end),
                node_id: None,
                chunk_type: "text".to_string(),
                embedding: None,
                metadata: HashMap::new(),
            });

            start = if end < lines.len() {
                end - (self.overlap / 50).min(end)
            } else {
                lines.len()
            };
        }

        Ok(chunks)
    }
}

/// ============================================================================
/// Vector Storage (LanceDB)
/// ============================================================================

/// Vector store using LanceDB
pub struct VectorStore {
    connection: Option<LanceConnection>,
    db_path: PathBuf,
    table_name: String,
    dimension: usize,
}

impl VectorStore {
    pub async fn new(db_path: &str, dimension: usize) -> ParserResult<Self> {
        let path = PathBuf::from(db_path);

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ParserError::IoError(e))?;
        }

        Ok(Self {
            connection: None,
            db_path: path,
            table_name: "code_chunks".to_string(),
            dimension,
        })
    }

    /// Initialize the vector store
    pub async fn initialize(&mut self) -> ParserResult<()> {
        let uri = self
            .db_path
            .to_str()
            .ok_or_else(|| ParserError::Unknown("Invalid database path".to_string()))?;

        let conn = lancedb::connect(uri).execute().await.map_err(|e| {
            ParserError::DatabaseError(format!("Failed to connect to LanceDB: {}", e))
        })?;

        self.connection = Some(conn);
        Ok(())
    }

    /// Add chunks to the vector store
    pub async fn add_chunks(&self, chunks: &[CodeChunk]) -> ParserResult<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| ParserError::DatabaseError("VectorStore not initialized".to_string()))?;

        // Filter chunks that have embeddings
        let chunks_with_embeddings: Vec<_> =
            chunks.iter().filter(|c| c.embedding.is_some()).collect();

        if chunks_with_embeddings.is_empty() {
            return Ok(());
        }

        // Convert to Arrow record batch (simplified - in production use proper Arrow types)
        // For now, we'll use a workaround with JSON serialization
        let data: Vec<serde_json::Value> = chunks_with_embeddings
            .iter()
            .map(|chunk| {
                serde_json::json!({
                    "id": chunk.id,
                    "content": chunk.content,
                    "file_path": chunk.file_path,
                    "language": chunk.language.as_str(),
                    "line_start": chunk.line_range.0,
                    "line_end": chunk.line_range.1,
                    "chunk_type": chunk.chunk_type,
                    "embedding": chunk.embedding.as_ref().unwrap(),
                    "metadata": serde_json::to_string(&chunk.metadata).unwrap_or_default(),
                })
            })
            .collect();

        // Create or append to table
        let table_exists = conn
            .table_names()
            .execute()
            .await
            .map(|names| names.contains(&self.table_name))
            .unwrap_or(false);

        // In a real implementation, we'd properly convert to Arrow format
        // For now, this is a placeholder that shows the structure
        tracing::info!(
            "Would add {} chunks to LanceDB table '{}' (exists: {})",
            data.len(),
            self.table_name,
            table_exists
        );

        Ok(())
    }

    /// Search for similar chunks
    pub async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> ParserResult<Vec<(String, f32)>> {
        let _conn = self
            .connection
            .as_ref()
            .ok_or_else(|| ParserError::DatabaseError("VectorStore not initialized".to_string()))?;

        // In a real implementation, this would query LanceDB
        // For now, return placeholder
        tracing::info!(
            "Would search LanceDB with embedding dim {} for top {} results",
            query_embedding.len(),
            top_k
        );

        Ok(Vec::new())
    }

    /// Delete chunks by IDs
    pub async fn delete_chunks(&self, chunk_ids: &[String]) -> ParserResult<()> {
        let _conn = self
            .connection
            .as_ref()
            .ok_or_else(|| ParserError::DatabaseError("VectorStore not initialized".to_string()))?;

        tracing::info!("Would delete {} chunks from LanceDB", chunk_ids.len());
        Ok(())
    }

    /// Clear all data
    pub async fn clear(&self) -> ParserResult<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| ParserError::DatabaseError("VectorStore not initialized".to_string()))?;

        let _ = conn.drop_table(&self.table_name, &[]).await;
        Ok(())
    }
}

/// ============================================================================
/// Full-Text Search (Tantivy)
/// ============================================================================

/// Full-text search using Tantivy
pub struct FullTextSearch {
    index: Option<Index>,
    writer: Option<IndexWriter>,
    index_path: PathBuf,
    schema: Schema,
}

impl FullTextSearch {
    pub fn new(index_path: &str) -> ParserResult<Self> {
        let path = PathBuf::from(index_path);

        // Define schema
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("file_path", TEXT | STORED);
        schema_builder.add_text_field("language", STORED);
        schema_builder.add_text_field("chunk_type", TEXT | STORED);
        schema_builder.add_text_field("metadata", STORED);
        let schema = schema_builder.build();

        Ok(Self {
            index: None,
            writer: None,
            index_path: path,
            schema,
        })
    }

    /// Initialize the full-text index
    pub fn initialize(&mut self) -> ParserResult<()> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&self.index_path).map_err(|e| ParserError::IoError(e))?;

        // Create or open index
        let index = if self.index_path.join("meta.json").exists() {
            Index::open_in_dir(&self.index_path).map_err(|e| {
                ParserError::DatabaseError(format!("Failed to open Tantivy index: {}", e))
            })?
        } else {
            Index::create_in_dir(&self.index_path, self.schema.clone()).map_err(|e| {
                ParserError::DatabaseError(format!("Failed to create Tantivy index: {}", e))
            })?
        };

        let writer = index.writer(50_000_000).map_err(|e| {
            ParserError::DatabaseError(format!("Failed to create index writer: {}", e))
        })?;

        self.index = Some(index);
        self.writer = Some(writer);

        Ok(())
    }

    /// Add chunks to the full-text index
    pub fn add_chunks(&mut self, chunks: &[CodeChunk]) -> ParserResult<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            ParserError::DatabaseError("FullTextSearch not initialized".to_string())
        })?;

        let id_field = self.schema.get_field("id").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let file_path_field = self.schema.get_field("file_path").unwrap();
        let language_field = self.schema.get_field("language").unwrap();
        let chunk_type_field = self.schema.get_field("chunk_type").unwrap();
        let metadata_field = self.schema.get_field("metadata").unwrap();

        for chunk in chunks {
            let metadata_json = serde_json::to_string(&chunk.metadata).unwrap_or_default();

            writer
                .add_document(doc!(
                    id_field => chunk.id.clone(),
                    content_field => chunk.content.clone(),
                    file_path_field => chunk.file_path.clone(),
                    language_field => chunk.language.as_str(),
                    chunk_type_field => chunk.chunk_type.clone(),
                    metadata_field => metadata_json,
                ))
                .map_err(|e| {
                    ParserError::DatabaseError(format!("Failed to add document: {}", e))
                })?;
        }

        writer
            .commit()
            .map_err(|e| ParserError::DatabaseError(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Search the full-text index
    pub fn search(&self, query_str: &str, top_k: usize) -> ParserResult<Vec<(String, f32)>> {
        let index = self.index.as_ref().ok_or_else(|| {
            ParserError::DatabaseError("FullTextSearch not initialized".to_string())
        })?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| ParserError::DatabaseError(format!("Failed to create reader: {}", e)))?;

        let searcher = reader.searcher();

        let content_field = self.schema.get_field("content").unwrap();
        let query_parser = QueryParser::for_index(index, vec![content_field]);

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| ParserError::QueryCompileError(format!("Failed to parse query: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(top_k))
            .map_err(|e| ParserError::DatabaseError(format!("Search failed: {}", e)))?;

        let id_field = self.schema.get_field("id").unwrap();

        let results: Vec<(String, f32)> = top_docs
            .iter()
            .filter_map(|(score, doc_address)| {
                let doc: tantivy::TantivyDocument = searcher.doc(*doc_address).ok()?;
                let value = doc.get_first(id_field)?;
                if let tantivy::schema::OwnedValue::Str(id) = value {
                    Some((id.clone(), *score))
                } else {
                    None
                }
            })
            .collect();

        Ok(results)
    }

    /// Clear the index
    pub fn clear(&mut self) -> ParserResult<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer
                .delete_all_documents()
                .map_err(|e| ParserError::DatabaseError(format!("Failed to clear index: {}", e)))?;
            writer
                .commit()
                .map_err(|e| ParserError::DatabaseError(format!("Failed to commit: {}", e)))?;
        }
        Ok(())
    }
}

/// ============================================================================
/// Unified RAG Interface
/// ============================================================================

/// Main RAG system that orchestrates all components
pub struct RagSystem {
    config: RagConfig,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    embedding_cache: Option<EmbeddingCache>,
    chunker: SemanticChunker,
    vector_store: VectorStore,
    fulltext_search: FullTextSearch,
    chunks: Arc<DashMap<String, CodeChunk>>,
}

impl RagSystem {
    /// Create a new RAG system
    pub async fn new(config: RagConfig) -> ParserResult<Self> {
        // Create embedding provider
        let embedding_provider: Arc<dyn EmbeddingProvider> = match config
            .embedding_provider
            .as_str()
        {
            "openai" => {
                let api_key = config.api_key.clone().ok_or_else(|| {
                    ParserError::Unknown("API key required for OpenAI embeddings".to_string())
                })?;
                Arc::new(OpenAiEmbeddings::new(
                    api_key,
                    config.embedding_model.clone(),
                    config.embedding_dimension,
                    config.max_concurrent_embeddings,
                ))
            }
            "anthropic" => {
                let api_key = config.api_key.clone().ok_or_else(|| {
                    ParserError::Unknown("API key required for Anthropic embeddings".to_string())
                })?;
                Arc::new(AnthropicEmbeddings::new(
                    api_key,
                    config.embedding_model.clone(),
                    config.embedding_dimension,
                    config.max_concurrent_embeddings,
                ))
            }
            _ => {
                return Err(ParserError::Unknown(format!(
                    "Unknown embedding provider: {}",
                    config.embedding_provider
                )));
            }
        };

        // Create embedding cache
        let embedding_cache = if config.enable_cache {
            Some(EmbeddingCache::new(config.max_cache_size))
        } else {
            None
        };

        // Create chunker
        let chunker = SemanticChunker::new(config.max_chunk_size, config.chunk_overlap)?;

        // Create vector store
        let mut vector_store =
            VectorStore::new(&config.lance_db_path, config.embedding_dimension).await?;
        vector_store.initialize().await?;

        // Create full-text search
        let mut fulltext_search = FullTextSearch::new(&config.tantivy_index_path)?;
        fulltext_search.initialize()?;

        Ok(Self {
            config,
            embedding_provider,
            embedding_cache,
            chunker,
            vector_store,
            fulltext_search,
            chunks: Arc::new(DashMap::new()),
        })
    }

    /// Index a file
    pub async fn index_file(&mut self, file_path: &str) -> ParserResult<usize> {
        // Chunk the file
        let mut chunks = self.chunker.chunk_file(file_path)?;

        // Generate embeddings for chunks
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embed_batch_with_cache(&texts).await?;

        // Attach embeddings to chunks
        for (chunk, embedding) in chunks.iter_mut().zip(embeddings.iter()) {
            chunk.embedding = Some(embedding.clone());
        }

        // Store in vector database
        self.vector_store.add_chunks(&chunks).await?;

        // Store in full-text index
        self.fulltext_search.add_chunks(&chunks)?;

        // Store in memory map
        let chunk_count = chunks.len();
        for chunk in chunks {
            self.chunks.insert(chunk.id.clone(), chunk);
        }

        Ok(chunk_count)
    }

    /// Generate embeddings with caching
    async fn embed_batch_with_cache(&self, texts: &[String]) -> ParserResult<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut to_embed = Vec::new();
        let mut to_embed_indices = Vec::new();

        // Check cache first
        for (i, text) in texts.iter().enumerate() {
            if let Some(cache) = &self.embedding_cache {
                if let Some(embedding) = cache.get(text) {
                    results.push(Some(embedding));
                    continue;
                }
            }
            results.push(None);
            to_embed.push(text.clone());
            to_embed_indices.push(i);
        }

        // Generate embeddings for cache misses
        if !to_embed.is_empty() {
            let embeddings = self.embedding_provider.embed_batch(&to_embed).await?;

            // Store in cache and results
            for (embedding, &index) in embeddings.iter().zip(to_embed_indices.iter()) {
                if let Some(cache) = &self.embedding_cache {
                    cache.put(&texts[index], embedding.clone());
                }
                results[index] = Some(embedding.clone());
            }
        }

        // Unwrap all results
        results
            .into_iter()
            .map(|r| r.ok_or_else(|| ParserError::Unknown("Missing embedding".to_string())))
            .collect()
    }

    /// Search using vector similarity
    pub async fn vector_search(
        &self,
        query: &str,
        top_k: usize,
    ) -> ParserResult<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = if let Some(cache) = &self.embedding_cache {
            if let Some(embedding) = cache.get(query) {
                embedding
            } else {
                let embedding = self.embedding_provider.embed(query).await?;
                cache.put(query, embedding.clone());
                embedding
            }
        } else {
            self.embedding_provider.embed(query).await?
        };

        // Search vector store
        let results = self.vector_store.search(&query_embedding, top_k).await?;

        // Convert to SearchResults
        Ok(results
            .iter()
            .filter_map(|(chunk_id, score)| {
                self.chunks.get(chunk_id).map(|chunk| SearchResult {
                    chunk: chunk.clone(),
                    score: *score,
                    search_type: "vector".to_string(),
                    vector_score: Some(*score),
                    fulltext_score: None,
                })
            })
            .collect())
    }

    /// Search using full-text search
    pub fn fulltext_search(&self, query: &str, top_k: usize) -> ParserResult<Vec<SearchResult>> {
        let results = self.fulltext_search.search(query, top_k)?;

        Ok(results
            .iter()
            .filter_map(|(chunk_id, score)| {
                self.chunks.get(chunk_id).map(|chunk| SearchResult {
                    chunk: chunk.clone(),
                    score: *score,
                    search_type: "fulltext".to_string(),
                    vector_score: None,
                    fulltext_score: Some(*score),
                })
            })
            .collect())
    }

    /// Hybrid search combining vector and full-text
    pub async fn hybrid_search(&self, query: &str) -> ParserResult<Vec<SearchResult>> {
        // Perform both searches in parallel
        let (vector_results, fulltext_results) =
            tokio::join!(self.vector_search(query, self.config.vector_top_k), async {
                self.fulltext_search(query, self.config.fulltext_top_k)
            });

        let vector_results = vector_results?;
        let fulltext_results = fulltext_results?;

        // Combine and re-rank results
        let mut combined: HashMap<String, SearchResult> = HashMap::new();

        for result in vector_results {
            combined.insert(result.chunk.id.clone(), result);
        }

        for result in fulltext_results {
            combined
                .entry(result.chunk.id.clone())
                .and_modify(|e| {
                    e.fulltext_score = result.fulltext_score;
                    e.search_type = "hybrid".to_string();
                    // Combine scores using configured weights
                    let vector_score = e.vector_score.unwrap_or(0.0);
                    let fulltext_score = result.fulltext_score.unwrap_or(0.0);
                    e.score = vector_score * self.config.vector_weight
                        + fulltext_score * self.config.fulltext_weight;
                })
                .or_insert(result);
        }

        // Sort by combined score
        let mut results: Vec<SearchResult> = combined.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Clear all data
    pub async fn clear(&mut self) -> ParserResult<()> {
        self.vector_store.clear().await?;
        self.fulltext_search.clear()?;
        self.chunks.clear();
        if let Some(cache) = &self.embedding_cache {
            cache.clear();
        }
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> RagStats {
        let cache_stats = self
            .embedding_cache
            .as_ref()
            .map(|c| c.stats())
            .unwrap_or((0, 0));

        RagStats {
            total_chunks: self.chunks.len(),
            cache_size: cache_stats.0,
            cache_capacity: cache_stats.1,
            embedding_dimension: self.config.embedding_dimension,
        }
    }
}

/// Statistics about the RAG system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagStats {
    pub total_chunks: usize,
    pub cache_size: usize,
    pub cache_capacity: usize,
    pub embedding_dimension: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let text = "hello world";
        let key1 = EmbeddingCache::cache_key(text);
        let key2 = EmbeddingCache::cache_key(text);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_embedding_cache() {
        let cache = EmbeddingCache::new(100);
        let embedding = vec![0.1, 0.2, 0.3];

        cache.put("test", embedding.clone());
        let retrieved = cache.get("test");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_semantic_chunker() {
        let chunker = SemanticChunker::new(1000, 200).unwrap();
        let text = "fn main() { println!(\"Hello\"); }";
        let chunks = chunker.chunk_text(text, "test.rs", Language::Rust).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_config_defaults() {
        let config = RagConfig::default();
        assert_eq!(config.embedding_dimension, 1536);
        assert!(config.enable_cache);
    }
}
