//! Link picker overlay drawing.

use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::{modal, theme};
use crate::markdown::LinkInfo;

pub(super) fn draw_links_overlay(
    frame: &mut Frame,
    area: Rect,
    links: &[&LinkInfo],
    selected: usize,
    search_query: &str,
    bg_color: Color,
    cursor_visible: bool,
) {
    let content_width = links
        .iter()
        .map(|l| UnicodeWidthStr::width(l.display_text.as_str()))
        .max()
        .unwrap_or(20)
        .max(search_query.len() + 15)
        .min(60);
    let popup_width = (content_width as u16 + 10).min(area.width.saturating_sub(4));
    let content_rows = links.len().max(1);
    let max_height = (area.height * 3 / 4).max(10);
    let popup_height =
        (content_rows as u16 + 10).min(max_height).min(area.height.saturating_sub(4));

    let popup_area = modal::centered_rect(popup_width, popup_height, area);

    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        "Links",
        Some((search_query, "Search")),
        &[("open", "enter"), ("navigate", "↕"), ("close", "esc")],
        bg_color,
        cursor_visible,
    );

    let mut text_lines: Vec<Line> = Vec::new();

    if links.is_empty() {
        text_lines.push(Line::from(Span::styled("No matching links", theme::MODAL_HINT)));
    } else {
        let visible_height = content_area.height as usize;
        let scroll_offset =
            if selected >= visible_height { selected - visible_height + 1 } else { 0 };

        for (i, link) in links.iter().enumerate().skip(scroll_offset).take(visible_height) {
            let display = link.display_text.clone();
            let max_text_width = content_area.width as usize;
            let display_width = UnicodeWidthStr::width(display.as_str());
            let truncated = if display_width > max_text_width {
                let mut width = 0;
                let mut end = 0;
                for (i, ch) in display.char_indices() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if width + ch_width > max_text_width.saturating_sub(1) {
                        break;
                    }
                    width += ch_width;
                    end = i + ch.len_utf8();
                }
                format!("{}…", &display[..end])
            } else {
                display
            };

            if i == selected {
                let padded = format!("{:<width$}", truncated, width = max_text_width);
                text_lines.push(Line::from(Span::styled(padded, theme::MODAL_SELECTED)));
            } else {
                text_lines.push(Line::from(truncated));
            }
        }
    }

    let links_content = Paragraph::new(text_lines);
    frame.render_widget(links_content, content_area);
}
