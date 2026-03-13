//! Application state and logic.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};
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

/// Top-level application state.
pub struct App {
    pub tree_state: TreeState<String>,
    pub tree_items: Vec<TreeItem<'static, String>>,
    pub path_map: HashMap<String, (PathBuf, bool)>,
    pub current_file: Option<PathBuf>,
    pub file_content: String,
    pub rendered_lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub mode: AppMode,
    pub focus: Focus,
    pub should_quit: bool,
    pub status_message: String,
    /// Pending key for composed commands like `gg`.
    pub pending_key: Option<(char, Instant)>,
    /// Buffer for command-mode input (e.g., `:q`).
    pub command_buffer: String,
    /// Active text editor (Some when in editor mode).
    pub textarea: Option<TextArea<'static>>,
    /// Whether the editor has unsaved changes.
    pub is_dirty: bool,
    /// Whether search input is active.
    pub search_active: bool,
    /// Current search query string.
    pub search_query: String,
    /// Line numbers containing matches (for in-document search).
    pub search_matches: Vec<usize>,
    /// Current match index (into search_matches).
    pub search_current: usize,
    /// Filtered tree items for file search (None = show all).
    pub filtered_tree_items: Option<Vec<TreeItem<'static, String>>>,
    /// Filtered path map for file search.
    pub filtered_path_map: Option<HashMap<String, (PathBuf, bool)>>,
    /// Whether the help overlay is shown.
    pub show_help: bool,
    /// Whether the file tree panel is visible.
    pub show_file_tree: bool,
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

    /// Handle key events in Insert mode — forward to TextArea.
    fn handle_insert_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            // Esc returns to Normal mode (stay in editor view).
            self.mode = AppMode::Normal;
            self.status_message.clear();
            return;
        }

        // Forward all other keys to the TextArea.
        if let Some(ref mut textarea) = self.textarea {
            let modified = textarea.input(key);
            if modified {
                self.is_dirty = true;
            }
        }
    }

    /// Handle key events in Normal mode.
    fn handle_normal_key(&mut self, key: KeyEvent) {
        // If we're in editor view (textarea is Some), handle editor normal-mode keys.
        if self.textarea.is_some() {
            self.handle_editor_normal_key(key);
            return;
        }

        // Check for composed commands (e.g., gg) — works in both FileList and Preview.
        if let Some((pending_char, instant)) = self.pending_key.take() {
            if instant.elapsed().as_millis() < 500 {
                match (pending_char, key.code) {
                    ('g', KeyCode::Char('g')) => {
                        match self.focus {
                            Focus::Preview => self.scroll_to_top(),
                            Focus::FileList => {
                                self.tree_state.select_first();
                            }
                        }
                        return;
                    }
                    (' ', KeyCode::Char('e')) => {
                        self.toggle_file_tree();
                        return;
                    }
                    _ => {} // expired or unrecognized — fall through
                }
            }
            // Pending key expired or didn't match — fall through to normal handling.
        }

        match key.code {
            // --- Navigation (focus-dependent) ---
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::FileList => {
                    self.tree_state.key_down();
                }
                Focus::Preview => self.scroll_down(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::FileList => {
                    self.tree_state.key_up();
                }
                Focus::Preview => self.scroll_up(),
            },
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Tab => self.toggle_focus(),

            // --- FileList-only navigation ---
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                if self.focus == Focus::FileList {
                    self.tree_state.key_left();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus == Focus::FileList {
                    self.tree_state.key_right();
                }
            }

            // --- G: last item (FileList) or scroll bottom (Preview) ---
            KeyCode::Char('G') => match self.focus {
                Focus::FileList => {
                    self.tree_state.select_last();
                }
                Focus::Preview => self.scroll_to_bottom(),
            },

            // --- g: start pending key for gg (both focuses) ---
            KeyCode::Char('g') => {
                self.pending_key = Some(('g', Instant::now()));
            }

            // --- Space: start pending key for Space+e (leader key) ---
            KeyCode::Char(' ') => {
                self.pending_key = Some((' ', Instant::now()));
            }

            // --- Preview-only scrolling ---
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == Focus::Preview {
                    self.scroll_half_page_down();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == Focus::Preview {
                    self.scroll_half_page_up();
                }
            }

            // --- Mode transitions ---
            KeyCode::Char(':') => {
                self.mode = AppMode::Command;
                self.command_buffer.clear();
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                self.search_matches.clear();
                self.search_current = 0;
                self.mode = AppMode::Search;
            }
            KeyCode::Char('n') => self.next_search_match(),
            KeyCode::Char('N') => self.prev_search_match(),
            KeyCode::Esc => {
                // Clear active search results
                self.clear_search();
            }
            KeyCode::Char('i') | KeyCode::Char('e') => {
                if self.focus == Focus::Preview {
                    self.enter_editor();
                }
            }

            // --- Quit ---
            // --- Help ---
            KeyCode::Char('?') => {
                if self.textarea.is_none() {
                    self.show_help = !self.show_help;
                }
            }

            KeyCode::Char('q') => self.should_quit = true,

            _ => {}
        }
    }

    /// Handle Normal-mode keys while in editor view (textarea is Some).
    fn handle_editor_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            // Enter Insert mode in editor.
            KeyCode::Char('i') => {
                self.mode = AppMode::Insert;
                self.status_message = "-- INSERT --".to_string();
            }
            // Enter Command mode.
            KeyCode::Char(':') => {
                self.mode = AppMode::Command;
                self.command_buffer.clear();
            }
            // Exit editor (with dirty-check warning).
            KeyCode::Esc => {
                if self.is_dirty {
                    self.status_message = "Unsaved changes! :w to save, :q! to discard".to_string();
                } else {
                    self.exit_editor();
                }
            }
            // Forward navigation keys to TextArea (h/j/k/l, arrows, etc.).
            _ => {
                if let Some(ref mut textarea) = self.textarea {
                    textarea.input(key);
                }
            }
        }
    }

    /// Enter the editor: create TextArea from current file content.
    fn enter_editor(&mut self) {
        if self.current_file.is_none() {
            self.status_message = "No file open".to_string();
            return;
        }

        let mut textarea = TextArea::from(self.file_content.lines());

        let title = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| format!(" Editor: {} ", n.to_string_lossy()))
            .unwrap_or_else(|| " Editor ".to_string());

        textarea.set_block(Block::default().title(title).borders(Borders::ALL));
        textarea.set_line_number_style(Style::default());

        self.textarea = Some(textarea);
        self.is_dirty = false;
        self.mode = AppMode::Insert;
        self.status_message = "-- INSERT --".to_string();
    }

    /// Exit the editor, returning to preview mode.
    fn exit_editor(&mut self) {
        self.textarea = None;
        self.is_dirty = false;
        self.mode = AppMode::Normal;
        self.scroll_offset = 0;
    }

    /// Save the editor content to disk, re-render markdown.
    fn save_editor(&mut self) -> bool {
        let Some(ref path) = self.current_file else {
            self.status_message = "No file path".to_string();
            return false;
        };
        let Some(ref textarea) = self.textarea else {
            self.status_message = "Not in editor".to_string();
            return false;
        };

        let content = textarea.lines().join("\n");
        let path = path.clone();

        match fs::write(&path, &content) {
            Ok(()) => {
                // Update stored content and re-render markdown preview.
                self.file_content = content;
                let rendered = render_markdown(&self.file_content);
                self.rendered_lines = rendered.lines;
                self.is_dirty = false;

                let name =
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                self.status_message = format!("\"{}\" written", name);
                true
            }
            Err(e) => {
                self.status_message = format!("Error saving: {e}");
                false
            }
        }
    }

    /// Handle key events in Command mode (`:` prefix).
    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.command_buffer.clear();
                self.status_message.clear();
            }
            KeyCode::Enter => {
                let cmd = self.command_buffer.trim().to_string();
                self.mode = AppMode::Normal;
                self.command_buffer.clear();
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    // Empty buffer after backspace — return to Normal.
                    self.mode = AppMode::Normal;
                    self.status_message.clear();
                }
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
    }

    /// Execute a command-mode command.
    fn execute_command(&mut self, cmd: &str) {
        let in_editor = self.textarea.is_some();

        match cmd {
            "q" | "quit" => {
                if in_editor {
                    if self.is_dirty {
                        self.status_message = "Unsaved changes! :q! to force quit".to_string();
                    } else {
                        self.exit_editor();
                    }
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                if in_editor {
                    self.exit_editor();
                } else {
                    self.should_quit = true;
                }
            }
            "w" | "write" => {
                if in_editor {
                    self.save_editor();
                } else {
                    self.status_message = "Not in editor".to_string();
                }
            }
            "wq" | "x" => {
                if in_editor {
                    if self.save_editor() {
                        self.exit_editor();
                    }
                } else {
                    self.status_message = "Not in editor".to_string();
                }
            }
            other => {
                self.status_message = format!("Unknown command: :{other}");
            }
        }
    }

    /// Open the selected file tree entry.
    fn handle_enter(&mut self) {
        if self.focus != Focus::FileList {
            return;
        }
        let selected: Vec<String> = self.tree_state.selected().to_vec();
        if selected.is_empty() {
            return;
        }
        let id = selected.last().unwrap();
        let info = self.path_map.get(id).cloned();
        if let Some((path, is_dir)) = info {
            if is_dir {
                self.tree_state.toggle(selected);
            } else {
                self.open_file(&path);
            }
        }
    }

    /// Read a file, render its markdown, and store the result.
    pub fn open_file(&mut self, path: &Path) {
        match fs::read_to_string(path) {
            Ok(content) => {
                let rendered = render_markdown(&content);
                self.rendered_lines = rendered.lines;
                self.file_content = content;
                self.current_file = Some(path.to_path_buf());
                self.scroll_offset = 0;
                self.status_message =
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            }
            Err(e) => {
                self.status_message = format!("Error: {e}");
            }
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileList => Focus::Preview,
            Focus::Preview => Focus::FileList,
        };
    }

    fn toggle_file_tree(&mut self) {
        self.show_file_tree = !self.show_file_tree;
        if self.show_file_tree {
            self.focus = Focus::FileList;
        } else {
            self.focus = Focus::Preview;
        }
    }

    fn scroll_down(&mut self) {
        if !self.rendered_lines.is_empty() {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.clamp_scroll();
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    fn scroll_half_page_down(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_add(half.max(1));
        self.clamp_scroll();
    }

    fn scroll_half_page_up(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_sub(half.max(1));
    }

    fn max_scroll(&self) -> usize {
        self.rendered_lines.len().saturating_sub(self.viewport_height)
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// Handle key events in Search mode (`/` prefix).
    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // Cancel search, restore full list.
                self.search_active = false;
                self.search_query.clear();
                self.search_matches.clear();
                self.search_current = 0;
                self.filtered_tree_items = None;
                self.filtered_path_map = None;
                self.mode = AppMode::Normal;
                self.status_message.clear();
            }
            KeyCode::Enter => {
                // Confirm search.
                self.search_active = false;
                if self.focus == Focus::Preview {
                    self.perform_document_search();
                }
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            _ => {}
        }
    }

    /// Rebuild filtered tree items based on current search query (file search).
    fn update_file_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_tree_items = None;
            self.filtered_path_map = None;
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        let mut filtered_items = Vec::new();
        let mut filtered_map = HashMap::new();

        for (id, (path, is_dir)) in &self.path_map {
            if *is_dir {
                continue;
            }
            let name =
                path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if name.contains(&query_lower) {
                let display_name =
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let item = TreeItem::new_leaf(id.clone(), display_name);
                filtered_items.push(item);
                filtered_map.insert(id.clone(), (path.clone(), *is_dir));
            }
        }

        // Sort filtered items alphabetically by their identifier.
        filtered_items.sort_by(|a, b| a.identifier().cmp(b.identifier()));

        self.filtered_tree_items = Some(filtered_items);
        self.filtered_path_map = Some(filtered_map);
    }

    /// Perform in-document search: find all lines containing the query.
    fn perform_document_search(&mut self) {
        self.search_matches.clear();
        self.search_current = 0;

        if self.search_query.is_empty() || self.file_content.is_empty() {
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        for (i, line) in self.file_content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                self.search_matches.push(i);
            }
        }

        // Scroll to first match.
        if let Some(&line_num) = self.search_matches.first() {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message =
                format!("/{} [{}/{}]", self.search_query, 1, self.search_matches.len());
        } else {
            self.status_message = format!("Pattern not found: {}", self.search_query);
        }
    }

    /// Navigate to the next search match.
    fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            // For file search, just keep filter active.
            return;
        }
        if self.search_current + 1 < self.search_matches.len() {
            self.search_current += 1;
        } else {
            self.search_current = 0; // Wrap around.
        }
        if let Some(&line_num) = self.search_matches.get(self.search_current) {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search_query,
                self.search_current + 1,
                self.search_matches.len()
            );
        }
    }

    /// Navigate to the previous search match.
    fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_current > 0 {
            self.search_current -= 1;
        } else {
            self.search_current = self.search_matches.len().saturating_sub(1); // Wrap.
        }
        if let Some(&line_num) = self.search_matches.get(self.search_current) {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search_query,
                self.search_current + 1,
                self.search_matches.len()
            );
        }
    }

    /// Clear all search state.
    fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
        self.filtered_tree_items = None;
        self.filtered_path_map = None;
        self.status_message.clear();
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
}
