//! Application state and logic.


use ratatui::text::{Line, Span, Text};

/// Convert raw markdown into styled ratatui [`Text`] for rendering in a `Paragraph` widget.
///
/// - Pre-expands tabs to 4 spaces (ratatui `Paragraph` silently drops tab characters).
/// - Respects the `NO_COLOR` environment variable: when set, returns plain unstyled text.
/// - Delegates all markdown parsing and styling to [`tui_markdown::from_str`], which handles
///   headings, bold/italic, strikethrough, inline code, fenced code blocks (syntax-highlighted),
///   blockquotes, lists, task lists, links, YAML front matter, and horizontal rules.
pub fn render_markdown(input: &str) -> Text<'static> {
    // Pre-expand tabs (ratatui Paragraph silently drops tab characters)
    let cleaned = input.replace('\t', "    ");

    // Respect NO_COLOR env var — return plain text when set
    if std::env::var("NO_COLOR").is_ok() {
        return Text::raw(cleaned);
    }

    let text = tui_markdown::from_str(&cleaned);
    text_to_owned(text)
}

/// Convert a borrowed [`Text`] into an owned `Text<'static>` by cloning all string data.
fn text_to_owned(text: Text<'_>) -> Text<'static> {
    let lines: Vec<Line<'static>> = text
        .lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            Line {
                spans,
                style: line.style,
                alignment: line.alignment,
            }
        })
        .collect();
    Text {
        lines,
        style: text.style,
        alignment: text.alignment,
    }
}
