# DAG Editor Controls & User Guide

## Overview

The Descartes DAG Editor provides a visual interface for creating and managing task dependency graphs. This guide covers all interactive features including drag-and-drop, selection, panning, zooming, and keyboard shortcuts.

## Table of Contents

1. [Tools](#tools)
2. [Node Operations](#node-operations)
3. [Edge Operations](#edge-operations)
4. [Selection](#selection)
5. [View Navigation](#view-navigation)
6. [Keyboard Shortcuts](#keyboard-shortcuts)
7. [Undo/Redo](#undoredo)
8. [Tips & Best Practices](#tips--best-practices)

---

## Tools

The toolbar at the top provides access to different editing modes:

### Select Tool (↖)
- **Purpose**: Select, move, and manipulate nodes
- **Activation**: Click the "Select" button or press `S`
- **Usage**:
  - Click on nodes to select them
  - Click and drag to move selected nodes
  - Click empty space and drag to create selection box
  - Ctrl+Click to add/remove nodes from selection

### Add Node Tool (+)
- **Purpose**: Add new task nodes to the graph
- **Activation**: Click the "Add Node" button or press `N`
- **Usage**:
  - Click anywhere on the canvas to create a new node
  - Node will be automatically labeled (Task 1, Task 2, etc.)
  - If "Snap to Grid" is enabled, node will align to grid

### Add Edge Tool (→)
- **Purpose**: Create dependencies between nodes
- **Activation**: Click the "Add Edge" button or press `E`
- **Usage**:
  - Click on the source node
  - Drag to the target node
  - Release to create the edge
  - Preview line shows connection while dragging
  - **Validation**: Edges that would create cycles are automatically rejected

### Delete Tool (×)
- **Purpose**: Remove nodes and edges
- **Activation**: Click the "Delete" button or press `D`
- **Usage**:
  - Click on a node to delete it (and all connected edges)
  - Click on an edge to delete just that edge
  - Alternatively, select items and press `Delete` key

### Pan Tool (✋)
- **Purpose**: Move the canvas view
- **Activation**: Click the "Pan" button or press `P`
- **Usage**:
  - Click and drag to pan the canvas
  - Alternative: Use middle mouse button or Space+drag with any tool

---

## Node Operations

### Creating Nodes

1. **Method 1: Add Node Tool**
   - Select the Add Node tool
   - Click on the canvas where you want the node
   - Node appears with default label

2. **Method 2: Context Menu** (Coming soon)
   - Right-click on empty canvas
   - Select "Add Node" from menu

### Moving Nodes

1. **Single Node**
   - Select the Select tool
   - Click on a node to select it (highlighted border appears)
   - Drag to new position
   - Release to drop

2. **Multiple Nodes**
   - Select first node
   - Hold `Ctrl` and click additional nodes
   - Drag any selected node - all move together
   - Maintains relative positions

3. **With Grid Snap**
   - Enable "Snap to Grid" button
   - Nodes automatically align to 20px grid when moved
   - Useful for clean, organized layouts

### Editing Node Properties

- Click on a node to select it
- Properties panel on the right shows:
  - Node label
  - Node ID (UUID)
  - Position (x, y)
  - Incoming/outgoing edge counts

---

## Edge Operations

### Creating Edges (Dependencies)

1. **Via Drag**
   - Select the Add Edge tool
   - Click on source node
   - Drag to target node (preview line shown)
   - Release on target to create edge
   - Edge appears with arrow pointing to target

2. **Edge Types**
   - **Dependency** (solid line): Hard dependency - target waits for source
   - **Soft Dependency** (dashed line): Optional dependency
   - **Data Flow** (green): Data passes between nodes
   - **Trigger** (orange): Source triggers target execution

### Edge Validation

The editor automatically prevents invalid edges:

- **Self-loops**: Cannot connect a node to itself
- **Cycles**: Cannot create circular dependencies
  - Example: If A → B → C exists, cannot create C → A
  - Error message shown if attempted

### Deleting Edges

1. **Method 1: Delete Tool**
   - Select Delete tool
   - Click on edge

2. **Method 2: Delete Key**
   - Select edge (when implemented)
   - Press `Delete` key

---

## Selection

### Single Selection

- Click on a node with Select tool
- Previously selected nodes are deselected
- Selected node shows highlighted border (gold color)

### Multi-Selection

1. **Ctrl+Click**
   - Hold `Ctrl` key
   - Click on nodes to add to selection
   - Click selected node again to remove from selection

2. **Box Selection**
   - Click on empty canvas
   - Drag to create selection rectangle (shown in real-time)
   - Release to select all nodes within rectangle
   - Useful for selecting groups of nodes

### Select All

- Press `Ctrl+A` to select all nodes in the graph

### Deselect

- Click on empty canvas (without dragging)
- Press `Escape` key

---

## View Navigation

### Panning

1. **Middle Mouse Button**
   - Press and hold middle mouse button anywhere
   - Drag to pan the view
   - Works with any active tool

2. **Pan Tool**
   - Select Pan tool from toolbar
   - Click and drag with left mouse button

3. **Space + Drag**
   - Hold `Space` key
   - Click and drag with left mouse button
   - Release `Space` to return to current tool

### Zooming

1. **Mouse Wheel**
   - Scroll up to zoom in
   - Scroll down to zoom out
   - **Zoom-to-Cursor**: View zooms toward/away from cursor position
   - Maintains cursor position in world space

2. **Zoom Buttons**
   - Click "Zoom In [+]" to zoom in one step
   - Click "Zoom Out [-]" to zoom out one step
   - Click "Reset" to return to 100% zoom and center view
   - Click "Fit" to fit entire graph in view

3. **Zoom Limits**
   - Minimum: 10% (0.1x)
   - Maximum: 500% (5.0x)
   - Zoom level shown in statistics bar

### Fit to View

- Click "Fit" button in toolbar
- Automatically calculates optimal zoom and pan
- Centers entire graph in viewport
- Useful after adding many nodes

---

## Keyboard Shortcuts

### Tool Selection

| Key | Tool |
|-----|------|
| `S` | Select Tool |
| `N` | Add Node Tool |
| `E` | Add Edge Tool |
| `D` | Delete Tool |
| `P` | Pan Tool |

### Selection

| Shortcut | Action |
|----------|--------|
| `Click` | Select single node |
| `Ctrl+Click` | Toggle node in selection |
| `Ctrl+A` | Select all nodes |
| `Escape` | Deselect all / Cancel operation |

### Editing

| Shortcut | Action |
|----------|--------|
| `Delete` | Delete selected nodes/edges |
| `Ctrl+Z` | Undo last operation |
| `Ctrl+Shift+Z` or `Ctrl+Y` | Redo last undone operation |

### View

| Shortcut | Action |
|----------|--------|
| `Mouse Wheel` | Zoom in/out at cursor |
| `Middle Mouse + Drag` | Pan view |
| `Space + Drag` | Pan view (temporary) |
| `+` or `=` | Zoom in |
| `-` | Zoom out |
| `0` | Reset zoom to 100% |

### Grid

| Shortcut | Action |
|----------|--------|
| `G` | Toggle grid visibility |
| `Ctrl+G` | Toggle snap to grid |

---

## Undo/Redo

The DAG Editor maintains a complete history of operations for undo/redo support.

### Supported Operations

All these operations can be undone/redone:

- **Node Operations**
  - Add node
  - Delete node
  - Move node (position change)
  - Update node properties

- **Edge Operations**
  - Add edge
  - Delete edge
  - Change edge type

### Using Undo/Redo

1. **Undo**
   - Press `Ctrl+Z`
   - Click "Undo" button (if available)
   - Reverts last operation

2. **Redo**
   - Press `Ctrl+Shift+Z` or `Ctrl+Y`
   - Click "Redo" button (if available)
   - Re-applies last undone operation

### History Limits

- Maximum 100 operations stored
- Older operations are automatically removed
- Creating a new operation clears redo history

---

## Tips & Best Practices

### Organization

1. **Use Grid Snap**
   - Enable for clean, aligned layouts
   - Disable for precise positioning
   - 20px grid works well for most layouts

2. **Arrange by Layers**
   - Place start nodes on left
   - Arrange flow left-to-right
   - Group related nodes together

3. **Zoom Levels**
   - Use 100% for normal editing
   - Zoom out (50-75%) for overview
   - Zoom in (150-200%) for precise work

### Performance

1. **Large Graphs (100+ nodes)**
   - Use Fit to View to see entire graph
   - Zoom in to specific areas for editing
   - Use box selection for bulk operations
   - Grid rendering optimized for 100+ nodes

2. **Smooth Dragging**
   - Editor targets 60 FPS
   - Cache invalidation optimized
   - Multi-node drag uses efficient updates

### Workflow

1. **Creating a New Workflow**
   ```
   1. Click "New" to start fresh
   2. Add start nodes (no incoming edges)
   3. Add intermediate task nodes
   4. Connect with edges (dependencies)
   5. Add end nodes (no outgoing edges)
   6. Validate: Check statistics for acyclic status
   ```

2. **Editing Existing Workflow**
   ```
   1. Load DAG from file
   2. Click "Fit" to see entire graph
   3. Use Select tool to examine nodes
   4. Make changes with appropriate tools
   5. Save when complete
   ```

3. **Avoiding Cycles**
   - Plan dependency direction first
   - Generally flow top-to-bottom or left-to-right
   - Use "Find Critical Path" to analyze
   - Editor prevents cycle creation automatically

### Debugging

1. **Check Statistics Panel**
   - Node count, edge count
   - Start/end node counts
   - Maximum depth
   - Connectivity status
   - Acyclic validation

2. **Validation Messages**
   - Red border: Invalid state
   - Error messages shown in console
   - Check for unreachable nodes

---

## Advanced Features

### Multi-Select Dragging

When dragging multiple nodes:
- All nodes maintain relative positions
- Snap to grid applies to primary node
- Undo records all position changes
- Works smoothly with 50+ nodes

### Edge Creation Validation

Real-time validation during edge creation:
- Preview line shows connection path
- Hover feedback on target node
- Red preview if cycle would be created
- Automatic rejection with error message

### Zoom-to-Cursor

Zooming behavior is optimized:
- World position under cursor stays fixed
- Offset automatically adjusted
- Smooth zoom levels
- No cursor drift

### Box Selection

Intelligent box selection:
- Real-time rectangle preview
- Nodes selected if ANY part intersects
- Works at any zoom level
- Can be combined with Ctrl+Click

---

## Troubleshooting

### "Cannot create edge" message

**Cause**: Edge would create a cycle

**Solution**:
1. Check existing dependencies
2. Verify desired direction
3. Consider if dependency is necessary
4. Use indirect path if needed

### Nodes moving incorrectly

**Cause**: Zoom or pan state issue

**Solution**:
1. Press "Reset" to return to 100% zoom
2. Try operation again
3. If persists, reload DAG

### Can't select node

**Cause**: Node may be behind another node

**Solution**:
1. Zoom in to area
2. Move overlapping nodes
3. Use properties panel to edit directly

### Performance issues

**Cause**: Very large graph (500+ nodes)

**Solution**:
1. Consider splitting into sub-graphs
2. Use zoom to focus on specific areas
3. Limit visible nodes
4. Check for unnecessary edges

---

## Statistics Panel

The bottom statistics panel shows:

- **Nodes**: Total node count
- **Edges**: Total edge count
- **Start**: Nodes with no incoming edges
- **End**: Nodes with no outgoing edges
- **Depth**: Maximum dependency chain length
- **Connected**: All nodes reachable from start
- **Acyclic**: No circular dependencies (✓ or ✗)
- **Zoom**: Current zoom percentage

---

## Properties Panel

The right properties panel shows:

- **Selected Node Info**
  - Node label
  - Unique ID (UUID)
  - Position coordinates
  - Incoming edge count
  - Outgoing edge count

- **Multi-Selection**
  - Shows count of selected nodes
  - Common properties (future)

---

## Future Enhancements

Coming soon:

1. **Context Menus**
   - Right-click node for options
   - Right-click canvas for add node
   - Right-click edge for delete/edit

2. **Node Editing**
   - Double-click to edit label
   - Inline property editing
   - Custom colors and icons

3. **Edge Styling**
   - Custom colors
   - Line styles (dashed, dotted)
   - Edge labels

4. **Layout Algorithms**
   - Auto-arrange (hierarchical)
   - Force-directed layout
   - Circular layout

5. **Export/Import**
   - Export as PNG/SVG
   - Import from JSON
   - Swarm.toml integration

---

## Support

For issues or questions:

- Check this documentation first
- Review example workflows
- Submit issue on GitHub
- Contact development team

---

**Version**: Phase 3.8.4
**Last Updated**: 2025-11-24
**Performance Target**: 60 FPS
**Tested With**: Graphs up to 500 nodes
