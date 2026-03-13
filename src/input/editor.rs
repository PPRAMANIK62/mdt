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
        if let Some(ref mut textarea) = self.editor.textarea {
            let modified = textarea.input(key);
            if modified {
                self.editor.is_dirty = true;
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
                if self.editor.is_dirty {
                    self.status_message = "Unsaved changes! :w to save, :q! to discard".to_string();
                } else {
                    self.exit_editor();
                }
            }
            // Forward navigation keys to TextArea (h/j/k/l, arrows, etc.).
            _ => {
                if let Some(ref mut textarea) = self.editor.textarea {
                    textarea.input(key);
                }
            }
        }
    }

    /// Enter the editor: create TextArea from current file content.
    pub(crate) fn enter_editor(&mut self) {
        if self.document.current_file.is_none() {
            self.status_message = "No file open".to_string();
            return;
        }

        let mut textarea = TextArea::from(self.document.file_content.lines());

        let file_path = self.display_file_path();
        let title = if file_path.is_empty() {
            " Editor ".to_string()
        } else {
            format!(" Editor: {} ", file_path)
        };

        textarea.set_block(Block::default().title(title).borders(Borders::ALL));
        textarea.set_line_number_style(Style::default());

        self.editor.textarea = Some(textarea);
        self.editor.is_dirty = false;
        self.mode = AppMode::Insert;
        self.status_message = "-- INSERT --".to_string();
    }

    /// Exit the editor, returning to preview mode.
    pub(crate) fn exit_editor(&mut self) {
        self.editor.textarea = None;
        self.editor.is_dirty = false;
        self.mode = AppMode::Normal;
        self.document.scroll_offset = 0;
    }

    /// Save the editor content to disk, re-render markdown.
    pub(crate) fn save_editor(&mut self) -> bool {
        let Some(ref path) = self.document.current_file else {
            self.status_message = "No file path".to_string();
            return false;
        };
        let Some(ref textarea) = self.editor.textarea else {
            self.status_message = "Not in editor".to_string();
            return false;
        };

        let content = textarea.lines().join("\n") + "\n";
        let path = path.clone();

        match std::fs::write(&path, &content) {
            Ok(()) => {
                // Update stored content and re-render markdown preview.
                self.document.file_content = content;
                let rendered = render_markdown(&self.document.file_content);
                self.document.rendered_lines = rendered.lines;
                self.editor.is_dirty = false;

                self.status_message = "written".to_string();
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
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

    #[test]
    fn esc_in_insert_mode_returns_to_normal() {
        let dir = TempTestDir::new("mdt-test-editor-esc");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Insert;

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_insert_key(key);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.status_message.is_empty());
    }

    #[test]
    fn i_key_in_editor_normal_enters_insert() {
        let dir = TempTestDir::new("mdt-test-editor-i");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.enter_editor();

        // enter_editor sets Insert mode; switch to Normal for this test.
        app.mode = AppMode::Normal;

        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        app.handle_editor_normal_key(key);

        assert_eq!(app.mode, AppMode::Insert);
        assert_eq!(app.status_message, "-- INSERT --");
    }

    #[test]
    fn save_editor_appends_trailing_newline() {
        let dir = TempTestDir::new("mdt-test-editor-trailing-nl");
        dir.create_file("test.md", "# Hello\nWorld");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        let saved = app.save_editor();
        assert!(saved);

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(on_disk.ends_with('\n'), "saved file must end with newline");
        assert!(!on_disk.ends_with("\n\n"), "must not have double trailing newline");
    }

    #[test]
    fn save_editor_empty_content_is_single_newline() {
        let dir = TempTestDir::new("mdt-test-editor-empty-nl");
        dir.create_file("test.md", "");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        let saved = app.save_editor();
        assert!(saved);

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert_eq!(on_disk, "\n", "empty editor should save as single newline");
    }
}
