//! Event dispatch: `handle_event()` method on `App`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::types::Overlay;
use super::App;

impl App {
    /// Dispatch a key press based on current mode and focus.
    pub fn handle_event(&mut self, key: KeyEvent) {
        // Ctrl+C always quits regardless of mode.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.mode {
            super::types::AppMode::Normal => {
                // Overlays — handle their own keys before normal dispatch.
                match self.overlay {
                    Overlay::FileOp(_) => {
                        self.handle_file_op_key(key);
                        return;
                    }
                    Overlay::LinkPicker => {
                        match key.code {
                            KeyCode::Down => {
                                let len = self.filtered_link_indices().len();
                                if len > 0 {
                                    self.link_picker.selected =
                                        (self.link_picker.selected + 1) % len;
                                }
                            }
                            KeyCode::Up => {
                                let len = self.filtered_link_indices().len();
                                if len > 0 {
                                    self.link_picker.selected = if self.link_picker.selected == 0 {
                                        len.saturating_sub(1)
                                    } else {
                                        self.link_picker.selected - 1
                                    };
                                }
                            }
                            KeyCode::Enter => {
                                self.open_selected_link();
                            }
                            KeyCode::Backspace => {
                                self.link_picker.search_query.pop();
                                self.link_picker.selected = 0;
                            }
                            KeyCode::Esc => {
                                if self.link_picker.search_query.is_empty() {
                                    self.overlay = Overlay::None;
                                } else {
                                    self.link_picker.search_query.clear();
                                    self.link_picker.selected = 0;
                                }
                            }
                            KeyCode::Char(c) => {
                                self.link_picker.search_query.push(c);
                                self.link_picker.selected = 0;
                            }
                            _ => {}
                        }
                        return;
                    }
                    Overlay::FileFinder => {
                        self.handle_file_finder_key(key);
                        return;
                    }
                    Overlay::Help => {
                        if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
                            self.overlay = Overlay::None;
                        }
                        return;
                    }
                    Overlay::None => {}
                }
                self.handle_normal_key(key);
            }
            super::types::AppMode::Insert => self.handle_insert_key(key),
            super::types::AppMode::Command => self.handle_command_key(key),
            super::types::AppMode::Search => self.handle_search_key(key),
        }
    }
}
