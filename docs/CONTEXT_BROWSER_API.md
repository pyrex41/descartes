# Context Browser API Reference

Technical reference for developers extending or integrating with the Descartes Context Browser.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Data Models](#core-data-models)
3. [File Tree View API](#file-tree-view-api)
4. [Code Preview Panel API](#code-preview-panel-api)
5. [Knowledge Graph Panel API](#knowledge-graph-panel-api)
6. [Knowledge Graph Overlay](#knowledge-graph-overlay)
7. [Events and Messages](#events-and-messages)
8. [Extension Points](#extension-points)
9. [Performance Considerations](#performance-considerations)
10. [Examples](#examples)

## Architecture Overview

### Component Structure

```
┌─────────────────────────────────────┐
│     Context Browser (Main)          │
├──────────┬───────────────┬──────────┤
│File Tree │ Code Preview  │ KG Panel │
│  View    │    Panel      │          │
└────┬─────┴───────┬───────┴─────┬────┘
     │             │             │
     └─────────────┼─────────────┘
                   │
          ┌────────▼────────┐
          │  Knowledge      │
          │  Graph Overlay  │
          └────────┬────────┘
                   │
          ┌────────▼────────┐
          │  Data Models    │
          │ (FileTree, KG)  │
          └─────────────────┘
```

### Module Layout

```
descartes/
├── agent-runner/
│   └── src/
│       ├── knowledge_graph.rs      # Core data models
│       └── knowledge_graph_overlay.rs  # Overlay logic
└── gui/
    └── src/
        ├── file_tree_view.rs       # File tree GUI
        ├── code_preview_panel.rs   # Code preview GUI
        └── knowledge_graph_panel.rs # KG visualization
```

### Data Flow

```
1. File System → FileTreeBuilder → FileTree
2. FileTree → KnowledgeGraphOverlay → KnowledgeGraph
3. FileTree ←→ KnowledgeGraph (bidirectional links)
4. GUI Components ← FileTree & KnowledgeGraph (read)
5. User Actions → GUI Components → Messages → State Updates
```

## Core Data Models

### FileTree

**Location:** `descartes-agent-runner/src/knowledge_graph.rs`

```rust
pub struct FileTree {
    pub root_id: Option<String>,
    pub nodes: HashMap<String, FileTreeNode>,
    pub path_index: HashMap<PathBuf, String>,
    pub file_count: usize,
    pub directory_count: usize,
    pub base_path: PathBuf,
}
```

**Key Methods:**

```rust
impl FileTree {
    /// Create new file tree
    pub fn new(base_path: PathBuf) -> Self;

    /// Add node to tree
    pub fn add_node(&mut self, node: FileTreeNode) -> String;

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&FileTreeNode>;

    /// Get mutable node
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut FileTreeNode>;

    /// Get node by path
    pub fn get_node_by_path(&self, path: &PathBuf) -> Option<&FileTreeNode>;

    /// Find nodes by name
    pub fn find_by_name(&self, name: &str) -> Vec<&FileTreeNode>;

    /// Filter by type
    pub fn filter_by_type(&self, node_type: FileNodeType) -> Vec<&FileTreeNode>;

    /// Get all files
    pub fn get_all_files(&self) -> Vec<&FileTreeNode>;

    /// Traverse depth-first
    pub fn traverse_depth_first<F>(&self, visitor: F)
    where
        F: FnMut(&FileTreeNode);

    /// Get statistics
    pub fn stats(&self) -> FileTreeStats;
}
```

### FileTreeNode

```rust
pub struct FileTreeNode {
    pub node_id: String,
    pub path: PathBuf,
    pub name: String,
    pub node_type: FileNodeType,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub metadata: FileMetadata,
    pub knowledge_links: Vec<String>,
    pub indexed: bool,
    pub depth: usize,
}
```

**Key Methods:**

```rust
impl FileTreeNode {
    /// Create new node
    pub fn new(
        path: PathBuf,
        node_type: FileNodeType,
        parent_id: Option<String>,
        depth: usize
    ) -> Self;

    /// Check if directory
    pub fn is_directory(&self) -> bool;

    /// Check if file
    pub fn is_file(&self) -> bool;

    /// Add child
    pub fn add_child(&mut self, child_id: String);

    /// Add knowledge link
    pub fn add_knowledge_link(&mut self, knowledge_node_id: String);

    /// Get extension
    pub fn extension(&self) -> Option<String>;
}
```

### KnowledgeGraph

**Location:** `descartes-agent-runner/src/knowledge_graph.rs`

```rust
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, KnowledgeNode>,
    pub edges: HashMap<String, KnowledgeEdge>,
    pub name_index: HashMap<String, String>,
    pub outgoing_edges: HashMap<String, Vec<String>>,
    pub incoming_edges: HashMap<String, Vec<String>>,
    pub type_index: HashMap<KnowledgeNodeType, Vec<String>>,
}
```

**Key Methods:**

```rust
impl KnowledgeGraph {
    /// Create new graph
    pub fn new() -> Self;

    /// Add node
    pub fn add_node(&mut self, node: KnowledgeNode) -> String;

    /// Add edge
    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> String;

    /// Get node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&KnowledgeNode>;

    /// Get node by qualified name
    pub fn get_node_by_name(&self, qualified_name: &str) -> Option<&KnowledgeNode>;

    /// Get nodes by type
    pub fn get_nodes_by_type(&self, node_type: KnowledgeNodeType) -> Vec<&KnowledgeNode>;

    /// Get outgoing edges
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&KnowledgeEdge>;

    /// Get incoming edges
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&KnowledgeEdge>;

    /// Get neighbors
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&KnowledgeNode>;

    /// Get neighbors by relationship
    pub fn get_neighbors_by_relationship(
        &self,
        node_id: &str,
        relationship: RelationshipType,
    ) -> Vec<&KnowledgeNode>;

    /// Find shortest path
    pub fn find_path(&self, from_id: &str, to_id: &str) -> Option<Vec<String>>;

    /// Find nodes matching predicate
    pub fn find_nodes<F>(&self, predicate: F) -> Vec<&KnowledgeNode>
    where
        F: Fn(&KnowledgeNode) -> bool;

    /// Get statistics
    pub fn stats(&self) -> KnowledgeGraphStats;
}
```

### KnowledgeNode

```rust
pub struct KnowledgeNode {
    pub node_id: String,
    pub content_type: KnowledgeNodeType,
    pub name: String,
    pub qualified_name: String,
    pub description: Option<String>,
    pub source_code: Option<String>,
    pub language: Option<Language>,
    pub signature: Option<String>,
    pub return_type: Option<String>,
    pub parameters: Vec<String>,
    pub type_parameters: Vec<String>,
    pub file_references: Vec<FileReference>,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub visibility: Option<String>,
    pub tags: HashSet<String>,
    pub metadata: HashMap<String, String>,
}
```

### Enumerations

**FileNodeType:**
```rust
pub enum FileNodeType {
    File,
    Directory,
    Symlink,
}
```

**KnowledgeNodeType:**
```rust
pub enum KnowledgeNodeType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    TypeAlias,
    Constant,
    Variable,
    Macro,
    Concept,
    Documentation,
}
```

**RelationshipType:**
```rust
pub enum RelationshipType {
    Calls,
    Imports,
    Inherits,
    Implements,
    Uses,
    Defines,
    DefinedIn,
    Overrides,
    Extends,
    DependsOn,
    SimilarTo,
    Related,
}
```

## File Tree View API

**Location:** `descartes-gui/src/file_tree_view.rs`

### State

```rust
pub struct FileTreeState {
    pub tree: Option<FileTree>,
    pub expanded_nodes: HashSet<String>,
    pub selected_node: Option<String>,
    pub search_query: String,
    pub filter_language: Option<Language>,
    pub show_hidden: bool,
    pub show_only_linked: bool,
    pub sort_order: SortOrder,
    pub highlighted_files: HashSet<String>,
    pub show_knowledge_for: Option<String>,
    pub navigation_history: Vec<String>,
    pub history_position: usize,
    pub bookmarked_nodes: HashSet<String>,
    pub hovered_node: Option<String>,
    pub regex_search: bool,
    pub recent_files: Vec<String>,
    pub pinned_files: HashSet<String>,
}
```

### Messages

```rust
pub enum FileTreeMessage {
    TreeLoaded(FileTree),
    ToggleExpand(String),
    SelectNode(String),
    OpenNode(String),
    SearchQueryChanged(String),
    ToggleShowHidden,
    ToggleShowOnlyLinked,
    FilterByLanguage(Option<Language>),
    SetSortOrder(SortOrder),
    ClearFilters,
    ExpandAll,
    CollapseAll,
    ShowKnowledgeNodes(String),
    NavigateToKnowledgeNode(String),
    HighlightRelatedFiles(Vec<String>),
    ClearHighlights,
    NavigateBack,
    NavigateForward,
    AddBookmark(String),
    RemoveBookmark(String),
    ClearBookmarks,
    JumpToBookmark(String),
    HoverNode(Option<String>),
    ToggleRegexSearch,
    PinFile(String),
    UnpinFile(String),
    FindReferences(String),
    GoToDefinition(String),
    ShowUsages(String),
    RevealInExplorer(String),
    CopyPath(String),
    CopyRelativePath(String),
}
```

### Update Function

```rust
pub fn update(state: &mut FileTreeState, message: FileTreeMessage);
```

### View Function

```rust
pub fn view(state: &FileTreeState) -> Element<FileTreeMessage>;
```

### Utility Functions

```rust
/// Get selected node
pub fn get_selected_node<'a>(state: &'a FileTreeState) -> Option<&'a FileTreeNode>;

/// Get selected file path
pub fn get_selected_path(state: &FileTreeState) -> Option<PathBuf>;

/// Check if node is visible
pub fn is_node_visible(state: &FileTreeState, node_id: &str) -> bool;
```

## Code Preview Panel API

**Location:** `descartes-gui/src/code_preview_panel.rs`

### State

```rust
pub struct CodePreviewState {
    pub file_path: Option<PathBuf>,
    pub lines: Vec<String>,
    pub scroll_position: usize,
    pub highlighted_ranges: Vec<(usize, usize)>,
    pub current_line: Option<usize>,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub search_index: usize,
    pub show_line_numbers: bool,
    pub show_whitespace: bool,
    pub word_wrap: bool,
    pub view_mode: ViewMode,
    pub diff_file: Option<PathBuf>,
    pub diff_lines: Vec<String>,
    pub bookmarks: Vec<usize>,
    pub annotations: HashMap<usize, String>,
    pub folded_ranges: Vec<(usize, usize)>,
    pub syntax_highlighting: bool,
    pub language: Option<String>,
}
```

### Messages

```rust
pub enum CodePreviewMessage {
    LoadFile(PathBuf),
    FileLoaded(PathBuf, Vec<String>, Option<String>),
    LoadDiffFile(PathBuf),
    DiffFileLoaded(PathBuf, Vec<String>),
    Clear,
    JumpToLine(usize),
    HighlightRange(usize, usize),
    ClearHighlights,
    SearchQueryChanged(String),
    NextSearchResult,
    PreviousSearchResult,
    ToggleLineNumbers,
    ToggleWhitespace,
    ToggleWordWrap,
    ToggleSyntaxHighlighting,
    SetViewMode(ViewMode),
    AddBookmark(usize),
    RemoveBookmark(usize),
    ClearBookmarks,
    AddAnnotation(usize, String),
    RemoveAnnotation(usize),
    ToggleFold(usize, usize),
    ScrollTo(usize),
    CopyLine(usize),
    CopySelection(usize, usize),
}
```

### Async Operations

```rust
/// Load file contents
pub async fn load_file(path: PathBuf) -> Result<(Vec<String>, Option<String>), String>;
```

### Utility Functions

```rust
/// Get visible line range
pub fn get_visible_range(state: &CodePreviewState, viewport_height: usize) -> (usize, usize);

/// Check if line is visible
pub fn is_line_visible(state: &CodePreviewState, line: usize, viewport_height: usize) -> bool;

/// Navigate to next bookmark
pub fn next_bookmark(state: &CodePreviewState) -> Option<usize>;

/// Navigate to previous bookmark
pub fn previous_bookmark(state: &CodePreviewState) -> Option<usize>;
```

## Knowledge Graph Panel API

**Location:** `descartes-gui/src/knowledge_graph_panel.rs`

### State

```rust
pub struct KnowledgeGraphPanelState {
    pub graph: Option<KnowledgeGraph>,
    pub selected_node: Option<String>,
    pub hovered_node: Option<String>,
    pub search_query: String,
    pub fuzzy_search: bool,
    pub type_filters: HashSet<KnowledgeNodeType>,
    pub relationship_filters: HashSet<RelationshipType>,
    pub show_only_connected: bool,
    pub layout_algorithm: LayoutAlgorithm,
    pub viewport: ViewportState,
    pub node_positions: HashMap<String, Point>,
    pub dragging_node: Option<String>,
    pub visualization_mode: VisualizationMode,
    pub show_labels: bool,
    pub show_edge_labels: bool,
    pub search_results: Vec<String>,
    pub canvas_cache: canvas::Cache,
    pub dependency_path: Option<Vec<String>>,
    pub impact_nodes: HashSet<String>,
    pub related_suggestions: Vec<String>,
    pub comparison_nodes: Vec<String>,
    pub show_minimap: bool,
    pub file_filter: Option<String>,
    pub call_hierarchy: Option<Vec<Vec<String>>>,
}
```

### Messages

```rust
pub enum KnowledgeGraphMessage {
    GraphLoaded(KnowledgeGraph),
    SelectNode(String),
    HoverNode(Option<String>),
    JumpToNode(String),
    SearchQueryChanged(String),
    ToggleFuzzySearch,
    ToggleTypeFilter(KnowledgeNodeType),
    ToggleRelationshipFilter(RelationshipType),
    ToggleShowOnlyConnected,
    SetLayoutAlgorithm(LayoutAlgorithm),
    SetVisualizationMode(VisualizationMode),
    ToggleLabels,
    ToggleEdgeLabels,
    ResetViewport,
    ZoomIn,
    ZoomOut,
    Pan(Vector),
    StartDragNode(String),
    DragNode(Point),
    EndDragNode,
    RelayoutGraph,
    ClearFilters,
    ExpandNode(String),
    FocusNode(String),
    FindReferences(String),
    GoToDefinition(String),
    FindUsages(String),
    ShowDependencyPath(String, String),
    AnalyzeImpact(String),
    ShowRelatedCode(String),
    AddToComparison(String),
    ClearComparison,
    ExportAsImage,
    ExportAsJson,
    ShowNodeHistory(String),
    FilterByFile(String),
    ShowCallHierarchy(String),
    ShowInheritanceHierarchy(String),
    ToggleMinimap,
}
```

### Layout Algorithms

```rust
pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
}
```

### Utility Functions

```rust
/// Get node color based on type
pub fn get_node_color(node_type: KnowledgeNodeType) -> Color;

/// Get node icon based on type
pub fn get_node_icon(node_type: KnowledgeNodeType) -> &'static str;
```

## Knowledge Graph Overlay

**Location:** `descartes-agent-runner/src/knowledge_graph_overlay.rs`

### KnowledgeGraphOverlay

```rust
pub struct KnowledgeGraphOverlay {
    config: OverlayConfig,
    parser: SemanticParser,
    cache: HashMap<PathBuf, CacheEntry>,
}
```

### Configuration

```rust
pub struct OverlayConfig {
    pub enabled_languages: Vec<Language>,
    pub extract_relationships: bool,
    pub max_file_size: Option<u64>,
    pub enable_cache: bool,
    pub cache_dir: Option<PathBuf>,
    pub cache_ttl: Duration,
    pub parallel_parsing: bool,
}
```

### Key Methods

```rust
impl KnowledgeGraphOverlay {
    /// Create new overlay
    pub fn new() -> ParserResult<Self>;

    /// Create with custom config
    pub fn with_config(config: OverlayConfig) -> ParserResult<Self>;

    /// Generate knowledge graph from file tree
    pub fn generate_knowledge_overlay(
        &mut self,
        file_tree: &FileTree,
    ) -> ParserResult<KnowledgeGraph>;

    /// Generate and link to file tree
    pub fn generate_and_link(
        &mut self,
        file_tree: &mut FileTree,
    ) -> ParserResult<KnowledgeGraph>;

    /// Update knowledge graph when file changes
    pub fn update_file(
        &mut self,
        file_path: &Path,
        file_tree: &FileTree,
        knowledge_graph: &mut KnowledgeGraph,
    ) -> ParserResult<()>;

    /// Find entities in file
    pub fn find_entities_in_file(
        &self,
        file_path: &Path,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode>;

    /// Find definition location
    pub fn find_definition(
        &self,
        qualified_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Option<&FileReference>;

    /// Find all references
    pub fn find_references(
        &self,
        entity_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&FileReference>;

    /// Traverse call graph
    pub fn traverse_call_graph(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
        max_depth: usize,
    ) -> Vec<Vec<String>>;

    /// Find by type
    pub fn find_by_type(
        &self,
        node_type: KnowledgeNodeType,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode>;

    /// Find by name pattern
    pub fn find_by_name_pattern(
        &self,
        pattern: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode>;

    /// Find callers
    pub fn find_callers(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode>;

    /// Find callees
    pub fn find_callees(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode>;

    /// Clear cache
    pub fn clear_cache(&mut self);

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats;
}
```

## Events and Messages

### Message Flow

```
User Action → GUI Component → Message → Update Function → State Change → Re-render
```

### Cross-Component Communication

```rust
// Example: File tree selection triggers code preview
match file_tree_message {
    FileTreeMessage::SelectNode(node_id) => {
        // Update file tree state
        file_tree_state.selected_node = Some(node_id.clone());

        // Trigger code preview
        if let Some(path) = get_selected_path(&file_tree_state) {
            code_preview_update(
                &mut code_preview_state,
                CodePreviewMessage::LoadFile(path)
            );
        }
    }
}
```

### Event Handling Patterns

**Synchronous Updates:**
```rust
pub fn update(state: &mut State, message: Message) {
    match message {
        Message::ImmediateAction => {
            // Update state directly
            state.field = new_value;
        }
    }
}
```

**Asynchronous Operations:**
```rust
pub fn update(state: &mut State, message: Message) -> Command<Message> {
    match message {
        Message::LoadFile(path) => {
            // Return command for async operation
            Command::perform(
                load_file(path),
                |result| Message::FileLoaded(result)
            )
        }
        Message::FileLoaded(result) => {
            // Handle async result
            state.content = result;
            Command::none()
        }
    }
}
```

## Extension Points

### Custom Node Types

Add new knowledge node types:

```rust
// Extend KnowledgeNodeType enum
pub enum KnowledgeNodeType {
    // ... existing types
    CustomType,
}

// Implement display
impl KnowledgeNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ... existing cases
            KnowledgeNodeType::CustomType => "custom",
        }
    }
}

// Add color mapping
pub fn get_node_color(node_type: KnowledgeNodeType) -> Color {
    match node_type {
        // ... existing cases
        KnowledgeNodeType::CustomType => Color::from_rgb8(R, G, B),
    }
}
```

### Custom Relationship Types

Add new relationship types:

```rust
pub enum RelationshipType {
    // ... existing types
    CustomRelation,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ... existing cases
            RelationshipType::CustomRelation => "custom_relation",
        }
    }
}
```

### Custom Filters

Implement custom filtering logic:

```rust
fn custom_filter(state: &FileTreeState, tree: &FileTree) -> HashSet<String> {
    let mut filtered = HashSet::new();

    for (node_id, node) in &tree.nodes {
        if meets_custom_criteria(node) {
            filtered.insert(node_id.clone());
        }
    }

    filtered
}
```

### Custom Layout Algorithms

Add graph layout algorithms:

```rust
pub enum LayoutAlgorithm {
    // ... existing algorithms
    Custom,
}

fn custom_layout(graph: &KnowledgeGraph) -> HashMap<String, Point> {
    let mut positions = HashMap::new();

    // Implement layout logic
    for (node_id, _node) in &graph.nodes {
        positions.insert(
            node_id.clone(),
            compute_custom_position(node_id, graph)
        );
    }

    positions
}
```

### Plugin System (Future)

Planned plugin architecture:

```rust
pub trait ContextBrowserPlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    fn on_file_tree_loaded(&mut self, tree: &FileTree);
    fn on_graph_loaded(&mut self, graph: &KnowledgeGraph);
    fn on_node_selected(&mut self, node_id: &str);

    fn custom_filters(&self) -> Vec<Box<dyn Filter>>;
    fn custom_actions(&self) -> Vec<Box<dyn Action>>;
}
```

## Performance Considerations

### Memory Management

**File Tree:**
- Nodes stored in `HashMap` for O(1) lookup
- Path index for fast path-based queries
- Lazy loading for large directories

**Knowledge Graph:**
- Indexed by ID, name, and type
- Edge indices for fast traversal
- Sparse graph representation

### Optimization Strategies

**1. Lazy Loading:**
```rust
// Only load visible nodes
fn load_visible_nodes(tree: &FileTree, viewport: &Viewport) -> Vec<&FileTreeNode> {
    tree.nodes
        .values()
        .filter(|node| is_in_viewport(node, viewport))
        .collect()
}
```

**2. Caching:**
```rust
// Cache computed layouts
pub struct KnowledgeGraphPanelState {
    pub canvas_cache: canvas::Cache,
    pub node_positions: HashMap<String, Point>,
    // ...
}

// Invalidate cache on changes
state.canvas_cache.clear();
```

**3. Batch Updates:**
```rust
// Batch multiple operations
fn batch_update(state: &mut State, messages: Vec<Message>) {
    for message in messages {
        update_without_render(state, message);
    }
    invalidate_cache(state);
}
```

**4. Incremental Processing:**
```rust
// Process large graphs incrementally
async fn process_large_graph(graph: &KnowledgeGraph, batch_size: usize) {
    for chunk in graph.nodes.values().chunks(batch_size) {
        process_chunk(chunk).await;
        yield_to_event_loop().await;
    }
}
```

### Performance Metrics

**Target Performance:**
- File tree load: < 500ms for 10,000 files
- Node lookup: < 1ms
- Path finding: < 100ms for 1,000 nodes
- Graph layout: < 2s for 10,000 nodes
- Search: < 50ms for 10,000 entities

**Monitoring:**
```rust
use std::time::Instant;

let start = Instant::now();
let result = expensive_operation();
let duration = start.elapsed();

if duration > Duration::from_millis(100) {
    tracing::warn!("Slow operation: {:?}", duration);
}
```

## Examples

### Example 1: Creating and Displaying a File Tree

```rust
use descartes_agent_runner::file_tree_builder::FileTreeBuilder;
use descartes_gui::file_tree_view::*;

// Build file tree
let mut builder = FileTreeBuilder::new();
let file_tree = builder.scan_directory("/path/to/project")?;

// Create state
let mut state = FileTreeState::default();

// Load tree
update(&mut state, FileTreeMessage::TreeLoaded(file_tree));

// Render
let view = view(&state);
```

### Example 2: Generating Knowledge Graph

```rust
use descartes_agent_runner::knowledge_graph_overlay::*;

// Create overlay
let mut overlay = KnowledgeGraphOverlay::new()?;

// Generate graph
let mut file_tree = /* ... */;
let knowledge_graph = overlay.generate_and_link(&mut file_tree)?;

// Now file_tree has bidirectional links to knowledge_graph
```

### Example 3: Finding References

```rust
use descartes_agent_runner::knowledge_graph_overlay::KnowledgeGraphOverlay;

let overlay = KnowledgeGraphOverlay::new()?;
let graph = /* ... */;

// Find all references to a function
let references = overlay.find_references("my_function", &graph);

for reference in references {
    println!("Found in: {:?} at line {}",
        reference.file_path,
        reference.line_range.0
    );
}
```

### Example 4: Impact Analysis

```rust
use descartes_gui::knowledge_graph_panel::*;

let mut state = KnowledgeGraphPanelState::default();
state.graph = Some(knowledge_graph);

// Analyze impact of changing a function
update(&mut state, KnowledgeGraphMessage::AnalyzeImpact("func_id".to_string()));

// Check affected nodes
println!("Affected nodes: {}", state.impact_nodes.len());
for node_id in &state.impact_nodes {
    if let Some(node) = state.graph.as_ref()?.get_node(node_id) {
        println!("- {}: {}", node.content_type.as_str(), node.name);
    }
}
```

### Example 5: Custom Search

```rust
use descartes_agent_runner::knowledge_graph::*;

let graph = /* ... */;

// Find all test functions
let test_functions = graph.find_nodes(|node| {
    node.content_type == KnowledgeNodeType::Function &&
    node.name.starts_with("test_")
});

println!("Found {} test functions", test_functions.len());
```

### Example 6: Code Preview with Annotations

```rust
use descartes_gui::code_preview_panel::*;

let mut state = CodePreviewState::default();

// Load file
let path = PathBuf::from("src/main.rs");
let (lines, language) = load_file(path.clone()).await?;
update(&mut state, CodePreviewMessage::FileLoaded(path, lines, language));

// Add bookmark
update(&mut state, CodePreviewMessage::AddBookmark(10));

// Add annotation
update(&mut state, CodePreviewMessage::AddAnnotation(
    15,
    "TODO: Optimize this loop".to_string()
));

// Jump to line
update(&mut state, CodePreviewMessage::JumpToLine(10));
```

### Example 7: Graph Traversal

```rust
use descartes_agent_runner::knowledge_graph::*;

let graph = /* ... */;

// Find call chain from main to target
if let Some(main_node) = graph.get_node_by_name("main") {
    if let Some(target) = graph.get_node_by_name("target_function") {
        if let Some(path) = graph.find_path(&main_node.node_id, &target.node_id) {
            println!("Call path:");
            for node_id in path {
                if let Some(node) = graph.get_node(&node_id) {
                    println!("  -> {}", node.name);
                }
            }
        }
    }
}
```

### Example 8: Performance Monitoring

```rust
use std::time::Instant;

let graph = create_large_graph(10000, 5);

let start = Instant::now();
let results = graph.find_nodes(|n| n.name.contains("test"));
let duration = start.elapsed();

println!("Search took: {:?}", duration);
println!("Found: {} results", results.len());

assert!(duration < Duration::from_millis(50), "Search too slow");
```

## Best Practices

### State Management

1. **Keep State Minimal:**
   - Only store what can't be derived
   - Use computed properties when possible

2. **Immutable Updates:**
   - Clone before modifying
   - Use structural sharing

3. **Cache Invalidation:**
   - Clear caches on relevant changes
   - Track dependencies

### Error Handling

```rust
pub fn safe_operation(state: &mut State) -> Result<(), Error> {
    // Validate inputs
    if !is_valid(&state.input) {
        return Err(Error::InvalidInput);
    }

    // Perform operation with error handling
    let result = risky_operation()?;

    // Update state
    state.result = Some(result);

    Ok(())
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let mut state = State::default();

        update(&mut state, Message::Action);

        assert_eq!(state.field, expected_value);
    }
}
```

## Version History

- **1.0.0** (2025-11-24): Initial release
  - File tree view
  - Code preview panel
  - Knowledge graph panel
  - Basic interactive features

## Future Enhancements

- Plugin system
- Custom themes
- More layout algorithms
- Real-time collaboration
- AI-powered suggestions
- Code metrics visualization
- Dependency analysis tools

---

For user documentation, see [CONTEXT_BROWSER_GUIDE.md](CONTEXT_BROWSER_GUIDE.md).
