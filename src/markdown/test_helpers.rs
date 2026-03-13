//! Shared test helpers for markdown module tests.

use ratatui::text::Text;
use unicode_width::UnicodeWidthStr;

use super::render_markdown;

/// Helper: collect all text content from a Text, joining spans per line.
pub(super) fn text_content(text: &Text<'_>) -> Vec<String> {
    text.lines
        .iter()
        .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect()
}

/// Render markdown with a specific width constraint.
pub(super) fn render_at_width(input: &str, width: usize) -> Text<'static> {
    render_markdown(input, Some(width))
}

/// Return the maximum visual width across all lines in a `Text`.
pub(super) fn max_line_width(text: &Text) -> usize {
    text.lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.width()).sum::<usize>())
        .max()
        .unwrap_or(0)
}
