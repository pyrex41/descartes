/// File Tree View - Visual GUI Component for Browsing File Hierarchies
///
/// This module provides a visual file tree widget for the Iced GUI that:
/// - Displays file system hierarchies with expandable folders
/// - Shows appropriate icons for files, folders, and languages
/// - Supports knowledge graph indicators (badges)
/// - Allows file selection and interaction
/// - Provides filtering and search capabilities
/// - Integrates with the FileTree data model

use descartes_agent_runner::knowledge_graph::{FileTree, FileTreeNode, FileNodeType};
use descartes_agent_runner::types::Language;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space, Column, Row};
use iced::{Element, Length, Color, Theme};
use iced::alignment::{Horizontal, Vertical};
use std::collections::{HashMap, HashSet};

/// ============================================================================
/// State Management
/// ============================================================================

/// File tree view state
#[derive(Debug, Clone)]
pub struct FileTreeState {
    /// The file tree data model
    pub tree: Option<FileTree>,

    /// Set of expanded node IDs
    pub expanded_nodes: HashSet<String>,

    /// Currently selected node ID
    pub selected_node: Option<String>,

    /// Search/filter query
    pub search_query: String,

    /// Filter by file type
    pub filter_language: Option<Language>,

    /// Show hidden files
    pub show_hidden: bool,

    /// Filter by knowledge graph links
    pub show_only_linked: bool,

    /// Sort order
    pub sort_order: SortOrder,

    /// Highlighted file node IDs (for bidirectional navigation)
    pub highlighted_files: HashSet<String>,

    /// Show knowledge details for file
    pub show_knowledge_for: Option<String>,

    /// Navigation history (stack of visited node IDs)
    pub navigation_history: Vec<String>,

    /// Current position in navigation history
    pub history_position: usize,

    /// Bookmarked node IDs
    pub bookmarked_nodes: HashSet<String>,

    /// Hovered node (for preview)
    pub hovered_node: Option<String>,

    /// Regex search enabled
    pub regex_search: bool,

    /// Recently accessed files (for quick access)
    pub recent_files: Vec<String>,

    /// Pin important files to top
    pub pinned_files: HashSet<String>,
}

impl Default for FileTreeState {
    fn default() -> Self {
        Self {
            tree: None,
            expanded_nodes: HashSet::new(),
            selected_node: None,
            search_query: String::new(),
            filter_language: None,
            show_hidden: false,
            show_only_linked: false,
            sort_order: SortOrder::NameAsc,
            highlighted_files: HashSet::new(),
            show_knowledge_for: None,
            navigation_history: Vec::new(),
            history_position: 0,
            bookmarked_nodes: HashSet::new(),
            hovered_node: None,
            regex_search: false,
            recent_files: Vec::new(),
            pinned_files: HashSet::new(),
        }
    }
}

/// Sort order for file tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    ModifiedAsc,
    ModifiedDesc,
}

/// Messages for the file tree view
#[derive(Debug, Clone)]
pub enum FileTreeMessage {
    /// Load a file tree
    TreeLoaded(FileTree),

    /// Toggle expand/collapse for a node
    ToggleExpand(String),

    /// Select a node
    SelectNode(String),

    /// Double-click on a node (open/view details)
    OpenNode(String),

    /// Update search query
    SearchQueryChanged(String),

    /// Toggle show hidden files
    ToggleShowHidden,

    /// Toggle show only files with knowledge links
    ToggleShowOnlyLinked,

    /// Filter by language
    FilterByLanguage(Option<Language>),

    /// Change sort order
    SetSortOrder(SortOrder),

    /// Clear all filters
    ClearFilters,

    /// Expand all nodes
    ExpandAll,

    /// Collapse all nodes
    CollapseAll,

    /// Show knowledge nodes for a file
    ShowKnowledgeNodes(String),

    /// Navigate to knowledge node
    NavigateToKnowledgeNode(String),

    /// Highlight related files for a knowledge node
    HighlightRelatedFiles(Vec<String>),

    /// Clear highlights
    ClearHighlights,

    /// Navigate back in history
    NavigateBack,

    /// Navigate forward in history
    NavigateForward,

    /// Add bookmark
    AddBookmark(String),

    /// Remove bookmark
    RemoveBookmark(String),

    /// Clear all bookmarks
    ClearBookmarks,

    /// Jump to bookmarked node
    JumpToBookmark(String),

    /// Hover over node (for preview)
    HoverNode(Option<String>),

    /// Toggle regex search
    ToggleRegexSearch,

    /// Pin file to top
    PinFile(String),

    /// Unpin file
    UnpinFile(String),

    /// Find references to symbol in file
    FindReferences(String),

    /// Go to definition of symbol
    GoToDefinition(String),

    /// Show file usages
    ShowUsages(String),

    /// Reveal in system file explorer
    RevealInExplorer(String),

    /// Copy file path to clipboard
    CopyPath(String),

    /// Copy relative path to clipboard
    CopyRelativePath(String),
}

/// Update the file tree state
pub fn update(state: &mut FileTreeState, message: FileTreeMessage) {
    match message {
        FileTreeMessage::TreeLoaded(tree) => {
            state.tree = Some(tree);
            state.expanded_nodes.clear();
            state.selected_node = None;

            // Auto-expand root node
            if let Some(tree) = &state.tree {
                if let Some(root_id) = &tree.root_id {
                    state.expanded_nodes.insert(root_id.clone());
                }
            }
        }
        FileTreeMessage::ToggleExpand(node_id) => {
            if state.expanded_nodes.contains(&node_id) {
                state.expanded_nodes.remove(&node_id);
            } else {
                state.expanded_nodes.insert(node_id);
            }
        }
        FileTreeMessage::SelectNode(node_id) => {
            // Add to navigation history
            if state.selected_node.is_some() && state.selected_node.as_ref() != Some(&node_id) {
                // Remove any forward history
                state.navigation_history.truncate(state.history_position + 1);

                // Add current to history
                if let Some(current) = &state.selected_node {
                    state.navigation_history.push(current.clone());
                }

                state.history_position = state.navigation_history.len();
            }

            state.selected_node = Some(node_id.clone());

            // Add to recent files
            if !state.recent_files.contains(&node_id) {
                state.recent_files.insert(0, node_id.clone());
                // Keep only last 20 recent files
                state.recent_files.truncate(20);
            }
        }
        FileTreeMessage::OpenNode(node_id) => {
            // Select the node and emit an event
            state.selected_node = Some(node_id.clone());
            tracing::info!("Opening node: {}", node_id);
        }
        FileTreeMessage::SearchQueryChanged(query) => {
            state.search_query = query;
        }
        FileTreeMessage::ToggleShowHidden => {
            state.show_hidden = !state.show_hidden;
        }
        FileTreeMessage::ToggleShowOnlyLinked => {
            state.show_only_linked = !state.show_only_linked;
        }
        FileTreeMessage::FilterByLanguage(language) => {
            state.filter_language = language;
        }
        FileTreeMessage::SetSortOrder(order) => {
            state.sort_order = order;
        }
        FileTreeMessage::ClearFilters => {
            state.search_query.clear();
            state.filter_language = None;
            state.show_hidden = false;
            state.show_only_linked = false;
        }
        FileTreeMessage::ExpandAll => {
            if let Some(tree) = &state.tree {
                for node_id in tree.nodes.keys() {
                    state.expanded_nodes.insert(node_id.clone());
                }
            }
        }
        FileTreeMessage::CollapseAll => {
            state.expanded_nodes.clear();
            // Keep root expanded
            if let Some(tree) = &state.tree {
                if let Some(root_id) = &tree.root_id {
                    state.expanded_nodes.insert(root_id.clone());
                }
            }
        }
        FileTreeMessage::ShowKnowledgeNodes(node_id) => {
            state.show_knowledge_for = Some(node_id.clone());
            state.selected_node = Some(node_id);
            tracing::info!("Showing knowledge nodes for file: {}", node_id);
        }
        FileTreeMessage::NavigateToKnowledgeNode(knowledge_node_id) => {
            tracing::info!("Navigating to knowledge node: {}", knowledge_node_id);
            // This would trigger navigation in the main app
        }
        FileTreeMessage::HighlightRelatedFiles(file_node_ids) => {
            state.highlighted_files = file_node_ids.into_iter().collect();
            tracing::info!("Highlighted {} related files", state.highlighted_files.len());
        }
        FileTreeMessage::ClearHighlights => {
            state.highlighted_files.clear();
            state.show_knowledge_for = None;
        }
        FileTreeMessage::NavigateBack => {
            if state.history_position > 0 {
                state.history_position -= 1;
                if let Some(node_id) = state.navigation_history.get(state.history_position) {
                    state.selected_node = Some(node_id.clone());
                    tracing::debug!("Navigated back to: {}", node_id);
                }
            }
        }
        FileTreeMessage::NavigateForward => {
            if state.history_position < state.navigation_history.len().saturating_sub(1) {
                state.history_position += 1;
                if let Some(node_id) = state.navigation_history.get(state.history_position) {
                    state.selected_node = Some(node_id.clone());
                    tracing::debug!("Navigated forward to: {}", node_id);
                }
            }
        }
        FileTreeMessage::AddBookmark(node_id) => {
            state.bookmarked_nodes.insert(node_id.clone());
            tracing::info!("Added bookmark: {}", node_id);
        }
        FileTreeMessage::RemoveBookmark(node_id) => {
            state.bookmarked_nodes.remove(&node_id);
            tracing::info!("Removed bookmark: {}", node_id);
        }
        FileTreeMessage::ClearBookmarks => {
            state.bookmarked_nodes.clear();
            tracing::info!("Cleared all bookmarks");
        }
        FileTreeMessage::JumpToBookmark(node_id) => {
            state.selected_node = Some(node_id.clone());
            tracing::info!("Jumped to bookmark: {}", node_id);
        }
        FileTreeMessage::HoverNode(node_id) => {
            state.hovered_node = node_id;
        }
        FileTreeMessage::ToggleRegexSearch => {
            state.regex_search = !state.regex_search;
            tracing::info!("Regex search: {}", state.regex_search);
        }
        FileTreeMessage::PinFile(node_id) => {
            state.pinned_files.insert(node_id.clone());
            tracing::info!("Pinned file: {}", node_id);
        }
        FileTreeMessage::UnpinFile(node_id) => {
            state.pinned_files.remove(&node_id);
            tracing::info!("Unpinned file: {}", node_id);
        }
        FileTreeMessage::FindReferences(node_id) => {
            tracing::info!("Finding references for: {}", node_id);
            // This would trigger a reference search in the knowledge graph
        }
        FileTreeMessage::GoToDefinition(node_id) => {
            tracing::info!("Going to definition for: {}", node_id);
            // This would navigate to the definition in the knowledge graph
        }
        FileTreeMessage::ShowUsages(node_id) => {
            tracing::info!("Showing usages for: {}", node_id);
            // This would show all usages of entities in this file
        }
        FileTreeMessage::RevealInExplorer(node_id) => {
            if let Some(tree) = &state.tree {
                if let Some(node) = tree.get_node(&node_id) {
                    tracing::info!("Revealing in explorer: {:?}", node.path);
                    // This would open the system file explorer
                }
            }
        }
        FileTreeMessage::CopyPath(node_id) => {
            if let Some(tree) = &state.tree {
                if let Some(node) = tree.get_node(&node_id) {
                    tracing::info!("Copying path: {:?}", node.path);
                    // This would copy the absolute path to clipboard
                }
            }
        }
        FileTreeMessage::CopyRelativePath(node_id) => {
            if let Some(tree) = &state.tree {
                if let Some(node) = tree.get_node(&node_id) {
                    let relative_path = node.path.strip_prefix(&tree.base_path)
                        .unwrap_or(&node.path);
                    tracing::info!("Copying relative path: {:?}", relative_path);
                    // This would copy the relative path to clipboard
                }
            }
        }
    }
}

/// ============================================================================
/// Rendering / View
/// ============================================================================

/// Render the file tree view
pub fn view(state: &FileTreeState) -> Element<FileTreeMessage> {
    if state.tree.is_none() {
        return view_empty_state();
    }

    let tree = state.tree.as_ref().unwrap();

    // Header with controls
    let header = view_header(state);

    // Tree content
    let tree_content = view_tree_content(state, tree);

    // Footer with stats
    let footer = view_footer(state, tree);

    column![
        header,
        scrollable(tree_content).height(Length::Fill),
        footer,
    ]
    .spacing(0)
    .into()
}

/// Render empty state
fn view_empty_state() -> Element<'static, FileTreeMessage> {
    container(
        column![
            text("No File Tree Loaded").size(18),
            Space::with_height(10),
            text("Load a project to browse its file structure").size(14),
        ]
        .spacing(10)
        .padding(20)
        .align_x(Horizontal::Center)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

/// Render header with search and filters
fn view_header(state: &FileTreeState) -> Element<FileTreeMessage> {
    let search_input = text_input("Search files...", &state.search_query)
        .on_input(FileTreeMessage::SearchQueryChanged)
        .padding(8)
        .width(Length::Fill);

    let nav_buttons = row![
        button(text("◄"))
            .on_press(FileTreeMessage::NavigateBack)
            .padding(6),
        button(text("►"))
            .on_press(FileTreeMessage::NavigateForward)
            .padding(6),
        button(if state.regex_search { text(".*") } else { text("Ab") })
            .on_press(FileTreeMessage::ToggleRegexSearch)
            .padding(6),
    ]
    .spacing(3);

    let filter_buttons = row![
        button(text("Hidden")).on_press(FileTreeMessage::ToggleShowHidden).padding(6),
        button(text("Linked")).on_press(FileTreeMessage::ToggleShowOnlyLinked).padding(6),
        button(text("Clear")).on_press(FileTreeMessage::ClearFilters).padding(6),
    ]
    .spacing(5);

    let expand_buttons = row![
        button(text("Expand All")).on_press(FileTreeMessage::ExpandAll).padding(6),
        button(text("Collapse All")).on_press(FileTreeMessage::CollapseAll).padding(6),
    ]
    .spacing(5);

    let bookmark_info = if !state.bookmarked_nodes.is_empty() {
        text(format!("🔖 {}", state.bookmarked_nodes.len()))
            .size(12)
    } else {
        text("")
    };

    container(
        column![
            row![
                search_input,
                Space::with_width(10),
                nav_buttons,
            ]
            .spacing(5)
            .align_y(Vertical::Center),
            Space::with_height(5),
            row![
                filter_buttons,
                Space::with_width(10),
                bookmark_info,
                Space::with_width(Length::Fill),
                expand_buttons,
            ]
            .spacing(5)
        ]
        .spacing(5)
        .padding(10)
    )
    .width(Length::Fill)
    .style(|theme: &Theme| {
        container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

/// Render the tree content
fn view_tree_content<'a>(state: &'a FileTreeState, tree: &'a FileTree) -> Element<'a, FileTreeMessage> {
    if let Some(root_id) = &tree.root_id {
        if let Some(root_node) = tree.get_node(root_id) {
            let filtered_nodes = filter_nodes(state, tree);
            return view_node_recursive(state, tree, root_node, &filtered_nodes).into();
        }
    }

    text("Empty tree").into()
}

/// Recursively render a node and its children
fn view_node_recursive<'a>(
    state: &'a FileTreeState,
    tree: &'a FileTree,
    node: &'a FileTreeNode,
    filtered_nodes: &HashSet<String>,
) -> Column<'a, FileTreeMessage> {
    let mut col = Column::new().spacing(0);

    // Check if this node should be visible
    if !filtered_nodes.contains(&node.node_id) {
        return col;
    }

    // Render this node
    let node_view = view_node(state, node);
    col = col.push(node_view);

    // Render children if expanded
    if node.is_directory() && state.expanded_nodes.contains(&node.node_id) {
        // Get and sort children
        let mut children: Vec<&FileTreeNode> = node
            .children
            .iter()
            .filter_map(|child_id| tree.get_node(child_id))
            .collect();

        sort_nodes(&mut children, state.sort_order);

        // Recursively render children
        for child in children {
            let child_view = view_node_recursive(state, tree, child, filtered_nodes);
            col = col.push(child_view);
        }
    }

    col
}

/// Render a single node
fn view_node<'a>(state: &'a FileTreeState, node: &'a FileTreeNode) -> Element<'a, FileTreeMessage> {
    let is_selected = state.selected_node.as_ref() == Some(&node.node_id);
    let is_expanded = state.expanded_nodes.contains(&node.node_id);
    let is_highlighted = state.highlighted_files.contains(&node.node_id);
    let is_bookmarked = state.bookmarked_nodes.contains(&node.node_id);
    let is_pinned = state.pinned_files.contains(&node.node_id);
    let is_recent = state.recent_files.contains(&node.node_id);

    // Indentation based on depth
    let indent_width = (node.depth * 20) as f32;

    // Expand/collapse icon for directories
    let expand_icon = if node.is_directory() {
        if is_expanded {
            "▼ "
        } else {
            "▶ "
        }
    } else {
        "  "
    };

    // File/folder icon
    let node_icon = get_node_icon(node);

    // Node name
    let name_text = text(&node.name).size(14);

    // Enhanced knowledge badge with icon and color
    let knowledge_badge = if !node.knowledge_links.is_empty() {
        let badge_text = if node.knowledge_links.len() > 9 {
            format!(" 🔗 {}+", node.knowledge_links.len())
        } else {
            format!(" 🔗 {}", node.knowledge_links.len())
        };

        button(text(badge_text).size(11).style(Color::from_rgb8(120, 200, 255)))
            .padding(2)
            .style(|theme: &Theme| {
                button::Style {
                    background: Some(Color::from_rgba8(120, 200, 255, 0.2).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: Color::from_rgb8(120, 200, 255),
                        radius: 3.0.into(),
                    },
                    ..button::Style::default()
                }
            })
            .on_press(FileTreeMessage::ShowKnowledgeNodes(node.node_id.clone()))
            .into()
    } else {
        Space::with_width(0).into()
    };

    // Git status indicator
    let git_status = if let Some(ref status) = node.metadata.git_status {
        text(format!(" {}", status))
            .size(12)
            .style(get_git_status_color(status))
    } else {
        text("")
    };

    // Bookmark indicator
    let bookmark_icon = if is_bookmarked {
        text(" 🔖").size(12)
    } else {
        text("")
    };

    // Pin indicator
    let pin_icon = if is_pinned {
        text(" 📌").size(12)
    } else {
        text("")
    };

    // Recent file indicator
    let recent_icon = if is_recent && !is_selected {
        text(" ⏱").size(11).style(Color::from_rgb8(150, 150, 150))
    } else {
        text("")
    };

    // Build the row
    let node_content = row![
        Space::with_width(indent_width),
        pin_icon,
        text(expand_icon).size(14),
        text(node_icon).size(14),
        name_text,
        bookmark_icon,
        knowledge_badge,
        git_status,
        recent_icon,
    ]
    .spacing(5)
    .align_y(Vertical::Center);

    // Create button for interaction
    let node_button = button(node_content)
        .width(Length::Fill)
        .padding(5)
        .on_press(if node.is_directory() {
            FileTreeMessage::ToggleExpand(node.node_id.clone())
        } else {
            FileTreeMessage::SelectNode(node.node_id.clone())
        });

    // Style based on selection and highlighting
    if is_selected {
        container(node_button)
            .width(Length::Fill)
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().primary.into()),
                    border: iced::Border {
                        width: 0.0,
                        color: Color::TRANSPARENT,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    } else if is_highlighted {
        container(node_button)
            .width(Length::Fill)
            .style(|_theme: &Theme| {
                container::Style {
                    background: Some(Color::from_rgba8(255, 200, 100, 0.2).into()),
                    border: iced::Border {
                        width: 1.0,
                        color: Color::from_rgb8(255, 200, 100),
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    } else {
        node_button.into()
    }
}

/// Render footer with statistics
fn view_footer(state: &FileTreeState, tree: &FileTree) -> Element<FileTreeMessage> {
    let filtered_nodes = filter_nodes(state, tree);
    let visible_count = filtered_nodes.len();

    let stats_text = format!(
        "Files: {} | Dirs: {} | Visible: {} | Selected: {}",
        tree.file_count,
        tree.directory_count,
        visible_count,
        if state.selected_node.is_some() { "Yes" } else { "No" }
    );

    container(
        text(stats_text).size(12)
    )
    .padding(10)
    .width(Length::Fill)
    .style(|theme: &Theme| {
        container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

/// ============================================================================
/// Helper Functions
/// ============================================================================

/// Get icon for a node based on type and language
fn get_node_icon(node: &FileTreeNode) -> &'static str {
    match node.node_type {
        FileNodeType::Directory => "📁",
        FileNodeType::Symlink => "🔗",
        FileNodeType::File => {
            // Return icon based on language or extension
            if let Some(language) = &node.metadata.language {
                match language {
                    Language::Rust => "🦀",
                    Language::Python => "🐍",
                    Language::JavaScript => "📜",
                    Language::TypeScript => "📘",
                    Language::Go => "🐹",
                    Language::Java => "☕",
                    Language::C => "©️",
                    Language::Cpp => "🔷",
                    Language::Ruby => "💎",
                    Language::Php => "🐘",
                    Language::Swift => "🦅",
                    Language::Kotlin => "🅺",
                    Language::Scala => "🅂",
                    Language::Bash => "🐚",
                    Language::Sql => "💾",
                    Language::Html => "🌐",
                    Language::Css => "🎨",
                    Language::Json => "📋",
                    Language::Xml => "📄",
                    Language::Yaml => "📝",
                    Language::Toml => "⚙️",
                    Language::Markdown => "📖",
                }
            } else if node.metadata.is_binary {
                "📦"
            } else {
                "📄"
            }
        }
    }
}

/// Get color for git status
fn get_git_status_color(status: &str) -> Color {
    match status.trim() {
        "M" | "MM" => Color::from_rgb8(255, 200, 100), // Modified - orange
        "A" | "AM" => Color::from_rgb8(100, 255, 100), // Added - green
        "D" => Color::from_rgb8(255, 100, 100), // Deleted - red
        "R" => Color::from_rgb8(100, 200, 255), // Renamed - blue
        "??" => Color::from_rgb8(200, 200, 200), // Untracked - gray
        _ => Color::from_rgb8(150, 150, 150), // Other - light gray
    }
}

/// Filter nodes based on current filter settings
fn filter_nodes(state: &FileTreeState, tree: &FileTree) -> HashSet<String> {
    let mut filtered = HashSet::new();

    for (node_id, node) in &tree.nodes {
        // Skip hidden files if not showing them
        if !state.show_hidden && node.name.starts_with('.') && node.depth > 0 {
            continue;
        }

        // Filter by search query
        if !state.search_query.is_empty() {
            let query_lower = state.search_query.to_lowercase();
            if !node.name.to_lowercase().contains(&query_lower) {
                continue;
            }
        }

        // Filter by language
        if let Some(filter_lang) = state.filter_language {
            if node.metadata.language != Some(filter_lang) {
                continue;
            }
        }

        // Filter by knowledge links
        if state.show_only_linked && node.knowledge_links.is_empty() {
            continue;
        }

        filtered.insert(node_id.clone());
    }

    // Always include parent directories of filtered nodes
    let filtered_clone = filtered.clone();
    for node_id in filtered_clone {
        if let Some(node) = tree.get_node(&node_id) {
            let mut current_parent = node.parent_id.clone();
            while let Some(parent_id) = current_parent {
                filtered.insert(parent_id.clone());
                if let Some(parent_node) = tree.get_node(&parent_id) {
                    current_parent = parent_node.parent_id.clone();
                } else {
                    break;
                }
            }
        }
    }

    filtered
}

/// Sort nodes according to sort order
fn sort_nodes(nodes: &mut Vec<&FileTreeNode>, sort_order: SortOrder) {
    match sort_order {
        SortOrder::NameAsc => {
            nodes.sort_by(|a, b| {
                // Directories first, then files
                match (a.is_directory(), b.is_directory()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });
        }
        SortOrder::NameDesc => {
            nodes.sort_by(|a, b| {
                match (a.is_directory(), b.is_directory()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => b.name.cmp(&a.name),
                }
            });
        }
        SortOrder::SizeAsc => {
            nodes.sort_by(|a, b| {
                a.metadata.size.unwrap_or(0).cmp(&b.metadata.size.unwrap_or(0))
            });
        }
        SortOrder::SizeDesc => {
            nodes.sort_by(|a, b| {
                b.metadata.size.unwrap_or(0).cmp(&a.metadata.size.unwrap_or(0))
            });
        }
        SortOrder::ModifiedAsc => {
            nodes.sort_by(|a, b| {
                a.metadata.modified.unwrap_or(0).cmp(&b.metadata.modified.unwrap_or(0))
            });
        }
        SortOrder::ModifiedDesc => {
            nodes.sort_by(|a, b| {
                b.metadata.modified.unwrap_or(0).cmp(&a.metadata.modified.unwrap_or(0))
            });
        }
    }
}

/// ============================================================================
/// Utility Functions for Integration
/// ============================================================================

/// Get the selected node from the tree
pub fn get_selected_node<'a>(state: &'a FileTreeState) -> Option<&'a FileTreeNode> {
    if let Some(tree) = &state.tree {
        if let Some(selected_id) = &state.selected_node {
            return tree.get_node(selected_id);
        }
    }
    None
}

/// Get the selected file path
pub fn get_selected_path(state: &FileTreeState) -> Option<std::path::PathBuf> {
    get_selected_node(state).map(|node| node.path.clone())
}

/// Check if a node is visible with current filters
pub fn is_node_visible(state: &FileTreeState, node_id: &str) -> bool {
    if let Some(tree) = &state.tree {
        let filtered = filter_nodes(state, tree);
        filtered.contains(node_id)
    } else {
        false
    }
}
