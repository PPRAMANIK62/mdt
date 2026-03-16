//! Markdown preview rendering with virtual scrolling.
//!
//! Virtual scrolling: only the visible slice of lines is rendered, avoiding
//! the 1-2 second lag that occurs when putting 1000+ lines into a single Paragraph.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

use crate::app::App;
use crate::markdown::rewrap_blocks;

/// Draw the preview pane with virtual scrolling.
///
/// # Side Effects (Intentional)
/// This function updates `app.document.viewport_height` and `app.document.viewport_width`
/// on every frame. This follows Ratatui's `StatefulWidget` pattern where layout-dependent
/// state is updated during render, since the actual viewport dimensions are only known at
/// render time (they depend on terminal size, file tree visibility, and padding).
/// Input handlers (scroll, search) depend on these values being current.
pub fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default().padding(Padding::new(2, 2, 1, 0));

    // Inner area height (excluding borders) is the viewport.
    let inner = block.inner(area);
    // Side effect: viewport_height must be updated every frame because the terminal
    // can resize at any time. Scroll clamping in input handling depends on this value.
    app.document.viewport_height = inner.height as usize;

    // Re-render when viewport width changes (e.g. terminal resize, file tree toggle).
    let new_width = inner.width as usize;
    if new_width != app.document.viewport_width && app.document.current_file.is_some() {
        let (lines, block_line_starts) =
            rewrap_blocks(&app.document.rendered_blocks, Some(new_width));
        app.document.rendered_lines = lines;
        app.document.block_line_starts = block_line_starts;
        app.document.rebuild_lower_cache();
        app.document.viewport_width = new_width;
        app.document.rebuild_heading_index();
        // Clamp scroll offset after re-render
        let max_scroll = app.document.rendered_lines.len().saturating_sub(inner.height as usize);
        if app.document.scroll_offset > max_scroll {
            app.document.scroll_offset = max_scroll;
        }
    }

    if app.document.rendered_lines.is_empty() {
        super::welcome::draw_welcome(frame, area, app.bg_color);
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
        let active_style = Style::default().bg(Color::LightRed).fg(Color::Black);
        let query_lower = app.search.query.to_lowercase();
        let current_match_line = app.search.matches.get(app.search.current).copied();
        Text::from(
            visible_slice
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let abs_line = app.document.scroll_offset + i;
                    let style = if current_match_line == Some(abs_line) {
                        active_style
                    } else {
                        highlight_style
                    };
                    highlight_line(line.clone(), &query_lower, style)
                })
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

    // Scrollbar: only render when content exceeds viewport.
    let total_lines = app.document.rendered_lines.len();
    let viewport_height = app.document.viewport_height;
    if total_lines > viewport_height {
        let max_scroll = total_lines.saturating_sub(viewport_height);
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll).position(app.document.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_symbol("┃")
            .thumb_style(Style::default().fg(Color::DarkGray))
            .track_symbol(Some(" "))
            .track_style(Style::default());
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Highlight all occurrences of `query` (lowercase) in a line by splitting spans.
fn highlight_line<'a>(line: Line<'a>, query: &str, highlight_style: Style) -> Line<'a> {
    let mut new_spans: Vec<Span<'a>> = Vec::with_capacity(line.spans.len());
    for span in line.spans {
        new_spans.extend(highlight_span(span, query, highlight_style));
    }
    Line { spans: new_spans, style: line.style, alignment: line.alignment }
}

/// Split a single span at match boundaries, applying highlight style to matching portions.
fn highlight_span<'a>(span: Span<'a>, query: &str, highlight_style: Style) -> Vec<Span<'a>> {
    let text: &str = span.content.as_ref();
    let text_lower = text.to_lowercase();
    let original_style = span.style;

    let mut result = Vec::with_capacity(3);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Position;
    use ratatui::Terminal;

    use crate::app::App;
    use crate::test_util::TempTestDir;

    #[test]
    fn highlight_span_no_match_returns_original() {
        let span = Span::raw("hello world");
        let style = Style::default().fg(Color::Red);
        let result = highlight_span(span.clone(), "xyz", style);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "hello world");
    }

    #[test]
    fn highlight_span_splits_at_match_boundary() {
        let span = Span::raw("hello world!");
        let style = Style::default().fg(Color::Red);
        let result = highlight_span(span, "world", style);
        assert_eq!(result.len(), 3); // "hello " + "world" + "!"
        assert_eq!(result[0].content, "hello ");
        assert_eq!(result[1].content, "world");
        assert_eq!(result[1].style.fg, Some(Color::Red));
        assert_eq!(result[2].content, "!");
    }

    #[test]
    fn highlight_line_highlights_multiple_occurrences() {
        let line = Line::from(vec![Span::raw("foo bar foo baz foo")]);
        let style = Style::default().fg(Color::Yellow);
        let result = highlight_line(line, "foo", style);
        let foo_count = result.spans.iter().filter(|s| s.content == "foo").count();
        assert_eq!(foo_count, 3);
    }

    #[test]
    fn draw_preview_empty_shows_welcome() {
        let dir = TempTestDir::new("mdt-test-preview");
        dir.create_file("test.md", "");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        let backend = TestBackend::new(60, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                draw_preview(f, &mut app, area);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buf.cell(Position::new(x, y)))
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Terminal Markdown Viewer"));
    }
}
