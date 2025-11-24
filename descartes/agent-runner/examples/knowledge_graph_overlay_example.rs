/// Example: Knowledge Graph Overlay Usage
///
/// This example demonstrates how to use the knowledge graph overlay system
/// to analyze a codebase, extract entities, and perform semantic queries.

use agent_runner::{
    FileTreeBuilder, KnowledgeGraphOverlay, KnowledgeNodeType, OverlayConfig,
};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Knowledge Graph Overlay Example ===\n");

    // Create a sample project to analyze
    let temp_dir = tempfile::TempDir::new()?;
    create_sample_project(temp_dir.path())?;

    println!("1. Building file tree...");
    let mut builder = FileTreeBuilder::new();
    let mut file_tree = builder.scan_directory(temp_dir.path())?;

    let stats = file_tree.stats();
    println!(
        "   File tree: {} files, {} directories\n",
        stats.file_count, stats.directory_count
    );

    println!("2. Generating knowledge graph overlay...");
    let mut overlay = KnowledgeGraphOverlay::new()?;
    let knowledge_graph = overlay.generate_and_link(&mut file_tree)?;

    let kg_stats = knowledge_graph.stats();
    println!(
        "   Knowledge graph: {} nodes, {} edges",
        kg_stats.total_nodes, kg_stats.total_edges
    );
    println!("   Node types:");
    for (node_type, count) in &kg_stats.node_type_counts {
        println!("     - {}: {}", node_type, count);
    }
    println!();

    // Demonstrate query operations
    demonstrate_queries(&overlay, &knowledge_graph)?;

    // Demonstrate file-based queries
    demonstrate_file_queries(&overlay, &knowledge_graph, temp_dir.path())?;

    // Demonstrate incremental updates
    demonstrate_incremental_update(&mut overlay, &file_tree, temp_dir.path())?;

    // Demonstrate cache
    demonstrate_caching(&overlay)?;

    println!("\n=== Example Complete ===");

    Ok(())
}

fn demonstrate_queries(
    overlay: &KnowledgeGraphOverlay,
    kg: &agent_runner::KnowledgeGraph,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Demonstrating query operations...\n");

    // Query 1: Find all functions
    println!("   Query 1: Find all functions");
    let functions = overlay.find_by_type(KnowledgeNodeType::Function, kg);
    println!("   Found {} functions:", functions.len());
    for func in functions.iter().take(5) {
        println!(
            "     - {} ({})",
            func.qualified_name,
            func.language.map(|l| l.as_str()).unwrap_or("unknown")
        );
        if let Some(sig) = &func.signature {
            println!("       Signature: {}", sig);
        }
    }
    if functions.len() > 5 {
        println!("     ... and {} more", functions.len() - 5);
    }
    println!();

    // Query 2: Find all classes
    println!("   Query 2: Find all classes");
    let classes = overlay.find_by_type(KnowledgeNodeType::Class, kg);
    println!("   Found {} classes:", classes.len());
    for class in classes.iter().take(5) {
        println!("     - {}", class.qualified_name);
        if let Some(doc) = &class.description {
            let preview = if doc.len() > 60 {
                format!("{}...", &doc[..60])
            } else {
                doc.clone()
            };
            println!("       Doc: {}", preview.replace('\n', " "));
        }
    }
    println!();

    // Query 3: Search by name pattern
    println!("   Query 3: Search for entities containing 'process'");
    let results = overlay.find_by_name_pattern("process", kg);
    println!("   Found {} matches:", results.len());
    for entity in results.iter().take(3) {
        println!(
            "     - {} ({})",
            entity.qualified_name,
            entity.content_type.as_str()
        );
    }
    println!();

    // Query 4: Find structs
    println!("   Query 4: Find all structs");
    let structs = overlay.find_by_type(KnowledgeNodeType::Struct, kg);
    println!("   Found {} structs:", structs.len());
    for s in structs.iter() {
        println!("     - {}", s.qualified_name);
        if !s.children.is_empty() {
            println!("       Methods: {}", s.children.len());
        }
    }
    println!();

    Ok(())
}

fn demonstrate_file_queries(
    overlay: &KnowledgeGraphOverlay,
    kg: &agent_runner::KnowledgeGraph,
    base_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Demonstrating file-based queries...\n");

    // Find entities in a specific file
    let main_rs = base_path.join("src/main.rs");
    if main_rs.exists() {
        println!("   Entities in main.rs:");
        let entities = overlay.find_entities_in_file(&main_rs, kg);
        for entity in entities.iter() {
            println!(
                "     - {} ({}) at line {}",
                entity.name,
                entity.content_type.as_str(),
                entity.file_references[0].line_range.0
            );
        }
        println!();
    }

    // Find definition of a specific entity
    if let Some(node) = kg.nodes.values().next() {
        println!("   Finding definition of '{}':", node.qualified_name);
        if let Some(def) = overlay.find_definition(&node.qualified_name, kg) {
            println!(
                "     Defined in: {:?}",
                def.file_path.file_name().unwrap()
            );
            println!("     Lines: {:?}", def.line_range);
        }
        println!();
    }

    Ok(())
}

fn demonstrate_incremental_update(
    overlay: &mut KnowledgeGraphOverlay,
    file_tree: &agent_runner::FileTree,
    base_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("5. Demonstrating incremental updates...\n");

    let mut kg = overlay.generate_knowledge_overlay(file_tree)?;
    let initial_count = kg.nodes.len();
    println!("   Initial knowledge graph: {} nodes", initial_count);

    // Modify a file
    let test_file = base_path.join("src/main.rs");
    let original_content = fs::read_to_string(&test_file)?;

    fs::write(
        &test_file,
        format!(
            "{}\n\n/// New function added dynamically\nfn dynamic_function() {{\n    println!(\"Dynamic\");\n}}\n",
            original_content
        ),
    )?;

    println!("   Modified main.rs (added new function)");

    // Update the knowledge graph
    overlay.update_file(&test_file, file_tree, &mut kg)?;

    let updated_count = kg.nodes.len();
    println!("   Updated knowledge graph: {} nodes", updated_count);
    println!(
        "   Change: {} nodes",
        updated_count as i32 - initial_count as i32
    );
    println!();

    // Restore original content
    fs::write(&test_file, original_content)?;

    Ok(())
}

fn demonstrate_caching(
    overlay: &KnowledgeGraphOverlay,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("6. Cache statistics...\n");

    let cache_stats = overlay.cache_stats();
    println!("   Total cache entries: {}", cache_stats.total_entries);
    println!("   Total cached nodes: {}", cache_stats.total_nodes);
    println!("   Total cached edges: {}", cache_stats.total_edges);
    println!();

    Ok(())
}

fn create_sample_project(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir.join("src"))?;

    // Create main.rs
    fs::write(
        dir.join("src/main.rs"),
        r#"//! Main application module
//!
//! This module contains the main entry point and core application logic.

mod database;
mod api;

use database::Database;
use api::ApiServer;

/// Main entry point
fn main() {
    let db = Database::new("app.db");
    let server = ApiServer::new(db);

    server.run();
}

/// Application configuration
struct Config {
    port: u16,
    host: String,
    debug: bool,
}

impl Config {
    /// Create default configuration
    fn default() -> Self {
        Config {
            port: 8080,
            host: "localhost".to_string(),
            debug: false,
        }
    }

    /// Load configuration from environment
    fn from_env() -> Self {
        // Implementation here
        Self::default()
    }
}

/// Process incoming requests
fn process_request(request: &str) -> String {
    let result = validate_request(request);
    format_response(&result)
}

/// Validate request format
fn validate_request(request: &str) -> bool {
    !request.is_empty()
}

/// Format response data
fn format_response(data: &bool) -> String {
    format!("{{\"valid\": {}}}", data)
}
"#,
    )?;

    // Create database.rs
    fs::write(
        dir.join("src/database.rs"),
        r#"//! Database module
//!
//! Provides database connectivity and operations.

/// Database connection manager
pub struct Database {
    connection_string: String,
    max_connections: usize,
}

impl Database {
    /// Create a new database connection
    pub fn new(connection_string: &str) -> Self {
        Database {
            connection_string: connection_string.to_string(),
            max_connections: 10,
        }
    }

    /// Execute a query
    pub fn query(&self, sql: &str) -> Result<Vec<String>, String> {
        // Implementation here
        Ok(Vec::new())
    }

    /// Insert data
    pub fn insert(&mut self, table: &str, data: &str) -> Result<(), String> {
        // Implementation here
        Ok(())
    }

    /// Close the database connection
    pub fn close(&mut self) {
        // Cleanup
    }
}

/// Database transaction
pub struct Transaction {
    db: Database,
    committed: bool,
}

impl Transaction {
    /// Start a new transaction
    pub fn begin(db: Database) -> Self {
        Transaction {
            db,
            committed: false,
        }
    }

    /// Commit the transaction
    pub fn commit(&mut self) {
        self.committed = true;
    }

    /// Rollback the transaction
    pub fn rollback(&mut self) {
        self.committed = false;
    }
}
"#,
    )?;

    // Create api.rs
    fs::write(
        dir.join("src/api.rs"),
        r#"//! API Server module
//!
//! HTTP API server implementation.

use crate::database::Database;

/// API Server
pub struct ApiServer {
    db: Database,
    port: u16,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(db: Database) -> Self {
        ApiServer { db, port: 8080 }
    }

    /// Run the server
    pub fn run(&self) {
        println!("Server running on port {}", self.port);
    }

    /// Handle GET request
    pub fn handle_get(&self, path: &str) -> String {
        format!("GET {}", path)
    }

    /// Handle POST request
    pub fn handle_post(&mut self, path: &str, body: &str) -> String {
        format!("POST {} with body: {}", path, body)
    }
}

/// Request handler
pub fn handle_request(method: &str, path: &str) -> String {
    match method {
        "GET" => format!("Handling GET {}", path),
        "POST" => format!("Handling POST {}", path),
        _ => "Method not allowed".to_string(),
    }
}
"#,
    )?;

    // Create a Python file
    fs::write(
        dir.join("src/utils.py"),
        r#"""Utility functions for the application"""

class DataValidator:
    """Validates data formats"""

    def __init__(self):
        self.rules = {}

    def add_rule(self, name, rule):
        """Add a validation rule"""
        self.rules[name] = rule

    def validate(self, data):
        """Validate data against all rules"""
        for name, rule in self.rules.items():
            if not rule(data):
                return False
        return True

def process_data(data):
    """Process data through the pipeline"""
    cleaned = clean_data(data)
    transformed = transform_data(cleaned)
    return validate_output(transformed)

def clean_data(data):
    """Clean and normalize data"""
    return data.strip().lower()

def transform_data(data):
    """Transform data format"""
    return data.upper()

def validate_output(data):
    """Validate output format"""
    return len(data) > 0
"#,
    )?;

    Ok(())
}
