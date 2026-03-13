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
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, AppMode};
    use ratatui::style::Color;

    #[test]
    fn execute_quit_sets_should_quit() {
        let dir = std::env::temp_dir().join("mdt-test-cmd-quit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        assert!(!app.should_quit);

        app.execute_command("q");

        assert!(app.should_quit);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn esc_in_command_mode_returns_to_normal() {
        let dir = std::env::temp_dir().join("mdt-test-cmd-esc");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.md"), "# Test").unwrap();

        let mut app = App::new(dir.clone(), Color::Reset).unwrap();
        app.mode = AppMode::Command;
        app.command_buffer = "some".to_string();

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_command_key(key);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.command_buffer.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
