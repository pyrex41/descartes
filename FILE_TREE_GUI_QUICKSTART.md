# File Tree GUI - Quick Start Guide

## Overview

The **File Tree GUI Component** is a visual browser for navigating project file structures within the Descartes GUI. It provides an intuitive, hierarchical view of your codebase with rich metadata and knowledge graph integration.

## Features at a Glance

### Core Functionality
✅ **Hierarchical Tree View** - Nested folders with expand/collapse
✅ **Smart Icons** - Language-specific icons for 23+ file types
✅ **Knowledge Badges** - Visual indicators for files with semantic links
✅ **Git Status** - Color-coded change indicators
✅ **Real-time Search** - Instant filtering as you type
✅ **Advanced Filters** - By language, hidden files, knowledge links
✅ **Multiple Sort Orders** - Name, size, modified date
✅ **Interactive Selection** - Click to select, visual highlight
✅ **Live Statistics** - File count, visible items, selection status

## Visual Preview

```
┌────────────────────────────────────────────────────┐
│ Search: [Search files...              ]           │
│ [Hidden] [Linked] [Clear]  [Expand] [Collapse]   │
├────────────────────────────────────────────────────┤
│ ▼ 📁 descartes                                    │
│   ▼ 📁 gui                                        │
│     ▼ 📁 src                                      │
│       🦀 main.rs [3] M                            │
│       🦀 file_tree_view.rs [5]                    │
│       🦀 task_board.rs [2] M                      │
│   ▼ 📁 agent-runner                               │
│     ▼ 📁 src                                      │
│       🦀 file_tree_builder.rs [7]                 │
│       🦀 knowledge_graph.rs [12]                  │
│   📖 README.md ??                                 │
├────────────────────────────────────────────────────┤
│ Files: 156 | Dirs: 42 | Visible: 198 | Selected  │
└────────────────────────────────────────────────────┘
```

## Icon Legend

### File Types
- 🦀 Rust
- 🐍 Python
- 📜 JavaScript
- 📘 TypeScript
- 🐹 Go
- ☕ Java
- 🌐 HTML
- 🎨 CSS
- 📋 JSON
- 📝 YAML
- 📖 Markdown
- 📁 Folder
- 📦 Binary

### Status Indicators
- **[3]** - Knowledge link count (blue badge)
- **M** - Modified (orange)
- **A** - Added (green)
- **D** - Deleted (red)
- **R** - Renamed (blue)
- **??** - Untracked (gray)

## Usage

### 1. Launch the GUI
```bash
cd descartes/gui
cargo run
```

### 2. Navigate to File Browser
Click **"File Browser"** in the left sidebar navigation.

### 3. Load a File Tree
Click **"Load Sample File Tree"** to browse the current project directory.

### 4. Interact with the Tree

#### Expand/Collapse Folders
- Click on a folder to toggle expansion
- Use **"Expand All"** to open entire tree
- Use **"Collapse All"** to close all folders

#### Select Files
- Click on any file to select it
- Selected file highlighted with blue background
- Selection status shown in footer

#### Search Files
- Type in the search box at the top
- Results update in real-time
- Parent folders auto-expand to show matches

#### Apply Filters
- **Hidden** - Toggle visibility of dot files
- **Linked** - Show only files with knowledge links
- **Clear** - Reset all filters

## API Usage

### Load a File Tree
```rust
use descartes_agent_runner::file_tree_builder::FileTreeBuilder;

let mut builder = FileTreeBuilder::new();
let tree = builder.scan_directory("/path/to/project")?;

file_tree_view::update(
    &mut state.file_tree_state,
    FileTreeMessage::TreeLoaded(tree),
);
```

### Get Selected File
```rust
// Get the selected file path
if let Some(path) = file_tree_view::get_selected_path(&state.file_tree_state) {
    println!("Selected: {:?}", path);
}

// Get the selected node with metadata
if let Some(node) = file_tree_view::get_selected_node(&state.file_tree_state) {
    println!("Language: {:?}", node.metadata.language);
    println!("Size: {:?}", node.metadata.size);
    println!("Knowledge links: {}", node.knowledge_links.len());
}
```

### Filter by Language
```rust
file_tree_view::update(
    &mut state.file_tree_state,
    FileTreeMessage::FilterByLanguage(Some(Language::Rust)),
);
```

### Search
```rust
file_tree_view::update(
    &mut state.file_tree_state,
    FileTreeMessage::SearchQueryChanged("main".to_string()),
);
```

## Architecture

### Component Structure
```
FileTreeView
├── State (FileTreeState)
│   ├── Tree data
│   ├── Expanded nodes
│   ├── Selected node
│   └── Filter settings
│
├── Messages (FileTreeMessage)
│   ├── User actions
│   ├── State updates
│   └── Tree operations
│
└── View (Iced widgets)
    ├── Header (search + filters)
    ├── Tree content (recursive)
    └── Footer (statistics)
```

### Data Flow
```
User Interaction
    ↓
FileTreeMessage
    ↓
update() function
    ↓
FileTreeState (modified)
    ↓
view() function
    ↓
Rendered GUI
```

## File Locations

### Implementation
- **Main Widget**: `descartes/gui/src/file_tree_view.rs` (633 lines)
- **Integration**: `descartes/gui/src/main.rs`
- **Exports**: `descartes/gui/src/lib.rs`

### Dependencies
- **Data Models**: `descartes/agent-runner/src/knowledge_graph.rs`
- **Builder**: `descartes/agent-runner/src/file_tree_builder.rs`

### Documentation
- **Implementation Report**: `PHASE3_9.4_IMPLEMENTATION_REPORT.md`
- **Quick Start**: `FILE_TREE_GUI_QUICKSTART.md` (this file)

## Configuration

### Sort Orders
```rust
pub enum SortOrder {
    NameAsc,        // A-Z
    NameDesc,       // Z-A
    SizeAsc,        // Smallest first
    SizeDesc,       // Largest first
    ModifiedAsc,    // Oldest first
    ModifiedDesc,   // Newest first
}
```

### Default Settings
- **Expanded**: Root node only
- **Show Hidden**: Off
- **Show Linked Only**: Off
- **Sort Order**: Name (ascending)
- **Search**: Empty

## Performance

### Benchmarks (Estimated)
- **Small Projects** (<1,000 files): <100ms load time
- **Medium Projects** (1,000-10,000 files): <500ms load time
- **Large Projects** (>10,000 files): <2s load time

### Optimization Tips
1. Use filters to reduce visible nodes
2. Collapse unused branches
3. Search for specific files rather than browsing
4. Consider implementing lazy loading for very large projects

## Troubleshoads

### Tree Not Loading
- Check file permissions
- Verify path exists
- Check console for error messages

### Performance Issues
- Enable filters to reduce visible items
- Collapse large folder branches
- Consider file count in project

### Icons Not Showing
- Ensure terminal/GUI supports emoji
- Check font rendering settings

## Future Enhancements

Coming soon:
- 🔲 Right-click context menu
- 🔲 Double-click to open file details
- 🔲 Drag-and-drop file operations
- 🔲 File watching for auto-refresh
- 🔲 Custom icon themes
- 🔲 Keyboard navigation
- 🔲 Virtual scrolling for large trees
- 🔲 Tree diff view

## Support

For issues or questions:
- Check the implementation report: `PHASE3_9.4_IMPLEMENTATION_REPORT.md`
- Review the source code: `descartes/gui/src/file_tree_view.rs`
- See the FileTree documentation: `descartes/agent-runner/FILE_TREE_QUICKSTART.md`

## License

Part of the Descartes project. See project LICENSE for details.

---

**Status**: ✅ Complete and Ready to Use
**Version**: Phase 3.9.4
**Build**: Passing (no errors)
