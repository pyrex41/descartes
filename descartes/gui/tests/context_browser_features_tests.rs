/// Context Browser Features Tests
///
/// Comprehensive test suite for the context browser interactive features including:
/// - Code preview panel functionality

#[cfg(test)]
mod code_preview_tests {
    use descartes_gui::code_preview_panel::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_file_message() {
        let mut state = CodePreviewState::default();
        let path = PathBuf::from("/test/file.rs");

        update(&mut state, CodePreviewMessage::LoadFile(path.clone()));

        assert_eq!(state.file_path, Some(path));
    }

    #[test]
    fn test_file_loaded_with_content() {
        let mut state = CodePreviewState::default();
        let path = PathBuf::from("/test/file.rs");
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];

        update(
            &mut state,
            CodePreviewMessage::FileLoaded(path.clone(), lines.clone(), Some("Rust".to_string())),
        );

        assert_eq!(state.file_path, Some(path));
        assert_eq!(state.lines.len(), 3);
        assert_eq!(state.language, Some("Rust".to_string()));
        assert_eq!(state.scroll_position, 0);
    }

    #[test]
    fn test_jump_to_line() {
        let mut state = CodePreviewState::default();
        state.lines = (1..=100).map(|i| format!("Line {}", i)).collect();

        update(&mut state, CodePreviewMessage::JumpToLine(50));

        assert_eq!(state.current_line, Some(50));
        assert!(state.scroll_position <= 50);
    }

    #[test]
    fn test_highlight_range() {
        let mut state = CodePreviewState::default();
        state.lines = (1..=100).map(|i| format!("Line {}", i)).collect();

        update(&mut state, CodePreviewMessage::HighlightRange(10, 20));

        assert_eq!(state.highlighted_ranges.len(), 1);
        assert_eq!(state.highlighted_ranges[0], (10, 20));
    }

    #[test]
    fn test_clear_highlights() {
        let mut state = CodePreviewState::default();
        state.highlighted_ranges.push((10, 20));
        state.highlighted_ranges.push((30, 40));

        update(&mut state, CodePreviewMessage::ClearHighlights);

        assert_eq!(state.highlighted_ranges.len(), 0);
    }

    #[test]
    fn test_search_in_file() {
        let mut state = CodePreviewState::default();
        state.lines = vec![
            "fn main() {".to_string(),
            "    let x = 42;".to_string(),
            "    println!(\"x = {}\", x);".to_string(),
            "}".to_string(),
        ];

        update(
            &mut state,
            CodePreviewMessage::SearchQueryChanged("println".to_string()),
        );

        assert_eq!(state.search_results, vec![3]);
        assert_eq!(state.current_line, Some(3));
    }

    #[test]
    fn test_search_navigation() {
        let mut state = CodePreviewState::default();
        state.lines = vec![
            "let x = 1;".to_string(),
            "let y = 2;".to_string(),
            "let z = 3;".to_string(),
        ];

        update(
            &mut state,
            CodePreviewMessage::SearchQueryChanged("let".to_string()),
        );
        assert_eq!(state.search_results.len(), 3);
        assert_eq!(state.search_index, 0);

        update(&mut state, CodePreviewMessage::NextSearchResult);
        assert_eq!(state.search_index, 1);

        update(&mut state, CodePreviewMessage::NextSearchResult);
        assert_eq!(state.search_index, 2);

        update(&mut state, CodePreviewMessage::NextSearchResult);
        assert_eq!(state.search_index, 0); // Wraps around

        update(&mut state, CodePreviewMessage::PreviousSearchResult);
        assert_eq!(state.search_index, 2); // Wraps backward
    }

    #[test]
    fn test_bookmarks() {
        let mut state = CodePreviewState::default();

        update(&mut state, CodePreviewMessage::AddBookmark(10));
        update(&mut state, CodePreviewMessage::AddBookmark(20));
        update(&mut state, CodePreviewMessage::AddBookmark(5));

        assert_eq!(state.bookmarks, vec![5, 10, 20]); // Should be sorted

        update(&mut state, CodePreviewMessage::RemoveBookmark(10));
        assert_eq!(state.bookmarks, vec![5, 20]);

        update(&mut state, CodePreviewMessage::ClearBookmarks);
        assert_eq!(state.bookmarks.len(), 0);
    }

    #[test]
    fn test_annotations() {
        let mut state = CodePreviewState::default();

        update(
            &mut state,
            CodePreviewMessage::AddAnnotation(5, "Important line".to_string()),
        );
        update(
            &mut state,
            CodePreviewMessage::AddAnnotation(10, "TODO: Fix this".to_string()),
        );

        assert_eq!(state.annotations.len(), 2);
        assert_eq!(
            state.annotations.get(&5),
            Some(&"Important line".to_string())
        );

        update(&mut state, CodePreviewMessage::RemoveAnnotation(5));
        assert_eq!(state.annotations.len(), 1);
    }

    #[test]
    fn test_view_mode_switching() {
        let mut state = CodePreviewState::default();

        assert_eq!(state.view_mode, ViewMode::Single);

        update(
            &mut state,
            CodePreviewMessage::SetViewMode(ViewMode::SideBySide),
        );
        assert_eq!(state.view_mode, ViewMode::SideBySide);

        update(
            &mut state,
            CodePreviewMessage::SetViewMode(ViewMode::UnifiedDiff),
        );
        assert_eq!(state.view_mode, ViewMode::UnifiedDiff);
    }

    #[test]
    fn test_toggle_options() {
        let mut state = CodePreviewState::default();

        assert_eq!(state.show_line_numbers, true);
        update(&mut state, CodePreviewMessage::ToggleLineNumbers);
        assert_eq!(state.show_line_numbers, false);

        assert_eq!(state.word_wrap, false);
        update(&mut state, CodePreviewMessage::ToggleWordWrap);
        assert_eq!(state.word_wrap, true);

        assert_eq!(state.show_whitespace, false);
        update(&mut state, CodePreviewMessage::ToggleWhitespace);
        assert_eq!(state.show_whitespace, true);

        assert_eq!(state.syntax_highlighting, true);
        update(&mut state, CodePreviewMessage::ToggleSyntaxHighlighting);
        assert_eq!(state.syntax_highlighting, false);
    }

    #[test]
    fn test_code_folding() {
        let mut state = CodePreviewState::default();

        update(&mut state, CodePreviewMessage::ToggleFold(5, 15));
        assert_eq!(state.folded_ranges.len(), 1);
        assert_eq!(state.folded_ranges[0], (5, 15));

        // Toggle again should unfold
        update(&mut state, CodePreviewMessage::ToggleFold(5, 15));
        assert_eq!(state.folded_ranges.len(), 0);
    }

    #[test]
    fn test_clear_resets_state() {
        let mut state = CodePreviewState::default();
        state.file_path = Some(PathBuf::from("/test.rs"));
        state.lines = vec!["test".to_string()];
        state.bookmarks.push(10);

        update(&mut state, CodePreviewMessage::Clear);

        assert_eq!(state.file_path, None);
        assert_eq!(state.lines.len(), 0);
        assert_eq!(state.bookmarks.len(), 0);
    }
}
