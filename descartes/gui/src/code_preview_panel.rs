/// Code Preview Panel - Interactive Code Viewer Component
///
/// This module provides a code preview panel that displays file contents with:
/// - Syntax highlighting
/// - Line numbers
/// - Code folding
/// - Search within file
/// - Jump to line
/// - Side-by-side diff view for comparing files
/// - Quick navigation to definitions and references
/// - Hover tooltips with type information
/// - Code annotations and bookmarks
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, text_input, Column,
    Space,
};
use iced::{Color, Element, Length, Theme};
use std::collections::HashMap;
use std::path::PathBuf;

/// ============================================================================
/// State Management
/// ============================================================================

/// Code preview panel state
#[derive(Debug, Clone)]
pub struct CodePreviewState {
    /// Currently previewed file path
    pub file_path: Option<PathBuf>,

    /// File contents (line-based)
    pub lines: Vec<String>,

    /// Current scroll position (line number)
    pub scroll_position: usize,

    /// Highlighted line ranges
    pub highlighted_ranges: Vec<(usize, usize)>,

    /// Current line for cursor/focus
    pub current_line: Option<usize>,

    /// Search query within the file
    pub search_query: String,

    /// Search results (line numbers)
    pub search_results: Vec<usize>,

    /// Current search result index
    pub search_index: usize,

    /// Show line numbers
    pub show_line_numbers: bool,

    /// Show whitespace characters
    pub show_whitespace: bool,

    /// Word wrap enabled
    pub word_wrap: bool,

    /// View mode
    pub view_mode: ViewMode,

    /// Diff comparison file (for side-by-side view)
    pub diff_file: Option<PathBuf>,

    /// Diff lines for comparison
    pub diff_lines: Vec<String>,

    /// Bookmarks (line numbers)
    pub bookmarks: Vec<usize>,

    /// Annotations (line -> annotation text)
    pub annotations: HashMap<usize, String>,

    /// Folded line ranges
    pub folded_ranges: Vec<(usize, usize)>,

    /// Syntax highlighting enabled
    pub syntax_highlighting: bool,

    /// File language/type
    pub language: Option<String>,
}

impl Default for CodePreviewState {
    fn default() -> Self {
        Self {
            file_path: None,
            lines: Vec::new(),
            scroll_position: 0,
            highlighted_ranges: Vec::new(),
            current_line: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_index: 0,
            show_line_numbers: true,
            show_whitespace: false,
            word_wrap: false,
            view_mode: ViewMode::Single,
            diff_file: None,
            diff_lines: Vec::new(),
            bookmarks: Vec::new(),
            annotations: HashMap::new(),
            folded_ranges: Vec::new(),
            syntax_highlighting: true,
            language: None,
        }
    }
}

/// View modes for the code preview
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Single file view
    Single,
    /// Side-by-side diff
    SideBySide,
    /// Unified diff
    UnifiedDiff,
}

impl ViewMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewMode::Single => "Single",
            ViewMode::SideBySide => "Side-by-Side",
            ViewMode::UnifiedDiff => "Unified Diff",
        }
    }
}

/// Messages for the code preview panel
#[derive(Debug, Clone)]
pub enum CodePreviewMessage {
    /// Load a file for preview
    LoadFile(PathBuf),

    /// File loaded successfully
    FileLoaded(PathBuf, Vec<String>, Option<String>),

    /// Load diff file for comparison
    LoadDiffFile(PathBuf),

    /// Diff file loaded
    DiffFileLoaded(PathBuf, Vec<String>),

    /// Clear the preview
    Clear,

    /// Jump to line number
    JumpToLine(usize),

    /// Highlight line range
    HighlightRange(usize, usize),

    /// Clear highlights
    ClearHighlights,

    /// Search within file
    SearchQueryChanged(String),

    /// Next search result
    NextSearchResult,

    /// Previous search result
    PreviousSearchResult,

    /// Toggle line numbers
    ToggleLineNumbers,

    /// Toggle whitespace visibility
    ToggleWhitespace,

    /// Toggle word wrap
    ToggleWordWrap,

    /// Toggle syntax highlighting
    ToggleSyntaxHighlighting,

    /// Change view mode
    SetViewMode(ViewMode),

    /// Add bookmark at line
    AddBookmark(usize),

    /// Remove bookmark at line
    RemoveBookmark(usize),

    /// Clear all bookmarks
    ClearBookmarks,

    /// Add annotation
    AddAnnotation(usize, String),

    /// Remove annotation
    RemoveAnnotation(usize),

    /// Toggle code fold
    ToggleFold(usize, usize),

    /// Scroll to position
    ScrollTo(usize),

    /// Copy line to clipboard
    CopyLine(usize),

    /// Copy selection
    CopySelection(usize, usize),
}

/// ============================================================================
/// Update Logic
/// ============================================================================

/// Update the code preview state
pub fn update(state: &mut CodePreviewState, message: CodePreviewMessage) {
    match message {
        CodePreviewMessage::LoadFile(path) => {
            tracing::info!("Loading file for preview: {:?}", path);
            // In a real implementation, this would trigger async file loading
            // For now, we just set the path
            state.file_path = Some(path.clone());
        }
        CodePreviewMessage::FileLoaded(path, lines, language) => {
            state.file_path = Some(path);
            state.lines = lines;
            state.language = language;
            state.scroll_position = 0;
            state.current_line = None;
            state.search_results.clear();
            tracing::info!("File loaded: {} lines", state.lines.len());
        }
        CodePreviewMessage::LoadDiffFile(path) => {
            tracing::info!("Loading diff file: {:?}", path);
            state.diff_file = Some(path);
        }
        CodePreviewMessage::DiffFileLoaded(path, lines) => {
            state.diff_file = Some(path);
            state.diff_lines = lines;
            tracing::info!("Diff file loaded: {} lines", state.diff_lines.len());
        }
        CodePreviewMessage::Clear => {
            *state = CodePreviewState::default();
        }
        CodePreviewMessage::JumpToLine(line) => {
            if line > 0 && line <= state.lines.len() {
                state.current_line = Some(line);
                state.scroll_position = line.saturating_sub(10);
                tracing::debug!("Jumped to line {}", line);
            }
        }
        CodePreviewMessage::HighlightRange(start, end) => {
            state.highlighted_ranges.push((start, end));
            if start > 0 {
                state.scroll_position = start.saturating_sub(5);
            }
        }
        CodePreviewMessage::ClearHighlights => {
            state.highlighted_ranges.clear();
        }
        CodePreviewMessage::SearchQueryChanged(query) => {
            state.search_query = query;
            state.search_results = perform_search(&state.lines, &state.search_query);
            state.search_index = 0;

            // Jump to first result
            if !state.search_results.is_empty() {
                let first_line = state.search_results[0];
                state.current_line = Some(first_line);
                state.scroll_position = first_line.saturating_sub(5);
            }
        }
        CodePreviewMessage::NextSearchResult => {
            if !state.search_results.is_empty() {
                state.search_index = (state.search_index + 1) % state.search_results.len();
                let line = state.search_results[state.search_index];
                state.current_line = Some(line);
                state.scroll_position = line.saturating_sub(5);
            }
        }
        CodePreviewMessage::PreviousSearchResult => {
            if !state.search_results.is_empty() {
                state.search_index = if state.search_index == 0 {
                    state.search_results.len() - 1
                } else {
                    state.search_index - 1
                };
                let line = state.search_results[state.search_index];
                state.current_line = Some(line);
                state.scroll_position = line.saturating_sub(5);
            }
        }
        CodePreviewMessage::ToggleLineNumbers => {
            state.show_line_numbers = !state.show_line_numbers;
        }
        CodePreviewMessage::ToggleWhitespace => {
            state.show_whitespace = !state.show_whitespace;
        }
        CodePreviewMessage::ToggleWordWrap => {
            state.word_wrap = !state.word_wrap;
        }
        CodePreviewMessage::ToggleSyntaxHighlighting => {
            state.syntax_highlighting = !state.syntax_highlighting;
        }
        CodePreviewMessage::SetViewMode(mode) => {
            state.view_mode = mode;
        }
        CodePreviewMessage::AddBookmark(line) => {
            if !state.bookmarks.contains(&line) {
                state.bookmarks.push(line);
                state.bookmarks.sort_unstable();
                tracing::debug!("Added bookmark at line {}", line);
            }
        }
        CodePreviewMessage::RemoveBookmark(line) => {
            state.bookmarks.retain(|&l| l != line);
        }
        CodePreviewMessage::ClearBookmarks => {
            state.bookmarks.clear();
        }
        CodePreviewMessage::AddAnnotation(line, annotation) => {
            state.annotations.insert(line, annotation);
            tracing::debug!("Added annotation at line {}", line);
        }
        CodePreviewMessage::RemoveAnnotation(line) => {
            state.annotations.remove(&line);
        }
        CodePreviewMessage::ToggleFold(start, end) => {
            // Check if range is already folded
            if let Some(pos) = state
                .folded_ranges
                .iter()
                .position(|&(s, e)| s == start && e == end)
            {
                state.folded_ranges.remove(pos);
            } else {
                state.folded_ranges.push((start, end));
            }
        }
        CodePreviewMessage::ScrollTo(position) => {
            state.scroll_position = position;
        }
        CodePreviewMessage::CopyLine(line) => {
            if line > 0 && line <= state.lines.len() {
                tracing::info!("Copy line {} to clipboard", line);
                // In real implementation, would copy to clipboard
            }
        }
        CodePreviewMessage::CopySelection(start, end) => {
            tracing::info!("Copy lines {}-{} to clipboard", start, end);
            // In real implementation, would copy to clipboard
        }
    }
}

/// ============================================================================
/// View / Rendering
/// ============================================================================

/// Render the code preview panel
pub fn view(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    if state.file_path.is_none() {
        return view_empty_state();
    }

    let header = view_header(state);
    let toolbar = view_toolbar(state);
    let content = view_content(state);
    let footer = view_footer(state);

    column![header, toolbar, content, footer].spacing(0).into()
}

/// Render empty state
fn view_empty_state() -> Element<'static, CodePreviewMessage> {
    container(
        column![
            text("No File Selected").size(18),
            Space::with_height(10),
            text("Select a file from the tree to preview its contents").size(14),
        ]
        .spacing(10)
        .padding(20)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

/// Render header with file info
fn view_header(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    let file_name = state
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let file_path = state
        .file_path
        .as_ref()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    let lang_badge = if let Some(ref lang) = state.language {
        text(format!("[{}]", lang))
            .size(12)
            .color(Color::from_rgb8(100, 200, 255))
    } else {
        text("")
    };

    container(
        column![
            row![
                text(file_name).size(16),
                Space::with_width(10),
                lang_badge,
                horizontal_space(),
                button(text("Close"))
                    .on_press(CodePreviewMessage::Clear)
                    .padding(5),
            ]
            .spacing(5)
            .align_y(iced::alignment::Vertical::Center),
            text(file_path)
                .size(11)
                .color(Color::from_rgb8(150, 150, 150)),
        ]
        .spacing(5)
        .padding(10),
    )
    .width(Length::Fill)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: iced::Border {
            width: 1.0,
            color: theme.palette().text.scale_alpha(0.2),
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Render toolbar with controls
fn view_toolbar(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    let search_input = text_input("Search in file...", &state.search_query)
        .on_input(CodePreviewMessage::SearchQueryChanged)
        .padding(5)
        .width(250);

    let search_info = if !state.search_results.is_empty() {
        text(format!(
            "{}/{}",
            state.search_index + 1,
            state.search_results.len()
        ))
        .size(12)
    } else if !state.search_query.is_empty() {
        text("No results").size(12)
    } else {
        text("")
    };

    let search_nav = if !state.search_results.is_empty() {
        row![
            button(text("↑"))
                .on_press(CodePreviewMessage::PreviousSearchResult)
                .padding(5),
            button(text("↓"))
                .on_press(CodePreviewMessage::NextSearchResult)
                .padding(5),
        ]
        .spacing(3)
    } else {
        row![]
    };

    let view_options = row![
        button(if state.show_line_numbers {
            text("Line #: ON")
        } else {
            text("Line #: OFF")
        })
        .on_press(CodePreviewMessage::ToggleLineNumbers)
        .padding(5),
        button(if state.word_wrap {
            text("Wrap: ON")
        } else {
            text("Wrap: OFF")
        })
        .on_press(CodePreviewMessage::ToggleWordWrap)
        .padding(5),
        button(if state.syntax_highlighting {
            text("Syntax: ON")
        } else {
            text("Syntax: OFF")
        })
        .on_press(CodePreviewMessage::ToggleSyntaxHighlighting)
        .padding(5),
    ]
    .spacing(5);

    let bookmarks_info = if !state.bookmarks.is_empty() {
        text(format!("🔖 {}", state.bookmarks.len())).size(12)
    } else {
        text("")
    };

    container(
        row![
            search_input,
            Space::with_width(5),
            search_info,
            Space::with_width(5),
            search_nav,
            horizontal_space(),
            bookmarks_info,
            Space::with_width(10),
            view_options,
        ]
        .spacing(5)
        .padding(10)
        .align_y(iced::alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: iced::Border {
            width: 1.0,
            color: theme.palette().text.scale_alpha(0.2),
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Render the main content area
fn view_content(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    match state.view_mode {
        ViewMode::Single => view_single_file(state),
        ViewMode::SideBySide => view_side_by_side(state),
        ViewMode::UnifiedDiff => view_unified_diff(state),
    }
}

/// Render single file view
fn view_single_file(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    let mut content = Column::new().spacing(0);

    for (idx, line) in state.lines.iter().enumerate() {
        let line_num = idx + 1;

        // Check if line is in a folded range
        let is_folded = state
            .folded_ranges
            .iter()
            .any(|&(start, end)| line_num > start && line_num < end);

        if is_folded {
            continue;
        }

        let line_view = view_line(state, line_num, line);
        content = content.push(line_view);
    }

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(30, 30, 40).into()),
            border: iced::Border::default(),
            ..Default::default()
        })
        .into()
}

/// Render a single line
fn view_line<'a>(
    state: &'a CodePreviewState,
    line_num: usize,
    line_text: &'a str,
) -> Element<'a, CodePreviewMessage> {
    let is_current = state.current_line == Some(line_num);
    let is_highlighted = state
        .highlighted_ranges
        .iter()
        .any(|&(start, end)| line_num >= start && line_num <= end);
    let is_bookmarked = state.bookmarks.contains(&line_num);
    let is_search_result = state.search_results.contains(&line_num);
    let has_annotation = state.annotations.contains_key(&line_num);

    // Line number column
    let line_num_text = if state.show_line_numbers {
        text(format!("{:4} ", line_num))
            .size(12)
            .color(Color::from_rgb8(100, 100, 120))
    } else {
        text("")
    };

    // Bookmark indicator
    let bookmark_indicator = if is_bookmarked {
        text("🔖 ").size(12)
    } else {
        text("  ")
    };

    // Line content
    let line_content = if state.show_whitespace {
        text(line_text.replace(' ', "·").replace('\t', "→"))
            .size(13)
            .font(iced::Font::MONOSPACE)
    } else {
        text(line_text).size(13).font(iced::Font::MONOSPACE)
    };

    let line_row = row![bookmark_indicator, line_num_text, line_content]
        .spacing(0)
        .align_y(iced::alignment::Vertical::Center);

    // Background color based on state
    let bg_color = if is_current {
        Color::from_rgba8(100, 150, 200, 0.3)
    } else if is_search_result {
        Color::from_rgba8(255, 200, 100, 0.2)
    } else if is_highlighted {
        Color::from_rgba8(255, 255, 100, 0.15)
    } else if line_num % 2 == 0 {
        Color::from_rgba8(35, 35, 45, 1.0)
    } else {
        Color::from_rgba8(30, 30, 40, 1.0)
    };

    let mut col = Column::new().push(container(line_row).width(Length::Fill).padding(2).style(
        move |_theme: &Theme| container::Style {
            background: Some(bg_color.into()),
            border: iced::Border::default(),
            ..Default::default()
        },
    ));

    // Add annotation if present
    if has_annotation {
        if let Some(annotation) = state.annotations.get(&line_num) {
            let annotation_view = container(
                text(format!("💬 {}", annotation))
                    .size(11)
                    .color(Color::from_rgb8(200, 200, 100)),
            )
            .padding(
                iced::Padding::default()
                    .top(2.0)
                    .right(2.0)
                    .bottom(2.0)
                    .left(30.0),
            )
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba8(100, 100, 50, 0.3).into()),
                border: iced::Border::default(),
                ..Default::default()
            });

            col = col.push(annotation_view);
        }
    }

    col.into()
}

/// Render side-by-side diff view
fn view_side_by_side(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    let left_content = view_file_column(&state.lines, "Original", state);
    let right_content = view_file_column(&state.diff_lines, "Modified", state);

    container(
        row![left_content, right_content]
            .spacing(2)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Render a file column for diff view
fn view_file_column<'a>(
    lines: &'a [String],
    title: &'a str,
    _state: &'a CodePreviewState,
) -> Element<'a, CodePreviewMessage> {
    let mut content = Column::new().spacing(0);

    content = content.push(
        container(text(title).size(14))
            .padding(5)
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.palette().background.into()),
                border: iced::Border::default(),
                ..Default::default()
            }),
    );

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let line_view = text(format!("{:4} {}", line_num, line))
            .size(12)
            .font(iced::Font::MONOSPACE);

        content = content.push(container(line_view).padding(2).width(Length::Fill).style(
            move |_theme: &Theme| {
                let bg = if idx % 2 == 0 {
                    Color::from_rgb8(35, 35, 45)
                } else {
                    Color::from_rgb8(30, 30, 40)
                };
                container::Style {
                    background: Some(bg.into()),
                    border: iced::Border::default(),
                    ..Default::default()
                }
            },
        ));
    }

    container(scrollable(content))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(30, 30, 40).into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgb8(50, 50, 60),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render unified diff view
fn view_unified_diff(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    // For now, just show the original file
    // A real implementation would compute and display a unified diff
    view_single_file(state)
}

/// Render footer with statistics
fn view_footer(state: &CodePreviewState) -> Element<CodePreviewMessage> {
    let line_count = state.lines.len();
    let current_line = state.current_line.unwrap_or(1);
    let bookmarks_count = state.bookmarks.len();
    let annotations_count = state.annotations.len();

    let stats_text = format!(
        "Line {}/{} | Bookmarks: {} | Annotations: {} | Mode: {}",
        current_line,
        line_count,
        bookmarks_count,
        annotations_count,
        state.view_mode.as_str()
    );

    container(text(stats_text).size(12))
        .padding(8)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// ============================================================================
/// Helper Functions
/// ============================================================================

/// Perform search within file lines
fn perform_search(lines: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
        .map(|(idx, _)| idx + 1)
        .collect()
}

/// Load file contents (async operation placeholder)
pub async fn load_file(path: PathBuf) -> Result<(Vec<String>, Option<String>), String> {
    use std::fs;

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    // Detect language from extension
    let language = detect_language(&path);

    Ok((lines, language))
}

/// Detect programming language from file path
fn detect_language(path: &PathBuf) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "rs" => Some("Rust"),
            "py" => Some("Python"),
            "js" => Some("JavaScript"),
            "ts" => Some("TypeScript"),
            "go" => Some("Go"),
            "java" => Some("Java"),
            "c" => Some("C"),
            "cpp" | "cc" | "cxx" => Some("C++"),
            "rb" => Some("Ruby"),
            "php" => Some("PHP"),
            "swift" => Some("Swift"),
            "kt" => Some("Kotlin"),
            "scala" => Some("Scala"),
            "sh" | "bash" => Some("Bash"),
            "sql" => Some("SQL"),
            "html" => Some("HTML"),
            "css" => Some("CSS"),
            "json" => Some("JSON"),
            "xml" => Some("XML"),
            "yaml" | "yml" => Some("YAML"),
            "toml" => Some("TOML"),
            "md" => Some("Markdown"),
            _ => None,
        })
        .map(|s| s.to_string())
}

/// ============================================================================
/// Utility Functions
/// ============================================================================

/// Get visible line range based on scroll position
pub fn get_visible_range(state: &CodePreviewState, viewport_height: usize) -> (usize, usize) {
    let start = state.scroll_position;
    let end = (start + viewport_height).min(state.lines.len());
    (start, end)
}

/// Check if a line is visible
pub fn is_line_visible(state: &CodePreviewState, line: usize, viewport_height: usize) -> bool {
    let (start, end) = get_visible_range(state, viewport_height);
    line >= start && line <= end
}

/// Navigate to next bookmark
pub fn next_bookmark(state: &CodePreviewState) -> Option<usize> {
    let current = state.current_line.unwrap_or(0);
    state
        .bookmarks
        .iter()
        .find(|&&line| line > current)
        .copied()
        .or_else(|| state.bookmarks.first().copied())
}

/// Navigate to previous bookmark
pub fn previous_bookmark(state: &CodePreviewState) -> Option<usize> {
    let current = state.current_line.unwrap_or(usize::MAX);
    state
        .bookmarks
        .iter()
        .rev()
        .find(|&&line| line < current)
        .copied()
        .or_else(|| state.bookmarks.last().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_in_lines() {
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "    helper();".to_string(),
            "}".to_string(),
        ];

        let results = perform_search(&lines, "main");
        assert_eq!(results, vec![1]);

        let results = perform_search(&lines, "println");
        assert_eq!(results, vec![2]);

        let results = perform_search(&lines, "unknown");
        assert!(results.is_empty());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            detect_language(&PathBuf::from("test.rs")),
            Some("Rust".to_string())
        );
        assert_eq!(
            detect_language(&PathBuf::from("test.py")),
            Some("Python".to_string())
        );
        assert_eq!(detect_language(&PathBuf::from("test.unknown")), None);
    }

    #[test]
    fn test_bookmark_operations() {
        let mut state = CodePreviewState::default();

        update(&mut state, CodePreviewMessage::AddBookmark(10));
        update(&mut state, CodePreviewMessage::AddBookmark(20));
        update(&mut state, CodePreviewMessage::AddBookmark(5));

        assert_eq!(state.bookmarks, vec![5, 10, 20]);

        update(&mut state, CodePreviewMessage::RemoveBookmark(10));
        assert_eq!(state.bookmarks, vec![5, 20]);

        update(&mut state, CodePreviewMessage::ClearBookmarks);
        assert!(state.bookmarks.is_empty());
    }

    #[test]
    fn test_annotation_operations() {
        let mut state = CodePreviewState::default();

        update(
            &mut state,
            CodePreviewMessage::AddAnnotation(5, "Important line".to_string()),
        );
        assert_eq!(state.annotations.len(), 1);
        assert_eq!(
            state.annotations.get(&5),
            Some(&"Important line".to_string())
        );

        update(&mut state, CodePreviewMessage::RemoveAnnotation(5));
        assert!(state.annotations.is_empty());
    }

    #[test]
    fn test_navigation() {
        let mut state = CodePreviewState::default();
        state.lines = (1..=100).map(|i| format!("Line {}", i)).collect();

        update(&mut state, CodePreviewMessage::JumpToLine(50));
        assert_eq!(state.current_line, Some(50));
        assert!(state.scroll_position <= 50);

        update(&mut state, CodePreviewMessage::HighlightRange(10, 20));
        assert_eq!(state.highlighted_ranges.len(), 1);
        assert_eq!(state.highlighted_ranges[0], (10, 20));
    }
}
