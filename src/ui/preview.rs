//! Markdown preview rendering with virtual scrolling.
//!
//! Virtual scrolling: only the visible slice of lines is rendered, avoiding
//! the 1-2 second lag that occurs when putting 1000+ lines into a single Paragraph.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus};

/// Draw the preview pane with virtual scrolling.
///
/// Sets `app.viewport_height` as a side effect so scroll clamping works.
pub fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Preview {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
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
    let mut visible_lines = app.rendered_lines[app.scroll_offset..end].to_vec();

    // Apply search highlighting if there's an active search query.
    if !app.search_query.is_empty() && !app.search_matches.is_empty() {
        let highlight_style = Style::default().bg(Color::Yellow).fg(Color::Black);
        let query_lower = app.search_query.to_lowercase();
        visible_lines = visible_lines
            .into_iter()
            .map(|line| highlight_line(line, &query_lower, highlight_style))
            .collect();
    }

    let text = Text::from(visible_lines);
    // scroll((0, 0)) because we already sliced the lines ourselves.
    let paragraph = Paragraph::new(text).block(block).scroll((0, 0));
    frame.render_widget(paragraph, area);
}

/// Highlight all occurrences of `query` (lowercase) in a line by splitting spans.
fn highlight_line<'a>(line: Line<'a>, query: &str, highlight_style: Style) -> Line<'a> {
    let new_spans: Vec<Span<'a>> = line
        .spans
        .into_iter()
        .flat_map(|span| highlight_span(span, query, highlight_style))
        .collect();
    Line {
        spans: new_spans,
        style: line.style,
        alignment: line.alignment,
    }
}

/// Split a single span at match boundaries, applying highlight style to matching portions.
fn highlight_span<'a>(span: Span<'a>, query: &str, highlight_style: Style) -> Vec<Span<'a>> {
    let text = span.content.to_string();
    let text_lower = text.to_lowercase();
    let original_style = span.style;

    let mut result = Vec::new();
    let mut last_end = 0;

    // Find all case-insensitive matches.
    let mut search_start = 0;
    while let Some(pos) = text_lower[search_start..].find(query) {
        let abs_pos = search_start + pos;
        let match_end = abs_pos + query.len();

        // Add pre-match text with original style.
        if abs_pos > last_end {
            result.push(Span::styled(
                text[last_end..abs_pos].to_string(),
                original_style,
            ));
        }

        // Add matched text with highlight style.
        result.push(Span::styled(
            text[abs_pos..match_end].to_string(),
            highlight_style,
        ));

        last_end = match_end;
        search_start = match_end;
    }

    // Add remaining text after last match.
    if last_end < text.len() {
        result.push(Span::styled(text[last_end..].to_string(), original_style));
    }

    // No matches found — return original span unchanged.
    if result.is_empty() {
        return vec![span];
    }

    result
}
