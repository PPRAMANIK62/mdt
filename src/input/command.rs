use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, AppMode};

impl App {
    /// Handle key events in Command mode (`:` prefix).
    pub(crate) fn handle_command_key(&mut self, key: KeyEvent) {
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
    pub(crate) fn execute_command(&mut self, cmd: &str) {
        let in_editor = self.editor.textarea.is_some();

        match cmd {
            "q" | "quit" => {
                if in_editor {
                    if self.editor.is_dirty {
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
                    let _ = self.save_editor();
                } else {
                    self.status_message = "Not in editor".to_string();
                }
            }
            "wq" | "x" => {
                if in_editor {
                    if self.save_editor().is_ok() {
                        self.exit_editor();
                    }
                } else {
                    self.status_message = "Not in editor".to_string();
                }
            }
            "e" | "edit" => {
                if in_editor {
                    self.reload_editor_from_disk();
                } else {
                    self.status_message = "Not in editor".to_string();
                }
            }
            other => {
                self.status_message = format!("Unknown command: :{other}");
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
    fn execute_quit_sets_should_quit() {
        let dir = TempTestDir::new("mdt-test-cmd-quit");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(!app.should_quit);

        app.execute_command("q");

        assert!(app.should_quit);
    }

    #[test]
    fn esc_in_command_mode_returns_to_normal() {
        let dir = TempTestDir::new("mdt-test-cmd-esc");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Command;
        app.command_buffer = "some".to_string();

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_command_key(key);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.command_buffer.is_empty());
    }

    #[test]
    fn execute_e_reloads_in_editor() {
        let dir = TempTestDir::new("mdt-test-cmd-e-reload");
        dir.create_file("test.md", "# V1");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        // Overwrite on disk.
        std::fs::write(&file, "# V2").unwrap();
        app.execute_command("e");

        assert_eq!(app.document.file_content, "# V2");
        assert!(!app.editor.is_dirty);
        assert!(!app.editor.external_change_detected);
        assert_eq!(app.status_message, "reloaded");
    }

    #[test]
    fn execute_e_not_in_editor() {
        let dir = TempTestDir::new("mdt-test-cmd-e-no-editor");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);

        app.execute_command("e");

        assert_eq!(app.status_message, "Not in editor");
    }

    #[test]
    fn char_key_adds_to_command_buffer() {
        let dir = TempTestDir::new("mdt-test-cmd-char");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Command;

        app.handle_command_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_command_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

        assert_eq!(app.command_buffer, "hi");
        assert_eq!(app.mode, AppMode::Command);
    }

    #[test]
    fn backspace_pops_char() {
        let dir = TempTestDir::new("mdt-test-cmd-bksp");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Command;
        app.command_buffer = "abc".to_string();

        app.handle_command_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert_eq!(app.command_buffer, "ab");
        assert_eq!(app.mode, AppMode::Command);
    }

    #[test]
    fn backspace_on_empty_returns_to_normal() {
        let dir = TempTestDir::new("mdt-test-cmd-bksp-empty");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Command;
        app.command_buffer = "a".to_string();

        // Pop the last char
        app.handle_command_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(app.command_buffer.is_empty());
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn enter_executes_command_and_returns_to_normal() {
        let dir = TempTestDir::new("mdt-test-cmd-enter");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Command;
        app.command_buffer = "q".to_string();

        app.handle_command_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.command_buffer.is_empty());
        assert!(app.should_quit);
    }

    #[test]
    fn unknown_command_shows_error() {
        let dir = TempTestDir::new("mdt-test-cmd-unknown");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.execute_command("foobar");

        assert!(app.status_message.contains("Unknown command"));
        assert!(app.status_message.contains("foobar"));
    }

    #[test]
    fn quit_alias_works() {
        let dir = TempTestDir::new("mdt-test-cmd-quit-alias");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.execute_command("quit");

        assert!(app.should_quit);
    }

    #[test]
    fn q_bang_force_quits_editor() {
        let dir = TempTestDir::new("mdt-test-cmd-q-bang");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.editor.is_dirty = true;

        app.execute_command("q!");

        // Should exit editor despite dirty flag
        assert!(app.editor.textarea.is_none());
    }

    #[test]
    fn q_with_dirty_editor_shows_warning() {
        let dir = TempTestDir::new("mdt-test-cmd-q-dirty");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.editor.is_dirty = true;

        app.execute_command("q");

        assert!(app.status_message.contains("Unsaved changes"));
        assert!(app.editor.textarea.is_some()); // still in editor
    }

    #[test]
    fn w_not_in_editor_shows_error() {
        let dir = TempTestDir::new("mdt-test-cmd-w-no-editor");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.execute_command("w");

        assert_eq!(app.status_message, "Not in editor");
    }

    #[test]
    fn wq_not_in_editor_shows_error() {
        let dir = TempTestDir::new("mdt-test-cmd-wq-no-editor");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.execute_command("wq");

        assert_eq!(app.status_message, "Not in editor");
    }

    #[test]
    fn edit_alias_works() {
        let dir = TempTestDir::new("mdt-test-cmd-edit-alias");
        dir.create_file("test.md", "# V1");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        std::fs::write(&file, "# V2").unwrap();
        app.execute_command("edit");

        assert_eq!(app.document.file_content, "# V2");
    }
}
