use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::app::{App, Focus};

impl App {
    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.is_in_preview(mouse.column, mouse.row) {
                    for _ in 0..3 {
                        self.document.scroll_down();
                    }
                } else if self.is_in_file_list(mouse.column, mouse.row) {
                    for _ in 0..3 {
                        self.tree.tree_state.key_down();
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_preview(mouse.column, mouse.row) {
                    for _ in 0..3 {
                        self.document.scroll_up();
                    }
                } else if self.is_in_file_list(mouse.column, mouse.row) {
                    for _ in 0..3 {
                        self.tree.tree_state.key_up();
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.is_in_preview(mouse.column, mouse.row) {
                    self.focus = Focus::Preview;
                } else if self.is_in_file_list(mouse.column, mouse.row) {
                    self.focus = Focus::FileList;
                }
            }
            _ => {}
        }
    }

    fn is_in_preview(&self, col: u16, row: u16) -> bool {
        self.preview_area.is_some_and(|r| r.contains(Position::new(col, row)))
    }

    fn is_in_file_list(&self, col: u16, row: u16) -> bool {
        self.file_list_area.is_some_and(|r| r.contains(Position::new(col, row)))
    }
}
