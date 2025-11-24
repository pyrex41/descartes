# DAG Editor UI Mockup and Design Specification

## Visual Design Overview

This document provides detailed mockups and design specifications for the DAG Editor UI implemented in Phase 3.8.3.

---

## Main Application Layout

### Full Window Layout (1200x800 pixels)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│ Descartes GUI                    ● Daemon: Connected    [Disconnect]            │
│ Status: DAG loaded successfully                                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│ ↖Select  +Add  →Edge  ×Del  ✋Pan │ [+] [-] Reset Fit │ Grid:ON Snap:OFF  [New] │
├──────────┬──────────────────────────────────────────────────────────────┬────────┤
│          │                                                              │        │
│ Dashbrd  │                    Canvas Area                               │ Props  │
│ TaskBrd  │           (Interactive DAG Visualization)                    │        │
│ Swarm    │                                                              │ Node:  │
│ Debuggr  │     ╔════════════════════╗                                  │ Start  │
│►DAG Ed◄  │     ║      Start         ║                                  │        │
│ Context  │     ╚══════════╤═════════╝                                  │ ID:    │
│          │                │                                             │ 3fa2b  │
│          │       ┌────────┼────────┐                                   │        │
│          │       │        │        │                                   │ Pos:   │
│          │   ╭───▼───╮╭───▼───╮╭───▼───╮                              │ 400,50 │
│          │   │ Load  ││ Init  ││Setup  │                              │        │
│          │   │ Data  ││Config ││ Res.  │                              │ Desc:  │
│          │   ╰───┬───╯╰───┬───╯╰───┬───╯                              │ Entry  │
│          │       │        │        │                                   │ point  │
│          │       └────────┼────────┘                                   │        │
│          │                │                                             │ In: 0  │
│          │           ╭────▼────╮                                        │ Out: 3 │
│          │           │Transform│                                        │        │
│          │           │  Data   │                                        │ Tags:  │
│          │           ╰────┬────╯                                        │ entry  │
│          │                │                                             │        │
│          │           ╭────▼────╮                                        │ [Edit] │
│          │           │Aggregate│                                        │ [Del]  │
│          │           │ Results │                                        │        │
│          │           ╰────┬────╯                                        │        │
│          │                │                                             │        │
│          │         ╭──────▼──────╮                                     │        │
│          │         │  Complete   │                                     │        │
│          │         ╰─────────────╯                                     │        │
│          │                                                              │        │
├──────────┴──────────────────────────────────────────────────────────────┴────────┤
│ Nodes:9 │Edges:13 │Start:1 │End:1 │Depth:4 │Connected:✓ │Acyclic:✓ │Zoom:100%  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Dimensions

| Component          | Width      | Height      | Notes                    |
|-------------------|------------|-------------|--------------------------|
| Header            | 100%       | 60px        | Fixed                    |
| Toolbar           | 100%       | 50px        | Fixed, toggle-able       |
| Navigation Sidebar| 200px      | Fill        | Fixed width              |
| Canvas Area       | Fill       | Fill        | Flexible                 |
| Properties Panel  | 300px      | Fill        | Fixed width, toggle-able |
| Statistics Panel  | 100%       | 40px        | Fixed, toggle-able       |

---

## Node Design Specifications

### Node Dimensions

```
┌────────────────────────────────────────┐
│                                        │
│  160px width                           │
│  NODE_WIDTH = 160.0                    │
│                                        │
│    ╭────────────────────────╮         │
│    │                        │         │ ← 8px border radius
│    │   Task Name Label      │ 60px   │   NODE_RADIUS = 8.0
│    │   (14-24px font)       │ height │
│    │                        │         │
│    ╰────────────────────────╯         │
│                                        │
│  NODE_HEIGHT = 60.0                    │
│                                        │
└────────────────────────────────────────┘
```

### Node States with Colors

#### 1. Normal State
```
╭──────────────────────────╮
│                          │
│   Transform Data         │  Colors:
│   (centered, white)      │  - Background: rgb(70, 130, 180)   [Steel Blue]
│                          │  - Border: rgb(50, 90, 130)        [Dark Steel Blue]
╰──────────────────────────╯  - Text: rgb(255, 255, 255)       [White]
                              - Border Width: 1.5px
```

#### 2. Hover State
```
╭──────────────────────────╮
│                          │  Colors:
│   Transform Data         │  - Background: rgb(70, 130, 180)   [Steel Blue]
│   (slightly glow)        │  - Border: rgb(150, 200, 255)      [Light Blue]
│                          │  - Text: rgb(255, 255, 255)       [White]
╰──────────────────────────╯  - Border Width: 1.5px
                              - Effect: Subtle glow
```

#### 3. Selected State
```
╔══════════════════════════╗
║                          ║  Colors:
║   Transform Data         ║  - Background: rgb(70, 130, 180)   [Steel Blue]
║   (highlighted)          ║  - Border: rgb(255, 200, 0)        [Gold]
║                          ║  - Text: rgb(255, 255, 255)       [White]
╚══════════════════════════╝  - Border Width: 3.0px
                              - Effect: Strong highlight
```

#### 4. Multi-Selected State
```
╔══════════════════════════╗  ╔══════════════════════════╗
║   Node 1                 ║  ║   Node 2                 ║
║   (selected)             ║  ║   (selected)             ║
╚══════════════════════════╝  ╚══════════════════════════╝

Both nodes: Gold border (3px)
Properties panel shows: "2 nodes selected"
```

### Node with Metadata Display (Future)

```
╭──────────────────────────╮
│ ⭐                       │ ← Icon (top-left)
│   Important Task         │
│   [Critical]             │ ← Badge (priority)
│                          │
│ ⚙️ Processing            │ ← Status (bottom)
╰──────────────────────────╯
```

---

## Edge Design Specifications

### Edge Types and Visual Styles

#### 1. Dependency (Hard)
```
Node A ═════════════════════════════════════════════> Node B
       ▲───────────────────────────────────────────▶
       │
       └─ Solid line, 2px width
          Color: rgb(200, 200, 200) [Light Gray]
          Arrow: 10px triangle
```

**Code:**
```rust
EdgeType::Dependency
Color: rgb(200, 200, 200)
Width: 2.0px
Style: Solid
```

#### 2. Soft Dependency
```
Node A ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─> Node B
       ▲─────────────────────────────────────────▶
       │
       └─ Dashed line (future), 2px width, 60% opacity
          Color: rgba(200, 200, 200, 0.6)
```

**Code:**
```rust
EdgeType::SoftDependency
Color: rgba(200, 200, 200, 0.6)
Width: 2.0px
Style: Dashed (rendered as solid currently)
```

#### 3. Data Flow
```
Node A ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━> Node B
       ▲───────────────────────────────────────────▶
       │
       └─ Solid line, 2px width
          Color: rgb(100, 200, 100) [Green]
          Indicates data passing
```

**Code:**
```rust
EdgeType::DataFlow
Color: rgb(100, 200, 100)
Width: 2.0px
Style: Solid
```

#### 4. Trigger
```
Node A ∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿∿> Node B
       ▲───────────────────────────────────────────▶
       │
       └─ Wavy line (rendered as solid), 2px width
          Color: rgb(255, 150, 50) [Orange]
          Event-driven activation
```

**Code:**
```rust
EdgeType::Trigger
Color: rgb(255, 150, 50)
Width: 2.0px
Style: Solid (wavy in future)
```

#### 5. Selected Edge
```
Node A ═══════════════════════════════════════════> Node B
       ╔═══════════════════════════════════════════╗
       │
       └─ Thicker line, 3px width
          Color: rgb(255, 200, 0) [Gold]
          Highlights selection
```

**Code:**
```rust
is_selected: true
Color: rgb(255, 200, 0)
Width: 3.0px (1.5x normal)
```

### Arrow Head Geometry

```
                    Target Node
                         ▲
                        ╱ ╲
                       ╱   ╲
                      ╱     ╲
                     ╱───────╲
                    ▕         ▕
                     ╲       ╱
                      ╲     ╱
                       ╲   ╱
                        ╲ ╱
                         ▼
                    From Edge

Arrow Size: 10px
Angle: 0.5 radians (~28.6°)
Fill: Same as edge color
```

**Math:**
```rust
let arrow_size = 10.0 * zoom;
let arrow_angle = 0.5; // radians

// Calculate arrow points
p1 = to - arrow_size * rotate(unit_vector, arrow_angle)
p2 = to - arrow_size * rotate(unit_vector, -arrow_angle)

// Draw filled triangle: to -> p1 -> p2 -> to
```

---

## Grid Background Design

### Grid Pattern

```
┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
│     │     │     │     │     │     │     │     │
│     │     │     │     │     │     │     │     │
├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
│     │     │     │     │     │     │     │     │
│     │     │     │     │     │     │     │     │
├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
│     │     │╭────┼─────┼────╮│     │     │     │
│     │     ││Node│     │    ││     │     │     │
├─────┼─────┼┴────┼─────┼────┴┼─────┼─────┼─────┤
│     │     │     │     │     │     │     │     │
│     │     │     │     │     │     │     │     │
└─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘

Grid Cell: 20x20 pixels
Line Color: rgba(100, 100, 100, 0.2) [Light Gray, 20% opacity]
Line Width: 1px
```

**Snap-to-Grid Example:**
```
Before Snap:              After Snap:
  (237, 143)    ──────>     (240, 140)

Snap formula: round(pos / 20) * 20
```

### Grid at Different Zoom Levels

#### Zoom 50%
```
┌───────────┬───────────┬───────────┐
│           │           │           │
│  (10px)   │  (10px)   │  (10px)   │
├───────────┼───────────┼───────────┤
│           │           │           │
│           │           │           │
└───────────┴───────────┴───────────┘
Grid appears more dense
```

#### Zoom 100%
```
┌─────┬─────┬─────┐
│     │     │     │
│(20px│(20px│(20px│
├─────┼─────┼─────┤
│     │     │     │
└─────┴─────┴─────┘
Normal grid spacing
```

#### Zoom 200%
```
┌──────────┬──────────┬──────────┐
│          │          │          │
│          │          │          │
│  (40px)  │  (40px)  │  (40px)  │
│          │          │          │
├──────────┼──────────┼──────────┤
Grid appears more sparse
```

---

## Toolbar Design

### Tool Buttons

```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│  [↖Select] [+Add] [→Edge] [×Del] [✋Pan] │ [+][-][Reset][Fit] │
│                                          │                     │
│  Active: Blue background                 │  Grid:ON Snap:OFF  │
│  Inactive: Gray background               │                    │
│                                          │  [New]             │
└────────────────────────────────────────────────────────────────┘
```

#### Button States

**Active Tool:**
```
╔═══════════╗
║ ↖ Select  ║  Color: Theme primary (blue)
╚═══════════╝  Border: 2px solid
               Background: Highlighted
```

**Inactive Tool:**
```
┌───────────┐
│ + Add     │  Color: Theme secondary (gray)
└───────────┘  Border: 1px solid
               Background: Normal
```

**Hovered Tool:**
```
┌───────────┐
│ → Edge    │  Color: Slightly lighter
└───────────┘  Border: 1px solid
               Background: Hover effect
```

### Tool Icons and Labels

| Icon | Label      | Keyboard | Function                    |
|------|------------|----------|-----------------------------|
| ↖    | Select     | V        | Select and move nodes       |
| +    | Add Node   | A        | Create new node             |
| →    | Add Edge   | E        | Connect nodes               |
| ×    | Delete     | Del      | Remove selected items       |
| ✋    | Pan        | Space    | Move canvas                 |

### View Controls

```
┌────────────────────────────────────┐
│ Zoom In [+]     Shortcut: +/=     │
│ Zoom Out [-]    Shortcut: -       │
│ Reset View      Shortcut: 0       │
│ Fit to View     Shortcut: F       │
└────────────────────────────────────┘
```

---

## Properties Panel Design

### Panel Layout (300px width)

```
╔════════════════════════════════╗
║         Properties             ║
╠════════════════════════════════╣
║                                ║
║ Node: Transform Data           ║
║ ────────────────────           ║
║                                ║
║ ID: 7fa3b2c1                   ║
║                                ║
║ Position: (400, 310)           ║
║                                ║
║ Description:                   ║
║ Apply data transformations     ║
║                                ║
║ Tags:                          ║
║ • processing                   ║
║                                ║
║ Connections:                   ║
║ • Incoming: 2                  ║
║   - Load Data (DataFlow)       ║
║   - Init Config (Dependency)   ║
║                                ║
║ • Outgoing: 1                  ║
║   - Aggregate Results          ║
║                                ║
║ Metadata:                      ║
║ • created: 2025-11-24          ║
║ • updated: 2025-11-24          ║
║                                ║
║ [Edit Node]  [Delete Node]     ║
║                                ║
╚════════════════════════════════╝
```

### Empty State

```
╔════════════════════════════════╗
║         Properties             ║
╠════════════════════════════════╣
║                                ║
║   No node selected             ║
║                                ║
║   Select a node to view        ║
║   its properties               ║
║                                ║
║                                ║
║   Tip: Click on any node       ║
║   in the canvas                ║
║                                ║
╚════════════════════════════════╝
```

### Multi-Select State

```
╔════════════════════════════════╗
║         Properties             ║
╠════════════════════════════════╣
║                                ║
║   3 nodes selected             ║
║                                ║
║   Bulk Actions:                ║
║                                ║
║   [Delete Selected]            ║
║   [Clear Selection]            ║
║                                ║
║   Statistics:                  ║
║   • Total Incoming: 5          ║
║   • Total Outgoing: 4          ║
║                                ║
╚════════════════════════════════╝
```

---

## Statistics Panel Design

### Full Statistics Bar

```
╔═════════════════════════════════════════════════════════════════════════════╗
║ Nodes: 9 │ Edges: 13 │ Start: 1 │ End: 1 │ Depth: 4 │ Conn: ✓ │ Acyc: ✓ │ Zoom: 100% ║
╚═════════════════════════════════════════════════════════════════════════════╝
```

### Statistics with Issues

```
╔═════════════════════════════════════════════════════════════════════════════╗
║ Nodes: 15 │ Edges: 20 │ Start: 2 │ End: 3 │ Depth: 6 │ Conn: ✗ │ Acyc: ✓ │ Zoom: 75% ║
╚═════════════════════════════════════════════════════════════════════════════╝
                                                          ▲
                                                          │
                                               Warning: Not connected
```

### Metric Colors

| Metric      | Good     | Warning  | Error    |
|-------------|----------|----------|----------|
| Connected   | ✓ Green  | ✗ Orange | ✗ Red    |
| Acyclic     | ✓ Green  | ✗ Orange | ✗ Red    |
| Nodes       | White    | -        | -        |
| Edges       | White    | -        | -        |

---

## Interaction Feedback

### Node Selection Feedback

```
Click Node:
┌──────────┐          ╔══════════╗
│  Node    │   ──>    ║  Node    ║
└──────────┘          ╚══════════╝
Normal               Selected (Gold border)


Shift+Click Another:
╔══════════╗          ╔══════════╗  ╔══════════╗
║  Node 1  ║   ──>    ║  Node 1  ║  ║  Node 2  ║
╚══════════╝          ╚══════════╝  ╚══════════╝
One selected         Both selected (Multi-select)
```

### Zoom Feedback

```
Zoom In (+):
┌────────┐     ┌──────────────┐     ┌────────────────────┐
│ Small  │ ──> │   Medium     │ ──> │     Large          │
└────────┘     └──────────────┘     └────────────────────┘
  50%              100%                  200%


Statistics bar updates: "Zoom: 50%" -> "Zoom: 100%" -> "Zoom: 200%"
```

### Pan Feedback

```
Before Pan:             After Pan (moved right):
╭─────╮                              ╭─────╮
│Node │                              │Node │
╰─────╯                              ╰─────╯
   ▲                                        ▲
   │                                        │
   └─ Position: (100, 100)     Position: (300, 100)

Canvas offset changes: (0, 0) -> (200, 0)
```

---

## Color Palette

### Primary Colors

| Element          | RGB              | Hex     | Usage                    |
|------------------|------------------|---------|--------------------------|
| Node Background  | (70, 130, 180)   | #4682B4 | Default node fill        |
| Node Border      | (50, 90, 130)    | #325A82 | Normal node outline      |
| Selection        | (255, 200, 0)    | #FFC800 | Selected highlight       |
| Hover            | (150, 200, 255)  | #96C8FF | Hover effect             |
| Grid             | (100, 100, 100)* | #646464 | Grid lines (20% opacity) |

*With alpha: rgba(100, 100, 100, 0.2)

### Edge Colors

| Edge Type         | RGB              | Hex     | Visual Effect           |
|------------------|------------------|---------|-------------------------|
| Dependency       | (200, 200, 200)  | #C8C8C8 | Light gray, standard    |
| Soft Dependency  | (200, 200, 200)* | #C8C8C8 | Translucent (60%)       |
| Data Flow        | (100, 200, 100)  | #64C864 | Green, data indicator   |
| Trigger          | (255, 150, 50)   | #FF9632 | Orange, event-driven    |
| Custom           | (150, 150, 150)  | #969696 | Medium gray             |

*With alpha: rgba(200, 200, 200, 0.6)

### UI Element Colors

| Element          | Color            | Usage                    |
|------------------|------------------|--------------------------|
| Text (Light)     | (255, 255, 255)  | Node labels, headings    |
| Text (Dark)      | (50, 50, 50)     | Property panel text      |
| Background       | Theme-based      | Panels, toolbar          |
| Border           | Theme-based      | Panel dividers           |
| Active Tool      | Theme primary    | Selected toolbar button  |

---

## Responsive Behavior

### Window Resize

```
Small Window (800x600):         Large Window (1600x1000):
┌─────────────────────┐         ┌──────────────────────────────────┐
│ Header              │         │ Header                           │
├────┬────────┬───────┤         ├────┬──────────────────────┬──────┤
│Nav │Canvas  │ Props │         │Nav │Canvas (expanded)     │Props │
│    │(small) │       │         │    │                      │      │
└────┴────────┴───────┘         └────┴──────────────────────┴──────┘

Canvas area expands/contracts to fill available space
Properties panel width remains fixed at 300px
Navigation bar width remains fixed at 200px
```

### Toggle Panels

```
Full Layout:
├────┬────────┬───────┤
│Nav │Canvas  │ Props │
└────┴────────┴───────┘


Properties Hidden:
├────┬───────────────┤
│Nav │Canvas (wide)  │
└────┴───────────────┘


All Panels Hidden:
├──────────────────────┤
│Canvas (full width)   │
└──────────────────────┘
```

---

## Animation and Transitions (Future)

### Planned Animations

**Node Selection:**
```
Duration: 150ms
Easing: ease-out
Effect: Border color transition
  Gray -> Gold (smooth color interpolation)
```

**Zoom:**
```
Duration: 200ms
Easing: ease-in-out
Effect: Scale transformation
  Current zoom -> Target zoom (smooth scaling)
```

**Pan:**
```
Duration: 300ms (auto-pan to node)
Easing: ease-in-out
Effect: Offset transition
  Current position -> Target position
```

**Node Creation:**
```
Duration: 200ms
Easing: ease-out
Effect: Scale from 0 to 1
  Appear: scale(0) -> scale(1)
```

---

## Accessibility Considerations

### Keyboard Navigation (Planned)

| Key           | Action                   |
|---------------|--------------------------|
| Tab           | Cycle through nodes      |
| Shift+Tab     | Reverse cycle            |
| Arrow Keys    | Move selected node       |
| +/=           | Zoom in                  |
| -             | Zoom out                 |
| 0             | Reset zoom               |
| F             | Fit to view              |
| Delete        | Delete selected          |
| Ctrl+A        | Select all               |
| Ctrl+Z        | Undo (future)            |
| Ctrl+Y        | Redo (future)            |

### Screen Reader Support (Future)

- Node descriptions read aloud
- Edge relationships announced
- Toolbar button labels
- Status updates for actions

---

## Performance Visual Indicators

### Large Graph Warning

```
╔════════════════════════════════════════════════╗
║  ⚠ Large Graph Detected                        ║
║                                                ║
║  Your graph has 523 nodes.                     ║
║  Performance may be reduced.                   ║
║                                                ║
║  Recommendations:                              ║
║  • Hide grid for better FPS                    ║
║  • Disable labels at low zoom                  ║
║  • Use minimap for navigation                  ║
║                                                ║
║  [Optimize] [Dismiss]                          ║
╚════════════════════════════════════════════════╝
```

### FPS Indicator (Debug Mode)

```
Top-right corner:
┌─────────┐
│ 60 FPS  │ ← Green (60 FPS)
└─────────┘

┌─────────┐
│ 45 FPS  │ ← Orange (30-60 FPS)
└─────────┘

┌─────────┐
│ 25 FPS  │ ← Red (<30 FPS)
└─────────┘
```

---

## Conclusion

This mockup document provides complete visual specifications for the DAG Editor UI. All elements have been implemented in Phase 3.8.3 with the exception of future enhancements noted.

**Key Visual Principles:**
1. **Clarity** - Clean, readable interface
2. **Feedback** - Visual response to all interactions
3. **Consistency** - Unified design language
4. **Performance** - Smooth rendering at 60 FPS
5. **Accessibility** - Keyboard support and contrast

**Color Scheme:**
- Professional blue palette
- Clear state differentiation
- High contrast for readability
- Theme-aware components

**Layout Philosophy:**
- Maximize canvas space
- Toggle-able panels
- Fixed-width sidebars
- Responsive to window size

For implementation details, see:
- `/home/user/descartes/DAG_RENDERER_REPORT.md`
- `/home/user/descartes/descartes/gui/src/dag_editor.rs`

---

**End of Mockup Document**
