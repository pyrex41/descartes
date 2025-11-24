# Knowledge Graph Navigation Guide

## Overview

The Knowledge Graph feature in Descartes GUI provides a powerful way to navigate and understand your codebase through semantic relationships. It automatically extracts code entities (functions, classes, modules, etc.) and visualizes their relationships, making it easy to explore code dependencies and structure.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Features](#features)
3. [Workflows](#workflows)
4. [UI Components](#ui-components)
5. [Tips and Tricks](#tips-and-tricks)
6. [Troubleshooting](#troubleshooting)

---

## Getting Started

### Generating a Knowledge Graph

There are two ways to get started with the Knowledge Graph:

#### Option 1: Generate from Your Project

1. Navigate to **File Browser** view
2. Click **"Load Sample File Tree"** or scan your project directory
3. Navigate to **Knowledge Graph** view
4. Click **"Generate from File Tree"**
5. Wait for the knowledge graph to be generated (this may take a few seconds for large projects)

#### Option 2: Load a Sample

1. Navigate to **Knowledge Graph** view
2. Click **"Load Sample Knowledge Graph"**
3. Explore the sample graph to understand the features

### Supported Languages

The Knowledge Graph currently supports:
- **Rust** (primary support)
- **Python**
- **JavaScript**
- **TypeScript**

Additional languages can be configured in the knowledge graph overlay settings.

---

## Features

### 1. Knowledge Graph Visualization

The main panel displays an interactive graph of code entities:

- **Nodes**: Represent code entities (functions, classes, modules, etc.)
- **Edges**: Show relationships between entities (calls, uses, inherits, etc.)
- **Colors**: Each node type has a distinct color for easy identification

#### Node Types and Colors

| Type | Color | Icon |
|------|-------|------|
| Function | Blue | ƒ |
| Method | Light Blue | m |
| Class | Orange | C |
| Struct | Light Orange | S |
| Enum | Purple | E |
| Interface | Green | I |
| Module | Yellow | M |
| Type Alias | Gray | T |
| Constant | Pink | K |
| Variable | Light Green | V |
| Macro | Magenta | ! |

#### Relationship Types

| Relationship | Description |
|--------------|-------------|
| Calls | Function A calls Function B |
| Uses | Entity A uses/references Entity B |
| Inherits | Class A inherits from Class B |
| Implements | Class A implements Interface B |
| DefinedIn | Entity A is defined in Module B |
| DependsOn | Entity A depends on Entity B |

### 2. Enhanced File Tree with Knowledge Overlays

The File Browser now shows knowledge graph information:

- **Badge Indicators**: Files with extracted entities show a badge with the count (e.g., 🔗 5)
- **Clickable Badges**: Click a badge to see the entities defined in that file
- **Highlighted Files**: When selecting a knowledge node, related files are highlighted
- **Quick Navigation**: Double-click a file to open it and see its entities

### 3. Bidirectional Navigation

Navigate seamlessly between file tree and knowledge graph:

#### From File Tree → Knowledge Graph

1. In File Browser, find a file with a knowledge badge
2. Click the badge button (e.g., "🔗 3")
3. The Knowledge Graph view will focus on entities from that file

#### From Knowledge Graph → File Tree

1. In Knowledge Graph, select a node
2. View the node details in the side panel
3. Click **"Jump to Code"** to navigate to the definition
4. The File Browser will highlight the file containing that entity

### 4. Semantic Search

The Knowledge Graph includes powerful search capabilities:

#### Basic Search

1. Type in the search box at the top
2. Search supports:
   - Entity names (e.g., "initialize")
   - Qualified names (e.g., "app::initialize")
   - Entity types (e.g., "function", "class")
   - File paths

#### Fuzzy Search

Enable fuzzy search to find entities with approximate matches:

1. Check the **"Fuzzy"** checkbox next to the search box
2. Type partial names (e.g., "init" will find "initialize", "initialization", etc.)
3. Fuzzy matching works by finding characters in order (e.g., "hw" matches "hello_world")

#### Search Results

- Results appear in the side panel under **"Search Results"**
- Click any result to focus on that node in the graph
- Limited to 10 results for performance

### 5. Filtering

Filter the knowledge graph to focus on specific aspects:

#### Node Type Filters

In the side panel, check/uncheck node types to show/hide:
- Functions (with count)
- Methods (with count)
- Classes (with count)
- Structs (with count)
- And more...

#### Relationship Type Filters

Filter edges by relationship type:
- Calls
- Uses
- Inherits
- Implements
- DefinedIn

#### Other Filters

- **Only Connected**: Show only nodes with at least one connection
- **Clear Filters**: Reset all filters to defaults

### 6. Layout Algorithms

The Knowledge Graph supports multiple layout algorithms:

#### Force-Directed (Default)
- Simulates physical forces to position nodes
- Good for general-purpose visualization
- Nodes with more connections appear more central

#### Hierarchical
- Top-down tree layout
- Good for showing call chains and dependencies
- Root nodes at the top, dependencies below

#### Circular
- Arranges nodes in a circle
- Good for showing cyclic relationships
- Equal spacing between all nodes

#### Grid
- Simple grid arrangement
- Good for large graphs with many nodes
- Predictable positioning

To change layout:
1. Use the **Layout** dropdown in the header
2. Or click **"Relayout"** to recompute the current layout

### 7. Graph Navigation Controls

#### Zoom
- **Zoom In**: Click "Zoom +" or use mouse wheel
- **Zoom Out**: Click "Zoom -" or use mouse wheel
- **Reset View**: Click "Reset View" to return to default zoom

#### Pan
- Click and drag to pan around the graph
- Or use the Pan offset controls

#### Node Selection
- **Click** a node to select it and view details
- **Double-click** a node to jump to its code location
- **Hover** over a node to see a tooltip (if enabled)

### 8. Node Details Panel

When a node is selected, the side panel shows:

- **Name**: The entity's name
- **Type**: The entity type (function, class, etc.)
- **Signature**: Function/method signature (if applicable)
- **Connections**: Count of incoming and outgoing edges
- **Jump to Code**: Button to navigate to the source file

---

## Workflows

### Workflow 1: Understanding a Function's Dependencies

**Goal**: See what functions/classes a specific function uses

1. Use the search box to find the function (e.g., "initialize")
2. Select the function node in the results or graph
3. In the side panel, note the **outgoing connections**
4. Check the **Relationship Type Filters** and enable only "Calls" and "Uses"
5. The graph now shows only the direct dependencies

### Workflow 2: Finding All Callers of a Function

**Goal**: See what code calls a specific function

1. Search for the function
2. Select it in the graph
3. Look at the **incoming connections** in the side panel
4. Filter relationships to show only "Calls"
5. The graph shows all callers

### Workflow 3: Exploring a Module's Structure

**Goal**: See all entities defined in a module

1. Go to **File Browser**
2. Find a file with knowledge entities (look for the badge)
3. Click the knowledge badge
4. The Knowledge Graph will show all entities from that file
5. Use the **Hierarchical** layout to see the structure

### Workflow 4: Tracing a Call Chain

**Goal**: Follow the execution flow through multiple functions

1. Find the entry point function (e.g., "main")
2. Select it in the graph
3. Use **Hierarchical** layout for a clear view
4. Follow the outgoing "Calls" edges to see what it calls
5. Continue clicking nodes to trace deeper into the call chain

### Workflow 5: Finding Similar Code

**Goal**: Find functions with similar names or purposes

1. Enable **Fuzzy Search**
2. Type a partial name (e.g., "process")
3. Browse the search results
4. Click results to explore each one
5. Compare signatures and implementations

---

## UI Components

### Header Bar

- **Search Input**: Type to search for entities
- **Fuzzy Checkbox**: Enable approximate matching
- **Layout Dropdown**: Choose graph layout algorithm
- **Reset View**: Reset zoom and pan
- **Relayout**: Recompute node positions
- **Zoom +/-**: Zoom controls
- **Labels**: Toggle node labels on/off
- **Edge Labels**: Toggle relationship labels on/off

### Side Panel

**Top Section: Filters**
- Node type checkboxes with counts
- Relationship type checkboxes
- "Only Connected" filter
- "Clear Filters" button

**Middle Section: Search Results**
- List of matching entities
- Click to focus on a node

**Bottom Section: Selected Node Details**
- Node information
- Connection statistics
- "Jump to Code" button

### Main Canvas

- Interactive graph visualization
- Nodes represent entities
- Edges represent relationships
- Click to select
- Double-click to navigate
- Drag to pan
- Scroll to zoom

### Footer Bar

- **Statistics**: Node count, edge count, average degree
- **Visible Count**: Number of nodes passing filters
- **Selection Status**: Whether a node is selected

---

## Tips and Tricks

### Performance Tips

1. **Use Filters**: For large graphs (1000+ nodes), use filters to reduce complexity
2. **Type-Specific Views**: Filter to show only one type (e.g., just functions)
3. **Incremental Loading**: Load smaller portions of your codebase first
4. **Clear Cache**: If the graph becomes sluggish, regenerate it

### Navigation Tips

1. **Keyboard Shortcuts** (planned):
   - Arrow keys: Move between connected nodes
   - Spacebar: Expand/collapse node connections
   - +/-: Zoom in/out

2. **Use the Minimap** (planned): For large graphs, use the minimap to navigate

3. **Bookmark Important Nodes** (planned): Save frequently accessed entities

### Search Tips

1. **Use Qualified Names**: Search for "module::function" for precise results
2. **Search by Type**: Type "function" to see all functions
3. **Combine Filters**: Use search + type filters for powerful queries
4. **Fuzzy Search Power**: "cfg" can find "configuration", "config", "configure"

### Visualization Tips

1. **Color Coding**: Learn the color scheme to quickly identify entity types
2. **Edge Thickness** (planned): Thicker edges indicate stronger relationships
3. **Node Size** (planned): Larger nodes have more connections
4. **Clustering** (planned): Related nodes are visually grouped

---

## Troubleshooting

### No Entities Extracted

**Problem**: Knowledge graph is empty after generation

**Solutions**:
1. Check that your files use supported languages (Rust, Python, JS, TS)
2. Ensure files are not empty or have parseable code
3. Check that file extensions are correct (.rs, .py, .js, .ts)
4. Review parser logs for errors

### Graph is Too Large

**Problem**: Graph has too many nodes and is hard to navigate

**Solutions**:
1. Use type filters to show only specific entity types
2. Enable "Only Connected" to hide isolated nodes
3. Use search to focus on specific areas
4. Consider generating separate graphs for different modules

### Slow Performance

**Problem**: GUI becomes sluggish with large graphs

**Solutions**:
1. Reduce the number of visible nodes using filters
2. Disable edge labels for better performance
3. Use simpler layouts (Grid instead of Force-Directed)
4. Close and reopen the Knowledge Graph view to reset state

### Missing Relationships

**Problem**: Expected edges are not shown

**Solutions**:
1. Check relationship type filters - you may have hidden the relationship
2. Ensure both nodes are visible (check type filters)
3. Some relationships require deeper analysis - try regenerating
4. Static analysis has limitations - not all relationships can be detected

### Jump to Code Not Working

**Problem**: "Jump to Code" button doesn't navigate

**Solutions**:
1. Ensure the file still exists at the expected location
2. Check that file paths are correct (relative vs absolute)
3. Verify the file hasn't been moved or renamed since graph generation
4. Try regenerating the knowledge graph

### Outdated Graph

**Problem**: Code changes not reflected in graph

**Solutions**:
1. Regenerate the entire knowledge graph from the file tree
2. Use incremental updates (planned feature)
3. Clear cache and rebuild
4. Ensure file timestamps are current

---

## Advanced Features (Planned)

The following features are planned for future releases:

### Interactive Editing
- Drag nodes to manually adjust layout
- Create custom groupings
- Add annotations and comments

### Export Options
- Export graph as image (PNG, SVG)
- Export as Graphviz DOT format
- Export node/edge data as JSON

### Custom Queries
- Query language for complex searches
- Save and reuse queries
- Query templates for common patterns

### Collaborative Features
- Share graph views with team
- Add collaborative annotations
- Track knowledge graph history

### Integration
- Sync with IDE/editor
- Real-time updates on code changes
- Integration with git history

---

## Best Practices

1. **Regular Regeneration**: Regenerate the knowledge graph after significant code changes
2. **Modular Analysis**: Analyze one module at a time for large codebases
3. **Document Findings**: Use external tools to document insights from the graph
4. **Combine with Other Views**: Use Knowledge Graph alongside File Browser for best results
5. **Iterative Exploration**: Start broad, then filter down to specific areas of interest

---

## Keyboard Shortcuts (Planned)

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Focus search box |
| `Ctrl+G` | Generate/regenerate graph |
| `Ctrl+R` | Reset view |
| `+` | Zoom in |
| `-` | Zoom out |
| `Space` | Toggle node expansion |
| `Enter` | Jump to selected node code |
| `Esc` | Clear selection |
| `←/→` | Navigate between connected nodes |
| `Ctrl+H` | Toggle hierarchical layout |
| `Ctrl+L` | Toggle labels |

---

## FAQ

**Q: How long does graph generation take?**
A: For typical projects (100-500 files), generation takes 5-30 seconds. Larger projects may take 1-2 minutes.

**Q: Can I generate graphs for non-code files?**
A: No, only code files in supported languages are analyzed.

**Q: Does the graph update automatically?**
A: Not currently. You must manually regenerate after code changes.

**Q: Can I export the graph?**
A: Export features are planned for a future release.

**Q: How much memory does a large graph use?**
A: Approximately 1KB per node, so 10,000 nodes = ~10MB.

**Q: Can I customize the graph appearance?**
A: Theme customization is planned for a future release.

---

## Support

For issues, bugs, or feature requests:

1. Check the [Troubleshooting](#troubleshooting) section
2. Review the [GitHub Issues](https://github.com/descartes/issues)
3. Join the community Discord
4. Submit detailed bug reports with:
   - Project size (file count)
   - Steps to reproduce
   - Screenshots if applicable
   - Error logs from console

---

## Changelog

### Version 1.0 (Current)
- Initial knowledge graph implementation
- Support for Rust, Python, JavaScript, TypeScript
- Multiple layout algorithms
- Bidirectional navigation with file tree
- Semantic search with fuzzy matching
- Type and relationship filtering
- Performance optimized for graphs up to 10,000 nodes

---

## Credits

Knowledge Graph implementation uses:
- `tree-sitter` for code parsing
- `iced` for GUI rendering
- Graph layout algorithms adapted from research literature

Developed as part of the Descartes AI Agent Framework - Phase 3, Task 9.5.

