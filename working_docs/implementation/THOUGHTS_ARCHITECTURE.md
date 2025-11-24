# Thoughts System - Architecture Reference

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Descartes Core Library                   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            ThoughtsStorage (public API)              │  │
│  │                                                      │  │
│  │  ├─ initialize()                                   │  │
│  │  ├─ save_thought(ThoughtMetadata)                 │  │
│  │  ├─ load_thought(id: &str)                        │  │
│  │  ├─ list_thoughts()                               │  │
│  │  ├─ list_thoughts_by_tag(tag: &str)               │  │
│  │  ├─ create_project_symlink(path)                  │  │
│  │  ├─ remove_project_symlink(path)                  │  │
│  │  ├─ archive_thought(id)                           │  │
│  │  ├─ get_statistics()                              │  │
│  │  └─ clear_all()                                   │  │
│  │                                                      │  │
│  │  config: ThoughtsConfig                             │  │
│  └──────────────────────────────────────────────────────┘  │
│            ↓                                                 │
│  ┌─────────────────────────┬──────────────────┐            │
│  │   ThoughtsConfig        │  ThoughtMetadata │            │
│  │                         │                  │            │
│  │ • global_root: PathBuf  │ • id: String     │            │
│  │ • dir_perms: u32 (0o700)│ • title: String  │            │
│  │ • file_perms: u32 (0o600)│ • content: String│           │
│  │                         │ • tags: Vec<T>   │            │
│  │                         │ • created_at: Ts │            │
│  │                         │ • modified_at: Ts│            │
│  │                         │ • agent_id: Opt  │            │
│  │                         │ • project_id: Opt│            │
│  └─────────────────────────┴──────────────────┘            │
│            ↓                                                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │             Error Handling                           │  │
│  │                                                      │  │
│  │  ThoughtsError (enum)       ThoughtsResult<T>       │  │
│  │  ├─ IoError                 = Result<T, E>          │  │
│  │  ├─ NoHomeDirectory                                 │  │
│  │  ├─ InvalidPath                                     │  │
│  │  ├─ PermissionDenied                                │  │
│  │  ├─ SymlinkError                                    │  │
│  │  └─ SerializationError                              │  │
│  └──────────────────────────────────────────────────────┘  │
│            ↓                                                 │
└─────────────────────────────────────────────────────────────┘
         ↓
    File System Layer
         ↓
┌──────────────────────────────────────────────────────────────┐
│                  ~/.descartes/thoughts/                      │
│                                                              │
│  ┌────────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   .metadata/   │  │   archive/   │  │  projects/   │    │
│  │ index.json     │  │ [thought]/   │  │ [symlinks]   │    │
│  └────────────────┘  │   └─ ...     │  └──────────────┘    │
│                      └──────────────┘                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ categories/  │  │ thought-1/   │  │ thought-N/   │      │
│  │ [symlinks]   │  │ ├─ metadata  │  │ ├─ metadata  │      │
│  └──────────────┘  │ └─ content   │  │ └─ content   │      │
│                    └──────────────┘  └──────────────┘      │
└──────────────────────────────────────────────────────────────┘
```

## Data Flow Diagrams

### Saving a Thought

```
Application
    ↓
    └─→ ThoughtMetadata {
        id, title, content, tags,
        created_at, modified_at,
        agent_id, project_id
    }
    ↓
ThoughtsStorage::save_thought()
    ├─→ Validate thought
    ├─→ Create directory
    │   └─→ ~/.descartes/thoughts/[thought-id]/
    ├─→ Serialize metadata
    │   └─→ ~/.descartes/thoughts/[thought-id]/metadata.json
    ├─→ Write content
    │   └─→ ~/.descartes/thoughts/[thought-id]/content.txt
    ├─→ Set permissions (0o700, 0o600)
    └─→ Return ThoughtsResult<()>
    ↓
File System
    └─→ Persistent Storage
```

### Loading and Listing Thoughts

```
Application
    ├─→ load_thought(id)
    │   ├─→ Read metadata.json
    │   ├─→ Deserialize to ThoughtMetadata
    │   └─→ Return to caller
    │
    ├─→ list_thoughts()
    │   ├─→ Scan ~/.descartes/thoughts/
    │   ├─→ Filter for thought directories
    │   └─→ Return Vec<String> of IDs
    │
    └─→ list_thoughts_by_tag(tag)
        ├─→ Get all thought IDs
        ├─→ Load each metadata.json
        ├─→ Filter by matching tag
        └─→ Return filtered Vec<String>
```

### Project Symlink Creation

```
Application
    ├─→ Project Path: /path/to/project
    │
    └─→ create_project_symlink()
        ├─→ Global Root: ~/.descartes/thoughts/
        ├─→ Target: /path/to/project/.thoughts
        ├─→ Create symlink
        │   └─→ .thoughts → ~/.descartes/thoughts/
        ├─→ Set permissions
        └─→ Return ThoughtsResult<()>
```

## Component Interactions

### ThoughtsStorage ↔ File System

```
ThoughtsStorage
    │
    ├─→ [Create] initialize()
    │   └─→ fs::create_dir_all()
    │   └─→ fs::set_permissions()
    │
    ├─→ [Write] save_thought()
    │   ├─→ fs::create_dir_all()
    │   ├─→ fs::write() metadata
    │   ├─→ fs::write() content
    │   └─→ fs::set_permissions()
    │
    ├─→ [Read] load_thought()
    │   └─→ fs::read_to_string()
    │   └─→ serde_json::from_str()
    │
    ├─→ [Scan] list_thoughts()
    │   └─→ fs::read_dir()
    │   └─→ path.is_dir()
    │
    ├─→ [Move] archive_thought()
    │   └─→ fs::rename()
    │
    ├─→ [Link] create_project_symlink()
    │   └─→ unix_fs::symlink()
    │
    └─→ [Delete] remove_project_symlink()
        └─→ fs::remove_file()
```

## Data Organization Patterns

### Thought Storage Pattern

```
Individual Thought Directory
└── ~/.descartes/thoughts/[thought-id]/
    ├── metadata.json
    │   └─ Complete serialized ThoughtMetadata
    │      {
    │        "id": "thought-id",
    │        "title": "...",
    │        "content": "...",
    │        "tags": [...],
    │        "created_at": "ISO8601",
    │        "modified_at": "ISO8601",
    │        "agent_id": "...",
    │        "project_id": "..."
    │      }
    │
    └── content.txt
        └─ Plain text for quick access
           (identical to metadata.content)
```

### Root Directory Structure

```
~/.descartes/thoughts/
├── .metadata/
│   └── index.json
│       └─ System-level statistics
│          {
│            "version": "1.0",
│            "created_at": "...",
│            "total_thoughts": N,
│            "categories": [...]
│          }
│
├── archive/
│   └── [archived-thought-id]/
│       ├── metadata.json
│       └── content.txt
│
├── projects/
│   └── [project-specific-thoughts]
│
├── categories/
│   └── [categorized-thoughts]
│
└── [active-thought-id]/
    ├── metadata.json
    └── content.txt
```

## State Transitions

### Thought Lifecycle

```
[Created]
   │
   ├─→ save_thought()
   │   └─→ [Persisted]
   │
   ├─→ load_thought() / list_thoughts()
   │   └─→ [Accessed]
   │
   ├─→ (modify and save)
   │   └─→ [Updated]
   │
   └─→ archive_thought()
       └─→ [Archived]
```

## Synchronization Model

### Thread Safety

```
ThoughtsStorage
    ├─→ NOT internally thread-safe
    │   └─→ Caller responsible for synchronization
    │
    ├─→ For multi-threaded use:
    │   └─→ Arc<Mutex<ThoughtsStorage>>
    │       OR
    │       Arc<RwLock<ThoughtsStorage>>
    │
    └─→ Async-safe (uses only blocking I/O)
        └─→ Wrap in Arc<tokio::sync::Mutex<T>>
            for async contexts
```

## Error Handling Flow

```
Operation Request
    ↓
    ├─→ Path Validation
    │   └─→ ThoughtsError::InvalidPath
    │
    ├─→ Permission Check
    │   └─→ ThoughtsError::PermissionDenied
    │
    ├─→ I/O Operation
    │   └─→ ThoughtsError::IoError (std::io::Error)
    │
    ├─→ Home Directory Detection
    │   └─→ ThoughtsError::NoHomeDirectory
    │
    ├─→ Symlink Operation
    │   └─→ ThoughtsError::SymlinkError
    │
    └─→ Serialization
        └─→ ThoughtsError::SerializationError

        ↓
    ThoughtsResult<T> = Result<T, ThoughtsError>
        ↓
    Caller handles error via pattern match
```

## Integration Points

### With Other Descartes Components

```
CLI (descartes-cli)
    ├─→ Import: use descartes_core::ThoughtsStorage
    ├─→ Store agent insights automatically
    └─→ Provide commands: thoughts list, save, load

Agent Runners
    ├─→ Auto-persist thoughts after execution
    ├─→ Load previous thoughts for context
    └─→ Tag thoughts with agent_id and project_id

Core Library (descartes-core)
    ├─→ Provides ThoughtsStorage API
    ├─→ Manages persistence layer
    └─→ Handles file system operations

Project Configuration
    ├─→ Initialize .thoughts symlink
    ├─→ Reference global thoughts
    └─→ Enable thought sharing across projects
```

## Performance Characteristics

### Time Complexity Analysis

```
Operation              Time        Space   Notes
─────────────────────────────────────────────────────
initialize()          O(1)        O(1)    Fixed directories
save_thought()        O(1)        O(n)    n = content size
load_thought()        O(1)        O(n)    n = metadata size
list_thoughts()       O(d)        O(d)    d = # directories
list_by_tag()         O(d*m)      O(d)    m = avg metadata size
archive_thought()     O(1)        O(1)    Single rename
get_statistics()      O(d*m)      O(d)    Scans all, loads all
clear_all()          O(d)        O(1)    Recursive delete

d = number of thoughts
m = average metadata size
n = size of individual thought content
```

### Memory Usage

```
ThoughtsStorage Instance: ~200 bytes
  ├─ PathBuf: ~48 bytes
  ├─ u32: 4 bytes (permissions × 2)
  └─ Struct overhead: ~150 bytes

Per Operation:
  ├─ load_thought(): O(n) where n = file size
  ├─ list_thoughts(): O(d) = number of dirs
  └─ list_by_tag(): O(d*m) = load all metadata
```

## Security Architecture

### Permission Model

```
User Home Directory
├─ rwxr-xr-x (755) - standard home
│
└─ ~/.descartes/
   └─ rwx------ (700)  ← user only
      │
      └─ thoughts/
         └─ rwx------ (700)  ← user only
            │
            ├─ .metadata/
            │  └─ rwx------ (700)
            │     └─ index.json  (rw------) 600
            │
            └─ [thought-id]/
               └─ rwx------ (700)
                  ├─ metadata.json  (rw------) 600
                  └─ content.txt    (rw------) 600
```

### Attack Surface

```
Threat Model:
├─ Local User Attack
│  └─ MITIGATED: 0o700 permissions
│
├─ Process Privilege Escalation
│  └─ MITIGATED: Standard Unix permissions
│
├─ Symlink Attack
│  └─ MITIGATED: Symlink validation
│
├─ Directory Traversal via ID
│  └─ MITIGATED: No path concatenation
│                (uses direct directory naming)
│
└─ Content Exposure
   └─ NOT MITIGATED: No encryption
      RECOMMENDATION: Use encrypted home dir
```

## Extension Architecture

### Adding Features

The module is designed for extension:

```
ThoughtsStorage → Enhanced Features
    ├─→ Search
    │   └─ Add search index on save
    │   └─ Query implementation
    │
    ├─→ Encryption
    │   └─ Encrypt before save
    │   └─ Decrypt on load
    │
    ├─→ Database
    │   └─ Replace file storage
    │   └─ Keep same public API
    │
    ├─→ Relationships
    │   └─ Add thought_id references
    │   └─ Build graph queries
    │
    └─→ Replication
        └─ Sync on save
        └─ Merge on load
```

### Trait-Based Extension

```rust
pub trait ThoughtsBackend {
    fn save(&self, thought: &ThoughtMetadata) -> ThoughtsResult<()>;
    fn load(&self, id: &str) -> ThoughtsResult<ThoughtMetadata>;
    fn list(&self) -> ThoughtsResult<Vec<String>>;
}

// FilesystemBackend (current)
impl ThoughtsBackend for FilesystemBackend { ... }

// Future: DatabaseBackend
impl ThoughtsBackend for DatabaseBackend { ... }

// Future: EncryptedBackend
pub struct EncryptedBackend<B: ThoughtsBackend> {
    backend: B,
    cipher: AesGcm,
}
```

## Testing Architecture

### Test Organization

```
thoughts.rs
└── #[cfg(test)] mod tests
    ├─ test_initialize_creates_directories
    ├─ test_save_and_load_thought
    ├─ test_list_thoughts
    ├─ test_list_thoughts_by_tag
    ├─ test_archive_thought
    ├─ test_get_statistics
    ├─ test_clear_all
    └─ create_test_storage() helper
       └─ TempDir for isolation
```

## Deployment Model

### Local Development
```
~/.descartes/thoughts/  ← User's home directory
```

### Production
```
~/.descartes/thoughts/  ← Same, user's home
                        ← Typically on encrypted filesystem
```

### Multi-Machine Setup
```
Machine A: ~/.descartes/thoughts/
Machine B: ~/.descartes/thoughts/  ← Future: sync with Cloud
Machine C: ~/.descartes/thoughts/
```

## Conclusion

The Thoughts System provides a clean, extensible architecture for persistent agent memory with:
- Clear separation of concerns
- Proper error handling
- Strong security model
- Performance-conscious design
- Extension-friendly structure
