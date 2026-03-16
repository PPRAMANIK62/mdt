pub mod editor;
mod file_list;
mod file_op;
mod help;
mod link_picker;
pub mod modal;
pub mod preview;
pub mod status_bar;
pub mod theme;
pub mod welcome;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::app::{App, Overlay};
use crate::markdown::LinkInfo;

/// Draw the full UI: file list | preview, plus status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    // Fill entire frame with solid terminal background color to prevent transparency.
    frame.render_widget(Block::default().style(Style::default().bg(app.bg_color)), frame.area());

    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    if app.show_file_tree {
        let main_chunks =
            Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(main_area);
        file_list::draw_file_list(frame, app, main_chunks[0]);
        app.file_list_area = Some(main_chunks[0]);
        let content_area = main_chunks[1];
        if let Some(ref textarea) = app.editor.textarea {
            editor::draw_editor(frame, textarea, content_area);
            app.preview_area = None;
        } else {
            app.preview_area = Some(content_area);
            preview::draw_preview(frame, app, content_area);
        }
    } else {
        app.file_list_area = None;
        if let Some(ref textarea) = app.editor.textarea {
            editor::draw_editor(frame, textarea, main_area);
            app.preview_area = None;
        } else {
            app.preview_area = Some(main_area);
            preview::draw_preview(frame, app, main_area);
        }
    }

    // --- Status bar ---
    status_bar::draw_status_bar(frame, app, status_area);

    // --- Overlays (rendered last so they're on top) ---
    match app.overlay {
        Overlay::Help => {
            help::draw_help_overlay(frame, frame.area(), app.bg_color);
        }
        Overlay::LinkPicker => {
            let filtered_indices: Vec<usize> = app.filtered_link_indices().to_vec();
            let filtered_links: Vec<&LinkInfo> =
                filtered_indices.iter().filter_map(|&i| app.document.links.get(i)).collect();
            link_picker::draw_links_overlay(
                frame,
                frame.area(),
                &filtered_links,
                app.link_picker.selected,
                &app.link_picker.search_query,
                app.bg_color,
                app.cursor.visible,
            );
        }
        Overlay::FileOp(_) => {
            file_op::draw_file_op_overlay(frame, frame.area(), app);
        }
        Overlay::None => {}
    }
}
