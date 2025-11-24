# Thoughts System - Quick Start Guide

## 5-Minute Overview

The Thoughts System stores persistent memories for AI agents in `~/.descartes/thoughts/`. It's simple to use, secure (user-only access), and project-aware.

## Installation & Setup

Add to your Rust project's dependencies:

```toml
[dependencies]
descartes-core = { path = "../../core" }
```

## Basic Usage

### 1. Initialize Storage

```rust
use descartes_core::ThoughtsStorage;

// Create storage (initializes ~/.descartes/thoughts/ on first use)
let storage = ThoughtsStorage::new()?;
```

### 2. Save a Thought

```rust
use descartes_core::ThoughtMetadata;
use chrono::Utc;

let thought = ThoughtMetadata {
    id: "my-first-thought".to_string(),
    title: "How to optimize queries".to_string(),
    content: "Found that query X runs 3x faster with index Y".to_string(),
    tags: vec!["optimization".to_string(), "databases".to_string()],
    created_at: Utc::now().to_rfc3339(),
    modified_at: Utc::now().to_rfc3339(),
    agent_id: Some("agent-001".to_string()),
    project_id: None,
};

storage.save_thought(thought)?;
println!("Thought saved!");
```

### 3. Load a Thought

```rust
let thought = storage.load_thought("my-first-thought")?;
println!("Title: {}", thought.title);
println!("Content: {}", thought.content);
```

### 4. List All Thoughts

```rust
let thoughts = storage.list_thoughts()?;
println!("Found {} thoughts", thoughts.len());

for id in thoughts {
    let thought = storage.load_thought(&id)?;
    println!("- {}: {}", thought.title, thought.tags.join(", "));
}
```

### 5. Find by Tag

```rust
let optimization_thoughts = storage.list_thoughts_by_tag("optimization")?;
println!("Found {} optimization thoughts", optimization_thoughts.len());
```

## Directory Structure

After first use, you'll find:

```
~/.descartes/thoughts/
├── .metadata/index.json         # System info
├── archive/                     # Old thoughts
├── projects/                    # Project-specific
├── categories/                  # Categorized
└── my-first-thought/            # Your thought
    ├── metadata.json            # Full metadata
    └── content.txt              # Easy text access
```

## Common Tasks

### Archive Old Thoughts

```rust
storage.archive_thought("my-first-thought")?;
// Moved to ~/.descartes/thoughts/archive/my-first-thought/
```

### Get Storage Stats

```rust
let stats = storage.get_statistics()?;
println!("Total: {} thoughts, {} bytes",
    stats.total_thoughts,
    stats.total_size_bytes);

for (tag, count) in stats.tags {
    println!("  {}: {}", tag, count);
}
```

### Link Project to Global Thoughts

```rust
use std::path::Path;

let project_path = Path::new("/path/to/my/project");
storage.create_project_symlink(project_path)?;
// Now: /path/to/my/project/.thoughts -> ~/.descartes/thoughts/
```

## Data Format

Each thought is stored as JSON metadata + plain text content:

**metadata.json:**
```json
{
  "id": "my-first-thought",
  "title": "How to optimize queries",
  "content": "Found that query X runs 3x faster with index Y",
  "tags": ["optimization", "databases"],
  "created_at": "2024-11-23T21:30:00Z",
  "modified_at": "2024-11-23T21:30:00Z",
  "agent_id": "agent-001",
  "project_id": null
}
```

**content.txt:**
```
Found that query X runs 3x faster with index Y
```

## Error Handling

```rust
use descartes_core::ThoughtsError;

match storage.save_thought(thought) {
    Ok(_) => println!("Saved!"),
    Err(ThoughtsError::IoError(e)) => {
        eprintln!("IO error: {}", e);
    }
    Err(ThoughtsError::NoHomeDirectory) => {
        eprintln!("Could not find home directory");
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## ID Generation Tips

Generate unique IDs for thoughts:

```rust
use uuid::Uuid;

// Option 1: UUID
let id = Uuid::new_v4().to_string();

// Option 2: Agent + Timestamp
let id = format!("agent-001-{}", Uuid::new_v4());

// Option 3: Descriptive
let id = "agent-001-database-optimization-2024-11-23";
```

## Security

- Directories: `rwx------` (0o700) - user only
- Files: `rw-------` (0o600) - user only
- No encryption (use encrypted home directory if needed)
- No multi-user support (one user per thoughts store)

## Async Example

For use in async code:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    let storage = Arc::new(Mutex::new(ThoughtsStorage::new()?));

    // Spawn multiple tasks
    let storage1 = Arc::clone(&storage);
    let handle1 = tokio::spawn(async move {
        let s = storage1.lock().await;
        s.save_thought(thought1)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    });

    let storage2 = Arc::clone(&storage);
    let handle2 = tokio::spawn(async move {
        let s = storage2.lock().await;
        let thought = s.load_thought("my-thought")?;
        println!("{}", thought.title);
        Ok::<_, Box<dyn std::error::Error>>(())
    });

    handle1.await??;
    handle2.await??;
    Ok(())
}
```

## Testing

Run the thoughts module tests:

```bash
cd descartes/core
cargo test thoughts --lib -- --nocapture
```

## Next Steps

1. Read full documentation: `THOUGHTS_SYSTEM.md`
2. Check API docs: `cargo doc --open`
3. Integrate with your agent code
4. Start persisting insights!

## Limits & Performance

- Per-thought size: No hard limit (tested to 10MB+)
- Total thoughts: O(n) lookup time for list operations
- Recommended max: 10,000 thoughts before archiving
- Archive old thoughts regularly for performance

## Troubleshooting

**Q: "No home directory" error?**
- A: Use custom `ThoughtsConfig` to specify custom path

**Q: Permission denied?**
- A: Check `~/.descartes/` permissions with `ls -la ~/`

**Q: Symlinks not working on Windows?**
- A: Use `#[cfg(unix)]` or skip symlink feature on Windows

**Q: How do I clear everything?**
- A: `storage.clear_all()?;` (WARNING: destructive!)

## API Reference

| Method | Purpose |
|--------|---------|
| `new()` | Initialize with defaults |
| `with_config(config)` | Initialize with custom config |
| `initialize()` | Set up directory structure |
| `save_thought(thought)` | Persist a thought |
| `load_thought(id)` | Retrieve a thought |
| `list_thoughts()` | Get all thought IDs |
| `list_thoughts_by_tag(tag)` | Filter by tag |
| `create_project_symlink(path)` | Link project directory |
| `remove_project_symlink(path)` | Unlink project |
| `archive_thought(id)` | Move to archive |
| `get_statistics()` | Get storage stats |
| `clear_all()` | Delete everything |

## Examples Included

Full examples are available in:
- `/Users/reuben/gauntlet/cap/descartes/core/src/thoughts.rs` (inline tests)
- THOUGHTS_SYSTEM.md (usage patterns)
