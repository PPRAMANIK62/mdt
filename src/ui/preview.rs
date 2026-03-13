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
/// Sets `app.document.viewport_height` as a side effect so scroll clamping works.
pub fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.focus == Focus::Preview {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = app
        .document
        .current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " Preview ".to_string());

    let block = Block::default().title(title).borders(Borders::ALL).border_style(border_style);

    // Inner area height (excluding borders) is the viewport.
    let inner = block.inner(area);
    // Side effect: viewport_height must be updated every frame because the terminal
    // can resize at any time. Scroll clamping in input handling depends on this value.
    app.document.viewport_height = inner.height as usize;

    if app.document.rendered_lines.is_empty() {
        let placeholder = Paragraph::new("Select a file to preview").block(block);
        frame.render_widget(placeholder, area);
        return;
    }

    // Clamp scroll offset before rendering.
    let max_scroll = app.document.rendered_lines.len().saturating_sub(app.document.viewport_height);
    if app.document.scroll_offset > max_scroll {
        app.document.scroll_offset = max_scroll;
    }

    // Virtual scrolling: only take the visible slice.
    let end = (app.document.scroll_offset + app.document.viewport_height)
        .min(app.document.rendered_lines.len());
    let visible_slice = &app.document.rendered_lines[app.document.scroll_offset..end];

    let search_active = !app.search.query.is_empty() && !app.search.matches.is_empty();

    let text = if search_active {
        // Search active: clone lines and apply highlighting in a single pass.
        let highlight_style = Style::default().bg(Color::Yellow).fg(Color::Black);
        let query_lower = app.search.query.to_lowercase();
        Text::from(
            visible_slice
                .iter()
                .map(|line| highlight_line(line.clone(), &query_lower, highlight_style))
                .collect::<Vec<_>>(),
        )
    } else {
        // No search: borrow string content from rendered_lines — avoids deep-cloning
        // string data every frame. Only Vec<Line>/Vec<Span> wrappers are allocated;
        // the actual text bytes stay zero-copy via Cow::Borrowed.
        let lines: Vec<Line<'_>> = visible_slice
            .iter()
            .map(|line| {
                let spans: Vec<Span<'_>> =
                    line.spans.iter().map(|s| Span::styled(s.content.as_ref(), s.style)).collect();
                Line { spans, style: line.style, alignment: line.alignment }
            })
            .collect();
        Text::from(lines)
    };

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
    Line { spans: new_spans, style: line.style, alignment: line.alignment }
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
            result.push(Span::styled(text[last_end..abs_pos].to_string(), original_style));
        }

        // Add matched text with highlight style.
        result.push(Span::styled(text[abs_pos..match_end].to_string(), highlight_style));

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
