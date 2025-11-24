# Context Browser User Guide

Complete guide to using the Descartes Context Browser for efficient code navigation and exploration.

## Table of Contents

1. [Overview](#overview)
2. [File Tree View](#file-tree-view)
3. [Code Preview Panel](#code-preview-panel)
4. [Knowledge Graph Panel](#knowledge-graph-panel)
5. [Interactive Features](#interactive-features)
6. [Navigation Workflows](#navigation-workflows)
7. [Performance Tips](#performance-tips)
8. [Keyboard Shortcuts](#keyboard-shortcuts)

## Overview

The Context Browser is a powerful tool for understanding and navigating code. It combines three main components:

- **File Tree View**: Browse your project's file structure
- **Code Preview Panel**: View and analyze file contents
- **Knowledge Graph Panel**: Visualize code entities and relationships

### Key Features

- Multi-file diff comparison
- Dependency path visualization
- Impact analysis
- Related code suggestions
- Navigation history
- Bookmarks and annotations
- Search with regex support
- Performance optimized for large codebases (10,000+ files)

## File Tree View

### Basic Navigation

The file tree displays your project's file structure in a hierarchical view:

```
📁 project-root/
  📁 src/
    📌 🦀 main.rs 🔗 5
    🦀 lib.rs 🔗 3
    📁 components/
      🦀 button.rs ⏱
  📁 tests/
    🦀 integration_tests.rs
```

**Icons Legend:**
- `📁` Directory
- `🦀` Rust file (language-specific icons)
- `🔗 N` N knowledge graph entities in this file
- `📌` Pinned file
- `🔖` Bookmarked file
- `⏱` Recently accessed
- Git status indicators (M, A, D, ??)

### Search and Filtering

**Basic Search:**
```
Type in the search box to filter files by name
```

**Regex Search:**
Click the `.*` button to enable regex patterns:
```regex
.*\.rs$         # All Rust files
test_.*         # Files starting with "test_"
.*controller.*  # Files containing "controller"
```

**Filters:**
- **Hidden**: Toggle visibility of hidden files (starting with `.`)
- **Linked**: Show only files with knowledge graph entities
- **Language**: Filter by programming language

### Bookmarks

**Adding Bookmarks:**
1. Right-click on a file
2. Select "Add Bookmark" OR
3. Use keyboard shortcut `Ctrl+D`

**Managing Bookmarks:**
- View count in header: `🔖 5`
- Jump to bookmarked files via bookmark panel
- Remove: Right-click > "Remove Bookmark"
- Clear all: Click "Clear Bookmarks"

### Navigation History

Navigate through your browsing history:

- **Back**: `◄` button or `Alt+Left`
- **Forward**: `►` button or `Alt+Right`

History is preserved across sessions and tracks:
- File selections
- Directory expansions
- Knowledge graph node visits

### Pinned Files

Keep important files at the top:

1. Right-click file > "Pin to Top"
2. Pinned files show `📌` icon
3. Always visible regardless of filters

### Recent Files

Recently accessed files show `⏱` icon. Access history:
- Last 20 files tracked
- Quick access via Recent panel
- Sorted by access time

### Context Menu Actions

Right-click any file for:
- **Open**: View in code preview
- **Find References**: Show all references in knowledge graph
- **Go to Definition**: Navigate to symbol definitions
- **Show Usages**: Display where this file is used
- **Reveal in Explorer**: Open in system file manager
- **Copy Path**: Copy absolute path
- **Copy Relative Path**: Copy path relative to project root
- **Add/Remove Bookmark**
- **Pin/Unpin File**

## Code Preview Panel

### Opening Files

**Methods to open:**
1. Double-click in file tree
2. Select file and press `Enter`
3. Click knowledge graph node's "Jump to Code"

### Viewing Code

The code preview shows:
```
┌─────────────────────────────────────────────┐
│ file.rs [Rust]                    [Close]  │
├─────────────────────────────────────────────┤
│ [Search] 🔍  ↑ ↓   Line #: ON  Wrap: OFF  │
├─────────────────────────────────────────────┤
│   1  fn main() {                            │
│ 🔖 2      println!("Hello");                │
│   3      helper();                          │
│   4  }                                      │
│   5  💬 Important function                  │
├─────────────────────────────────────────────┤
│ Line 1/100 | Bookmarks: 1 | Mode: Single  │
└─────────────────────────────────────────────┘
```

### Search Within File

1. Type in search box
2. Results highlighted in yellow
3. Navigate: `↑` (previous) `↓` (next)
4. Counter shows: `2/5` (result 2 of 5)

**Search is case-insensitive by default**

### Bookmarks in Code

**Add bookmark:**
- Click line number + `Ctrl+B` OR
- Right-click line > "Add Bookmark"

**Navigate bookmarks:**
- `F2`: Next bookmark
- `Shift+F2`: Previous bookmark
- Shows `🔖` indicator

### Annotations

Add notes to specific lines:

1. Right-click line > "Add Annotation"
2. Enter annotation text
3. Shows as: `💬 Your note here`

**Use cases:**
- Mark TODOs
- Explain complex logic
- Note bug locations
- Track review comments

### View Options

**Toggle features:**
- **Line Numbers**: Show/hide line numbers (default: ON)
- **Word Wrap**: Wrap long lines (default: OFF)
- **Syntax Highlighting**: Color code (default: ON)
- **Show Whitespace**: Display spaces as `·`, tabs as `→`

### Jump to Line

1. Press `Ctrl+G`
2. Enter line number
3. View scrolls to line and highlights it

### Code Folding

**Fold/unfold code sections:**
- Click fold icon (▼/▶) next to line numbers
- Collapses function/class bodies
- Useful for large files

### View Modes

#### Single File View
Default view showing one file.

#### Side-by-Side Diff
Compare two files:

1. Open first file
2. Click "Load Diff File"
3. Select second file
4. View shows both files side-by-side

```
┌──────────────┬──────────────┐
│  Original    │   Modified   │
├──────────────┼──────────────┤
│ 1 fn foo()   │ 1 fn foo()   │
│ 2     let x  │ 2     let y  │
│ 3 }          │ 3 }          │
└──────────────┴──────────────┘
```

**Use cases:**
- Compare before/after changes
- Review refactorings
- Analyze different implementations

#### Unified Diff
Shows changes in single column with +/- indicators (coming soon).

## Knowledge Graph Panel

### Overview

Visualizes code entities and their relationships:

```
     ┌─────────┐
     │ main    │
     └────┬────┘
          │ calls
     ┌────▼────┐
     │ helper  │
     └────┬────┘
          │ uses
     ┌────▼────┐
     │MyClass  │
     └─────────┘
```

### Node Types

- **Function**: `ƒ` (blue)
- **Method**: `m` (light blue)
- **Class**: `C` (orange)
- **Struct**: `S` (orange-yellow)
- **Enum**: `E` (purple)
- **Interface**: `I` (green)
- **Module**: `M` (yellow)
- **Type Alias**: `T` (gray)
- **Constant**: `K` (pink)

### Relationship Types

- **Calls**: Function A calls function B
- **Uses**: Entity A uses entity B
- **Inherits**: Class A inherits from class B
- **Implements**: Class A implements interface B
- **DefinedIn**: Entity A defined in module B
- **DependsOn**: Entity A depends on entity B

### Search and Filtering

**Search entities:**
```
Type name, type, or file to find entities
```

**Fuzzy search:**
- Enable with checkbox
- `hw` matches `hello_world`
- `fn` matches `FunctionName`

**Filter by type:**
Check/uncheck node types in side panel:
- ☑ Function (25)
- ☑ Class (10)
- ☐ Module (5)

**Filter by relationship:**
Show only specific edge types:
- ☑ Calls
- ☑ Uses
- ☐ Inherits

**Show only connected:**
Hide isolated nodes with no edges.

### Layout Algorithms

**Force-Directed** (default):
- Nodes repel each other
- Connected nodes attract
- Natural-looking layout

**Hierarchical**:
- Top-down tree structure
- Shows call hierarchy
- Good for dependencies

**Circular**:
- Nodes arranged in circle
- Shows all entities
- Good for overview

**Grid**:
- Regular grid layout
- Easy to scan
- Predictable positions

### Navigation

**Pan:**
- Click and drag background
- Arrow keys

**Zoom:**
- Mouse wheel
- `+` / `-` buttons
- Pinch gesture (touchpad)

**Reset View:**
Click "Reset View" to return to original position and zoom.

### Node Interactions

**Select node:**
- Click to select
- Shows in side panel

**Hover node:**
- Shows tooltip with:
  - Full name
  - Type
  - Signature
  - Connection count

**Double-click:**
- Jump to code definition
- Opens in code preview

**Right-click menu:**
- Find References
- Go to Definition
- Find Usages
- Show Dependency Path
- Analyze Impact
- Show Related Code
- Add to Comparison
- Show Call Hierarchy

### Advanced Features

#### Dependency Path

Find path between two entities:

1. Select first node
2. Right-click > "Show Dependency Path"
3. Select second node
4. Path highlighted in graph

```
A → B → C → D
```

**Use cases:**
- Understand call chains
- Find indirect dependencies
- Trace data flow

#### Impact Analysis

See what's affected by changes:

1. Select entity
2. Right-click > "Analyze Impact"
3. Affected nodes highlighted in red

**Shows:**
- Direct dependents
- Transitive dependents
- Ripple effects

**Use cases:**
- Assess change risk
- Plan refactoring
- Identify test scope

#### Related Code Suggestions

Find similar or related entities:

1. Select entity
2. Right-click > "Show Related Code"
3. Suggestions appear in side panel

**Finds:**
- Same type entities
- Similar names
- Connected entities
- Common patterns

#### Call Hierarchy

View complete call chain:

1. Select function/method
2. Right-click > "Show Call Hierarchy"
3. Shows:
   - Who calls this (callers)
   - What this calls (callees)

```
Callers:
  ← main()
  ← init()

Current:
  process()

Callees:
  → validate()
  → transform()
  → save()
```

#### Comparison View

Compare multiple entities:

1. Select first entity
2. Right-click > "Add to Comparison"
3. Repeat for other entities
4. View comparison panel

**Shows:**
- Side-by-side properties
- Relationship differences
- Complexity metrics

## Interactive Features

### Find References

**From File Tree:**
1. Right-click file
2. "Find References"
3. Shows all entities that reference this file

**From Knowledge Graph:**
1. Select node
2. Right-click > "Find References"
3. Highlights referencing nodes

### Go to Definition

Navigate to where symbol is defined:

1. Select symbol/entity
2. Right-click > "Go to Definition"
3. Jumps to definition in code preview

**Works across files**

### Find Usages

See everywhere an entity is used:

1. Select entity
2. Right-click > "Find Usages"
3. Lists all usage locations

**Use cases:**
- Understand usage patterns
- Find all call sites
- Assess API usage

## Navigation Workflows

### Workflow 1: Understanding a New Codebase

1. **Start with File Tree**
   - Explore directory structure
   - Look for main entry points
   - Note test directories

2. **Open Knowledge Graph**
   - View entity overview
   - Identify main modules
   - See high-level architecture

3. **Follow Dependencies**
   - Start at entry point (main)
   - Use "Show Call Hierarchy"
   - Trace execution paths

4. **Bookmark Important Files**
   - Mark key files
   - Pin frequently accessed files

### Workflow 2: Refactoring

1. **Select Target Entity**
   - Find function/class to refactor

2. **Analyze Impact**
   - Right-click > "Analyze Impact"
   - Review affected entities

3. **Find All Usages**
   - Right-click > "Find Usages"
   - List all call sites

4. **Compare Alternatives**
   - Add entities to comparison
   - Review differences

5. **Make Changes**
   - Update entity
   - Use bookmarks to track changed files

### Workflow 3: Debugging

1. **Find Entry Point**
   - Search for error location
   - Open in code preview

2. **Trace Call Path**
   - Use "Show Call Hierarchy"
   - Work backwards from error

3. **Analyze Dependencies**
   - Check what function uses
   - Review data flow

4. **Add Annotations**
   - Mark suspicious lines
   - Note investigation findings

### Workflow 4: Code Review

1. **Review Changed Files**
   - Filter by git status
   - Open in preview

2. **Check Impact**
   - Analyze affected entities
   - Review dependencies

3. **Compare Versions**
   - Side-by-side diff
   - Note differences

4. **Add Review Comments**
   - Annotate lines
   - Bookmark issues

## Performance Tips

### For Large Codebases (10,000+ files)

**1. Use Filters Aggressively**
- Filter by language
- Show only linked files
- Use regex for precise searches

**2. Lazy Loading**
- File tree loads on demand
- Expand only needed directories

**3. Knowledge Graph Optimization**
- Filter by type to reduce visible nodes
- Use hierarchical layout for large graphs
- Show only connected nodes

**4. Search Strategies**
- Use specific searches vs broad ones
- Prefer regex over fuzzy for large sets
- Use type filters before searching

**5. Preview Panel**
- Close when not needed
- Avoid opening very large files
- Use code folding for large files

### Memory Management

**File Tree:**
- Unload unused branches
- Clear search results regularly
- Limit expanded nodes

**Knowledge Graph:**
- Clear filters when done
- Reset view to free caches
- Use minimap for overview (less detail)

**Code Preview:**
- Close files after viewing
- Clear annotations periodically
- Limit bookmarks to essentials

### Performance Indicators

Watch for these signs of slowdown:
- Search takes >1 second
- Graph layout stutters
- File tree slow to expand

**Solutions:**
- Apply more filters
- Reduce visible nodes
- Clear caches (restart panel)

## Keyboard Shortcuts

### Global

| Shortcut | Action |
|----------|--------|
| `Ctrl+F` | Focus search |
| `Ctrl+P` | Quick file open |
| `Ctrl+Shift+P` | Command palette |
| `Esc` | Clear selection/close panel |

### File Tree

| Shortcut | Action |
|----------|--------|
| `Alt+Left` | Navigate back |
| `Alt+Right` | Navigate forward |
| `Ctrl+D` | Add bookmark |
| `Ctrl+Shift+D` | Clear bookmarks |
| `Enter` | Open selected file |
| `Space` | Toggle expand |
| `Ctrl+E` | Expand all |
| `Ctrl+Shift+E` | Collapse all |

### Code Preview

| Shortcut | Action |
|----------|--------|
| `Ctrl+G` | Jump to line |
| `Ctrl+F` | Search in file |
| `F3` | Next search result |
| `Shift+F3` | Previous search result |
| `Ctrl+B` | Add bookmark at line |
| `F2` | Next bookmark |
| `Shift+F2` | Previous bookmark |
| `Ctrl+/` | Toggle comment |
| `Ctrl+]` | Fold code |
| `Ctrl+[` | Unfold code |

### Knowledge Graph

| Shortcut | Action |
|----------|--------|
| `+` | Zoom in |
| `-` | Zoom out |
| `0` | Reset zoom |
| `R` | Reset view |
| `L` | Change layout |
| `F` | Toggle filters |
| `M` | Toggle minimap |
| `Ctrl+Click` | Multi-select nodes |
| `Shift+Click` | Add to selection |

## Tips and Best Practices

### Organization

1. **Use Bookmarks Strategically**
   - Entry points
   - Complex algorithms
   - Error-prone code
   - Frequently modified files

2. **Pin Essential Files**
   - Configuration
   - Main modules
   - Common utilities

3. **Leverage Annotations**
   - Document workarounds
   - Explain non-obvious code
   - Track technical debt

### Navigation

1. **Build Mental Map**
   - Start with file tree
   - Then knowledge graph
   - Cross-reference frequently

2. **Follow Relationships**
   - Use call hierarchy
   - Trace dependencies
   - Understand data flow

3. **Use History**
   - Navigate back to context
   - Track investigation paths
   - Resume interrupted work

### Analysis

1. **Impact Before Changes**
   - Always run impact analysis
   - Review all affected files
   - Plan test coverage

2. **Compare Alternatives**
   - Use comparison view
   - Evaluate different approaches
   - Learn from similar code

3. **Document Findings**
   - Add annotations
   - Bookmark locations
   - Track investigation

### Performance

1. **Keep It Clean**
   - Clear old bookmarks
   - Remove unused annotations
   - Reset filters when done

2. **Filter Early**
   - Narrow scope before searching
   - Use specific filters
   - Limit visible entities

3. **Progressive Disclosure**
   - Start with overview
   - Drill down as needed
   - Collapse when done

## Troubleshooting

### Search Not Working

**Issue:** Search returns no results

**Solutions:**
- Check spelling
- Disable filters temporarily
- Try fuzzy search
- Use broader pattern

### Graph Too Cluttered

**Issue:** Too many nodes visible

**Solutions:**
- Apply type filters
- Show only connected
- Use hierarchical layout
- Filter by file/module

### Slow Performance

**Issue:** Operations are slow

**Solutions:**
- Close unnecessary panels
- Clear filters
- Reduce visible nodes
- Restart context browser

### Missing Entities

**Issue:** Expected entities not in graph

**Solutions:**
- Check language is enabled
- Verify file is parsed
- Reload knowledge graph
- Check file isn't binary

### Navigation History Lost

**Issue:** Can't navigate back

**Solutions:**
- History cleared on tree reload
- Use bookmarks for persistence
- Pin important files

## Advanced Topics

### Custom Filters

Create complex filter combinations:
```
Language: Rust
+ Only linked
+ Search: "test"
+ Type: Function
= All Rust test functions with knowledge links
```

### Graph Analysis Patterns

**Identify Hotspots:**
1. View full graph
2. Look for highly connected nodes
3. These are central to system

**Find Isolation:**
1. Apply "only connected" filter
2. Remaining isolated nodes may be:
   - Dead code
   - Utilities
   - New features

**Detect Layers:**
1. Use hierarchical layout
2. Levels show architecture layers
3. Top = high-level, Bottom = utilities

### Integration with Workflow

**With Git:**
- Filter by git status
- Review changes before commit
- Compare feature branches

**With Testing:**
- Find test coverage gaps
- Identify untested paths
- Trace test execution

**With Documentation:**
- Annotate public APIs
- Document complex flows
- Track TODOs

## Getting Help

### Resources

- **API Reference**: See CONTEXT_BROWSER_API.md
- **Examples**: Check /examples directory
- **Issues**: Report bugs on GitHub

### Common Questions

**Q: How do I save my bookmarks?**
A: Bookmarks are auto-saved per project.

**Q: Can I export the knowledge graph?**
A: Yes, use Export > JSON or Export > Image.

**Q: How many files can it handle?**
A: Optimized for 10,000+ files with good performance.

**Q: Can I customize the layout?**
A: Yes, choose from 4 layout algorithms.

**Q: How do I share annotations?**
A: Annotations are stored locally. Use git to share.

---

**Version:** 1.0
**Last Updated:** 2025-11-24

For technical details, see [CONTEXT_BROWSER_API.md](CONTEXT_BROWSER_API.md).
