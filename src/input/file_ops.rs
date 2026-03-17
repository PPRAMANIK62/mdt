use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;
use std::time::Instant;

use crate::app::{App, FileOp, Overlay};
use crate::file_ops;

impl App {
    pub(crate) fn handle_file_op_key(&mut self, key: KeyEvent) {
        match &self.overlay {
            Overlay::FileOp(FileOp::Delete { .. }) => match key.code {
                KeyCode::Enter => self.confirm_file_op(),
                KeyCode::Esc => self.cancel_file_op(),
                _ => {}
            },
            _ => match key.code {
                KeyCode::Esc => self.cancel_file_op(),
                KeyCode::Enter => self.confirm_file_op(),
                KeyCode::Backspace => {
                    self.file_op_input.pop();
                }
                KeyCode::Char(c) => {
                    self.file_op_input.push(c);
                }
                _ => {}
            },
        }
    }

    pub(crate) fn cancel_file_op(&mut self) {
        self.overlay = Overlay::None;
        self.file_op_input.clear();
    }

    pub(crate) fn confirm_file_op(&mut self) {
        let op = match std::mem::replace(&mut self.overlay, Overlay::None) {
            Overlay::FileOp(op) => op,
            other => {
                self.overlay = other;
                return;
            }
        };
        let input = self.file_op_input.clone();
        self.file_op_input.clear();

        match op {
            FileOp::CreateFile { parent_dir } => {
                if input.is_empty() {
                    return;
                }
                match file_ops::create_file(&self.root_path, &parent_dir, &input) {
                    Ok(path) => {
                        let rel = self.relative_path_str(&path);
                        self.refresh_tree_add(&path, false, Some(&rel));
                        self.open_file(&path);
                        self.status_message = format!("Created {rel}");
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
            FileOp::CreateDir { parent_dir } => {
                if input.is_empty() {
                    return;
                }
                match file_ops::create_dir(&self.root_path, &parent_dir, &input) {
                    Ok(path) => {
                        let rel = self.relative_path_str(&path);
                        self.refresh_tree_add(&path, true, Some(&rel));
                        self.status_message = format!("Created {rel}/");
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
            FileOp::Delete { target, is_dir: _, name } => {
                let is_current = self.document.current_file.as_ref() == Some(&target);
                match file_ops::delete_entry(&target) {
                    Ok(()) => {
                        if is_current {
                            self.document.clear();
                        }
                        self.refresh_tree_remove(&target, None);
                        self.status_message = format!("Deleted {name}");
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
            FileOp::Rename { target, is_dir } => {
                if input.is_empty() {
                    return;
                }
                let is_current = self.document.current_file.as_ref() == Some(&target);
                match file_ops::rename_entry(&self.root_path, &target, &input) {
                    Ok(new_path) => {
                        let rel = self.relative_path_str(&new_path);
                        if is_current {
                            self.document.current_file = Some(new_path.clone());
                        }
                        self.refresh_tree_move(&target, &new_path, is_dir, Some(&rel));
                        self.status_message = format!("Renamed to {rel}");
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
            FileOp::Move { source, is_dir } => {
                if input.is_empty() {
                    return;
                }
                let is_current = self.document.current_file.as_ref() == Some(&source);
                match file_ops::move_entry(&self.root_path, &source, &input) {
                    Ok(new_path) => {
                        let rel = self.relative_path_str(&new_path);
                        if is_current {
                            self.document.current_file = Some(new_path.clone());
                        }
                        self.refresh_tree_move(&source, &new_path, is_dir, Some(&rel));
                        self.status_message = format!("Moved to {rel}");
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
        }
    }
}

impl App {
    fn selected_context(&self) -> Option<(PathBuf, bool)> {
        let selected = self.tree.tree_state.selected();
        let id = selected.last()?;
        self.tree.path_map.get(id).cloned()
    }

    fn show_file_op_dialog(&mut self, op: FileOp) {
        self.overlay = Overlay::FileOp(op);
        self.cursor.visible = true;
        self.cursor.last_toggle = Instant::now();
    }

    pub(crate) fn start_create_file(&mut self) {
        let parent = match self.selected_context() {
            Some((path, true)) => path,
            Some((path, false)) => path.parent().unwrap_or(&self.root_path).to_path_buf(),
            None => self.root_path.clone(),
        };
        self.file_op_input.clear();
        self.show_file_op_dialog(FileOp::CreateFile { parent_dir: parent });
    }

    pub(crate) fn start_create_dir(&mut self) {
        let parent = match self.selected_context() {
            Some((path, true)) => path,
            Some((path, false)) => path.parent().unwrap_or(&self.root_path).to_path_buf(),
            None => self.root_path.clone(),
        };
        self.file_op_input.clear();
        self.show_file_op_dialog(FileOp::CreateDir { parent_dir: parent });
    }

    pub(crate) fn start_delete(&mut self) {
        if let Some((path, is_dir)) = self.selected_context() {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            self.file_op_input.clear();
            self.show_file_op_dialog(FileOp::Delete { target: path, is_dir, name });
        }
    }

    pub(crate) fn start_rename(&mut self) {
        if let Some((path, is_dir)) = self.selected_context() {
            let current_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            self.file_op_input = current_name;
            self.show_file_op_dialog(FileOp::Rename { target: path, is_dir });
        }
    }

    pub(crate) fn start_move(&mut self) {
        if let Some((path, is_dir)) = self.selected_context() {
            self.file_op_input.clear();
            self.show_file_op_dialog(FileOp::Move { source: path, is_dir });
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::Color;

    use crate::app::{App, FileOp, Overlay};
    use crate::test_util::TempTestDir;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn cancel_file_op_clears_overlay_and_input() {
        let dir = TempTestDir::new("mdt-test-fop-cancel");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.file_op_input = "test".to_string();

        app.cancel_file_op();

        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.file_op_input.is_empty());
    }

    #[test]
    fn start_create_file_sets_overlay() {
        let dir = TempTestDir::new("mdt-test-fop-create-file");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        // Select the first file
        app.tree.tree_state.select(vec!["a.md".to_string()]);
        app.start_create_file();

        assert!(matches!(app.overlay, Overlay::FileOp(FileOp::CreateFile { .. })));
        assert!(app.file_op_input.is_empty());
    }

    #[test]
    fn start_create_dir_sets_overlay() {
        let dir = TempTestDir::new("mdt-test-fop-create-dir");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.start_create_dir();

        assert!(matches!(app.overlay, Overlay::FileOp(FileOp::CreateDir { .. })));
    }

    #[test]
    fn start_delete_with_selection() {
        let dir = TempTestDir::new("mdt-test-fop-delete");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.tree.tree_state.select(vec!["a.md".to_string()]);
        app.start_delete();

        match &app.overlay {
            Overlay::FileOp(FileOp::Delete { name, .. }) => {
                assert_eq!(name, "a.md");
            }
            _ => panic!("Expected Delete overlay"),
        }
    }

    #[test]
    fn start_rename_prefills_current_name() {
        let dir = TempTestDir::new("mdt-test-fop-rename");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.tree.tree_state.select(vec!["a.md".to_string()]);
        app.start_rename();

        assert!(matches!(app.overlay, Overlay::FileOp(FileOp::Rename { .. })));
        assert_eq!(app.file_op_input, "a.md");
    }

    #[test]
    fn start_move_with_selection() {
        let dir = TempTestDir::new("mdt-test-fop-move");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.tree.tree_state.select(vec!["a.md".to_string()]);
        app.start_move();

        assert!(matches!(app.overlay, Overlay::FileOp(FileOp::Move { .. })));
        assert!(app.file_op_input.is_empty());
    }

    #[test]
    fn handle_file_op_key_esc_cancels() {
        let dir = TempTestDir::new("mdt-test-fop-esc");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.handle_file_op_key(key(KeyCode::Esc));

        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn handle_file_op_key_char_adds_to_input() {
        let dir = TempTestDir::new("mdt-test-fop-char");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.handle_file_op_key(key(KeyCode::Char('t')));
        app.handle_file_op_key(key(KeyCode::Char('e')));

        assert_eq!(app.file_op_input, "te");
    }

    #[test]
    fn handle_file_op_key_backspace_removes_char() {
        let dir = TempTestDir::new("mdt-test-fop-bksp");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.file_op_input = "abc".to_string();
        app.handle_file_op_key(key(KeyCode::Backspace));

        assert_eq!(app.file_op_input, "ab");
    }

    #[test]
    fn confirm_create_file_creates_and_updates_tree() {
        let dir = TempTestDir::new("mdt-test-fop-confirm-create");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.file_op_input = "new.md".to_string();
        app.confirm_file_op();

        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.status_message.contains("Created"));
        assert!(dir.path().join("new.md").exists());
    }

    #[test]
    fn confirm_create_dir_creates_directory() {
        let dir = TempTestDir::new("mdt-test-fop-confirm-mkdir");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateDir { parent_dir: dir.path().to_path_buf() });
        app.file_op_input = "newdir".to_string();
        app.confirm_file_op();

        assert!(dir.path().join("newdir").is_dir());
        assert!(app.status_message.contains("Created"));
    }

    #[test]
    fn confirm_empty_input_does_nothing() {
        let dir = TempTestDir::new("mdt-test-fop-empty");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::CreateFile { parent_dir: dir.path().to_path_buf() });
        app.file_op_input.clear();
        app.confirm_file_op();

        // Should return silently without creating anything
        assert!(app.status_message.is_empty());
    }

    #[test]
    fn confirm_delete_removes_file() {
        let dir = TempTestDir::new("mdt-test-fop-confirm-delete");
        dir.create_file("a.md", "# A");
        let target = dir.path().join("a.md");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::Delete {
            target: target.clone(),
            is_dir: false,
            name: "a.md".to_string(),
        });
        app.confirm_file_op();

        assert!(!target.exists());
        assert!(app.status_message.contains("Deleted"));
    }

    #[test]
    fn confirm_delete_current_file_clears_document() {
        let dir = TempTestDir::new("mdt-test-fop-delete-current");
        dir.create_file("a.md", "# A");
        let target = dir.path().join("a.md");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&target);
        assert!(app.document.current_file.is_some());

        app.overlay = Overlay::FileOp(FileOp::Delete {
            target: target.clone(),
            is_dir: false,
            name: "a.md".to_string(),
        });
        app.confirm_file_op();

        assert!(app.document.current_file.is_none());
    }

    #[test]
    fn confirm_rename_renames_file() {
        let dir = TempTestDir::new("mdt-test-fop-confirm-rename");
        dir.create_file("old.md", "# Old");
        let target = dir.path().join("old.md");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay = Overlay::FileOp(FileOp::Rename { target, is_dir: false });
        app.file_op_input = "new.md".to_string();
        app.confirm_file_op();

        assert!(!dir.path().join("old.md").exists());
        assert!(dir.path().join("new.md").exists());
        assert!(app.status_message.contains("Renamed"));
    }

    #[test]
    fn delete_overlay_only_enter_or_esc() {
        let dir = TempTestDir::new("mdt-test-fop-delete-keys");
        dir.create_file("a.md", "# A");
        let target = dir.path().join("a.md");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.overlay =
            Overlay::FileOp(FileOp::Delete { target, is_dir: false, name: "a.md".to_string() });

        // Typing chars should do nothing in delete confirmation
        app.handle_file_op_key(key(KeyCode::Char('y')));
        assert!(app.file_op_input.is_empty());
    }
}
