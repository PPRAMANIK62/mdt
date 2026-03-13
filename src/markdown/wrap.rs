//! Text wrapping engine for styled spans.

use ratatui::style::Style;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Word-wrap a slice of styled [`Span`]s to fit within `max_width` columns.
///
/// Returns a `Vec` of visual lines, each a `Vec<Span>`. Styles are preserved
/// across split points — when a span must be broken, both halves retain the
/// original style. Trailing whitespace at wrap boundaries is trimmed.
///
/// Special cases:
/// - Empty input returns `vec![vec![]]`.
/// - `max_width == 0` returns one line per grapheme cluster (or `vec![vec![]]` if empty).
pub(super) fn wrap_spans(spans: &[Span], max_width: usize) -> Vec<Vec<Span<'static>>> {
    if spans.is_empty() {
        return vec![vec![]];
    }

    // Handle max_width == 0: one grapheme per line.
    if max_width == 0 {
        let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
        for span in spans {
            for g in span.content.graphemes(true) {
                let w = UnicodeWidthStr::width(g);
                if w > 0 {
                    lines.push(vec![Span::styled(g.to_string(), span.style)]);
                }
            }
        }
        if lines.is_empty() {
            return vec![vec![]];
        }
        return lines;
    }

    // Collect all graphemes with their style and display width.
    struct Grapheme<'a> {
        text: &'a str,
        style: Style,
        width: usize,
    }

    let estimated_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let mut graphemes: Vec<Grapheme<'_>> = Vec::with_capacity(estimated_len);
    for span in spans {
        for g in span.content.graphemes(true) {
            graphemes.push(Grapheme {
                text: g,
                style: span.style,
                width: UnicodeWidthStr::width(g),
            });
        }
    }

    // Split graphemes into words (whitespace-delimited chunks).
    // Each word is a run of non-whitespace graphemes, and whitespace is kept as
    // separate single-grapheme "words" so we can decide whether to emit or trim them.
    struct Word<'a> {
        graphemes: Vec<Grapheme<'a>>,
        width: usize,
        is_whitespace: bool,
    }

    let mut words: Vec<Word<'_>> = Vec::with_capacity(estimated_len / 4 + 1);
    let mut i = 0;
    while i < graphemes.len() {
        let is_ws = graphemes[i].text.chars().all(char::is_whitespace);
        if is_ws {
            // Each whitespace grapheme is its own word for trimming control.
            words.push(Word {
                width: graphemes[i].width,
                is_whitespace: true,
                graphemes: vec![], // placeholder, replaced below
            });
            // Move the grapheme out efficiently.
            let g = std::mem::replace(
                &mut graphemes[i],
                Grapheme { text: "", style: Style::default(), width: 0 },
            );
            words.last_mut().unwrap().graphemes = vec![g];
            i += 1;
        } else {
            // Accumulate non-whitespace graphemes into one word.
            let mut word_gs = Vec::new();
            let mut w = 0;
            while i < graphemes.len() && !graphemes[i].text.chars().all(char::is_whitespace) {
                let g = std::mem::replace(
                    &mut graphemes[i],
                    Grapheme { text: "", style: Style::default(), width: 0 },
                );
                w += g.width;
                word_gs.push(g);
                i += 1;
            }
            words.push(Word { graphemes: word_gs, width: w, is_whitespace: false });
        }
    }

    // Lay out words into lines respecting max_width.
    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
    let mut cur_spans: Vec<Span<'static>> = Vec::new();
    let mut cur_width: usize = 0;

    // Helper: push graphemes into cur_spans, merging consecutive same-style runs.
    fn push_graphemes(cur_spans: &mut Vec<Span<'static>>, gs: &[Grapheme<'_>]) {
        for g in gs {
            if let Some(last) = cur_spans.last_mut() {
                if last.style == g.style {
                    // Merge into existing span.
                    last.content = std::borrow::Cow::Owned(format!("{}{}", last.content, g.text));
                    continue;
                }
            }
            cur_spans.push(Span::styled(g.text.to_owned(), g.style));
        }
    }

    fn flush_line(
        lines: &mut Vec<Vec<Span<'static>>>,
        cur_spans: &mut Vec<Span<'static>>,
        cur_width: &mut usize,
    ) {
        // Trim trailing whitespace from the last span.
        if let Some(last) = cur_spans.last_mut() {
            let trimmed = last.content.trim_end().to_string();
            if trimmed.is_empty() {
                cur_spans.pop();
            } else {
                last.content = std::borrow::Cow::Owned(trimmed);
            }
        }
        lines.push(std::mem::take(cur_spans));
        *cur_width = 0;
    }

    for word in &words {
        if word.is_whitespace {
            // Only emit whitespace if it fits and we're not at line start.
            if cur_width > 0 && cur_width + word.width <= max_width {
                push_graphemes(&mut cur_spans, &word.graphemes);
                cur_width += word.width;
            }
            // Otherwise skip (trim at boundaries).
            continue;
        }

        // Non-whitespace word.
        if word.width <= max_width {
            // Word fits on a line.
            if cur_width + word.width <= max_width {
                // Fits on current line.
                push_graphemes(&mut cur_spans, &word.graphemes);
                cur_width += word.width;
            } else {
                // Wrap: start new line.
                flush_line(&mut lines, &mut cur_spans, &mut cur_width);
                push_graphemes(&mut cur_spans, &word.graphemes);
                cur_width += word.width;
            }
        } else {
            // Word too wide — character-wrap it.
            for g in &word.graphemes {
                // CJK handling: if a double-width char would leave 1 col, move to next line.
                if g.width == 2 && cur_width + 2 > max_width {
                    flush_line(&mut lines, &mut cur_spans, &mut cur_width);
                }
                if cur_width + g.width > max_width {
                    flush_line(&mut lines, &mut cur_spans, &mut cur_width);
                }
                push_graphemes(&mut cur_spans, std::slice::from_ref(g));
                cur_width += g.width;
            }
        }
    }

    // Don't forget the last line.
    if !cur_spans.is_empty() {
        flush_line(&mut lines, &mut cur_spans, &mut cur_width);
    }

    if lines.is_empty() {
        vec![vec![]]
    } else {
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;
    use ratatui::text::Span;

    /// Helper: collect text content from wrap_spans output lines.
    fn wrap_lines_content(lines: &[Vec<Span<'_>>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect()
    }

    #[test]
    fn wrap_spans_basic_word_wrap() {
        let spans = vec![Span::raw("hello world foo")];
        let lines = wrap_spans(&spans, 10);
        let content = wrap_lines_content(&lines);
        assert_eq!(content.len(), 2);
        assert!(content[0].len() <= 10, "first line too wide: {:?}", content[0]);
        assert!(content[1].len() <= 10, "second line too wide: {:?}", content[1]);
        assert_eq!(content[0], "hello");
        assert_eq!(content[1], "world foo");
    }

    #[test]
    fn wrap_spans_character_wrap() {
        let spans = vec![Span::raw("abcdefghij")];
        let lines = wrap_spans(&spans, 5);
        let content = wrap_lines_content(&lines);
        assert_eq!(content, vec!["abcde", "fghij"]);
    }

    #[test]
    fn wrap_spans_style_preservation() {
        let style = Style::new().add_modifier(Modifier::BOLD);
        let spans = vec![Span::styled("abcdefghij", style)];
        let lines = wrap_spans(&spans, 5);
        assert_eq!(lines.len(), 2);
        // Both halves must retain BOLD.
        for line in &lines {
            for span in line {
                assert!(
                    span.style.add_modifier.contains(Modifier::BOLD),
                    "Style lost after split: {:?}",
                    span
                );
            }
        }
        let content = wrap_lines_content(&lines);
        assert_eq!(content, vec!["abcde", "fghij"]);
    }

    #[test]
    fn wrap_spans_multi_span() {
        let spans = vec![
            Span::raw("hello "),
            Span::styled("world", Style::new().add_modifier(Modifier::BOLD)),
        ];
        let lines = wrap_spans(&spans, 5);
        let content = wrap_lines_content(&lines);
        assert_eq!(content.len(), 2);
        assert_eq!(content[0], "hello");
        assert_eq!(content[1], "world");
        // "world" should be bold.
        let world_span = &lines[1][0];
        assert!(world_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn wrap_spans_empty_input() {
        let result = wrap_spans(&[], 80);
        assert_eq!(result, vec![vec![] as Vec<Span<'static>>]);
    }

    #[test]
    fn wrap_spans_width_exact() {
        let spans = vec![Span::raw("12345")];
        let lines = wrap_spans(&spans, 5);
        let content = wrap_lines_content(&lines);
        assert_eq!(content.len(), 1);
        assert_eq!(content[0], "12345");
    }

    #[test]
    fn wrap_spans_trailing_whitespace() {
        // "hello " followed by "world" — at width 6, "hello" fits (5 chars),
        // the trailing space should be trimmed, and "world" on next line.
        let spans = vec![Span::raw("hello world")];
        let lines = wrap_spans(&spans, 6);
        let content = wrap_lines_content(&lines);
        assert_eq!(content.len(), 2);
        assert!(!content[0].ends_with(' '), "trailing space not trimmed: {:?}", content[0]);
        assert!(!content[1].starts_with(' '), "leading space on wrapped line: {:?}", content[1]);
    }

    #[test]
    fn wrap_spans_multiple_spaces() {
        let spans = vec![Span::raw("a  b")];
        let lines = wrap_spans(&spans, 10);
        let content = wrap_lines_content(&lines);
        // Should fit on one line; consecutive spaces preserved when they fit.
        assert_eq!(content.len(), 1);
        assert_eq!(content[0], "a  b");
    }

    #[test]
    fn wrap_spans_already_short() {
        let spans = vec![Span::raw("hi")];
        let lines = wrap_spans(&spans, 80);
        let content = wrap_lines_content(&lines);
        assert_eq!(content.len(), 1);
        assert_eq!(content[0], "hi");
    }

    #[test]
    fn wrap_spans_zero_width() {
        let spans = vec![Span::raw("abc")];
        let lines = wrap_spans(&spans, 0);
        let content = wrap_lines_content(&lines);
        assert_eq!(content, vec!["a", "b", "c"]);
    }

    #[test]
    fn wrap_spans_zero_width_empty() {
        let result = wrap_spans(&[], 0);
        assert_eq!(result, vec![vec![] as Vec<Span<'static>>]);
    }
}
