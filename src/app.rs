//! Application state and logic.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;
use ratatui_textarea::TextArea;
use tui_tree_widget::{TreeItem, TreeState};

use crate::file_tree;
use crate::markdown::render_markdown;

/// Current input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Insert,
    Command,
    Search,
}

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Command => write!(f, "COMMAND"),
            Self::Search => write!(f, "SEARCH"),
        }
    }
}

/// Which pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileList,
    Preview,
}

/// Search-related state (both file search and in-document search).
pub(crate) struct SearchState {
    pub(crate) active: bool,
    pub(crate) query: String,
    pub(crate) matches: Vec<usize>,
    pub(crate) current: usize,
}

/// Editor (TextArea) state.
pub(crate) struct EditorState {
    pub(crate) textarea: Option<TextArea<'static>>,
    pub(crate) is_dirty: bool,
}

/// File tree view state.
pub(crate) struct TreeViewState {
    pub(crate) tree_state: TreeState<String>,
    pub(crate) tree_items: Vec<TreeItem<'static, String>>,
    pub(crate) path_map: HashMap<String, (PathBuf, bool)>,
    pub(crate) filtered_tree_items: Option<Vec<TreeItem<'static, String>>>,
    pub(crate) filtered_path_map: Option<HashMap<String, (PathBuf, bool)>>,
}

/// Current document / preview state.
pub(crate) struct DocumentState {
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) file_content: String,
    pub(crate) rendered_lines: Vec<Line<'static>>,
    pub(crate) scroll_offset: usize,
    pub(crate) viewport_height: usize,
    pub(crate) viewport_width: usize,
}

impl DocumentState {
    pub(crate) fn scroll_down(&mut self) {
        if !self.rendered_lines.is_empty() {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.clamp_scroll();
        }
    }

    pub(crate) fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub(crate) fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    pub(crate) fn scroll_half_page_down(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_add(half.max(1));
        self.clamp_scroll();
    }

    pub(crate) fn scroll_half_page_up(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_sub(half.max(1));
    }

    pub(crate) fn max_scroll(&self) -> usize {
        self.rendered_lines.len().saturating_sub(self.viewport_height)
    }

    pub(crate) fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }
}

/// Top-level application state.
pub struct App {
    pub(crate) search: SearchState,
    pub(crate) editor: EditorState,
    pub(crate) tree: TreeViewState,
    pub(crate) document: DocumentState,
    pub(crate) mode: AppMode,
    pub(crate) focus: Focus,
    pub should_quit: bool,
    pub(crate) status_message: String,
    pub(crate) pending_key: Option<(char, Instant)>,
    pub(crate) command_buffer: String,
    pub(crate) show_help: bool,
    pub(crate) show_file_tree: bool,
    pub(crate) bg_color: ratatui::style::Color,
    pub(crate) root_path: PathBuf,
}

impl App {
    /// Create a new `App` rooted at `path`.
    pub fn new(path: &Path, bg_color: ratatui::style::Color) -> anyhow::Result<Self> {
        let (tree_items, path_map) = file_tree::build_tree_items(path)?;
        let root_path = std::fs::canonicalize(path)?;
        let mut tree_state = TreeState::default();
        if let Some(first_item) = tree_items.first() {
            tree_state.select(vec![first_item.identifier().clone()]);
        }
        Ok(Self {
            tree: TreeViewState {
                tree_state,
                tree_items,
                path_map,
                filtered_tree_items: None,
                filtered_path_map: None,
            },
            document: DocumentState {
                current_file: None,
                file_content: String::new(),
                rendered_lines: Vec::new(),
                scroll_offset: 0,
                viewport_height: 0,
                viewport_width: 0,
            },
            search: SearchState {
                active: false,
                query: String::new(),
                matches: Vec::new(),
                current: 0,
            },
            editor: EditorState { textarea: None, is_dirty: false },
            mode: AppMode::Normal,
            focus: Focus::FileList,
            should_quit: false,
            status_message: String::new(),
            pending_key: None,
            command_buffer: String::new(),
            show_help: false,
            show_file_tree: true,
            bg_color,
            root_path,
        })
    }

    /// Dispatch a key press based on current mode and focus.
    pub fn handle_event(&mut self, key: KeyEvent) {
        // Ctrl+C always quits regardless of mode.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.mode {
            AppMode::Normal => {
                // If help overlay is showing, Esc or ? dismisses it.
                if self.show_help {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
                        self.show_help = false;
                        return;
                    }
                    // Ignore other keys while help is showing.
                    return;
                }
                self.handle_normal_key(key);
            }
            AppMode::Insert => self.handle_insert_key(key),
            AppMode::Command => self.handle_command_key(key),
            AppMode::Search => self.handle_search_key(key),
        }
    }
}

impl App {
    /// Get the display path for the current file (relative to root).
    pub(crate) fn display_file_path(&self) -> String {
        self.document
            .current_file
            .as_ref()
            .map(|p| {
                p.strip_prefix(&self.root_path)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default()
    }
}

impl App {
    /// Read a file, render its markdown, and store the result.
    pub(crate) fn open_file(&mut self, path: &Path) {
        const MAX_FILE_SIZE: u64 = 5_000_000;
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > MAX_FILE_SIZE {
                self.status_message = "File too large (>5MB)".to_string();
                return;
            }
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                let rendered = render_markdown(&content, if self.document.viewport_width > 0 { Some(self.document.viewport_width) } else { None });
                self.document.rendered_lines = rendered.lines;
                self.document.file_content = content;
                self.document.current_file = Some(path.to_path_buf());
                self.document.scroll_offset = 0;
                self.status_message.clear();
            }
            Err(e) => {
                self.status_message = format!("Error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

    // ── App::new ─────────────────────────────────────────────────────

    #[test]
    fn app_new_with_temp_dir() {
        let dir = TempTestDir::new("mdt-test-app-new");
        dir.create_file("test.md", "# Test");

        let app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(!app.tree.tree_items.is_empty());
        assert!(!app.tree.path_map.is_empty());
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.focus, Focus::FileList);
        assert!(!app.should_quit);
        assert!(!app.show_help);
    }

    #[test]
    fn app_new_empty_dir() {
        let dir = TempTestDir::new("mdt-test-empty-dir");

        let app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(app.tree.tree_items.is_empty());
        assert!(app.tree.path_map.is_empty());
    }

    // ── Search state ─────────────────────────────────────────────────

    #[test]
    fn clear_search_resets_all_fields() {
        let dir = TempTestDir::new("mdt-test-clear-search");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.search.active = true;
        app.search.query = "test".to_string();
        app.search.matches = vec![1, 5, 10];
        app.search.current = 2;
        app.status_message = "something".to_string();

        app.clear_search();

        assert!(!app.search.active);
        assert!(app.search.query.is_empty());
        assert!(app.search.matches.is_empty());
        assert_eq!(app.search.current, 0);
        assert!(app.tree.filtered_tree_items.is_none());
        assert!(app.tree.filtered_path_map.is_none());
        assert!(app.status_message.is_empty());
    }

    // ── State machine transitions ──────────────────────────────────

    /// Helper: create a key press `KeyEvent` for use with `handle_event`.
    fn key_event(code: KeyCode) -> KeyEvent {
        use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn transition_normal_to_command_and_back() {
        let dir = TempTestDir::new("mdt-test-cmd-transition");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        // ':' enters Command mode.
        app.handle_event(key_event(KeyCode::Char(':')));
        assert_eq!(app.mode, AppMode::Command);

        // Esc returns to Normal.
        app.handle_event(key_event(KeyCode::Esc));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn transition_normal_to_search_and_back() {
        let dir = TempTestDir::new("mdt-test-search-transition");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        // '/' enters Search mode and activates search.
        app.handle_event(key_event(KeyCode::Char('/')));
        assert_eq!(app.mode, AppMode::Search);
        assert!(app.search.active);

        // Esc returns to Normal and deactivates search.
        app.handle_event(key_event(KeyCode::Esc));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.search.active);
    }

    #[test]
    fn help_toggle_on_and_off() {
        let dir = TempTestDir::new("mdt-test-help-toggle");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(!app.show_help);

        // '?' toggles help on.
        app.handle_event(key_event(KeyCode::Char('?')));
        assert!(app.show_help);

        // '?' again toggles help off (while help is showing, '?' dismisses it).
        app.handle_event(key_event(KeyCode::Char('?')));
        assert!(!app.show_help);
    }

    #[test]
    fn focus_toggle_cycles_between_panels() {
        let dir = TempTestDir::new("mdt-test-focus-toggle");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert_eq!(app.focus, Focus::FileList);

        // Tab switches to Preview.
        app.handle_event(key_event(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Preview);

        // Tab switches back to FileList.
        app.handle_event(key_event(KeyCode::Tab));
        assert_eq!(app.focus, Focus::FileList);
    }

    #[test]
    fn ctrl_c_quits_from_any_mode() {
        use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

        let dir = TempTestDir::new("mdt-test-ctrl-c-quit");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        // Enter Command mode first.
        app.handle_event(key_event(KeyCode::Char(':')));
        assert_eq!(app.mode, AppMode::Command);

        // Ctrl+C quits even from Command mode.
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        app.handle_event(ctrl_c);
        assert!(app.should_quit);
    }

    // ── Scroll (DocumentState) ──────────────────────────────────

    #[test]
    fn scroll_down_increments_offset() {
        let dir = TempTestDir::new("mdt-test-scroll-down");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("long.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("long.md"));
        app.document.viewport_height = 10;
        assert_eq!(app.document.scroll_offset, 0);

        app.document.scroll_down();

        assert_eq!(app.document.scroll_offset, 1);
    }

    #[test]
    fn scroll_half_page_down_moves_half_viewport() {
        let dir = TempTestDir::new("mdt-test-scroll-half-down");
        let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("long.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("long.md"));
        app.document.viewport_height = 20;

        app.document.scroll_half_page_down();

        assert_eq!(app.document.scroll_offset, 10);
    }

    #[test]
    fn scroll_to_top_resets_to_zero() {
        let dir = TempTestDir::new("mdt-test-scroll-top");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("long.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("long.md"));
        app.document.viewport_height = 10;
        app.document.scroll_offset = 15;

        app.document.scroll_to_top();

        assert_eq!(app.document.scroll_offset, 0);
    }

    #[test]
    fn scroll_to_bottom_sets_max_scroll() {
        let dir = TempTestDir::new("mdt-test-scroll-bottom");
        let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("long.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("long.md"));
        app.document.viewport_height = 10;

        app.document.scroll_to_bottom();

        let expected = app.document.rendered_lines.len().saturating_sub(10);
        assert_eq!(app.document.scroll_offset, expected);
        assert!(app.document.scroll_offset > 0);
    }

    // ── open_file ──────────────────────────────────────────────────

    #[test]
    fn open_file_rejects_large_files() {
        let dir = TempTestDir::new("mdt-test-open-file-large");
        // Create a file just over 5MB
        let big_path = dir.path().join("big.md");
        let data = vec![b'x'; 5_000_001];
        std::fs::write(&big_path, &data).unwrap();

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&big_path);

        assert_eq!(app.status_message, "File too large (>5MB)");
        assert!(app.document.current_file.is_none());
    }

    #[test]
    fn open_file_succeeds_for_small_file() {
        let dir = TempTestDir::new("mdt-test-open-file-small");
        dir.create_file("hello.md", "# Hello");
        let md_path = dir.path().join("hello.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&md_path);

        assert!(app.status_message.is_empty());
        assert_eq!(app.document.current_file, Some(md_path));
    }
}
