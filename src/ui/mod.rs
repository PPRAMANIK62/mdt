pub mod editor;
mod file_finder;
mod file_list;
mod file_op;
mod help;
mod link_picker;
pub mod modal;
pub mod preview;
pub mod status_bar;
pub mod theme;
pub mod welcome;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::app::{App, Overlay, SplitOrientation};
use crate::markdown::LinkInfo;

/// Minimum editor area width (columns) for horizontal preview split.
const MIN_HORIZONTAL_WIDTH: u16 = 40;
/// Minimum editor area height (rows) for vertical preview split.
const MIN_VERTICAL_HEIGHT: u16 = 10;

/// Render editor + optional live preview in the given content area.
///
/// Uses scoped borrows to avoid holding an immutable borrow of
/// `app.editor.textarea` while passing `&mut app` to `draw_live_preview`.
fn draw_editor_area(frame: &mut Frame, app: &mut App, content_area: Rect) {
    let has_editor = app.editor.textarea.is_some();

    if has_editor {
        let wants_split = app.live_preview.enabled;
        let can_split = if wants_split {
            match app.live_preview.orientation {
                SplitOrientation::Horizontal => content_area.width >= MIN_HORIZONTAL_WIDTH,
                SplitOrientation::Vertical => content_area.height >= MIN_VERTICAL_HEIGHT,
            }
        } else {
            false
        };

        if wants_split && can_split {
            let chunks = match app.live_preview.orientation {
                SplitOrientation::Horizontal => {
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(content_area)
                }
                SplitOrientation::Vertical => {
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(content_area)
                }
            };
            // Record editor inner height (area minus block borders) for scroll sync.
            app.live_preview.editor_inner_height =
                chunks[0].height.saturating_sub(2) as usize;
            // Scoped borrow: draw editor first, borrow drops before draw_live_preview.
            {
                let textarea = app.editor.textarea.as_ref().unwrap();
                editor::draw_editor(frame, textarea, chunks[0]);
            }
            preview::draw_live_preview(frame, app, chunks[1]);
            app.preview_area = None;
        } else {
            // Fallback: editor only (preview disabled, or terminal too small)
            let textarea = app.editor.textarea.as_ref().unwrap();
            editor::draw_editor(frame, textarea, content_area);
            app.preview_area = None;
        }
    } else {
        app.preview_area = Some(content_area);
        preview::draw_preview(frame, app, content_area);
    }
}

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
        draw_editor_area(frame, app, main_chunks[1]);
    } else {
        app.file_list_area = None;
        draw_editor_area(frame, app, main_area);
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
        Overlay::FileFinder => {
            file_finder::draw_file_finder_overlay(frame, frame.area(), app);
        }
        Overlay::FileOp(_) => {
            file_op::draw_file_op_overlay(frame, frame.area(), app);
        }
        Overlay::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempTestDir;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    #[test]
    fn draw_with_live_preview_does_not_panic() {
        let dir = TempTestDir::new("mdt-test-ui-live-preview");
        dir.create_file("test.md", "# Hello\n\nSome content here.");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.live_preview.enabled = true;
        app.update_live_preview();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
    }
}
