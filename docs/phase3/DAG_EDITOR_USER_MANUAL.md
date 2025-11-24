# DAG Editor User Manual

Complete guide to using the Descartes visual DAG editor for building workflows.

**Version:** 1.0
**Last Updated:** 2025-11-24
**Difficulty:** Beginner to Intermediate

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [User Interface](#user-interface)
4. [Creating Workflows](#creating-workflows)
5. [Editing Operations](#editing-operations)
6. [View Controls](#view-controls)
7. [Keyboard Shortcuts](#keyboard-shortcuts)
8. [Exporting Workflows](#exporting-workflows)
9. [Tips and Tricks](#tips-and-tricks)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

The DAG Editor is a visual tool for designing complex multi-agent workflows. It allows you to:

- **Visually design** task dependencies
- **Interactively edit** workflows with drag-and-drop
- **Validate** workflow structure in real-time
- **Export** to executable Swarm.toml format
- **Collaborate** using visual representations

### When to Use the DAG Editor

Use the DAG Editor when:
- You have multiple tasks with dependencies
- Visual representation aids understanding
- You need to collaborate on workflow design
- You want to iterate on complex workflows
- You need to validate dependencies before execution

---

## Getting Started

### Opening the Editor

```bash
# Launch the DAG editor
descartes gui dag-editor

# Open an existing workflow
descartes gui dag-editor --file my_workflow.dag.json
```

### First Workflow

Let's create a simple workflow:

1. **Add a Start Node**: Click the "Add Node" tool, then click on the canvas
2. **Add a Process Node**: Click again to add another node
3. **Add an End Node**: Add a third node
4. **Connect Them**: Click the "Add Edge" tool, click the Start node, drag to Process, release
5. **Connect Process to End**: Repeat for Process → End edge
6. **Validate**: The editor automatically validates for cycles

Your first workflow is complete!

---

## User Interface

### Layout Overview

```
┌─────────────────────────────────────────────────────┐
│  Toolbar: [Tools] [View] [Grid] [New]              │
├──────────────────────────────┬──────────────────────┤
│                              │                      │
│                              │   Properties Panel   │
│        Canvas                │                      │
│    (Main Work Area)          │   - Node details    │
│                              │   - Metadata        │
│                              │   - Configuration   │
│                              │                      │
├──────────────────────────────┴──────────────────────┤
│  Statistics: Nodes: 3 | Edges: 2 | Connected: ✓   │
└─────────────────────────────────────────────────────┘
```

### Toolbar

**Tool Selection:**
- **Select (↖)**: Select and move nodes
- **Add Node (+)**: Create new nodes
- **Add Edge (→)**: Create dependencies
- **Delete (×)**: Remove nodes/edges
- **Pan (✋)**: Move the canvas

**View Controls:**
- **Zoom In [+]**: Increase zoom level
- **Zoom Out [-]**: Decrease zoom level
- **Reset**: Reset zoom and pan
- **Fit**: Fit all nodes in view

**Grid Controls:**
- **Grid: ON/OFF**: Toggle grid visibility
- **Snap: ON/OFF**: Toggle snap-to-grid

**File Operations:**
- **New**: Create new workflow

### Canvas

The main work area where you:
- Add and arrange nodes
- Draw edges between nodes
- Select and manipulate elements
- View workflow structure

**Grid**: 20-pixel grid for alignment
**Zoom Range**: 10% to 500%
**Pan**: Middle mouse or Pan tool

### Properties Panel

Shows details for selected nodes:
- **Node label**: Display name
- **Node ID**: Unique identifier
- **Position**: (x, y) coordinates
- **Incoming edges**: Dependencies
- **Outgoing edges**: Dependents
- **Metadata**: Configuration values

### Statistics Panel

Shows real-time workflow statistics:
- **Nodes**: Total node count
- **Edges**: Total edge count
- **Start**: Root nodes (no incoming edges)
- **End**: Leaf nodes (no outgoing edges)
- **Depth**: Maximum depth
- **Connected**: Whether all nodes are reachable
- **Acyclic**: Whether DAG is valid (no cycles)
- **Zoom**: Current zoom level

---

## Creating Workflows

### Adding Nodes

**Method 1: Tool-Based**
1. Click the "Add Node" tool
2. Click anywhere on canvas
3. Node appears at click position

**Method 2: Keyboard**
1. Press `N` key (if enabled)
2. Click to place node

**Node Properties:**
- **Default Label**: "Task N" (auto-numbered)
- **Default Position**: Click location
- **Auto-sized**: 160×60 pixels

### Adding Edges

**Method 1: Drag-and-Drop**
1. Click the "Add Edge" tool
2. Click the source node
3. Drag to the target node
4. Release to create edge

**Edge Validation:**
- ✓ **Valid**: Different nodes, no cycle
- ✗ **Invalid**: Self-loop detected
- ✗ **Invalid**: Would create cycle

**Edge Types:**
The editor creates Dependency edges by default. For other types, use the API or metadata.

### Node Configuration

Click a node to select it, then use the Properties Panel to:

**Basic Properties:**
- Label: Display name
- Description: Detailed text
- Position: (x, y) coordinates

**Metadata (Advanced):**
```json
{
  "agents": ["agent1", "agent2"],
  "entry_actions": ["setup", "validate"],
  "exit_actions": ["cleanup"],
  "required_resources": ["database"],
  "parallel_execution": true,
  "timeout_seconds": 300
}
```

---

## Editing Operations

### Moving Nodes

**Single Node:**
1. Select the "Select" tool
2. Click and drag node

**Multiple Nodes:**
1. Select first node
2. Ctrl+Click additional nodes
3. Drag any selected node
4. All selected nodes move together

**Box Selection:**
1. Select the "Select" tool
2. Click and drag in empty area
3. All nodes in box are selected

### Deleting Elements

**Delete Node:**
- Method 1: Select node, press Delete key
- Method 2: Select "Delete" tool, click node
- Method 3: Select node, click "Delete Selected" button

**Delete Edge:**
- Method 1: Select "Delete" tool, click edge
- Method 2: Select edge, press Delete key

**Note:** Deleting a node also deletes all connected edges.

### Selection

**Select Single:**
- Click node with Select tool

**Multi-Select:**
- Ctrl+Click to add nodes to selection
- Ctrl+Click selected node to deselect

**Select All:**
- Press Ctrl+A

**Deselect All:**
- Click empty area
- Press Escape

**Box Selection:**
- Click and drag in empty area
- All nodes in rectangle are selected

### Undo/Redo

**Undo:**
- Press Ctrl+Z
- Or click Undo button

**Redo:**
- Press Ctrl+Shift+Z or Ctrl+Y
- Or click Redo button

**Undo History:**
- Last 100 operations
- Cleared on "New Workflow"

**Undoable Operations:**
- Add node
- Delete node
- Move node
- Add edge
- Delete edge
- Update node properties

---

## View Controls

### Zoom

**Zoom In:**
- Click "Zoom In [+]" button
- Press + key
- Scroll mouse wheel up

**Zoom Out:**
- Click "Zoom Out [-]" button
- Press - key
- Scroll mouse wheel down

**Zoom to Cursor:**
- Mouse wheel zooms to cursor position
- Cursor stays at same world point

**Zoom Limits:**
- Minimum: 10% (0.1x)
- Maximum: 500% (5.0x)

### Pan

**Method 1: Middle Mouse**
- Hold middle mouse button
- Drag to pan

**Method 2: Pan Tool**
- Select Pan tool
- Click and drag

**Method 3: Space+Drag**
- Hold Space key
- Click and drag with left mouse

### Fit to View

**Automatic Framing:**
- Click "Fit" button
- Automatically calculates zoom and position
- All nodes visible in viewport

**Use Cases:**
- After loading a workflow
- After adding many nodes
- Lost in canvas space

### Reset View

**Reset to Default:**
- Click "Reset" button
- Zoom: 100% (1.0x)
- Position: (0, 0)
- Useful when view is messed up

---

## Keyboard Shortcuts

### General

| Shortcut | Action |
|----------|--------|
| Ctrl+Z | Undo last operation |
| Ctrl+Shift+Z | Redo last operation |
| Ctrl+Y | Redo (alternative) |
| Ctrl+A | Select all nodes |
| Delete | Delete selected items |
| Escape | Cancel current operation |

### Navigation

| Shortcut | Action |
|----------|--------|
| + | Zoom in |
| - | Zoom out |
| Space+Drag | Pan canvas |
| Middle Mouse+Drag | Pan canvas |
| Scroll Wheel | Zoom to cursor |

### Selection

| Shortcut | Action |
|----------|--------|
| Click | Select node |
| Ctrl+Click | Add to selection |
| Drag in Empty | Box selection |
| Escape | Deselect all |

### Tools (if enabled)

| Shortcut | Action |
|----------|--------|
| S | Select tool |
| N | Add Node tool |
| E | Add Edge tool |
| D | Delete tool |
| P | Pan tool |

---

## Exporting Workflows

### Export to Swarm.toml

**Step 1: Validate Workflow**
- Check Statistics panel
- Ensure "Acyclic: ✓"
- Ensure "Connected: ✓"

**Step 2: Configure Export**
```rust
let config = SwarmExportConfig::default()
    .with_workflow_name("my_workflow")
    .with_agent("agent1", "claude-3-opus")
    .with_author("Your Name");
```

**Step 3: Export**
```rust
let toml = export_dag_to_swarm_toml(&dag, &config)?;
```

**Step 4: Save**
```rust
use std::path::Path;
save_dag_as_swarm_toml(&dag, Path::new("workflow.toml"), &config)?;
```

### Configuration Best Practices

**Always Specify:**
- Workflow name
- Agent configurations
- Author information

**Consider Specifying:**
- Timeout values
- Retry configuration
- Resource definitions
- Guard expressions

**Node Metadata:**
Ensure nodes have required metadata:
- `agents`: Which agents execute this state
- `entry_actions`: Setup actions
- `exit_actions`: Cleanup actions
- `required_resources`: Dependencies

### Validation Checklist

Before export:
- ✓ No cycles detected
- ✓ All nodes connected
- ✓ Start nodes defined
- ✓ End nodes defined
- ✓ Node metadata complete
- ✓ Edge labels meaningful
- ✓ Agent configurations valid

---

## Tips and Tricks

### Workflow Design

**Start Simple:**
1. Begin with linear workflow
2. Add branching incrementally
3. Test at each stage

**Use Descriptive Names:**
- Good: "LoadData", "ProcessRecords", "ValidateOutput"
- Bad: "Task1", "Task2", "Task3"

**Organize Visually:**
- Left to right: Sequential flow
- Top to bottom: Hierarchical levels
- Group related nodes

**Leverage Grid:**
- Enable snap-to-grid
- Align nodes horizontally/vertically
- Create clean, readable layouts

### Performance

**Large Workflows (50+ nodes):**
- Use subgraphs for modules
- Disable animations if laggy
- Consider splitting into multiple workflows
- Use Fit to View sparingly

**Smooth Interaction:**
- Keep grid enabled
- Use keyboard shortcuts
- Batch operations when possible

### Collaboration

**Version Control:**
- Save workflows as JSON
- Commit to Git
- Use descriptive commit messages

**Documentation:**
- Add descriptions to nodes
- Use meaningful labels
- Document complex logic in metadata

**Review Process:**
- Export to Swarm.toml
- Review in text editor
- Test in staging environment

### Advanced Techniques

**Hierarchical Workflows:**
```json
{
  "parent": "ParentState",
  "metadata": {
    "subworkflow": "child_workflow.toml"
  }
}
```

**Conditional Execution:**
```json
{
  "guards": ["is_valid", "has_permission"],
  "metadata": {
    "condition": "context.user.role == 'admin'"
  }
}
```

**Parallel Execution:**
```json
{
  "parallel_execution": true,
  "metadata": {
    "max_parallel": 5
  }
}
```

---

## Troubleshooting

### Common Issues

**Issue: "Cycle Detected" Error**
- **Cause**: Added edge creates circular dependency
- **Solution**: Remove the edge causing the cycle
- **Prevention**: Plan workflow structure before adding edges

**Issue: Cannot Add Edge**
- **Cause**: Self-loop or cycle would be created
- **Solution**: Check source and target nodes
- **Fix**: Remove existing edges creating cycle

**Issue: Nodes Overlap**
- **Cause**: Auto-placement or manual positioning
- **Solution**: Drag nodes to separate them
- **Prevention**: Enable snap-to-grid

**Issue: Lost in Canvas**
- **Cause**: Panned/zoomed too far
- **Solution**: Click "Fit to View" button
- **Alternative**: Click "Reset" to return to origin

**Issue: Selection Not Working**
- **Cause**: Wrong tool selected
- **Solution**: Click "Select" tool
- **Check**: Look at highlighted tool button

**Issue: Undo Not Available**
- **Cause**: No operations to undo
- **Check**: Undo history is limited to 100 operations
- **Note**: History cleared on "New Workflow"

### Performance Issues

**Editor Lag:**
- Reduce node count (split workflow)
- Disable grid temporarily
- Close properties panel
- Restart editor

**Slow Rendering:**
- Check for thousands of edges
- Simplify complex workflows
- Use modern GPU-enabled system

### Export Issues

**Invalid Swarm.toml:**
- Validate DAG before export
- Check node metadata
- Ensure agent configurations complete

**Missing Agents:**
- Add agent configurations to export config
- Verify node metadata includes "agents" key

**Validation Errors:**
- Run `dag.validate()` programmatically
- Check for disconnected nodes
- Verify no unreachable nodes

### Getting Help

1. **Check Documentation**: See [DAG_REFERENCE.md](../DAG_REFERENCE.md)
2. **Review Examples**: Check `/examples/dag_workflows/`
3. **Run Tests**: Execute test suite to verify installation
4. **Contact Support**: Reach out to development team

---

## Quick Reference Card

### Mouse Actions

| Action | Result |
|--------|--------|
| Left Click | Select node/canvas |
| Left Drag (Select tool) | Move nodes |
| Left Drag (Empty) | Box selection |
| Middle Drag | Pan canvas |
| Scroll Wheel | Zoom to cursor |
| Right Click | Context menu (future) |

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+A | Select all |
| Delete | Delete selected |
| Escape | Cancel/Deselect |
| +/- | Zoom in/out |
| Space+Drag | Pan |

### Tools

| Tool | Purpose | Shortcut |
|------|---------|----------|
| Select | Move and select | S |
| Add Node | Create nodes | N |
| Add Edge | Create dependencies | E |
| Delete | Remove elements | D |
| Pan | Move canvas | P |

---

## Next Steps

After mastering the DAG Editor:

1. **Explore Advanced Features**: Hierarchical workflows, guards, parallel execution
2. **Learn Swarm.toml**: Understand the export format
3. **Test Workflows**: Execute in staging environment
4. **Build Complex Systems**: Combine multiple workflows
5. **Contribute**: Share workflows with the community

---

## Appendix

### File Formats

**DAG JSON (.dag.json)**
```json
{
  "name": "My Workflow",
  "description": "A sample workflow",
  "nodes": [...],
  "edges": [...]
}
```

**Swarm TOML (.toml)**
```toml
[metadata]
name = "My Workflow"
version = "1.0"

[[workflows]]
name = "my_workflow"
```

### Color Scheme

**Node Colors:**
- Blue: Normal node
- Gold: Selected node
- Light Blue: Hovered node

**Edge Colors:**
- Gray: Dependency
- Light Gray: Soft Dependency
- Green: Data Flow
- Orange: Trigger

**Background:**
- Dark Gray: Canvas
- Light Gray: Grid lines

---

**For more information, see:**
- [DAG Reference](../DAG_REFERENCE.md) - Complete API documentation
- [Swarm Export Guide](SWARM_EXPORT_QUICKSTART.md) - Export walkthrough
- [Phase 3 Overview](README.md) - System architecture

**Last Updated:** 2025-11-24
**Version:** 1.0
