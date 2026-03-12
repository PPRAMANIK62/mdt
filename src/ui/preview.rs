//! Markdown preview rendering with virtual scrolling.
//!
//! Virtual scrolling: only the visible slice of lines is rendered, avoiding
//! the 1-2 second lag that occurs when putting 1000+ lines into a single Paragraph.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus};

/// Draw the preview pane with virtual scrolling.
///
/// Sets `app.viewport_height` as a side effect so scroll clamping works.
pub fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Preview {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let title = app
        .current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " Preview ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    // Inner area height (excluding borders) is the viewport.
    let inner = block.inner(area);
    app.viewport_height = inner.height as usize;

    if app.rendered_lines.is_empty() {
        let placeholder = Paragraph::new("Select a file to preview").block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    // Clamp scroll offset before rendering.
    let max_scroll = app.rendered_lines.len().saturating_sub(app.viewport_height);
    if app.scroll_offset > max_scroll {
        app.scroll_offset = max_scroll;
    }

    // Virtual scrolling: only take the visible slice.
    let end = (app.scroll_offset + app.viewport_height).min(app.rendered_lines.len());
    let visible_lines = app.rendered_lines[app.scroll_offset..end].to_vec();

    let text = Text::from(visible_lines);
    // scroll((0, 0)) because we already sliced the lines ourselves.
    let paragraph = Paragraph::new(text).block(block).scroll((0, 0));
    frame.render_widget(paragraph, area);
}
