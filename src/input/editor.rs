use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;

use crate::app::{App, AppMode};
use crate::markdown::{deduplicate_links, render_markdown_blocks, rewrap_blocks};

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
                if self.live_preview.enabled {
                    self.live_preview.debounce = Some(std::time::Instant::now());
                }
            }
        }
    }

    /// Handle Normal-mode keys while in editor view (textarea is Some).
    pub(crate) fn handle_editor_normal_key(&mut self, key: KeyEvent) {
        // Check for composed commands (Space+p, Space+s).
        if let Some((pending_char, instant)) = self.pending_key.take() {
            if instant.elapsed().as_millis() < 500 {
                match (pending_char, key.code) {
                    (' ', KeyCode::Char('p')) => {
                        self.toggle_live_preview();
                        return;
                    }
                    (' ', KeyCode::Char('s')) => {
                        self.toggle_split_orientation();
                        return;
                    }
                    _ => {} // fall through
                }
            }
        }

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
            // Leader key for composed commands.
            KeyCode::Char(' ') => {
                self.pending_key = Some((' ', std::time::Instant::now()));
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
        self.editor.external_change_detected = false;
        self.mode = AppMode::Insert;
        self.status_message = "-- INSERT --".to_string();
    }

    /// Exit the editor, returning to preview mode.
    pub(crate) fn exit_editor(&mut self) {
        self.editor.textarea = None;
        self.editor.is_dirty = false;
        self.editor.external_change_detected = false;
        self.mode = AppMode::Normal;
        self.document.scroll_offset = 0;
    }

    /// Reload the editor content from disk (`:e` command).
    pub(crate) fn reload_editor_from_disk(&mut self) {
        let Some(ref path) = self.document.current_file else {
            self.status_message = "No file path".to_string();
            return;
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("Error reading: {e}");
                return;
            }
        };

        let mut textarea = TextArea::from(content.lines());
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
        self.editor.external_change_detected = false;
        self.document.file_content = content;
        self.status_message = "reloaded".to_string();
    }

    /// Save the editor content to disk, re-render markdown.
    pub(crate) fn save_editor(&mut self) -> anyhow::Result<()> {
        let Some(ref path) = self.document.current_file else {
            self.status_message = "No file path".to_string();
            anyhow::bail!("No file path");
        };
        let Some(ref textarea) = self.editor.textarea else {
            self.status_message = "Not in editor".to_string();
            anyhow::bail!("Not in editor");
        };

        let content = textarea.lines().join("\n") + "\n";
        let path = path.clone();

        match std::fs::write(&path, &content) {
            Ok(()) => {
                self.document.file_content = content;
                let (blocks, links) = render_markdown_blocks(&self.document.file_content);
                let width = if self.document.viewport_width > 0 {
                    Some(self.document.viewport_width)
                } else {
                    None
                };
                let (rendered, block_line_starts) = rewrap_blocks(&blocks, width);
                self.document.rendered_lines = rendered;
                self.document.block_line_starts = block_line_starts;
                self.document.rebuild_lower_cache();
                self.document.rendered_blocks = blocks;
                self.document.links = deduplicate_links(links);
                self.document.rebuild_heading_index();
                self.editor.is_dirty = false;
                self.editor.external_change_detected = false;

                self.status_message = "written".to_string();
                Ok(())
            }
            Err(e) => {
                self.status_message = format!("Error saving: {e}");
                Err(e.into())
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
        assert!(saved.is_ok());

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
        assert!(saved.is_ok());

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert_eq!(on_disk, "\n", "empty editor should save as single newline");
    }

    #[test]
    fn reload_editor_from_disk_updates_content() {
        let dir = TempTestDir::new("mdt-test-editor-reload");
        dir.create_file("test.md", "# Original");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        // Overwrite on disk.
        std::fs::write(&file, "# Reloaded").unwrap();
        app.reload_editor_from_disk();

        assert_eq!(app.document.file_content, "# Reloaded");
        assert!(!app.editor.is_dirty);
        assert!(!app.editor.external_change_detected);
        assert_eq!(app.status_message, "reloaded");
    }

    #[test]
    fn reload_editor_clears_flags() {
        let dir = TempTestDir::new("mdt-test-editor-flags");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        app.editor.is_dirty = true;
        app.editor.external_change_detected = true;

        app.reload_editor_from_disk();

        assert!(!app.editor.is_dirty);
        assert!(!app.editor.external_change_detected);
    }

    #[test]
    fn insert_keystroke_sets_debounce_when_preview_enabled() {
        let dir = TempTestDir::new("mdt-test-editor-debounce");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.live_preview.enabled = true;

        assert!(app.live_preview.debounce.is_none());

        app.handle_insert_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(app.live_preview.debounce.is_some());
    }

    #[test]
    fn space_p_toggles_live_preview_in_editor_normal() {
        let dir = TempTestDir::new("mdt-test-editor-space-p");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.mode = AppMode::Normal;
        assert!(!app.live_preview.enabled);

        app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(app.live_preview.enabled);
    }

    #[test]
    fn space_s_toggles_orientation_in_editor_normal() {
        use crate::app::SplitOrientation;
        let dir = TempTestDir::new("mdt-test-editor-space-s");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.mode = AppMode::Normal;
        assert_eq!(app.live_preview.orientation, SplitOrientation::Horizontal);

        app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.live_preview.orientation, SplitOrientation::Vertical);
    }

    #[test]
    fn reload_editor_no_file_shows_error() {
        let dir = TempTestDir::new("mdt-test-editor-no-file");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        // Set up textarea without opening a file.
        app.editor.textarea = Some(ratatui_textarea::TextArea::default());

        app.reload_editor_from_disk();

        assert_eq!(app.status_message, "No file path");
    }
}
