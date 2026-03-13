//! Markdown renderer — converts pulldown-cmark events into styled ratatui output.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::syntax::highlight_code;
use super::theme::*;
use super::wrap::wrap_spans;

pub(super) struct Renderer {
    pub(super) lines: Vec<Line<'static>>,
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
    /// Available terminal width for wrapping (None = no wrapping).
    pub(super) available_width: Option<usize>,
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
    pub(super) fn new(available_width: Option<usize>) -> Self {
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
            available_width,
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

    pub(super) fn into_text(self) -> Text<'static> {
        Text::from(self.lines)
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
                let effective = self
                    .available_width
                    .map(|w| w.saturating_sub(self.blockquote_depth * BLOCKQUOTE_INDENT_COLS));
                self.render_code_block(effective);
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
                if let Some(dest) = self.link_dest.take() {
                    if !dest.is_empty() {
                        let url_span =
                            Span::styled(format!(" ({})", dest), Style::new().fg(Color::DarkGray));
                        self.current_spans.push(url_span);
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
        if !self.lines.is_empty() {
            self.push_blank_line();
        }
        let bq_offset = self.blockquote_depth * BLOCKQUOTE_INDENT_COLS;
        let width = self.available_width.map(|w| w.saturating_sub(bq_offset)).unwrap_or(40);
        let rule = "─".repeat(width);
        self.lines.push(Line::from(Span::styled(rule, HR_STYLE)));
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

    /// Minimum display width for code block boxes.
    const CODE_BLOCK_MIN_WIDTH: usize = 20;
    /// Padding added to each side of code block content (left + right borders).
    const CODE_BLOCK_BORDER_PAD: usize = 2;

    fn render_code_block(&mut self, available_width: Option<usize>) {
        let code = std::mem::take(&mut self.code_block_buf);
        let lang = self.code_block_lang.clone().unwrap_or_default();

        // Highlight first so we can measure widths.
        let highlighted_lines = highlight_code(&code, &lang);

        // Calculate display width of each line's spans.
        fn spans_display_width(spans: &[Span<'_>]) -> usize {
            spans.iter().map(|s| s.content.width()).sum()
        }

        let content_max = highlighted_lines
            .iter()
            .map(|spans| spans_display_width(spans))
            .max()
            .unwrap_or(Self::CODE_BLOCK_MIN_WIDTH)
            .max(Self::CODE_BLOCK_MIN_WIDTH);

        // Clamp to available terminal width if provided.
        let max_width = match available_width {
            // available_width includes the border chars (│ + space on each side = 4),
            // so inner content area = available - border_pad - 2 (for │ chars).
            Some(aw) if aw > Self::CODE_BLOCK_BORDER_PAD + 2 => {
                content_max.min(aw - Self::CODE_BLOCK_BORDER_PAD - 2)
            }
            _ => content_max,
        };

        // inner = " code_padded " = max_width + border_pad (one space each side)
        let inner = max_width + Self::CODE_BLOCK_BORDER_PAD;

        // Header: ┌─ lang ─...─┐
        let header_text = if lang.is_empty() {
            format!("┌{}┐", "─".repeat(inner))
        } else {
            let label = format!("─ {} ─", lang);
            let label_width = label.width();
            let remaining = inner.saturating_sub(label_width);
            format!("┌{}{}┐", label, "─".repeat(remaining))
        };
        self.lines.push(Line::from(Span::styled(header_text, CODE_BORDER_STYLE)));

        // Code lines with right border.
        for line_spans in highlighted_lines {
            let content_width = spans_display_width(&line_spans);
            let truncated_spans = if content_width > max_width {
                // Truncate: iterate spans, accumulate width, cut at max_width - 1 to fit "…"
                let budget = max_width.saturating_sub(1); // 1 col for "…"
                let mut result: Vec<Span<'static>> = Vec::new();
                let mut used = 0usize;
                for span in line_spans {
                    let sw = span.content.width();
                    if used + sw <= budget {
                        result.push(span);
                        used += sw;
                    } else {
                        // Partial span: take graphemes that fit
                        let remaining = budget - used;
                        if remaining > 0 {
                            let mut partial = String::new();
                            for g in span.content.graphemes(true) {
                                if partial.width() + g.width() > remaining {
                                    break;
                                }
                                partial.push_str(g);
                            }
                            if !partial.is_empty() {
                                result.push(Span::styled(partial, span.style));
                            }
                        }
                        result.push(Span::styled("…", CODE_BORDER_STYLE));
                        break;
                    }
                }
                result
            } else {
                line_spans
            };
            let truncated_width = spans_display_width(&truncated_spans);
            let padding = max_width.saturating_sub(truncated_width);
            let mut spans = vec![Span::styled("│ ", CODE_BORDER_STYLE)];
            spans.extend(truncated_spans);
            spans.push(Span::styled(format!("{} │", " ".repeat(padding)), CODE_BORDER_STYLE));
            self.lines.push(Line::from(spans));
        }

        // Footer: └─...─┘
        self.lines
            .push(Line::from(Span::styled(format!("└{}┘", "─".repeat(inner)), CODE_BORDER_STYLE)));
    }

    // ── Table rendering ─────────────────────────────────────────────────

    /// Minimum display width for table columns.
    const TABLE_MIN_COL_WIDTH: usize = 3;
    /// Padding added to each side of a table cell.
    const TABLE_CELL_PAD: usize = 2;

    fn render_table(&mut self) {
        // Filter out empty trailing rows
        let rows: Vec<&Vec<Vec<Span<'static>>>> =
            self.table_rows.iter().filter(|r| !r.is_empty()).collect();

        if rows.is_empty() {
            return;
        }

        let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if num_cols == 0 {
            return;
        }

        // Helper: display width of a slice of spans.
        fn cell_display_width(spans: &[Span<'_>]) -> usize {
            spans.iter().map(|s| s.content.width()).sum()
        }

        // Calculate column widths (max content width per column)
        let mut col_widths = vec![0usize; num_cols];
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                let width: usize = cell.iter().map(|s| s.content.width()).sum();
                col_widths[i] = col_widths[i].max(width);
            }
        }
        // Minimum column width of 3
        for w in &mut col_widths {
            *w = (*w).max(Self::TABLE_MIN_COL_WIDTH);
        }

        // Clamp to available width if provided.
        let effective_width = self
            .available_width
            .map(|w| w.saturating_sub(self.blockquote_depth * BLOCKQUOTE_INDENT_COLS));
        if let Some(aw) = effective_width {
            // Total table width: sum(col_widths) + (num_cols * TABLE_CELL_PAD) + num_cols + 1
            // Each cell: " content " = col_width + TABLE_CELL_PAD, plus separators
            let total: usize =
                col_widths.iter().sum::<usize>() + (num_cols * Self::TABLE_CELL_PAD) + num_cols + 1;
            if total > aw && num_cols > 0 {
                // Shrink columns proportionally
                let border_overhead = (num_cols * Self::TABLE_CELL_PAD) + num_cols + 1;
                let available_content = aw.saturating_sub(border_overhead);
                let current_content: usize = col_widths.iter().sum();
                for w in &mut col_widths {
                    let shrunk = (*w * available_content) / current_content.max(1);
                    *w = shrunk.max(Self::TABLE_MIN_COL_WIDTH);
                }
            }
        }

        // Helper to build a horizontal border line
        let build_border = |left: &str, mid: &str, right: &str, fill: &str| -> Line<'static> {
            let mut s = left.to_string();
            for (i, &w) in col_widths.iter().enumerate() {
                s.push_str(&fill.repeat(w + Self::TABLE_CELL_PAD)); // +2 for padding spaces
                if i < num_cols - 1 {
                    s.push_str(mid);
                }
            }
            s.push_str(right);
            Line::from(Span::styled(s, TABLE_BORDER_STYLE))
        };

        // Top border: ┌───┬───┐
        self.lines.push(build_border("┌", "┬", "┐", "─"));

        for (row_idx, row) in rows.iter().enumerate() {
            // Wrap each cell's content to get multiple visual lines per cell.
            let wrapped_cells: Vec<Vec<Vec<Span<'static>>>> = col_widths
                .iter()
                .enumerate()
                .map(|(col_idx, col_width)| {
                    let cell_spans: &[Span] = match row.get(col_idx) {
                        Some(c) => c,
                        None => &[],
                    };
                    wrap_spans(cell_spans, *col_width)
                })
                .collect();

            // Row height = tallest (most-wrapped) cell.
            let row_height = wrapped_cells.iter().map(Vec::len).max().unwrap_or(1);

            // Render each visual sub-line of this row.
            for sub_line in 0..row_height {
                let mut spans: Vec<Span<'static>> = Vec::with_capacity(col_widths.len() * 2 + 2);
                spans.push(Span::styled("│ ", TABLE_BORDER_STYLE));

                for (col_idx, col_width) in col_widths.iter().enumerate() {
                    let cell_line_spans =
                        wrapped_cells.get(col_idx).and_then(|lines| lines.get(sub_line));

                    let (line_spans, content_width) = match cell_line_spans {
                        Some(ls) => {
                            let w = cell_display_width(ls);
                            (ls.clone(), w)
                        }
                        None => (vec![], 0),
                    };

                    let padding = col_width.saturating_sub(content_width);
                    spans.extend(line_spans);
                    spans.push(Span::styled(
                        format!("{} │ ", " ".repeat(padding)),
                        TABLE_BORDER_STYLE,
                    ));
                }

                self.lines.push(Line::from(spans));
            }

            // After the first row (header), add separator: ├───┼───┤
            if row_idx == 0 && rows.len() > 1 {
                self.lines.push(build_border("├", "┼", "┤", "─"));
            }
        }

        // Bottom border: └───┴───┘
        self.lines.push(build_border("└", "┴", "┘", "─"));
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

    // ── Line management ─────────────────────────────────────────────────

    fn flush_line(&mut self) {
        if self.current_spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.current_spans);

        if let Some(width) = self.available_width {
            // Calculate effective width accounting for blockquote bars.
            let bq_prefix_width = self.blockquote_depth * BLOCKQUOTE_INDENT_COLS; // each "▎ " is 2 cols
            let effective_width = width.saturating_sub(bq_prefix_width);
            let wrapped_lines = wrap_spans(&spans, effective_width);
            let list_marker_width = self.list_marker_width;

            for (i, line_spans) in wrapped_lines.into_iter().enumerate() {
                let mut final_spans = Vec::new();

                // Prepend blockquote bars on every wrapped line.
                for _ in 0..self.blockquote_depth {
                    final_spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
                }

                // For list items, continuation lines get hanging indent.
                if i > 0 && list_marker_width > 0 {
                    final_spans.push(Span::styled(" ".repeat(list_marker_width), Style::default()));
                }

                final_spans.extend(line_spans);
                self.lines.push(Line::from(final_spans));
            }
        } else {
            // Original behavior: no wrapping.
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
