//! Application state and logic.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::text::Line;
use ratatui_textarea::TextArea;
use tui_tree_widget::{TreeItem, TreeState};

use crate::file_tree;

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

/// Top-level application state.
pub struct App {
    pub(crate) tree_state: TreeState<String>,
    pub(crate) tree_items: Vec<TreeItem<'static, String>>,
    pub(crate) path_map: HashMap<String, (PathBuf, bool)>,
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) file_content: String,
    pub(crate) rendered_lines: Vec<Line<'static>>,
    pub(crate) scroll_offset: usize,
    pub(crate) viewport_height: usize,
    pub(crate) mode: AppMode,
    pub(crate) focus: Focus,
    pub should_quit: bool,
    pub(crate) status_message: String,
    pub(crate) pending_key: Option<(char, Instant)>,
    pub(crate) command_buffer: String,
    pub(crate) textarea: Option<TextArea<'static>>,
    pub(crate) is_dirty: bool,
    pub(crate) search_active: bool,
    pub(crate) search_query: String,
    pub(crate) search_matches: Vec<usize>,
    pub(crate) search_current: usize,
    pub(crate) filtered_tree_items: Option<Vec<TreeItem<'static, String>>>,
    pub(crate) filtered_path_map: Option<HashMap<String, (PathBuf, bool)>>,
    pub(crate) show_help: bool,
    pub(crate) show_file_tree: bool,
}

impl App {
    /// Create a new `App` rooted at `path`.
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let (tree_items, path_map) = file_tree::build_tree_items(&path)?;
        let tree_state = TreeState::default();
        Ok(Self {
            tree_state,
            tree_items,
            path_map,
            current_file: None,
            file_content: String::new(),
            rendered_lines: Vec::new(),
            scroll_offset: 0,
            viewport_height: 0,
            mode: AppMode::Normal,
            focus: Focus::FileList,
            should_quit: false,
            status_message: String::new(),
            pending_key: None,
            command_buffer: String::new(),
            textarea: None,
            is_dirty: false,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
            filtered_tree_items: None,
            filtered_path_map: None,
            show_help: false,
            show_file_tree: true,
        })
    }

    /// Dispatch an event based on current mode and focus.
    pub fn handle_event(&mut self, event: Event) {
        // Only handle key press events (not release/repeat — Windows fires both).
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press {
            return;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── App::new ─────────────────────────────────────────────────────

    #[test]
    fn app_new_with_temp_dir() {
        let dir = std::env::temp_dir().join("mdt-test-app-new");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let app = App::new(dir.clone()).unwrap();
        assert!(!app.tree_items.is_empty());
        assert!(!app.path_map.is_empty());
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.focus, Focus::FileList);
        assert!(!app.should_quit);
        assert!(!app.show_help);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn app_new_empty_dir() {
        let dir = std::env::temp_dir().join("mdt-test-empty-dir");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let app = App::new(dir.clone()).unwrap();
        assert!(app.tree_items.is_empty());
        assert!(app.path_map.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Search state ─────────────────────────────────────────────────

    #[test]
    fn clear_search_resets_all_fields() {
        let dir = std::env::temp_dir().join("mdt-test-clear-search");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();
        app.search_active = true;
        app.search_query = "test".to_string();
        app.search_matches = vec![1, 5, 10];
        app.search_current = 2;
        app.status_message = "something".to_string();

        app.clear_search();

        assert!(!app.search_active);
        assert!(app.search_query.is_empty());
        assert!(app.search_matches.is_empty());
        assert_eq!(app.search_current, 0);
        assert!(app.filtered_tree_items.is_none());
        assert!(app.filtered_path_map.is_none());
        assert!(app.status_message.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── State machine transitions ──────────────────────────────────

    /// Helper: create a key press `Event` for use with `handle_event`.
    fn key_event(code: KeyCode) -> Event {
        use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn transition_normal_to_command_and_back() {
        let dir = std::env::temp_dir().join("mdt-test-cmd-transition");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        // ':' enters Command mode.
        app.handle_event(key_event(KeyCode::Char(':')));
        assert_eq!(app.mode, AppMode::Command);

        // Esc returns to Normal.
        app.handle_event(key_event(KeyCode::Esc));
        assert_eq!(app.mode, AppMode::Normal);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn transition_normal_to_search_and_back() {
        let dir = std::env::temp_dir().join("mdt-test-search-transition");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        // '/' enters Search mode and activates search.
        app.handle_event(key_event(KeyCode::Char('/')));
        assert_eq!(app.mode, AppMode::Search);
        assert!(app.search_active);

        // Esc returns to Normal and deactivates search.
        app.handle_event(key_event(KeyCode::Esc));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.search_active);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn help_toggle_on_and_off() {
        let dir = std::env::temp_dir().join("mdt-test-help-toggle");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();
        assert!(!app.show_help);

        // '?' toggles help on.
        app.handle_event(key_event(KeyCode::Char('?')));
        assert!(app.show_help);

        // '?' again toggles help off (while help is showing, '?' dismisses it).
        app.handle_event(key_event(KeyCode::Char('?')));
        assert!(!app.show_help);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn focus_toggle_cycles_between_panels() {
        let dir = std::env::temp_dir().join("mdt-test-focus-toggle");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();
        assert_eq!(app.focus, Focus::FileList);

        // Tab switches to Preview.
        app.handle_event(key_event(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Preview);

        // Tab switches back to FileList.
        app.handle_event(key_event(KeyCode::Tab));
        assert_eq!(app.focus, Focus::FileList);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_c_quits_from_any_mode() {
        use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

        let dir = std::env::temp_dir().join("mdt-test-ctrl-c-quit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut app = App::new(dir.clone()).unwrap();

        // Enter Command mode first.
        app.handle_event(key_event(KeyCode::Char(':')));
        assert_eq!(app.mode, AppMode::Command);

        // Ctrl+C quits even from Command mode.
        let ctrl_c = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        app.handle_event(ctrl_c);
        assert!(app.should_quit);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
