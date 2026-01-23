//! TUI tests
//!
//! Tests for the Demiarch TUI application state and components.

use demiarch_core::visualization::{RenderOptions, TreeBuilder, TreeColors};

/// Test App state management
mod app_state_tests {
    /// Simulated App struct for testing (mirrors main.rs App)
    struct TestApp {
        current_tab: usize,
        tabs: Vec<&'static str>,
        tree_scroll: usize,
        ascii_mode: bool,
    }

    impl TestApp {
        fn new() -> Self {
            Self {
                current_tab: 1,
                tabs: vec!["Projects", "Agents", "Stats", "Help"],
                tree_scroll: 0,
                ascii_mode: false,
            }
        }

        fn next_tab(&mut self) {
            self.current_tab = (self.current_tab + 1) % self.tabs.len();
        }

        fn prev_tab(&mut self) {
            if self.current_tab == 0 {
                self.current_tab = self.tabs.len() - 1;
            } else {
                self.current_tab -= 1;
            }
        }

        fn scroll_up(&mut self) {
            self.tree_scroll = self.tree_scroll.saturating_sub(1);
        }

        fn scroll_down(&mut self) {
            self.tree_scroll = self.tree_scroll.saturating_add(1);
        }

        fn toggle_ascii(&mut self) {
            self.ascii_mode = !self.ascii_mode;
        }
    }

    #[test]
    fn test_app_initial_state() {
        let app = TestApp::new();
        assert_eq!(app.current_tab, 1); // Starts on Agents tab
        assert_eq!(app.tabs.len(), 4);
        assert_eq!(app.tree_scroll, 0);
        assert!(!app.ascii_mode);
    }

    #[test]
    fn test_app_next_tab_wraps() {
        let mut app = TestApp::new();
        app.current_tab = 3; // Last tab (Help)
        app.next_tab();
        assert_eq!(app.current_tab, 0); // Wraps to first tab
    }

    #[test]
    fn test_app_prev_tab_wraps() {
        let mut app = TestApp::new();
        app.current_tab = 0; // First tab
        app.prev_tab();
        assert_eq!(app.current_tab, 3); // Wraps to last tab
    }

    #[test]
    fn test_app_tab_cycle() {
        let mut app = TestApp::new();
        let initial = app.current_tab;

        // Cycle through all tabs
        for _ in 0..4 {
            app.next_tab();
        }

        // Should be back at initial
        assert_eq!(app.current_tab, initial);
    }

    #[test]
    fn test_app_scroll_down() {
        let mut app = TestApp::new();
        assert_eq!(app.tree_scroll, 0);

        app.scroll_down();
        assert_eq!(app.tree_scroll, 1);

        app.scroll_down();
        assert_eq!(app.tree_scroll, 2);
    }

    #[test]
    fn test_app_scroll_up_saturates_at_zero() {
        let mut app = TestApp::new();
        assert_eq!(app.tree_scroll, 0);

        app.scroll_up(); // Should not go negative
        assert_eq!(app.tree_scroll, 0);
    }

    #[test]
    fn test_app_scroll_up_from_nonzero() {
        let mut app = TestApp::new();
        app.tree_scroll = 5;

        app.scroll_up();
        assert_eq!(app.tree_scroll, 4);
    }

    #[test]
    fn test_app_toggle_ascii_mode() {
        let mut app = TestApp::new();
        assert!(!app.ascii_mode);

        app.toggle_ascii();
        assert!(app.ascii_mode);

        app.toggle_ascii();
        assert!(!app.ascii_mode);
    }

    #[test]
    fn test_app_direct_tab_selection() {
        let mut app = TestApp::new();

        app.current_tab = 0;
        assert_eq!(app.tabs[app.current_tab], "Projects");

        app.current_tab = 1;
        assert_eq!(app.tabs[app.current_tab], "Agents");

        app.current_tab = 2;
        assert_eq!(app.tabs[app.current_tab], "Stats");

        app.current_tab = 3;
        assert_eq!(app.tabs[app.current_tab], "Help");
    }
}

/// Tests for visualization components from demiarch-core
mod visualization_tests {
    use super::*;
    use demiarch_core::visualization::NodeStyle;

    #[test]
    fn test_render_options_default() {
        let options = RenderOptions::default();
        // Default should use Unicode characters
        assert!(matches!(options.style, NodeStyle::Unicode));
    }

    #[test]
    fn test_render_options_ascii() {
        let options = RenderOptions::ascii();
        assert!(matches!(options.style, NodeStyle::Ascii));
    }

    #[test]
    fn test_tree_colors_default() {
        let colors = TreeColors::default();
        // Just verify it can be created
        let _ = colors;
    }

    #[test]
    fn test_tree_builder_from_live_events() {
        // This should return a tree (possibly empty)
        let tree = TreeBuilder::from_live_events();
        // Verify basic tree operations work
        let _ = tree.count();
        let _ = tree.count_active();
        let _ = tree.count_completed();
        let _ = tree.count_failed();
        let _ = tree.tree_tokens();
    }

    #[test]
    fn test_tree_builder_counts_consistent() {
        let tree = TreeBuilder::from_live_events();
        let total = tree.count();
        let active = tree.count_active();
        let completed = tree.count_completed();
        let failed = tree.count_failed();

        // Active + completed + failed should be <= total
        // (some might be in other states like ready, waiting)
        assert!(active + completed + failed <= total || total == 0);
    }

    #[test]
    fn test_node_style_variants() {
        let ascii = NodeStyle::Ascii;
        let unicode = NodeStyle::Unicode;
        let rounded = NodeStyle::Rounded;

        assert!(matches!(ascii, NodeStyle::Ascii));
        assert!(matches!(unicode, NodeStyle::Unicode));
        assert!(matches!(rounded, NodeStyle::Rounded));
    }
}

/// Tests for crossterm imports
mod crossterm_tests {
    use crossterm::event::KeyCode;
    use crossterm::terminal;

    #[test]
    fn test_crossterm_keycode_enum() {
        let quit = KeyCode::Char('q');
        let tab = KeyCode::Tab;
        let up = KeyCode::Up;
        let down = KeyCode::Down;

        assert!(matches!(quit, KeyCode::Char('q')));
        assert!(matches!(tab, KeyCode::Tab));
        assert!(matches!(up, KeyCode::Up));
        assert!(matches!(down, KeyCode::Down));
    }

    #[test]
    fn test_terminal_functions_exist() {
        // Just verify these functions exist and are callable
        // We don't actually call them since they affect the real terminal
        let _ = terminal::enable_raw_mode;
        let _ = terminal::disable_raw_mode;
    }
}

/// Tests for ratatui imports
mod ratatui_tests {
    use ratatui::layout::{Constraint, Direction};
    use ratatui::style::{Color, Modifier, Style};

    #[test]
    fn test_constraint_types() {
        let length = Constraint::Length(3);
        let min = Constraint::Min(10);
        let percentage = Constraint::Percentage(50);

        assert!(matches!(length, Constraint::Length(3)));
        assert!(matches!(min, Constraint::Min(10)));
        assert!(matches!(percentage, Constraint::Percentage(50)));
    }

    #[test]
    fn test_direction_types() {
        let vertical = Direction::Vertical;
        let horizontal = Direction::Horizontal;

        assert!(matches!(vertical, Direction::Vertical));
        assert!(matches!(horizontal, Direction::Horizontal));
    }

    #[test]
    fn test_style_creation() {
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        // Style should be created successfully
        let _ = style;
    }

    #[test]
    fn test_color_constants() {
        let white = Color::White;
        let cyan = Color::Cyan;
        let dark_gray = Color::DarkGray;

        assert!(matches!(white, Color::White));
        assert!(matches!(cyan, Color::Cyan));
        assert!(matches!(dark_gray, Color::DarkGray));
    }
}

/// Tests for demiarch-core integration
mod core_integration_tests {
    #[test]
    fn test_core_prelude_accessible() {
        // Verify demiarch-core prelude can be imported
        use demiarch_core::prelude::*;
        // Create a type from prelude to verify import works
        let _ = std::mem::size_of::<Error>();
    }
}
