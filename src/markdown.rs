//! Custom markdown-to-ratatui renderer using pulldown-cmark.
//!
//! Produces styled [`Text`] with all syntax markers stripped — headings, bold, italic,
//! strikethrough, inline code, code blocks, lists, blockquotes, links, horizontal rules,
//! and task lists are all rendered as properly styled text.

use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Render markdown input into styled ratatui [`Text`].
///
/// - Pre-expands tabs to 4 spaces (ratatui `Paragraph` silently drops tabs).
/// - Respects the `NO_COLOR` environment variable: when set, returns plain unstyled text.
/// - All markdown syntax markers are stripped; styling is applied via ratatui modifiers/colors.
pub fn render_markdown(input: &str) -> Text<'static> {
    let cleaned = input.replace('\t', "    ");

    if std::env::var("NO_COLOR").is_ok() {
        return Text::raw(cleaned);
    }

    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(&cleaned, options);

    let mut renderer = Renderer::new();
    renderer.run(parser);
    renderer.into_text()
}

// ── Styles ──────────────────────────────────────────────────────────────────

const H1_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Cyan);
const H2_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Green);
const H3_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);
const H4_STYLE: Style = Style::new()
    .add_modifier(Modifier::BOLD)
    .fg(Color::DarkGray);
const BOLD_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);
const ITALIC_STYLE: Style = Style::new().add_modifier(Modifier::ITALIC);
const STRIKETHROUGH_STYLE: Style = Style::new().add_modifier(Modifier::CROSSED_OUT);
const INLINE_CODE_STYLE: Style = Style::new().fg(Color::Yellow);
const LINK_STYLE: Style = Style::new()
    .add_modifier(Modifier::UNDERLINED)
    .fg(Color::Blue);
const BLOCKQUOTE_STYLE: Style = Style::new().fg(Color::DarkGray);
const CODE_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
const CODE_DEFAULT_STYLE: Style = Style::new().fg(Color::White);
const HR_STYLE: Style = Style::new().fg(Color::DarkGray);

// ── Renderer ────────────────────────────────────────────────────────────────

struct Renderer {
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    /// Stack of list contexts: None = unordered, Some(n) = ordered starting at n.
    list_stack: Vec<Option<u64>>,
    /// Whether we're inside a code block.
    in_code_block: bool,
    /// Language hint for the current code block.
    code_block_lang: Option<String>,
    /// Accumulated code block content.
    code_block_buf: String,
    /// Current nesting depth for blockquotes.
    blockquote_depth: usize,
    /// Whether the next text event is the first in a list item (needs marker).
    pending_list_marker: bool,
    /// Track if we need a blank line separator.
    needs_newline: bool,
    /// Current link destination (Some while inside a link).
    link_dest: Option<String>,
    /// Whether we're inside a heading (to apply heading style to all text).
    in_heading: bool,
}

impl Renderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: Vec::new(),
            list_stack: Vec::new(),
            in_code_block: false,
            code_block_lang: None,
            code_block_buf: String::new(),
            blockquote_depth: 0,
            pending_list_marker: false,
            needs_newline: false,
            link_dest: None,
            in_heading: false,
        }
    }

    fn run<'a>(&mut self, parser: impl Iterator<Item = Event<'a>>) {
        for event in parser {
            self.handle_event(event);
        }
        self.flush_line();
    }

    fn into_text(self) -> Text<'static> {
        Text::from(self.lines)
    }

    // ── Event dispatch ──────────────────────────────────────────────────

    fn handle_event<'a>(&mut self, event: Event<'a>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.on_text(text),
            Event::Code(code) => self.on_inline_code(code),
            Event::SoftBreak => self.on_soft_break(),
            Event::HardBreak => self.on_hard_break(),
            Event::Rule => self.on_rule(),
            Event::TaskListMarker(checked) => self.on_task_list_marker(checked),
            Event::Html(html) => self.on_html(html),
            Event::InlineHtml(html) => self.on_inline_html(html),
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    // ── Start tags ──────────────────────────────────────────────────────

    fn start_tag<'a>(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Heading { level, .. } => {
                if self.needs_newline {
                    self.push_blank_line();
                }
                let style = match level {
                    HeadingLevel::H1 => H1_STYLE,
                    HeadingLevel::H2 => H2_STYLE,
                    HeadingLevel::H3 => H3_STYLE,
                    HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6 => H4_STYLE,
                };
                self.style_stack.push(style);
                self.in_heading = true;
            }
            Tag::Paragraph => {
                if self.needs_newline && !self.is_in_list_item() {
                    self.push_blank_line();
                }
            }
            Tag::Strong => {
                self.push_merged_style(BOLD_STYLE);
            }
            Tag::Emphasis => {
                self.push_merged_style(ITALIC_STYLE);
            }
            Tag::Strikethrough => {
                self.push_merged_style(STRIKETHROUGH_STYLE);
            }
            Tag::BlockQuote(_) => {
                if self.needs_newline {
                    self.push_blank_line();
                }
                self.blockquote_depth += 1;
            }
            Tag::CodeBlock(kind) => {
                if self.needs_newline {
                    self.push_blank_line();
                }
                self.in_code_block = true;
                self.code_block_buf.clear();
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang = lang.split_whitespace().next().unwrap_or("").to_string();
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
            }
            Tag::List(start) => {
                if self.list_stack.is_empty() && self.needs_newline {
                    self.push_blank_line();
                }
                if !self.list_stack.is_empty() {
                    self.flush_line();
                }
                self.list_stack.push(start);
            }
            Tag::Item => {
                self.pending_list_marker = true;
            }
            Tag::Link { dest_url, .. } => {
                self.link_dest = Some(dest_url.to_string());
                self.push_merged_style(LINK_STYLE);
            }
            Tag::Image { .. }
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::FootnoteDefinition(_)
            | Tag::HtmlBlock
            | Tag::MetadataBlock(_) => {}
            _ => {}
        }
    }

    // ── End tags ────────────────────────────────────────────────────────

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.in_heading = false;
                self.style_stack.pop();
                self.flush_line();
                self.needs_newline = true;
            }
            TagEnd::Paragraph => {
                self.flush_line();
                self.needs_newline = true;
            }
            TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                self.style_stack.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.needs_newline = true;
            }
            TagEnd::CodeBlock => {
                self.render_code_block();
                self.in_code_block = false;
                self.code_block_lang = None;
                self.needs_newline = true;
            }
            TagEnd::List(_) => {
                self.list_stack.pop();
                if self.list_stack.is_empty() {
                    self.needs_newline = true;
                }
            }
            TagEnd::Item => {
                self.flush_line();
                self.pending_list_marker = false;
            }
            TagEnd::Link => {
                self.style_stack.pop();
                if let Some(dest) = self.link_dest.take() {
                    if !dest.is_empty() {
                        let url_span =
                            Span::styled(format!(" ({})", dest), Style::new().fg(Color::DarkGray));
                        self.current_spans.push(url_span);
                    }
                }
            }
            TagEnd::Image
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
            | TagEnd::FootnoteDefinition
            | TagEnd::HtmlBlock
            | TagEnd::MetadataBlock(_) => {}
            _ => {}
        }
    }

    // ── Inline events ───────────────────────────────────────────────────

    fn on_text<'a>(&mut self, text: CowStr<'a>) {
        if self.in_code_block {
            self.code_block_buf.push_str(&text);
            return;
        }

        let style = self.current_style();

        // If there's a pending list marker, emit it first.
        if self.pending_list_marker {
            self.emit_list_marker();
            self.pending_list_marker = false;
        }

        // Handle multi-line text (e.g., from HTML blocks).
        for (i, line) in text.lines().enumerate() {
            if i > 0 {
                self.flush_line();
            }
            if !line.is_empty() {
                self.current_spans
                    .push(Span::styled(line.to_string(), style));
            }
        }
    }

    fn on_inline_code<'a>(&mut self, code: CowStr<'a>) {
        if self.pending_list_marker {
            self.emit_list_marker();
            self.pending_list_marker = false;
        }
        // Render inline code with distinct style, padded with spaces.
        let span = Span::styled(format!(" {} ", code), INLINE_CODE_STYLE);
        self.current_spans.push(span);
    }

    fn on_soft_break(&mut self) {
        if self.in_code_block {
            self.code_block_buf.push('\n');
            return;
        }
        // Treat soft break as a space in inline context.
        let style = self.current_style();
        self.current_spans.push(Span::styled(" ", style));
    }

    fn on_hard_break(&mut self) {
        self.flush_line();
    }

    fn on_rule(&mut self) {
        self.flush_line();
        if !self.lines.is_empty() {
            self.push_blank_line();
        }
        let rule = "────────────────────────────────────────";
        self.lines.push(Line::from(Span::styled(rule, HR_STYLE)));
        self.needs_newline = true;
    }

    fn on_task_list_marker(&mut self, checked: bool) {
        // Replace the pending bullet/number marker with a checkbox.
        let checkbox = if checked { "☑ " } else { "☐ " };
        let indent = self.list_indent_prefix();
        let style = self.current_style();
        self.current_spans
            .push(Span::styled(indent, Style::default()));
        self.current_spans.push(Span::styled(checkbox, style));
        // Mark that we've already emitted the marker.
        self.pending_list_marker = false;
    }

    fn on_html<'a>(&mut self, html: CowStr<'a>) {
        // Render HTML blocks as plain text.
        for (i, line) in html.lines().enumerate() {
            if i > 0 {
                self.flush_line();
            }
            self.current_spans
                .push(Span::styled(line.to_string(), Style::default()));
        }
        self.flush_line();
    }

    fn on_inline_html<'a>(&mut self, html: CowStr<'a>) {
        // Strip inline HTML tags, render content if any.
        let content = html.to_string();
        if !content.is_empty() {
            self.current_spans
                .push(Span::styled(content, self.current_style()));
        }
    }

    // ── Code block rendering ────────────────────────────────────────────

    fn render_code_block(&mut self) {
        let code = std::mem::take(&mut self.code_block_buf);
        let lang = self.code_block_lang.clone().unwrap_or_default();

        // Highlight first so we can measure widths.
        let highlighted_lines = self.highlight_code(&code, &lang);

        // Calculate display width of each line's spans.
        fn spans_display_width(spans: &[Span<'_>]) -> usize {
            spans.iter().map(|s| s.content.chars().count()).sum()
        }

        let max_width = highlighted_lines
            .iter()
            .map(|spans| spans_display_width(spans))
            .max()
            .unwrap_or(20)
            .max(20);

        // inner = " code_padded " = max_width + 2 (one space each side)
        let inner = max_width + 2;

        // Header: ┌─ lang ─...─┐
        let header_text = if lang.is_empty() {
            format!("┌{}┐", "─".repeat(inner))
        } else {
            let label = format!("─ {} ─", lang);
            let label_width = label.chars().count();
            let remaining = inner.saturating_sub(label_width);
            format!("┌{}{}┐", label, "─".repeat(remaining))
        };
        self.lines
            .push(Line::from(Span::styled(header_text, CODE_BORDER_STYLE)));

        // Code lines with right border.
        for line_spans in highlighted_lines {
            let content_width = spans_display_width(&line_spans);
            let padding = max_width.saturating_sub(content_width);
            let mut spans = vec![Span::styled("│ ", CODE_BORDER_STYLE)];
            spans.extend(line_spans);
            spans.push(Span::styled(
                format!("{} │", " ".repeat(padding)),
                CODE_BORDER_STYLE,
            ));
            self.lines.push(Line::from(spans));
        }

        // Footer: └─...─┘
        self.lines.push(Line::from(Span::styled(
            format!("└{}┘", "─".repeat(inner)),
            CODE_BORDER_STYLE,
        )));
    }

    fn highlight_code(&self, code: &str, lang: &str) -> Vec<Vec<Span<'static>>> {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();

        // Try to find syntax for the language.
        let syntax = if lang.is_empty() {
            None
        } else {
            ss.find_syntax_by_token(lang)
        };

        match syntax {
            Some(syntax) => {
                let theme = &ts.themes["base16-ocean.dark"];
                let mut h = HighlightLines::new(syntax, theme);
                let mut result = Vec::new();

                for line in LinesWithEndings::from(code) {
                    let ranges = h.highlight_line(line, &ss).unwrap_or_default();
                    let spans: Vec<Span<'static>> = ranges
                        .into_iter()
                        .map(|(hl_style, text)| {
                            let fg = Color::Rgb(
                                hl_style.foreground.r,
                                hl_style.foreground.g,
                                hl_style.foreground.b,
                            );
                            let mut style = Style::new().fg(fg);
                            if hl_style.font_style.contains(FontStyle::BOLD) {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            if hl_style.font_style.contains(FontStyle::ITALIC) {
                                style = style.add_modifier(Modifier::ITALIC);
                            }
                            Span::styled(text.trim_end_matches('\n').to_string(), style)
                        })
                        .collect();
                    result.push(spans);
                }
                result
            }
            None => {
                // No syntax found — render with uniform code style.
                code.lines()
                    .map(|line| vec![Span::styled(line.to_string(), CODE_DEFAULT_STYLE)])
                    .collect()
            }
        }
    }

    // ── List helpers ────────────────────────────────────────────────────

    fn emit_list_marker(&mut self) {
        let depth = self.list_stack.len();
        let indent = self.list_indent_prefix();

        if let Some(list_type) = self.list_stack.last_mut() {
            match list_type {
                None => {
                    // Unordered list — use bullet character.
                    let bullet = if depth <= 1 {
                        "•"
                    } else if depth == 2 {
                        "◦"
                    } else {
                        "▪"
                    };
                    self.current_spans
                        .push(Span::styled(indent, Style::default()));
                    self.current_spans.push(Span::styled(
                        format!("{bullet} "),
                        Style::new().fg(Color::DarkGray),
                    ));
                }
                Some(ref mut num) => {
                    // Ordered list — use number.
                    self.current_spans
                        .push(Span::styled(indent, Style::default()));
                    self.current_spans.push(Span::styled(
                        format!("{num}. "),
                        Style::new().fg(Color::DarkGray),
                    ));
                    *num += 1;
                }
            }
        }
    }

    fn list_indent_prefix(&self) -> String {
        let depth = self.list_stack.len();
        if depth <= 1 {
            String::new()
        } else {
            "  ".repeat(depth - 1)
        }
    }

    fn is_in_list_item(&self) -> bool {
        !self.list_stack.is_empty()
    }

    // ── Style helpers ───────────────────────────────────────────────────

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_merged_style(&mut self, new_style: Style) {
        let current = self.current_style();
        self.style_stack.push(current.patch(new_style));
    }

    // ── Line management ─────────────────────────────────────────────────

    fn flush_line(&mut self) {
        if self.current_spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.current_spans);

        // Prepend blockquote indicators if inside a blockquote.
        if self.blockquote_depth > 0 {
            let mut final_spans = Vec::new();
            for _ in 0..self.blockquote_depth {
                final_spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
            }
            final_spans.extend(spans);
            self.lines.push(Line::from(final_spans));
        } else {
            self.lines.push(Line::from(spans));
        }
    }

    fn push_blank_line(&mut self) {
        self.flush_line();
        if self.blockquote_depth > 0 {
            let mut spans = Vec::new();
            for _ in 0..self.blockquote_depth {
                spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
            }
            self.lines.push(Line::from(spans));
        } else {
            self.lines.push(Line::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: collect all text content from a Text, joining spans per line.
    fn text_content(text: &Text<'_>) -> Vec<String> {
        text.lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }


    #[test]
    fn headings_stripped_and_styled() {
        let text = render_markdown("# Heading 1\n\n## Heading 2\n\n### Heading 3\n");
        let content = text_content(&text);

        // No "#" markers visible.
        for line in &content {
            assert!(!line.starts_with("# "), "H1 marker visible: {line}");
            assert!(!line.starts_with("## "), "H2 marker visible: {line}");
            assert!(!line.starts_with("### "), "H3 marker visible: {line}");
        }

        // Find the heading lines and check their styles.
        let h1_line = text
            .lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("Heading 1")));
        assert!(h1_line.is_some(), "H1 heading not found");
        let h1_span = h1_line
            .unwrap()
            .spans
            .iter()
            .find(|s| s.content.contains("Heading 1"))
            .unwrap();
        assert!(h1_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn bold_italic_strikethrough_styled() {
        let text = render_markdown("**bold** *italic* ~~struck~~\n");
        let content = text_content(&text);
        let joined = content.join(" ");

        // No syntax markers visible.
        assert!(!joined.contains("**"), "Bold markers visible");
        assert!(!joined.contains("~~"), "Strikethrough markers visible");

        // Check styles on specific spans.
        for line in &text.lines {
            for span in &line.spans {
                if span.content.contains("bold") {
                    assert!(span.style.add_modifier.contains(Modifier::BOLD));
                }
                if span.content.contains("italic") {
                    assert!(span.style.add_modifier.contains(Modifier::ITALIC));
                }
                if span.content.contains("struck") {
                    assert!(span.style.add_modifier.contains(Modifier::CROSSED_OUT));
                }
            }
        }
    }

    #[test]
    fn inline_code_no_backticks() {
        let text = render_markdown("Use `code` here\n");
        let content = text_content(&text);
        let joined = content.join(" ");
        assert!(!joined.contains('`'), "Backtick visible in: {joined}");
        assert!(joined.contains("code"), "Code content missing");
    }

    #[test]
    fn code_block_no_fences() {
        let text = render_markdown("```rust\nfn main() {}\n```\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.contains("```"), "Code fence visible in:\n{joined}");
        assert!(joined.contains("fn main()"), "Code content missing");
        // Should have border characters.
        assert!(joined.contains('┌'), "Missing code block header");
        assert!(joined.contains('└'), "Missing code block footer");
    }

    #[test]
    fn unordered_list_bullets() {
        let text = render_markdown("- item 1\n- item 2\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.contains("- item"), "Dash marker visible");
        assert!(joined.contains('•'), "Bullet character missing");
        assert!(joined.contains("item 1"));
        assert!(joined.contains("item 2"));
    }

    #[test]
    fn ordered_list_numbers() {
        let text = render_markdown("1. first\n2. second\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains("1."), "Ordered number missing");
        assert!(joined.contains("first"));
        assert!(joined.contains("second"));
    }

    #[test]
    fn blockquote_styled() {
        let text = render_markdown("> quoted text\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.starts_with("> "), "Raw blockquote marker visible");
        assert!(joined.contains('▎'), "Blockquote bar missing");
        assert!(joined.contains("quoted text"));
    }

    #[test]
    fn horizontal_rule() {
        let text = render_markdown("---\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.contains("---"), "Raw HR marker visible");
        assert!(joined.contains('─'), "HR line missing");
    }

    #[test]
    fn links_styled() {
        let text = render_markdown("[click here](https://example.com)\n");
        let content = text_content(&text);
        let joined = content.join(" ");
        assert!(joined.contains("click here"), "Link text missing");
        assert!(joined.contains("https://example.com"), "URL missing");
        assert!(!joined.contains('['), "Link bracket visible");
    }

    #[test]
    fn task_list_checkboxes() {
        let text = render_markdown("- [ ] unchecked\n- [x] checked\n");
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains('☐'), "Unchecked box missing");
        assert!(joined.contains('☑'), "Checked box missing");
        assert!(joined.contains("unchecked"));
        assert!(joined.contains("checked"));
    }

    #[test]
    fn tabs_expanded() {
        let text = render_markdown("\tindented\n");
        for line in &text.lines {
            for span in &line.spans {
                assert!(!span.content.contains('\t'), "Tab not expanded");
            }
        }
    }

    #[test]
    fn empty_input() {
        let text = render_markdown("");
        let _ = text; // Must not panic.
    }

    #[test]
    fn whitespace_only() {
        let text = render_markdown("   \n\n   \n");
        let _ = text; // Must not panic.
    }

    #[test]
    fn nested_list_indentation() {
        let text = render_markdown("- outer\n  - inner\n    - deep\n");
        let content = text_content(&text);

        // Inner items should have more indentation.
        let inner_line = content.iter().find(|l| l.contains("inner"));
        assert!(inner_line.is_some(), "Inner item missing");
        let deep_line = content.iter().find(|l| l.contains("deep"));
        assert!(deep_line.is_some(), "Deep item missing");
    }
}
