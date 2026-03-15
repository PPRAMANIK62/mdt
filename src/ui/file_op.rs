//! File operation overlay drawing.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{modal, theme};
use crate::app::{App, Overlay};

pub(super) fn draw_file_op_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let Overlay::FileOp(op) = &app.overlay else {
        return;
    };

    let (title, hint, shortcuts, show_input, placeholder) = match op {
        crate::app::FileOp::CreateFile { parent_dir } => {
            let rel = parent_dir.strip_prefix(&app.root_path).unwrap_or(parent_dir);
            let dir_display = if rel.as_os_str().is_empty() {
                "./".to_string()
            } else {
                format!("{}/", rel.display())
            };
            ("New File", format!("in {dir_display}"), vec![("confirm", "enter")], true, "filename")
        }
        crate::app::FileOp::CreateDir { parent_dir } => {
            let rel = parent_dir.strip_prefix(&app.root_path).unwrap_or(parent_dir);
            let dir_display = if rel.as_os_str().is_empty() {
                "./".to_string()
            } else {
                format!("{}/", rel.display())
            };
            (
                "New Directory",
                format!("in {dir_display}"),
                vec![("confirm", "enter")],
                true,
                "dirname",
            )
        }
        crate::app::FileOp::Rename { target, .. } => {
            let name = target.file_name().unwrap_or_default().to_string_lossy();
            ("Rename", format!("current: {name}"), vec![("confirm", "enter")], true, "new name")
        }
        crate::app::FileOp::Delete { name, is_dir, .. } => {
            let what = if *is_dir { "directory" } else { "file" };
            (
                "Delete",
                format!("Delete {what} \"{name}\"?"),
                vec![("confirm", "enter"), ("cancel", "esc")],
                false,
                "",
            )
        }
        crate::app::FileOp::Move { source, .. } => {
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            ("Move", format!("moving: {name}"), vec![("confirm", "enter")], true, "destination")
        }
    };

    let popup_area = modal::centered_rect(50, 11, area);

    let input = if show_input { Some((app.file_op_input.as_str(), placeholder)) } else { None };
    let shortcut_refs: Vec<(&str, &str)> = shortcuts.iter().map(|(a, b)| (*a, *b)).collect();

    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        title,
        input,
        &shortcut_refs,
        app.bg_color,
        app.cursor.visible,
    );

    let hint_line = Line::from(Span::styled(&*hint, theme::MODAL_HINT));
    frame.render_widget(Paragraph::new(hint_line), content_area);
}
