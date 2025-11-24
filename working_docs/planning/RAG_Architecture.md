# Descartes High-Performance RAG Architecture

**Goal**: Achieve "snappy" performance for context retrieval by leveraging best-in-class embedded Rust engines.

## 1. The "Tri-Store" Architecture

To balance speed, semantic understanding, and structural awareness, we will use a hybrid storage layer.

| Component | Engine | Role | Why? |
| :--- | :--- | :--- | :--- |
| **Vector Store** | **LanceDB** | Semantic Search (Embeddings) | Serverless, Rust-native, handles disk-based vectors efficiently, 100x faster than Parquet. |
| **Text Search** | **Tantivy** | Keyword Search (FTS) | Fastest Rust FTS engine, Lucene-like features, instant indexing. |
| **Graph/Relation** | **SQLite** | Structure & Metadata | Standard SQL for complex graph queries (recursive CTEs), robust, ubiquitous. |

### 1.1 Why not just one?
*   **Vector-only** misses exact keyword matches (e.g., function names).
*   **FTS-only** misses semantic concepts ("auth system" vs "login function").
*   **Graph-only** misses unstructured content.
*   **Tri-Store** covers all bases with specialized, high-performance engines.

---

## 2. Ingestion Pipeline: AST-Based Indexing

Instead of naive text splitting (e.g., "every 500 chars"), we use **Tree-sitter** to understand the code structure.

### 2.1 The Parser
We will integrate `tree-sitter` with the following grammars:
*   **Rust/Python/JS**: For code structure (Functions, Classes, Imports).
*   **Markdown** (`tree-sitter-md`): For documentation structure (Headers, Lists, Code Blocks).
*   **Git** (`tree-sitter-gitcommit`): For commit message parsing.
*   **Git History** (`gitoxide`): For high-performance commit traversal and diffing (~4x faster than libgit2).

### 2.2 The "Semantic Chunking" Strategy
1.  **Parse**: Generate AST for the file.
2.  **Traverse**: Identify "Nodes of Interest" (e.g., a complete function definition).
3.  **Chunk**:
    *   **Small Chunk**: The function signature + docstring (High semantic density).
    *   **Large Chunk**: The entire function body (Context).
4.  **Embed**: Generate embeddings for chunks.
5.  **Index**:
    *   Store **Embedding** in LanceDB.
    *   Store **Raw Text** in Tantivy (for FTS).
    *   Store **Node Metadata** (File path, Line range, Parent Node ID) in SQLite.

---

## 3. Performance Optimizations (The "Snappy" Factor)

### 3.1 Zero-Copy Reads
*   **LanceDB** and **Tantivy** both support memory-mapping (mmap).
*   We will configure them to map indices directly into memory, allowing the OS page cache to handle hot data.

### 3.2 The "Hot Cache" (Optional)
*   If disk I/O becomes a bottleneck for raw text retrieval, we can introduce **Redb** (Pure Rust, ACID, MVCC) as a read-through cache for the most frequently accessed chunks.
*   *Initial Plan*: Rely on OS page cache + NVMe speeds. Add Redb only if profiling shows a need.

### 3.3 Parallel Indexing
*   Use `rayon` to parallelize the Tree-sitter parsing and Embedding generation across all CPU cores during initial project ingestion.

---

## 4. Querying Strategy

When an agent asks a question:
1.  **Parallel Search**:
    *   Query **LanceDB** for semantic matches ("Find code about authentication").
    *   Query **Tantivy** for exact keywords ("Find `User` struct").
2.  **Graph Expansion (SQLite)**:
    *   Take the top K results from above.
    *   Use SQLite to find "Related Nodes" (e.g., "Show me the `impl` block for this `struct`", or "Show me the test file that imports this module").
3.  **Reranking**:
    *   Combine results and rerank based on a "Relevance Score" (Semantic + Keyword + Graph Distance).
4.  **Context Construction**:
    *   Stream the final context to the agent.

## 5. Implementation Roadmap
1.  **Prototype**: Build a standalone CLI tool `descartes-index` that takes a file, parses it with Tree-sitter, and inserts into LanceDB/Tantivy.
2.  **Integrate**: Add this pipeline to the `AgentRunner` background worker.
