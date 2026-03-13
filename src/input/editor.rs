use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;

use crate::app::{App, AppMode};
use crate::markdown::render_markdown;

impl App {
    /// Handle key events in Insert mode — forward to TextArea.
    pub(crate) fn handle_insert_key(&mut self, key: KeyEvent) {
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

    /// Handle Normal-mode keys while in editor view (textarea is Some).
    pub(crate) fn handle_editor_normal_key(&mut self, key: KeyEvent) {
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
    pub(crate) fn enter_editor(&mut self) {
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
    pub(crate) fn exit_editor(&mut self) {
        self.textarea = None;
        self.is_dirty = false;
        self.mode = AppMode::Normal;
        self.scroll_offset = 0;
    }

    /// Save the editor content to disk, re-render markdown.
    pub(crate) fn save_editor(&mut self) -> bool {
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

        match std::fs::write(&path, &content) {
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
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, AppMode};
    use ratatui::style::Color;

    #[test]
    fn esc_in_insert_mode_returns_to_normal() {
        let dir = std::env::temp_dir().join("mdt-test-editor-esc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        app.mode = AppMode::Insert;

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_insert_key(key);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.status_message.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn i_key_in_editor_normal_enters_insert() {
        let dir = std::env::temp_dir().join("mdt-test-editor-i");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        app.open_file(&dir.join("test.md"));
        app.enter_editor();

        // enter_editor sets Insert mode; switch to Normal for this test.
        app.mode = AppMode::Normal;

        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        app.handle_editor_normal_key(key);

        assert_eq!(app.mode, AppMode::Insert);
        assert_eq!(app.status_message, "-- INSERT --");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
