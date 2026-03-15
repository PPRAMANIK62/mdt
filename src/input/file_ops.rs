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
