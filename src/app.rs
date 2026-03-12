//! Application state and logic.

use std::fs;
use std::path::{Path, PathBuf};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::text::{Line, Span, Text};

use crate::file_tree::FileTree;

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
    pub file_tree: FileTree,
    pub current_file: Option<PathBuf>,
    pub file_content: String,
    pub rendered_lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub focus: Focus,
    pub should_quit: bool,
    #[allow(dead_code)]  // Used by future tasks for external redraw triggers.
    pub needs_redraw: bool,
    pub status_message: String,
}

impl App {
    /// Create a new `App` rooted at `path`.
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let file_tree = FileTree::scan(&path)?;
        Ok(Self {
            file_tree,
            current_file: None,
            file_content: String::new(),
            rendered_lines: Vec::new(),
            scroll_offset: 0,
            mode: AppMode::Normal,
            focus: Focus::FileList,
            should_quit: false,
            needs_redraw: true,
            status_message: String::new(),
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
            AppMode::Insert | AppMode::Command => {
                // Not implemented yet — fall back to normal handling.
                self.handle_normal_key(key);
            }
        }
    }

    /// Handle key events in Normal mode.
    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::FileList => self.file_tree.navigate_down(),
                Focus::Preview => self.scroll_down(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::FileList => self.file_tree.navigate_up(),
                Focus::Preview => self.scroll_up(),
            },
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                if self.focus == Focus::FileList {
                    self.file_tree.go_back();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus == Focus::FileList {
                    self.handle_enter();
                }
            }
            _ => {}
        }
    }

    /// Open the selected file tree entry.
    fn handle_enter(&mut self) {
        if self.focus != Focus::FileList {
            return;
        }
        if let Some(path) = self.file_tree.enter() {
            self.open_file(&path);
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
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
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
