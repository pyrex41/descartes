# Descartes Thoughts System

## Overview

The Thoughts System is a global persistent memory storage solution that enables AI agents in the Descartes orchestration platform to maintain context and learnings across sessions. This system provides a structured, secure, and efficient way to store and retrieve agent thoughts, insights, and contextual information.

## Architecture

### Directory Structure

The thoughts system organizes data in a hierarchical directory structure under `~/.descartes/thoughts/`:

```
~/.descartes/thoughts/
├── .metadata/               # System metadata directory
│   └── index.json          # Root metadata tracking file
├── archive/                # Archived thoughts
├── projects/               # Project-specific thoughts
├── categories/             # Categorized thoughts
├── [thought-id]/           # Individual thought directory
│   ├── metadata.json       # Thought metadata
│   └── content.txt         # Thought content (for easy access)
├── [thought-id]/           # ... more thoughts
└── ...
```

### Permissions & Security

All directories and files are created with user-only access permissions:
- **Directory permissions**: `0o700` (rwx------)
- **File permissions**: `0o600` (rw-------)

This ensures that thoughts are private and cannot be accessed by other users on the system.

## Core Components

### ThoughtsStorage

The main struct that manages all operations on the thoughts storage system.

```rust
pub struct ThoughtsStorage {
    config: ThoughtsConfig,
}
```

#### Key Methods

- `new()` - Create a new instance with default configuration
- `with_config(config)` - Create with custom configuration
- `initialize()` - Set up the global storage directory structure
- `save_thought(thought)` - Persist a thought to storage
- `load_thought(id)` - Retrieve a specific thought
- `list_thoughts()` - List all thought IDs
- `list_thoughts_by_tag(tag)` - Find thoughts by tag
- `create_project_symlink(path)` - Create project-specific symlink
- `remove_project_symlink(path)` - Remove project symlink
- `archive_thought(id)` - Move a thought to archive
- `get_statistics()` - Get storage usage statistics
- `clear_all()` - Destructive: clear all thoughts

### ThoughtsConfig

Configuration object for the storage system.

```rust
pub struct ThoughtsConfig {
    pub global_root: PathBuf,      // Root directory path
    pub dir_permissions: u32,      // Unix permissions for directories
    pub file_permissions: u32,     // Unix permissions for files
}
```

### ThoughtMetadata

Represents a single thought with all associated metadata.

```rust
pub struct ThoughtMetadata {
    pub id: String,                    // Unique identifier
    pub title: String,                 // Title/summary
    pub content: String,               // Full content
    pub tags: Vec<String>,             // Categorization tags
    pub created_at: String,            // ISO 8601 timestamp
    pub modified_at: String,           // ISO 8601 timestamp
    pub agent_id: Option<String>,      // Creating agent ID
    pub project_id: Option<String>,    // Associated project ID
}
```

### StorageStatistics

Aggregated statistics about the storage system.

```rust
pub struct StorageStatistics {
    pub total_thoughts: usize,                      // Total count
    pub total_size_bytes: u64,                      // Total disk usage
    pub tags: HashMap<String, usize>,               // Counts per tag
}
```

## Error Handling

The system uses a dedicated error type with comprehensive error variants:

```rust
pub enum ThoughtsError {
    IoError(std::io::Error),
    NoHomeDirectory,
    InvalidPath(String),
    PermissionDenied(String),
    SymlinkError(String),
    SerializationError(serde_json::Error),
}

pub type ThoughtsResult<T> = Result<T, ThoughtsError>;
```

## Usage Examples

### Basic Initialization

```rust
use descartes_core::ThoughtsStorage;

// Initialize with default settings
let storage = ThoughtsStorage::new()?;

// Or with custom configuration
let config = ThoughtsConfig {
    global_root: PathBuf::from("/custom/path/thoughts"),
    dir_permissions: 0o700,
    file_permissions: 0o600,
};
let storage = ThoughtsStorage::with_config(config)?;
```

### Saving and Loading Thoughts

```rust
use descartes_core::{ThoughtsStorage, ThoughtMetadata};
use chrono::Utc;

let storage = ThoughtsStorage::new()?;

// Create a thought
let thought = ThoughtMetadata {
    id: "agent-01-insight".to_string(),
    title: "Pattern Recognition Insight".to_string(),
    content: "Discovered that certain error patterns correlate with...".to_string(),
    tags: vec!["machine-learning".to_string(), "optimization".to_string()],
    created_at: Utc::now().to_rfc3339(),
    modified_at: Utc::now().to_rfc3339(),
    agent_id: Some("agent-01".to_string()),
    project_id: Some("project-alpha".to_string()),
};

// Save the thought
storage.save_thought(thought)?;

// Load it back
let loaded = storage.load_thought("agent-01-insight")?;
println!("Thought: {}", loaded.title);
```

### Organizing Thoughts by Tags

```rust
// Find all optimization-related thoughts
let opt_thoughts = storage.list_thoughts_by_tag("optimization")?;

for thought_id in opt_thoughts {
    let thought = storage.load_thought(&thought_id)?;
    println!("{}: {}", thought.title, thought.content);
}
```

### Project Integration

```rust
use std::path::Path;

let storage = ThoughtsStorage::new()?;
let project_path = Path::new("/path/to/project");

// Create a symlink so the project can access global thoughts
storage.create_project_symlink(project_path)?;

// Now the project has .thoughts -> ~/.descartes/thoughts

// Clean up when needed
storage.remove_project_symlink(project_path)?;
```

### Archiving Thoughts

```rust
// Archive completed or obsolete thoughts
storage.archive_thought("agent-01-insight")?;

// Archived thoughts are moved to ~/.descartes/thoughts/archive/
// and no longer appear in list_thoughts()
```

### Statistics and Monitoring

```rust
let stats = storage.get_statistics()?;

println!("Total thoughts: {}", stats.total_thoughts);
println!("Total size: {} bytes", stats.total_size_bytes);

for (tag, count) in stats.tags {
    println!("  {}: {} thoughts", tag, count);
}
```

## File Organization

### Thought Directory Layout

Each thought occupies its own directory:

```
~/.descartes/thoughts/agent-01-insight/
├── metadata.json          # Complete thought metadata (JSON)
└── content.txt            # Thought content (plain text)
```

The `metadata.json` file contains:
```json
{
  "id": "agent-01-insight",
  "title": "Pattern Recognition Insight",
  "content": "Discovered that...",
  "tags": ["machine-learning", "optimization"],
  "created_at": "2024-11-23T21:30:00Z",
  "modified_at": "2024-11-23T21:30:00Z",
  "agent_id": "agent-01",
  "project_id": "project-alpha"
}
```

### Root Metadata

The `.metadata/index.json` file tracks system-wide statistics:

```json
{
  "version": "1.0",
  "created_at": "2024-11-23T21:30:00Z",
  "total_thoughts": 42,
  "categories": ["optimization", "debugging", "architecture"]
}
```

## Symlink Strategy

The thoughts system uses symbolic links to enable project-specific access to global thoughts without duplication:

### Benefits

- **No duplication**: Single storage location for all thoughts
- **Shared context**: Multiple projects/agents can access the same thoughts
- **Easy access**: Projects have `.thoughts` symlink at their root
- **Cross-platform compatibility**: Symlinks work well on Unix/Linux/macOS

### Limitations

- **Windows support**: Symlinks require administrator privileges or specific OS versions
- **Symlink verification**: Always verify symlink targets before use

## Concurrency & Thread Safety

The `ThoughtsStorage` implementation:
- Uses standard Rust file operations (thread-safe via OS)
- Does NOT provide internal locking (caller responsible for synchronization)
- Suitable for single-threaded or async contexts with external coordination
- File system operations are atomic where possible

For multi-threaded scenarios, wrap the storage in a `Arc<Mutex<ThoughtsStorage>>`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let storage = Arc::new(Mutex::new(ThoughtsStorage::new()?));

// In an async context
{
    let storage = storage.lock().await;
    storage.save_thought(thought)?;
}
```

## Testing

The module includes comprehensive unit tests covering:
- Directory initialization
- Saving and loading thoughts
- Listing and filtering
- Archiving operations
- Statistics generation
- Clear operations

Run tests with:
```bash
cd descartes/core
cargo test thoughts --lib
```

## Integration with Descartes

### In the CLI

The thoughts system can be integrated into the CLI for agent interaction:

```rust
// In cli/src/main.rs
Commands::Thoughts { subcommand } => {
    match subcommand {
        ThoughtsSubcommand::Save { title, content, tags } => {
            let storage = ThoughtsStorage::new()?;
            let thought = ThoughtMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                title,
                content,
                tags,
                created_at: Utc::now().to_rfc3339(),
                modified_at: Utc::now().to_rfc3339(),
                agent_id: None,
                project_id: None,
            };
            storage.save_thought(thought)?;
        }
        // ... more subcommands
    }
}
```

### In Agent Runners

Agents can automatically persist insights:

```rust
// In agent implementation
impl Agent {
    async fn process_insight(&self, insight: String) -> Result<()> {
        let storage = ThoughtsStorage::new()?;
        let thought = ThoughtMetadata {
            id: format!("{}-{}", self.id, uuid::Uuid::new_v4()),
            title: "Agent Insight".to_string(),
            content: insight,
            tags: vec![self.domain.clone()],
            created_at: Utc::now().to_rfc3339(),
            modified_at: Utc::now().to_rfc3339(),
            agent_id: Some(self.id.clone()),
            project_id: Some(self.project_id.clone()),
        };
        storage.save_thought(thought)?;
        Ok(())
    }
}
```

## Performance Considerations

### Directory Lookup Efficiency

- `list_thoughts()` scans the main directory: O(n) where n = total thoughts
- `list_thoughts_by_tag()` loads and filters: O(n * k) where k = avg thoughts per tag
- For large collections (>10k thoughts), consider:
  - Implementing indexed tag lookup
  - Caching the metadata index
  - Archiving old thoughts regularly

### File I/O Optimization

- Thoughts are stored in separate directories for quick access
- Metadata and content are split for easy text-only retrieval
- Consider memory mapping for very large thought content

### Storage Growth

Monitor with `get_statistics()`:
```rust
let stats = storage.get_statistics()?;
if stats.total_size_bytes > 100_000_000 {
    // Archive old thoughts or implement retention policy
}
```

## Future Enhancements

### Planned Features

1. **Database Backend**: Optional SQLite backend for faster indexing
2. **Full-Text Search**: Index and search thought content
3. **Relationship Tracking**: Link related thoughts
4. **Version History**: Track changes to thoughts over time
5. **Encryption**: Optional encryption for sensitive thoughts
6. **Replication**: Sync thoughts across multiple systems
7. **TTL Support**: Auto-archive old thoughts

### Extension Points

The system is designed for extension:
- Custom `ThoughtsConfig` implementations
- Wrapper structs for domain-specific metadata
- Custom serialization formats (beyond JSON)
- Alternative storage backends

## Troubleshooting

### No Home Directory

```
Error: Failed to determine home directory
```

Solution: Explicitly specify `global_root` in custom `ThoughtsConfig`.

### Permission Denied

```
Error: Permission denied: /home/user/.descartes/thoughts
```

Solution: Ensure user has write access to home directory. Run:
```bash
chmod 700 ~/.descartes/thoughts
```

### Symlink Errors

```
Error: Symlinks not supported on this platform
```

Solution: Symlink feature is Unix-only. Use `#[cfg(unix)]` guards or skip symlink creation on Windows.

### Thought Not Found

```
Error: Invalid path: Thought not found: some-id
```

Solution: Verify the thought ID. List all thoughts with `list_thoughts()` to find valid IDs.

## References

- Implementation: `/Users/reuben/gauntlet/cap/descartes/core/src/thoughts.rs`
- Core Module: `/Users/reuben/gauntlet/cap/descartes/core/src/lib.rs`
- Tests: Built-in unit tests in `thoughts.rs` (cfg[test])
