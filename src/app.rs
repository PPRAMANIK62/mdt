//! Application state and logic.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;
use tui_tree_widget::{TreeItem, TreeState};

use crate::file_tree;

/// Current input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppMode {
    Normal,
    Insert,
    Command,
}

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Command => write!(f, "COMMAND"),
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
    #[allow(dead_code)] // Used by future tasks for external redraw triggers.
    pub needs_redraw: bool,
    pub status_message: String,
    /// Pending key for composed commands like `gg`.
    pub pending_key: Option<(char, Instant)>,
    /// Buffer for command-mode input (e.g., `:q`).
    pub command_buffer: String,
    /// Active text editor (Some when in editor mode).
    pub textarea: Option<TextArea<'static>>,
    /// Whether the editor has unsaved changes.
    pub is_dirty: bool,
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
            needs_redraw: true,
            status_message: String::new(),
            pending_key: None,
            command_buffer: String::new(),
            textarea: None,
            is_dirty: false,
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
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Insert => self.handle_insert_key(key),
            AppMode::Command => self.handle_command_key(key),
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
            if instant.elapsed().as_millis() < 500
                && pending_char == 'g'
                && key.code == KeyCode::Char('g')
            {
                match self.focus {
                    Focus::Preview => self.scroll_to_top(),
                    Focus::FileList => {
                        self.tree_state.select_first();
                    }
                }
                return;
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
                // Search placeholder — consume key, show hint.
                self.status_message = "Search not yet implemented".to_string();
            }
            KeyCode::Char('i') | KeyCode::Char('e') => {
                if self.focus == Focus::Preview {
                    self.enter_editor();
                }
            }

            // --- Quit ---
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

                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
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
                self.status_message = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
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
        self.rendered_lines
            .len()
            .saturating_sub(self.viewport_height)
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }
}

/// Convert raw markdown into styled ratatui [`Text`] for rendering in a `Paragraph` widget.
///
/// - Pre-expands tabs to 4 spaces (ratatui `Paragraph` silently drops tab characters).
/// - Respects the `NO_COLOR` environment variable: when set, returns plain unstyled text.
/// - Delegates all markdown parsing and styling to [`tui_markdown::from_str`], which handles
///   headings, bold/italic, strikethrough, inline code, fenced code blocks (syntax-highlighted),
///   blockquotes, lists, task lists, links, YAML front matter, and horizontal rules.
pub fn render_markdown(input: &str) -> Text<'static> {
    // Pre-expand tabs (ratatui Paragraph silently drops tab characters)
    let cleaned = input.replace('\t', "    ");

    // Respect NO_COLOR env var — return plain text when set
    if std::env::var("NO_COLOR").is_ok() {
        return Text::raw(cleaned);
    }

    let text = tui_markdown::from_str(&cleaned);
    text_to_owned(text)
}

/// Convert a borrowed [`Text`] into an owned `Text<'static>` by cloning all string data.
fn text_to_owned(text: Text<'_>) -> Text<'static> {
    let lines: Vec<Line<'static>> = text
        .lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line {
                spans,
                style: line.style,
                alignment: line.alignment,
            }
        })
        .collect();
    Text {
        lines,
        style: text.style,
        alignment: text.alignment,
    }
}
