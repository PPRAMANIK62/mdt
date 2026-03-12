//! Markdown editor widget backed by `ratatui-textarea`.

use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_textarea::TextArea;

/// Draw the editor pane (TextArea) in the given area.
pub fn draw_editor(frame: &mut Frame, textarea: &TextArea<'_>, area: Rect) {
    frame.render_widget(textarea, area);
}
