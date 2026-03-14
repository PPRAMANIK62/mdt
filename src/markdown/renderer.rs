//! Markdown renderer — converts pulldown-cmark events into width-independent blocks.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::blocks::RenderedBlock;
use super::syntax::highlight_code;
use super::theme::*;

/// Metadata for a link found in the markdown document.
#[derive(Clone, Debug)]
pub struct LinkInfo {
    pub display_text: String,
    pub url: String,
}

pub(super) struct Renderer {
    pub(super) blocks: Vec<RenderedBlock>,
    pub(super) current_spans: Vec<Span<'static>>,
    pub(super) style_stack: Vec<Style>,
    /// Stack of list contexts: None = unordered, Some(n) = ordered starting at n.
    pub(super) list_stack: Vec<Option<u64>>,
    /// Whether we're inside a code block.
    pub(super) in_code_block: bool,
    /// Language hint for the current code block.
    pub(super) code_block_lang: Option<String>,
    /// Accumulated code block content.
    pub(super) code_block_buf: String,
    /// Current nesting depth for blockquotes.
    pub(super) blockquote_depth: usize,
    /// Whether the next text event is the first in a list item (needs marker).
    pub(super) pending_list_marker: bool,
    /// Track if we need a blank line separator.
    pub(super) needs_newline: bool,
    /// Current link destination (Some while inside a link).
    pub(super) link_dest: Option<String>,
    /// Accumulated display text inside the current link (for comparison with URL).
    pub(super) link_text: String,
    /// Collected link metadata for the document.
    pub(super) link_infos: Vec<LinkInfo>,
    /// Whether we're inside a heading (to apply heading style to all text).
    /// Width of the current list marker (indent + bullet/number) for hanging indent.
    pub(super) list_marker_width: usize,
    pub(super) in_heading: bool,
    /// Whether we're inside a table.
    pub(super) in_table: bool,
    /// Column alignments for the current table.
    pub(super) table_alignments: Vec<pulldown_cmark::Alignment>,
    /// Buffered table rows. Each row is a vec of cells, each cell is a vec of spans.
    pub(super) table_rows: Vec<Vec<Vec<Span<'static>>>>,
    /// Spans for the current cell being built.
    pub(super) table_cell_spans: Vec<Span<'static>>,
    /// Whether we're in the header row.
    pub(super) in_table_header: bool,
}

impl Renderer {
    pub(super) fn new() -> Self {
        Self {
            blocks: Vec::new(),
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
            link_text: String::new(),
            link_infos: Vec::new(),
            in_heading: false,
            in_table: false,
            table_alignments: Vec::new(),
            table_rows: Vec::new(),
            table_cell_spans: Vec::new(),
            in_table_header: false,
            list_marker_width: 0,
        }
    }

    pub(super) fn run<'a>(&mut self, parser: impl Iterator<Item = Event<'a>>) {
        for event in parser {
            self.handle_event(event);
        }
        self.flush_line();
    }

    pub(super) fn into_blocks(self) -> (Vec<RenderedBlock>, Vec<LinkInfo>) {
        (self.blocks, self.link_infos)
    }

    // ── Event dispatch ──────────────────────────────────────────────────

    fn handle_event<'a>(&mut self, event: Event<'a>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.on_text(&text),
            Event::Code(code) => self.on_inline_code(&code),
            Event::SoftBreak => self.on_soft_break(),
            Event::HardBreak => self.on_hard_break(),
            Event::Rule => self.on_rule(),
            Event::TaskListMarker(checked) => self.on_task_list_marker(checked),
            Event::Html(html) => self.on_html(&html),
            Event::InlineHtml(html) => self.on_inline_html(&html),
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
                self.style_stack.push(Style::new().fg(Color::Gray));
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
                self.link_text.clear();
                self.push_merged_style(LINK_STYLE);
            }
            Tag::Table(alignments) => {
                if self.needs_newline {
                    self.push_blank_line();
                }
                self.in_table = true;
                self.table_alignments.clone_from(&alignments);
                self.table_rows.clear();
            }
            Tag::TableHead => {
                self.in_table_header = true;
            }
            Tag::TableRow => {
                // Start a new row — nothing special needed, cells accumulate
            }
            Tag::TableCell => {
                self.table_cell_spans.clear();
            }
            Tag::Image { .. }
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
                self.style_stack.pop();
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
                self.list_marker_width = 0;
            }
            TagEnd::Link => {
                self.style_stack.pop();
                let text = std::mem::take(&mut self.link_text);
                if let Some(url) = self.link_dest.take() {
                    if !url.is_empty() {
                        self.link_infos.push(LinkInfo {
                            display_text: if text.is_empty() { url.clone() } else { text },
                            url,
                        });
                    }
                }
            }
            TagEnd::TableCell => {
                let mut spans = std::mem::take(&mut self.table_cell_spans);
                if self.in_table_header {
                    spans = spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.patch(TABLE_HEADER_STYLE)))
                        .collect();
                }
                if let Some(last_row) = self.table_rows.last_mut() {
                    last_row.push(spans);
                } else {
                    self.table_rows.push(vec![spans]);
                }
            }
            TagEnd::TableHead => {
                self.in_table_header = false;
                // Push a new empty row for the next body row
                self.table_rows.push(Vec::new());
            }
            TagEnd::TableRow => {
                self.table_rows.push(Vec::new());
            }
            TagEnd::Table => {
                self.render_table();
                self.in_table = false;
                self.table_alignments.clear();
                self.table_rows.clear();
                self.needs_newline = true;
            }
            TagEnd::Image
            | TagEnd::FootnoteDefinition
            | TagEnd::HtmlBlock
            | TagEnd::MetadataBlock(_) => {}
            _ => {}
        }
    }

    // ── Inline events ───────────────────────────────────────────────────

    fn on_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_buf.push_str(text);
            return;
        }

        if self.in_table {
            let style = self.current_style();
            self.table_cell_spans.push(Span::styled(text.to_string(), style));
            return;
        }

        let style = self.current_style();

        if self.link_dest.is_some() {
            self.link_text.push_str(text);
        }

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
                self.current_spans.push(Span::styled(line.to_string(), style));
            }
        }
    }

    fn on_inline_code(&mut self, code: &str) {
        if self.link_dest.is_some() {
            self.link_text.push_str(code);
        }
        if self.pending_list_marker {
            self.emit_list_marker();
            self.pending_list_marker = false;
        }
        if self.in_table {
            self.table_cell_spans.push(Span::styled(format!(" {} ", code), INLINE_CODE_STYLE));
            return;
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
        if self.in_table {
            self.table_cell_spans.push(Span::styled(" ", self.current_style()));
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
        if !self.blocks.is_empty() {
            self.push_blank_line();
        }
        self.blocks.push(RenderedBlock::HorizontalRule { blockquote_depth: self.blockquote_depth });
        self.needs_newline = true;
    }

    fn on_task_list_marker(&mut self, checked: bool) {
        // Replace the pending bullet/number marker with a checkbox.
        let checkbox = if checked { "☑ " } else { "☐ " };
        let indent = self.list_indent_prefix();
        let style = self.current_style();
        self.current_spans.push(Span::styled(indent, Style::default()));
        self.current_spans.push(Span::styled(checkbox, style));
        // Mark that we've already emitted the marker.
        self.pending_list_marker = false;
    }

    fn on_html(&mut self, html: &str) {
        // Render HTML blocks as plain text.
        for (i, line) in html.lines().enumerate() {
            if i > 0 {
                self.flush_line();
            }
            self.current_spans.push(Span::styled(line.to_string(), Style::default()));
        }
        self.flush_line();
    }

    fn on_inline_html(&mut self, html: &str) {
        // Strip inline HTML tags, render content if any.
        if html.is_empty() {
            return;
        }
        let content = html.to_string();
        self.current_spans.push(Span::styled(content, self.current_style()));
    }

    // ── Code block rendering ────────────────────────────────────────────

    fn render_code_block(&mut self) {
        let code = std::mem::take(&mut self.code_block_buf);
        let lang = self.code_block_lang.clone().unwrap_or_default();

        // Highlight (width-independent) — the expensive part done once.
        let highlighted_lines = highlight_code(&code, &lang);

        self.blocks.push(RenderedBlock::CodeBlock {
            lang,
            highlighted_lines,
            blockquote_depth: self.blockquote_depth,
        });
    }

    // ── Table rendering ─────────────────────────────────────────────────

    fn render_table(&mut self) {
        // Filter out empty trailing rows.
        let rows: Vec<Vec<Vec<Span<'static>>>> =
            self.table_rows.iter().filter(|r| !r.is_empty()).cloned().collect();

        if rows.is_empty() {
            return;
        }

        let num_cols = rows.iter().map(Vec::len).max().unwrap_or(0);
        if num_cols == 0 {
            return;
        }

        self.blocks.push(RenderedBlock::Table {
            rows,
            alignments: self.table_alignments.clone(),
            blockquote_depth: self.blockquote_depth,
        });
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
                    self.current_spans.push(Span::styled(indent, Style::default()));
                    self.current_spans
                        .push(Span::styled(format!("{bullet} "), Style::new().fg(Color::Gray)));
                }
                Some(ref mut num) => {
                    // Ordered list — use number.
                    self.current_spans.push(Span::styled(indent, Style::default()));
                    self.current_spans
                        .push(Span::styled(format!("{num}. "), Style::new().fg(Color::Gray)));
                    *num += 1;
                }
            }
        }
        // Store the marker width for hanging indent on wrapped continuation lines.
        self.list_marker_width =
            self.current_spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
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

    // ── Block management ────────────────────────────────────────────────

    fn flush_line(&mut self) {
        if self.current_spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.current_spans);

        self.blocks.push(RenderedBlock::StyledLine {
            spans,
            blockquote_depth: self.blockquote_depth,
            list_marker_width: self.list_marker_width,
        });
    }

    fn push_blank_line(&mut self) {
        self.flush_line();
        self.blocks.push(RenderedBlock::BlankLine { blockquote_depth: self.blockquote_depth });
    }
}
