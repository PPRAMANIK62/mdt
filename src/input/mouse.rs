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

#[cfg(test)]
mod tests {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use crate::app::{App, Focus};
    use crate::test_util::TempTestDir;

    fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent { kind, column: col, row, modifiers: crossterm::event::KeyModifiers::NONE }
    }

    fn setup_app_with_areas() -> (TempTestDir, App) {
        let dir = TempTestDir::new("mdt-test-mouse");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 10;
        // Set up areas: file list on left, preview on right
        app.file_list_area = Some(Rect::new(0, 0, 30, 24));
        app.preview_area = Some(Rect::new(30, 0, 50, 24));
        (dir, app)
    }

    #[test]
    fn scroll_down_in_preview() {
        let (_dir, mut app) = setup_app_with_areas();
        let initial = app.document.scroll_offset;

        app.handle_mouse(mouse_event(MouseEventKind::ScrollDown, 40, 10));

        assert_eq!(app.document.scroll_offset, initial + 3);
    }

    #[test]
    fn scroll_up_in_preview() {
        let (_dir, mut app) = setup_app_with_areas();
        app.document.scroll_offset = 10;

        app.handle_mouse(mouse_event(MouseEventKind::ScrollUp, 40, 10));

        assert_eq!(app.document.scroll_offset, 7);
    }

    #[test]
    fn left_click_in_preview_sets_focus() {
        let (_dir, mut app) = setup_app_with_areas();
        app.focus = Focus::FileList;

        app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 40, 10));

        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn left_click_in_file_list_sets_focus() {
        let (_dir, mut app) = setup_app_with_areas();
        app.focus = Focus::Preview;

        app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 10, 10));

        assert_eq!(app.focus, Focus::FileList);
    }

    #[test]
    fn scroll_outside_areas_does_nothing() {
        let (_dir, mut app) = setup_app_with_areas();
        app.preview_area = None;
        app.file_list_area = None;
        let initial = app.document.scroll_offset;

        app.handle_mouse(mouse_event(MouseEventKind::ScrollDown, 40, 10));

        assert_eq!(app.document.scroll_offset, initial);
    }

    #[test]
    fn scroll_up_at_zero_stays_zero() {
        let (_dir, mut app) = setup_app_with_areas();
        app.document.scroll_offset = 0;

        app.handle_mouse(mouse_event(MouseEventKind::ScrollUp, 40, 10));

        assert_eq!(app.document.scroll_offset, 0);
    }
}
