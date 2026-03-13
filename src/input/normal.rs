use std::path::Path;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppMode, Focus};
use crate::markdown::render_markdown;

impl App {
    /// Handle key events in Normal mode.
    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) {
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

    /// Open the selected file tree entry.
    pub(crate) fn handle_enter(&mut self) {
        if self.focus != Focus::FileList {
            return;
        }
        let selected = self.tree_state.selected().to_vec();
        let Some(id) = selected.last() else {
            return;
        };
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
    pub(crate) fn open_file(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
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

    pub(crate) fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileList => Focus::Preview,
            Focus::Preview => Focus::FileList,
        };
    }

    pub(crate) fn toggle_file_tree(&mut self) {
        self.show_file_tree = !self.show_file_tree;
        if self.show_file_tree {
            self.focus = Focus::FileList;
        } else {
            self.focus = Focus::Preview;
        }
    }

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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, AppMode, Focus};
    use ratatui::style::Color;

    #[test]
    fn j_key_in_file_list_dispatches_tree_navigation() {
        let dir = std::env::temp_dir().join("mdt-test-normal-j");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "# A").unwrap();
        std::fs::write(dir.join("b.md"), "# B").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        assert_eq!(app.focus, Focus::FileList);
        assert_eq!(app.mode, AppMode::Normal);

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_normal_key(key);

        // Dispatch went to FileList branch — mode and focus unchanged.
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.focus, Focus::FileList);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn colon_key_enters_command_mode() {
        let dir = std::env::temp_dir().join("mdt-test-normal-colon");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
        app.handle_normal_key(key);

        assert_eq!(app.mode, AppMode::Command);
        assert!(app.command_buffer.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
