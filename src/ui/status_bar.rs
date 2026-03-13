//! Status bar widget.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, AppMode};

/// Draw the status bar at the bottom of the screen.
pub fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // In Command mode, show ":" + command_buffer as the full status bar.
    if app.mode == AppMode::Command {
        let line = Line::from(vec![Span::raw(format!(":{}█", app.command_buffer))]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
        return;
    }

    // In Search mode, show "/" + search_query as the full status bar.
    if app.mode == AppMode::Search {
        let line = Line::from(vec![Span::raw(format!("/{}█", app.search.query))]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
        return;
    }

    // Left: mode indicator (reversed style).
    let mode = format!(" {} ", app.mode);

    // Center: file path + dirty indicator + status message.
    let file_info = app.display_file_path();

    let dirty_indicator = if app.editor.is_dirty { " [+]" } else { "" };

    let center = if !app.status_message.is_empty() {
        if file_info.is_empty() {
            app.status_message.clone()
        } else {
            format!("{}{} {}", file_info, dirty_indicator, app.status_message)
        }
    } else if !file_info.is_empty() {
        format!("{}{}", file_info, dirty_indicator)
    } else {
        String::new()
    };

    // Right: line position when a file is open.
    let right = if app.document.current_file.is_some() && !app.document.rendered_lines.is_empty() {
        format!(
            "Ln {}/{} ",
            app.document.scroll_offset.saturating_add(1),
            app.document.rendered_lines.len()
        )
    } else {
        String::new()
    };

    // Calculate padding to right-align the line position.
    let mode_len = mode.width();
    let center_len = center.width() + 1; // +1 for space after mode
    let right_len = right.width();
    let used = mode_len + center_len + right_len;
    let padding = if area.width as usize > used { area.width as usize - used } else { 1 };

    let line = Line::from(vec![
        Span::styled(mode, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(" "),
        Span::raw(center),
        Span::raw(" ".repeat(padding)),
        Span::raw(right),
    ]);

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}
