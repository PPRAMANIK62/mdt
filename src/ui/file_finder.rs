//! File finder overlay drawing.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::{modal, theme};
use crate::app::App;

const POPUP_WIDTH: u16 = 50;

pub(super) fn draw_file_finder_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let popup_width = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let content_rows = app.file_finder.results.len().max(1);
    let max_height = (area.height * 3 / 4).max(10);
    let popup_height =
        (content_rows as u16 + 10).min(max_height).min(area.height.saturating_sub(4));

    let popup_area = modal::centered_rect(popup_width, popup_height, area);

    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        "Find File",
        Some((&app.file_finder.query, "Search files...")),
        &[("open", "enter"), ("navigate", "↕"), ("close", "esc")],
        app.bg_color,
        app.cursor.visible,
    );

    let mut text_lines: Vec<Line> = Vec::new();

    if app.file_finder.results.is_empty() {
        text_lines.push(Line::from(Span::styled("No matching files", theme::MODAL_HINT)));
    } else {
        let visible_height = content_area.height as usize;
        let selected = app.file_finder.selected;
        let scroll_offset =
            if selected >= visible_height { selected - visible_height + 1 } else { 0 };

        for (i, (rel, _)) in
            app.file_finder.results.iter().enumerate().skip(scroll_offset).take(visible_height)
        {
            let max_text_width = content_area.width as usize;
            let display_width = UnicodeWidthStr::width(rel.as_str());
            let truncated = if display_width > max_text_width {
                let mut width = 0;
                let mut end = 0;
                for (idx, ch) in rel.char_indices() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if width + ch_width > max_text_width.saturating_sub(1) {
                        break;
                    }
                    width += ch_width;
                    end = idx + ch.len_utf8();
                }
                format!("{}…", &rel[..end])
            } else {
                rel.clone()
            };

            if i == selected {
                let padded = format!("{:<width$}", truncated, width = max_text_width);
                text_lines.push(Line::from(Span::styled(padded, theme::MODAL_SELECTED)));
            } else {
                text_lines.push(Line::from(truncated));
            }
        }
    }

    let content = Paragraph::new(text_lines);
    frame.render_widget(content, content_area);
}
