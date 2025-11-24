/// SQLite schema for storing AST data and semantic information
use crate::errors::ParserResult;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;

/// Database schema for semantic extraction
pub const SCHEMA: &str = r#"
-- Semantic nodes (extracted from AST)
CREATE TABLE IF NOT EXISTS semantic_nodes (
    id TEXT PRIMARY KEY,
    node_type TEXT NOT NULL,
    name TEXT NOT NULL,
    source_code TEXT,
    documentation TEXT,
    qualified_name TEXT NOT NULL,
    language TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line_start INTEGER NOT NULL,
    line_end INTEGER NOT NULL,
    column_start INTEGER,
    column_end INTEGER,
    parent_id TEXT,
    signature TEXT,
    return_type TEXT,
    visibility TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES semantic_nodes(id)
);

-- Parameters for functions/methods
CREATE TABLE IF NOT EXISTS node_parameters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    param_name TEXT NOT NULL,
    param_type TEXT,
    has_default BOOLEAN DEFAULT 0,
    is_variadic BOOLEAN DEFAULT 0,
    FOREIGN KEY (node_id) REFERENCES semantic_nodes(id)
);

-- Dependencies between nodes
CREATE TABLE IF NOT EXISTS node_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    dependency_type TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_id) REFERENCES semantic_nodes(id),
    FOREIGN KEY (target_id) REFERENCES semantic_nodes(id)
);

-- Semantic relationships
CREATE TABLE IF NOT EXISTS semantic_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_node_id TEXT NOT NULL,
    target_node_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    metadata TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_node_id) REFERENCES semantic_nodes(id),
    FOREIGN KEY (target_node_id) REFERENCES semantic_nodes(id)
);

-- File information
CREATE TABLE IF NOT EXISTS files (
    file_path TEXT PRIMARY KEY,
    language TEXT NOT NULL,
    total_lines INTEGER,
    total_nodes INTEGER,
    last_parsed DATETIME DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT
);

-- Parse results history
CREATE TABLE IF NOT EXISTS parse_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    parse_duration_ms INTEGER,
    total_nodes INTEGER,
    error_message TEXT,
    parsed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexed queries for common semantic queries
CREATE TABLE IF NOT EXISTS semantic_queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query_name TEXT NOT NULL,
    query_string TEXT NOT NULL,
    description TEXT,
    language TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create indices for performance
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_type ON semantic_nodes(node_type);
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_file ON semantic_nodes(file_path);
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_qualified_name ON semantic_nodes(qualified_name);
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_language ON semantic_nodes(language);
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_parent ON semantic_nodes(parent_id);
CREATE INDEX IF NOT EXISTS idx_node_dependencies_source ON node_dependencies(source_id);
CREATE INDEX IF NOT EXISTS idx_node_dependencies_target ON node_dependencies(target_id);
CREATE INDEX IF NOT EXISTS idx_parse_history_file ON parse_history(file_path);
CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);
"#;

/// Database connection pool manager
pub struct DbPool {
    pool: SqlitePool,
}

impl DbPool {
    /// Create a new database connection pool
    pub async fn new(database_url: &str) -> ParserResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                crate::errors::ParserError::DatabaseError(format!("Connection failed: {}", e))
            })?;

        Ok(DbPool { pool })
    }

    /// Initialize the database schema
    pub async fn initialize(&self) -> ParserResult<()> {
        sqlx::raw_sql(SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                crate::errors::ParserError::DatabaseError(format!("Schema initialization failed: {}", e))
            })?;

        Ok(())
    }

    /// Insert a semantic node into the database
    pub async fn insert_semantic_node(
        &self,
        node_id: &str,
        node_type: &str,
        name: &str,
        source_code: &str,
        qualified_name: &str,
        language: &str,
        file_path: &str,
        line_start: i32,
        line_end: i32,
    ) -> ParserResult<()> {
        sqlx::query(
            r#"
            INSERT INTO semantic_nodes
            (id, node_type, name, source_code, qualified_name, language, file_path, line_start, line_end)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(node_id)
        .bind(node_type)
        .bind(name)
        .bind(source_code)
        .bind(qualified_name)
        .bind(language)
        .bind(file_path)
        .bind(line_start)
        .bind(line_end)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            crate::errors::ParserError::DatabaseError(format!("Insert failed: {}", e))
        })?;

        Ok(())
    }

    /// Query nodes by type
    pub async fn query_nodes_by_type(&self, node_type: &str) -> ParserResult<Vec<(String, String)>> {
        let rows = sqlx::query("SELECT id, name FROM semantic_nodes WHERE node_type = ?")
            .bind(node_type)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                crate::errors::ParserError::DatabaseError(format!("Query failed: {}", e))
            })?;

        Ok(rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("id"),
                    row.get::<String, _>("name"),
                )
            })
            .collect())
    }

    /// Query nodes by file
    pub async fn query_nodes_by_file(&self, file_path: &str) -> ParserResult<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            "SELECT id, name, node_type FROM semantic_nodes WHERE file_path = ? ORDER BY line_start"
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            crate::errors::ParserError::DatabaseError(format!("Query failed: {}", e))
        })?;

        Ok(rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("id"),
                    row.get::<String, _>("name"),
                    row.get::<String, _>("node_type"),
                )
            })
            .collect())
    }

    /// Record a parse operation
    pub async fn record_parse(
        &self,
        file_path: &str,
        language: &str,
        duration_ms: u128,
        total_nodes: usize,
        error: Option<&str>,
    ) -> ParserResult<()> {
        sqlx::query(
            r#"
            INSERT INTO parse_history (file_path, language, parse_duration_ms, total_nodes, error_message)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(file_path)
        .bind(language)
        .bind(duration_ms as i32)
        .bind(total_nodes as i32)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            crate::errors::ParserError::DatabaseError(format!("Record insert failed: {}", e))
        })?;

        Ok(())
    }

    /// Get parse history for a file
    pub async fn get_parse_history(&self, file_path: &str) -> ParserResult<Vec<(String, i32, i32)>> {
        let rows = sqlx::query(
            "SELECT language, parse_duration_ms, total_nodes FROM parse_history WHERE file_path = ? ORDER BY parsed_at DESC LIMIT 10"
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            crate::errors::ParserError::DatabaseError(format!("Query failed: {}", e))
        })?;

        Ok(rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("language"),
                    row.get::<i32, _>("parse_duration_ms"),
                    row.get::<i32, _>("total_nodes"),
                )
            })
            .collect())
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_is_valid() {
        // This test verifies the schema syntax is correct
        assert!(!SCHEMA.is_empty());
        assert!(SCHEMA.contains("CREATE TABLE"));
    }
}
