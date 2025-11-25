# Task Board GUI - Technical Architecture

## Component Hierarchy

```
TaskBoard (Main Container)
│
├── Header Section
│   ├── Title ("Task Board")
│   ├── Statistics Bar
│   │   └── Task counts by status
│   └── Refresh Button
│
├── Filter Controls Section
│   ├── Priority Filter Dropdown
│   ├── Complexity Filter Dropdown
│   ├── Sort Order Dropdown
│   ├── Blocked Only Toggle
│   └── Clear Filters Button
│
└── Kanban Board Layout
    │
    ├── Todo Column (Blue)
    │   ├── Column Header
    │   ├── Task Count
    │   └── Scrollable Card List
    │       ├── TaskCard 1
    │       ├── TaskCard 2
    │       └── ...
    │
    ├── In Progress Column (Orange)
    │   ├── Column Header
    │   ├── Task Count
    │   └── Scrollable Card List
    │       ├── TaskCard 1
    │       ├── TaskCard 2
    │       └── ...
    │
    ├── Done Column (Green)
    │   ├── Column Header
    │   ├── Task Count
    │   └── Scrollable Card List
    │       ├── TaskCard 1
    │       ├── TaskCard 2
    │       └── ...
    │
    └── Blocked Column (Red)
        ├── Column Header
        ├── Task Count
        └── Scrollable Card List
            ├── TaskCard 1
            ├── TaskCard 2
            └── ...
```

## TaskCard Component Structure

```
┌────────────────────────────────────────┐
│ ╔═══════════════════════════════════╗ │ ← Container (clickable button)
│ ║ Task Title (truncated if long)    ║ │
│ ║                                   ║ │
│ ║ ┌─────┐ ┌────────┐ ┌──────────┐  ║ │ ← Badges Row
│ ║ │PRIO │ │COMPLEX │ │X DEPS    │  ║ │
│ ║ └─────┘ └────────┘ └──────────┘  ║ │
│ ║                                   ║ │
│ ║ @assignee (or "Unassigned")       ║ │ ← Assignee Info
│ ║                                   ║ │
│ ║ Description text (truncated to    ║ │ ← Description Preview
│ ║ 60 characters...)                 ║ │
│ ╚═══════════════════════════════════╝ │
└────────────────────────────────────────┘
   │                                     │
   └─ Click → TaskClicked(task_id) ─────┘
```

## Data Flow Architecture

### State Management

```
┌─────────────────────────────────────────────────────────┐
│ TaskBoardState                                          │
│ ─────────────────────────────────────────────────────── │
│ • kanban_board: KanbanBoard                            │
│   ├── todo: Vec<Task>                                   │
│   ├── in_progress: Vec<Task>                            │
│   ├── done: Vec<Task>                                   │
│   └── blocked: Vec<Task>                                │
│                                                         │
│ • filters: TaskFilters                                  │
│   ├── priority: Option<TaskPriority>                    │
│   ├── complexity: Option<TaskComplexity>                │
│   ├── assignee: Option<String>                          │
│   ├── search_term: Option<String>                       │
│   └── show_blocked_only: bool                           │
│                                                         │
│ • sort: TaskSort                                        │
│   (Priority | Complexity | CreatedAt | UpdatedAt | Title) │
│                                                         │
│ • selected_task: Option<Uuid>                           │
│ • loading: bool                                         │
│ • error: Option<String>                                 │
└─────────────────────────────────────────────────────────┘
```

### Message Flow

```
User Action                Message Type              State Update
───────────               ─────────────             ─────────────

Click Task     ──────→   TaskClicked(id)    ──────→  selected_task = Some(id)
                                                      Trigger re-render

Click Refresh  ──────→   RefreshTasks       ──────→  loading = true
                                                      Send RPC request

Filter Change  ──────→   FilterByPriority() ──────→  filters.priority = ...
                                                      Apply filters
                                                      Trigger re-render

Sort Change    ──────→   ChangeSortOrder()  ──────→  sort = ...
                                                      Re-sort tasks
                                                      Trigger re-render

Load Complete  ──────→   TasksLoaded(board) ──────→  kanban_board = board
                                                      loading = false
                                                      Trigger re-render
```

### Rendering Pipeline

```
view() called
    │
    ├──> view_header()
    │     └─> Statistics + Refresh button
    │
    ├──> view_filters()
    │     └─> Filter controls + Sort controls
    │
    └──> view_kanban_board()
         │
         ├──> view_kanban_column("Todo", tasks, ...)
         │     │
         │     ├─> Apply filters: apply_filters(tasks)
         │     ├─> Apply sorting: sort_tasks(filtered)
         │     └─> Map to cards: view_task_card(task, state)
         │
         ├──> view_kanban_column("In Progress", tasks, ...)
         │     │
         │     ├─> Apply filters
         │     ├─> Apply sorting
         │     └─> Map to cards
         │
         ├──> view_kanban_column("Done", tasks, ...)
         │     │
         │     ├─> Apply filters
         │     ├─> Apply sorting
         │     └─> Map to cards
         │
         └──> view_kanban_column("Blocked", tasks, ...)
               │
               ├─> Apply filters
               ├─> Apply sorting
               └─> Map to cards
```

## Filtering Logic Flow

```
Original Task List
      │
      │ FILTER 1: Priority
      ├────────────────────┐
      │ if priority_filter │ ──YES──> Keep if task.priority == filter
      │ is Some            │ ──NO───> Keep all
      └────────────────────┘
      │
      │ FILTER 2: Complexity
      ├────────────────────┐
      │ if complexity_     │ ──YES──> Keep if task.complexity == filter
      │ filter is Some     │ ──NO───> Keep all
      └────────────────────┘
      │
      │ FILTER 3: Assignee
      ├────────────────────┐
      │ if assignee_filter │ ──YES──> Keep if task.assigned_to == filter
      │ is Some            │ ──NO───> Keep all
      └────────────────────┘
      │
      │ FILTER 4: Search Term
      ├────────────────────┐
      │ if search_term     │ ──YES──> Keep if title or desc contains term
      │ is Some            │ ──NO───> Keep all
      └────────────────────┘
      │
      │ FILTER 5: Blocked Only
      ├────────────────────┐
      │ if show_blocked_   │ ──YES──> Keep if task.status == Blocked
      │ only is true       │ ──NO───> Keep all
      └────────────────────┘
      │
      V
Filtered Task List
```

## Sorting Logic Flow

```
Unsorted Task List
      │
      V
┌─────────────────┐
│ Match sort type │
└─────────────────┘
      │
      ├─ Priority    ──> sort_by(|a,b| b.priority.cmp(&a.priority))
      │                  (Highest first: Critical > High > Medium > Low)
      │
      ├─ Complexity  ──> sort_by(|a,b| b.complexity.cmp(&a.complexity))
      │                  (Largest first: Epic > Complex > Moderate > Simple > Trivial)
      │
      ├─ CreatedAt   ──> sort_by(|a,b| b.created_at.cmp(&a.created_at))
      │                  (Newest first)
      │
      ├─ UpdatedAt   ──> sort_by(|a,b| b.updated_at.cmp(&a.updated_at))
      │                  (Most recently updated first)
      │
      └─ Title       ──> sort_by(|a,b| a.title.cmp(&b.title))
                         (Alphabetical A-Z)
      │
      V
Sorted Task List
```

## Color Coding System

### Status Colors (Column Headers)

```
┌──────────┬─────────┬──────────┬────────────────────────────┐
│ Status   │ Color   │ Hex      │ Meaning                    │
├──────────┼─────────┼──────────┼────────────────────────────┤
│ Todo     │ Blue    │ #3498db  │ Ready to start             │
│ InProg   │ Orange  │ #f39c12  │ Active work                │
│ Done     │ Green   │ #2ecc71  │ Completed                  │
│ Blocked  │ Red     │ #e74c3c  │ Waiting on dependencies    │
└──────────┴─────────┴──────────┴────────────────────────────┘
```

### Priority Badge Colors

```
┌──────────┬─────────┬──────────┬────────────────────────────┐
│ Priority │ Color   │ Hex      │ Visual Impact              │
├──────────┼─────────┼──────────┼────────────────────────────┤
│ Critical │ Red     │ #e74c3c  │ High urgency, demands      │
│          │         │          │ immediate attention        │
│ High     │ Orange  │ #f39c12  │ Important, prioritize      │
│ Medium   │ Blue    │ #3498db  │ Normal priority            │
│ Low      │ Gray    │ #95a5a6  │ Can be deferred            │
└──────────┴─────────┴──────────┴────────────────────────────┘
```

### Complexity Badge Colors

```
┌──────────┬─────────┬──────────┬────────────────────────────┐
│ Complexity│ Color   │ Hex     │ Effort Level               │
├──────────┼─────────┼──────────┼────────────────────────────┤
│ Epic     │ Red     │ #e74c3c  │ 1+ weeks, major effort     │
│ Complex  │ Dark Or │ #e67e22  │ 3-5 days, significant      │
│ Moderate │ Orange  │ #f39c12  │ 1-2 days, moderate         │
│ Simple   │ Blue    │ #3498db  │ 1-4 hours, straightforward │
│ Trivial  │ Gray    │ #95a5a6  │ < 1 hour, minimal effort   │
└──────────┴─────────┴──────────┴────────────────────────────┘
```

### Dependency Indicator Color

```
Purple (#9b59b6) - Indicates task has dependencies
```

## Integration Points

### Current Integration

```
Main GUI Application (main.rs)
        │
        ├─> TaskBoardState (state management)
        │
        ├─> Message::TaskBoard(msg) (message routing)
        │    │
        │    └─> task_board::update() (state updates)
        │
        └─> view_task_board() (rendering)
             │
             └─> task_board::view() (component rendering)
```

### Future Integration (RPC)

```
Daemon (descartes-daemon)
    │
    ├─> Task CRUD operations
    │   ├── Create task
    │   ├── Update task
    │   ├── Delete task
    │   └── Query tasks
    │
    └─> Task events
        ├── Task created
        ├── Task updated
        ├── Status changed
        └── Assignment changed
              │
              V
        RPC Client (GuiRpcClient)
              │
              V
        Task Board Component
              │
              ├─> Load tasks on mount
              ├─> Subscribe to updates
              └─> Send task mutations
```

## Performance Characteristics

### Time Complexity

```
Operation              | Complexity | Notes
─────────────────────────────────────────────────────────
Filter tasks           | O(n)       | Linear scan through tasks
Sort tasks             | O(n log n) | Standard comparison sort
Select task            | O(1)       | Direct UUID lookup
Render column          | O(n)       | One card per task
Toggle filter          | O(1)       | State update only
```

### Space Complexity

```
Component              | Space      | Notes
─────────────────────────────────────────────────────────
Task storage           | O(n)       | n = total tasks
Filtered view          | O(n)       | Copy of filtered tasks
Column rendering       | O(n)       | Widgets for each task
State management       | O(1)       | Fixed size state
```

### Scalability Limits

```
Recommended:  < 100 tasks per column (optimal UX)
Maximum:      < 500 tasks per column (performance maintained)
Beyond 500:   Consider pagination or virtual scrolling
```

## Error Handling

```
Error Source              Handler                   User Feedback
─────────────────────────────────────────────────────────────────────
Failed to load tasks  →  LoadError message    →  Error display in UI
Invalid task data     →  Skip task + log      →  Continue rendering
Missing dependencies  →  Show dep count       →  Visual indicator
RPC timeout           →  Retry logic          →  Loading indicator
Network error         →  Fallback to cache    →  Offline mode badge
```

## Accessibility Features

### Current Implementation

- Clear color differentiation for colorblind users
- High contrast text on all backgrounds
- Keyboard navigation support (via Iced)
- Clickable areas sized appropriately
- Text truncation with clear indicators

### Future Enhancements

- Screen reader annotations
- Keyboard shortcuts for common actions
- Focus indicators
- ARIA labels for UI elements
- Configurable color schemes

## Extension Points

### Easy to Add

1. **New Filters**
   ```rust
   // Add to TaskFilters struct
   pub struct TaskFilters {
       // ... existing filters
       pub created_after: Option<i64>,
       pub tags: Option<Vec<String>>,
   }
   ```

2. **New Sort Options**
   ```rust
   pub enum TaskSort {
       // ... existing options
       DueDate,
       Assignee,
       Dependencies,
   }
   ```

3. **New Task Actions**
   ```rust
   pub enum TaskBoardMessage {
       // ... existing messages
       EditTask(Uuid),
       DeleteTask(Uuid),
       DuplicateTask(Uuid),
   }
   ```

### Requires Refactoring

1. **Drag and Drop**: Need Iced drag-drop support
2. **Multi-select**: Need selection state model
3. **Custom Columns**: Need dynamic column system
4. **Sub-tasks**: Need hierarchical task model

## Testing Strategy

### Unit Tests (Implemented)

- State creation and initialization
- Filter application logic
- Sort functionality
- Message handling

### Integration Tests (Future)

- Full render cycle
- User interaction simulation
- RPC integration
- State persistence

### Visual Tests (Future)

- Screenshot comparison
- Responsive layout testing
- Theme variation testing
- Accessibility compliance

## Build and Deployment

### Build Commands

```bash
# Build GUI only
cargo build -p descartes-gui

# Build with optimizations
cargo build -p descartes-gui --release

# Run standalone demo
cargo run --example task_board_demo

# Run tests
cargo test -p descartes-gui task_board
```

### Dependencies Graph

```
descartes-gui
    │
    ├─> descartes-core (local)
    │    └─> Task data models
    │
    ├─> descartes-daemon (local)
    │    └─> RPC client types
    │
    ├─> iced (0.13)
    │    └─> GUI framework
    │
    ├─> uuid
    │    └─> Task identification
    │
    └─> chrono
         └─> Timestamp handling
```

## Conclusion

The Task Board GUI component is architected for:

1. **Maintainability**: Clear separation of concerns
2. **Extensibility**: Easy to add features
3. **Performance**: Efficient rendering and updates
4. **Testability**: Isolated, testable components
5. **Integration**: Clean interfaces with daemon

This architecture supports the current requirements while providing a solid foundation for future enhancements.
