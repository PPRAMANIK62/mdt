//! Custom markdown-to-ratatui renderer using pulldown-cmark.
//!
//! Produces styled [`Text`] with all syntax markers stripped — headings, bold, italic,
//! strikethrough, inline code, code blocks, lists, blockquotes, links, horizontal rules,
//! and task lists are all rendered as properly styled text.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::sync::OnceLock;
use syntect::easy::ScopeRegionIterator;
use syntect::parsing::SyntaxSet;
use syntect::parsing::{ParseState, Scope, ScopeStack};
use syntect::util::LinesWithEndings;
use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

/// Render markdown input into styled ratatui [`Text`].
///
/// - Pre-expands tabs to 4 spaces (ratatui `Paragraph` silently drops tabs).
/// - Respects the `NO_COLOR` environment variable: when set, returns plain unstyled text.
/// - All markdown syntax markers are stripped; styling is applied via ratatui modifiers/colors.
pub fn render_markdown(input: &str, available_width: Option<usize>) -> Text<'static> {
    let cleaned = input.replace('\t', "    ");

    if no_color() {
        return Text::raw(cleaned);
    }

    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(&cleaned, options);

    let mut renderer = Renderer::new(available_width);
    renderer.run(parser);
    renderer.into_text()
}

/// Word-wrap a slice of styled [`Span`]s to fit within `max_width` columns.
///
/// Returns a `Vec` of visual lines, each a `Vec<Span>`. Styles are preserved
/// across split points — when a span must be broken, both halves retain the
/// original style. Trailing whitespace at wrap boundaries is trimmed.
///
/// Special cases:
/// - Empty input returns `vec![vec![]]`.
/// - `max_width == 0` returns one line per grapheme cluster (or `vec![vec![]]` if empty).
fn wrap_spans(spans: &[Span], max_width: usize) -> Vec<Vec<Span<'static>>> {
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
                graphemes: vec![],  // placeholder, replaced below
            });
            // Move the grapheme out efficiently.
            let g = std::mem::replace(&mut graphemes[i], Grapheme {
                text: "",
                style: Style::default(),
                width: 0,
            });
            words.last_mut().unwrap().graphemes = vec![g];
            i += 1;
        } else {
            // Accumulate non-whitespace graphemes into one word.
            let mut word_gs = Vec::new();
            let mut w = 0;
            while i < graphemes.len() && !graphemes[i].text.chars().all(char::is_whitespace) {
                let g = std::mem::replace(&mut graphemes[i], Grapheme {
                    text: "",
                    style: Style::default(),
                    width: 0,
                });
                w += g.width;
                word_gs.push(g);
                i += 1;
            }
            words.push(Word {
                graphemes: word_gs,
                width: w,
                is_whitespace: false,
            });
        }
    }

    // Lay out words into lines respecting max_width.
    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
    let mut cur_spans: Vec<Span<'static>> = Vec::new();
    let mut cur_width: usize = 0;

    // Helper: push graphemes into cur_spans, merging consecutive same-style runs.
    fn push_graphemes(
        cur_spans: &mut Vec<Span<'static>>,
        gs: &[Grapheme<'_>],
    ) {
        for g in gs {
            if let Some(last) = cur_spans.last_mut() {
                if last.style == g.style {
                    // Merge into existing span.
                    last.content = std::borrow::Cow::Owned(
                        format!("{}{}", last.content, g.text),
                    );
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

// ── Styles ──────────────────────────────────────────────────────────────────

const H1_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Cyan);
const H2_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Green);
const H3_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Yellow);
const H4_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::DarkGray);
const BOLD_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::LightRed);
const ITALIC_STYLE: Style = Style::new().add_modifier(Modifier::ITALIC);
const STRIKETHROUGH_STYLE: Style = Style::new().add_modifier(Modifier::CROSSED_OUT);
const INLINE_CODE_STYLE: Style = Style::new().fg(Color::LightCyan);
const LINK_STYLE: Style = Style::new().add_modifier(Modifier::UNDERLINED).fg(Color::Blue);
const BLOCKQUOTE_STYLE: Style = Style::new().fg(Color::Cyan);
const CODE_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
const CODE_DEFAULT_STYLE: Style = Style::new();
const HR_STYLE: Style = Style::new().fg(Color::DarkGray);
const BLOCKQUOTE_INDENT_COLS: usize = 2;
const TABLE_HEADER_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Cyan);
const TABLE_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);

// ── Syntax highlighting ─────────────────────────────────────────────────────

/// Semantic token categories for ANSI code highlighting.
enum CodeToken {
    Comment,
    String,
    Number,
    Operator,
    Keyword,
    Function,
    Type,
    Tag,
    Punctuation,
    Variable,
    Constant,
    Normal,
}

impl CodeToken {
    fn to_style(&self) -> Style {
        match self {
            CodeToken::Comment => Style::new().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            CodeToken::String => Style::new().fg(Color::Green),
            CodeToken::Number => Style::new().fg(Color::Cyan),
            CodeToken::Operator => Style::new().fg(Color::Cyan),
            CodeToken::Keyword => Style::new().fg(Color::Magenta),
            CodeToken::Function => Style::new().fg(Color::Blue),
            CodeToken::Type => Style::new().fg(Color::Yellow),
            CodeToken::Tag => Style::new().fg(Color::Red),
            CodeToken::Punctuation => Style::default(),
            CodeToken::Variable => Style::new().fg(Color::Red),
            CodeToken::Constant => Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            CodeToken::Normal => Style::default(),
        }
    }
}

/// Pre-built `Scope` objects for prefix matching.
struct ScopeMatchers {
    comment: Scope,
    string: Scope,
    constant_character: Scope,
    constant_numeric: Scope,
    keyword_operator: Scope,
    keyword: Scope,
    storage: Scope,
    entity_name_function: Scope,
    support_function: Scope,
    entity_name_type: Scope,
    support_type: Scope,
    entity_name_tag: Scope,
    punctuation: Scope,
    variable: Scope,
    entity_name: Scope,
    constant: Scope,
}

impl ScopeMatchers {
    fn new() -> Self {
        Self {
            comment: Scope::new("comment").expect("valid scope literal"),
            string: Scope::new("string").expect("valid scope literal"),
            constant_character: Scope::new("constant.character").expect("valid scope literal"),
            constant_numeric: Scope::new("constant.numeric").expect("valid scope literal"),
            keyword_operator: Scope::new("keyword.operator").expect("valid scope literal"),
            keyword: Scope::new("keyword").expect("valid scope literal"),
            storage: Scope::new("storage").expect("valid scope literal"),
            entity_name_function: Scope::new("entity.name.function").expect("valid scope literal"),
            support_function: Scope::new("support.function").expect("valid scope literal"),
            entity_name_type: Scope::new("entity.name.type").expect("valid scope literal"),
            support_type: Scope::new("support.type").expect("valid scope literal"),
            entity_name_tag: Scope::new("entity.name.tag").expect("valid scope literal"),
            punctuation: Scope::new("punctuation").expect("valid scope literal"),
            variable: Scope::new("variable").expect("valid scope literal"),
            entity_name: Scope::new("entity.name").expect("valid scope literal"),
            constant: Scope::new("constant").expect("valid scope literal"),
        }
    }
}

fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn scope_matchers() -> &'static ScopeMatchers {
    static MATCHERS: OnceLock<ScopeMatchers> = OnceLock::new();
    MATCHERS.get_or_init(ScopeMatchers::new)
}

fn no_color() -> bool {
    static NO_COLOR: OnceLock<bool> = OnceLock::new();
    *NO_COLOR.get_or_init(|| std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty()))
}

/// Map the most-specific scope in the stack to an ANSI style.
/// Priority order: most-specific prefixes checked first.
fn scope_to_style(stack: &ScopeStack, m: &ScopeMatchers) -> Style {
    let Some(&scope) = stack.as_slice().last() else {
        return CodeToken::Normal.to_style();
    };

    // Priority order: most-specific first
    if m.comment.is_prefix_of(scope) {
        return CodeToken::Comment.to_style();
    }
    if m.string.is_prefix_of(scope) || m.constant_character.is_prefix_of(scope) {
        return CodeToken::String.to_style();
    }
    if m.constant_numeric.is_prefix_of(scope) {
        return CodeToken::Number.to_style();
    }
    // keyword.operator MUST be before keyword
    if m.keyword_operator.is_prefix_of(scope) {
        return CodeToken::Operator.to_style();
    }
    if m.keyword.is_prefix_of(scope) || m.storage.is_prefix_of(scope) {
        return CodeToken::Keyword.to_style();
    }
    if m.entity_name_function.is_prefix_of(scope) || m.support_function.is_prefix_of(scope) {
        return CodeToken::Function.to_style();
    }
    if m.entity_name_type.is_prefix_of(scope) || m.support_type.is_prefix_of(scope) {
        return CodeToken::Type.to_style();
    }
    if m.entity_name_tag.is_prefix_of(scope) {
        return CodeToken::Tag.to_style();
    }
    if m.punctuation.is_prefix_of(scope) {
        return CodeToken::Punctuation.to_style();
    }
    if m.variable.is_prefix_of(scope) || m.entity_name.is_prefix_of(scope) {
        return CodeToken::Variable.to_style();
    }
    // constant MUST be after specific constant prefixes
    if m.constant.is_prefix_of(scope) {
        return CodeToken::Constant.to_style();
    }

    CodeToken::Normal.to_style()
}

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
    /// Available terminal width for wrapping (None = no wrapping).
    available_width: Option<usize>,
    /// Whether we're inside a heading (to apply heading style to all text).
    /// Width of the current list marker (indent + bullet/number) for hanging indent.
    list_marker_width: usize,
    in_heading: bool,
    /// Whether we're inside a table.
    in_table: bool,
    /// Column alignments for the current table.
    table_alignments: Vec<pulldown_cmark::Alignment>,
    /// Buffered table rows. Each row is a vec of cells, each cell is a vec of spans.
    table_rows: Vec<Vec<Vec<Span<'static>>>>,
    /// Spans for the current cell being built.
    table_cell_spans: Vec<Span<'static>>,
    /// Whether we're in the header row.
    in_table_header: bool,
}

impl Renderer {
    fn new(available_width: Option<usize>) -> Self {
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
                let effective = self.available_width.map(|w| w.saturating_sub(self.blockquote_depth * BLOCKQUOTE_INDENT_COLS));
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
        let highlighted_lines = Self::highlight_code(&code, &lang);

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
        let effective_width = self.available_width.map(|w| w.saturating_sub(self.blockquote_depth * BLOCKQUOTE_INDENT_COLS));
        if let Some(aw) = effective_width {
            // Total table width: sum(col_widths) + (num_cols * TABLE_CELL_PAD) + num_cols + 1
            // Each cell: " content " = col_width + TABLE_CELL_PAD, plus separators
            let total: usize = col_widths.iter().sum::<usize>() + (num_cols * Self::TABLE_CELL_PAD) + num_cols + 1;
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
                    let cell_line_spans = wrapped_cells
                        .get(col_idx)
                        .and_then(|lines| lines.get(sub_line));

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

    fn highlight_code(code: &str, lang: &str) -> Vec<Vec<Span<'static>>> {
        let ss = syntax_set();

        // Try to find syntax for the language.
        let syntax = if lang.is_empty() { None } else { ss.find_syntax_by_token(lang) };

        match syntax {
            Some(syntax) => {
                let mut state = ParseState::new(syntax);
                let mut stack = ScopeStack::new();
                let matchers = scope_matchers();
                let mut result = Vec::new();

                for line in LinesWithEndings::from(code) {
                    let ops = state.parse_line(line, ss).unwrap_or_default();
                    let mut spans = Vec::new();

                    for (s, op) in ScopeRegionIterator::new(&ops, line) {
                        let _ = stack.apply(op);
                        if s.is_empty() {
                            continue;
                        }
                        let style = scope_to_style(&stack, matchers);
                        spans.push(Span::styled(s.trim_end_matches('\n').to_string(), style));
                    }

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
        self.list_marker_width = self
            .current_spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum();
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
                    final_spans.push(Span::styled(
                        " ".repeat(list_marker_width),
                        Style::default(),
                    ));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: collect all text content from a Text, joining spans per line.
    fn text_content(text: &Text<'_>) -> Vec<String> {
        text.lines
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect()
    }

    /// Render markdown with a specific width constraint.
    fn render_at_width(input: &str, width: usize) -> Text<'static> {
        render_markdown(input, Some(width))
    }

    /// Return the maximum visual width across all lines in a `Text`.
    fn max_line_width(text: &Text) -> usize {
        text.lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.width()).sum::<usize>())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn headings_stripped_and_styled() {
        let text = render_markdown("# Heading 1\n\n## Heading 2\n\n### Heading 3\n", None);
        let content = text_content(&text);

        // No "#" markers visible.
        for line in &content {
            assert!(!line.starts_with("# "), "H1 marker visible: {line}");
            assert!(!line.starts_with("## "), "H2 marker visible: {line}");
            assert!(!line.starts_with("### "), "H3 marker visible: {line}");
        }

        // Find the heading lines and check their styles.
        let h1_line =
            text.lines.iter().find(|l| l.spans.iter().any(|s| s.content.contains("Heading 1")));
        assert!(h1_line.is_some(), "H1 heading not found");
        let h1_span =
            h1_line.unwrap().spans.iter().find(|s| s.content.contains("Heading 1")).unwrap();
        assert!(h1_span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn bold_italic_strikethrough_styled() {
        let text = render_markdown("**bold** *italic* ~~struck~~\n", None);
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
        let text = render_markdown("Use `code` here\n", None);
        let content = text_content(&text);
        let joined = content.join(" ");
        assert!(!joined.contains('`'), "Backtick visible in: {joined}");
        assert!(joined.contains("code"), "Code content missing");
    }

    #[test]
    fn code_block_no_fences() {
        let text = render_markdown("```rust\nfn main() {}\n```\n", None);
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
        let text = render_markdown("- item 1\n- item 2\n", None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.contains("- item"), "Dash marker visible");
        assert!(joined.contains('•'), "Bullet character missing");
        assert!(joined.contains("item 1"));
        assert!(joined.contains("item 2"));
    }

    #[test]
    fn ordered_list_numbers() {
        let text = render_markdown("1. first\n2. second\n", None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains("1."), "Ordered number missing");
        assert!(joined.contains("first"));
        assert!(joined.contains("second"));
    }

    #[test]
    fn blockquote_styled() {
        let text = render_markdown("> quoted text\n", None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.starts_with("> "), "Raw blockquote marker visible");
        assert!(joined.contains('▎'), "Blockquote bar missing");
        assert!(joined.contains("quoted text"));
    }

    #[test]
    fn horizontal_rule() {
        let text = render_markdown("---\n", None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(!joined.contains("---"), "Raw HR marker visible");
        assert!(joined.contains('─'), "HR line missing");
    }

    #[test]
    fn links_styled() {
        let text = render_markdown("[click here](https://example.com)\n", None);
        let content = text_content(&text);
        let joined = content.join(" ");
        assert!(joined.contains("click here"), "Link text missing");
        assert!(joined.contains("https://example.com"), "URL missing");
        assert!(!joined.contains('['), "Link bracket visible");
    }

    #[test]
    fn task_list_checkboxes() {
        let text = render_markdown("- [ ] unchecked\n- [x] checked\n", None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains('☐'), "Unchecked box missing");
        assert!(joined.contains('☑'), "Checked box missing");
        assert!(joined.contains("unchecked"));
        assert!(joined.contains("checked"));
    }

    #[test]
    fn tabs_expanded() {
        let text = render_markdown("\tindented\n", None);
        for line in &text.lines {
            for span in &line.spans {
                assert!(!span.content.contains('\t'), "Tab not expanded");
            }
        }
    }

    #[test]
    fn empty_input() {
        let text = render_markdown("", None);
        let _ = text; // Must not panic.
    }

    #[test]
    fn whitespace_only() {
        let text = render_markdown("   \n\n   \n", None);
        let _ = text; // Must not panic.
    }

    #[test]
    fn nested_list_indentation() {
        let text = render_markdown("- outer\n  - inner\n    - deep\n", None);
        let content = text_content(&text);

        // Inner items should have more indentation.
        let inner_line = content.iter().find(|l| l.contains("inner"));
        assert!(inner_line.is_some(), "Inner item missing");
        let deep_line = content.iter().find(|l| l.contains("deep"));
        assert!(deep_line.is_some(), "Deep item missing");
    }

    // ── Scope-to-style mapping tests ──────────────────────────────────────────

    #[test]
    fn scope_keyword_operator_gets_operator_not_keyword() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("keyword.operator").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        // keyword.operator → Operator (Cyan), NOT Keyword (Magenta)
        assert_eq!(style.fg, Some(Color::Cyan));
        assert_ne!(style.fg, Some(Color::Magenta));
    }

    #[test]
    fn scope_comment_gets_darkgray_italic() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("comment.line.double-slash").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::DarkGray));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn scope_string_gets_green() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("string.quoted.double").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn scope_function_gets_blue() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("entity.name.function").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::Blue));
    }

    #[test]
    fn scope_type_gets_yellow() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("entity.name.type").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn scope_unknown_gets_default() {
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("unknown.scope.xyz").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style, Style::default());
    }

    #[test]
    fn scope_entity_name_function_before_entity_name() {
        // entity.name.function should match Function (Blue),
        // NOT fall through to Variable (Red) via entity.name
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("entity.name.function").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::Blue));
        assert_ne!(style.fg, Some(Color::Red));
    }

    #[test]
    fn scope_constant_numeric_before_constant() {
        // constant.numeric → Number (Cyan, no Bold),
        // NOT Constant (Cyan + Bold)
        let mut stack = ScopeStack::new();
        stack.push(Scope::new("constant.numeric").unwrap());
        let style = scope_to_style(&stack, scope_matchers());
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(!style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn scope_render_markdown_code_block_uses_ansi_colors() {
        let text = render_markdown("```rust\nfn main() { let x = 42; }\n```\n", None);
        for line in &text.lines {
            for span in &line.spans {
                if let Some(fg) = span.style.fg {
                    assert!(
                        !matches!(fg, Color::Rgb(_, _, _)),
                        "Found Color::Rgb in code block span: {:?} with content {:?}",
                        fg,
                        span.content,
                    );
                }
            }
        }
    }

    // ── Table rendering tests ──────────────────────────────────────────

    #[test]
    fn table_renders_with_borders() {
        let input = "| Key | Action |\n|---|---|\n| j/k | Navigate |\n| Enter | Open file |\n";
        let text = render_markdown(input, None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains('┌'), "Missing table top border");
        assert!(joined.contains('┘'), "Missing table bottom border");
        assert!(joined.contains('│'), "Missing table cell border");
        assert!(joined.contains("Key"), "Header content missing");
        assert!(joined.contains("Navigate"), "Cell content missing");
    }

    #[test]
    fn table_header_is_bold() {
        let input = "| Name | Value |\n|---|---|\n| a | b |\n";
        let text = render_markdown(input, None);
        // Find the line containing "Name" and check it has bold
        for line in &text.lines {
            for span in &line.spans {
                if span.content.contains("Name") {
                    assert!(
                        span.style.add_modifier.contains(Modifier::BOLD),
                        "Header should be bold"
                    );
                }
            }
        }
    }

    #[test]
    fn table_empty_does_not_panic() {
        let input = "| |\n|---|\n| |\n";
        let text = render_markdown(input, None);
        let _ = text; // Must not panic
    }

    #[test]
    fn table_with_inline_code() {
        let input = "| Command | Description |\n|---|---|\n| `ls` | List files |\n";
        let text = render_markdown(input, None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains("ls"), "Inline code content missing in table");
        assert!(joined.contains("List files"), "Cell content missing");
    }

    #[test]
    fn table_separator_between_header_and_body() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let text = render_markdown(input, None);
        let content = text_content(&text);
        let joined = content.join("\n");
        assert!(joined.contains('├'), "Missing header separator left");
        assert!(joined.contains('┼'), "Missing header separator cross");
        assert!(joined.contains('┤'), "Missing header separator right");
    }

    // ── wrap_spans tests ───────────────────────────────────────────────

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

    // ── Width-aware wrapping tests ──────────────────────────────────────

    #[test]
    fn paragraph_wraps_to_width() {
        let input = "This is a paragraph with enough words to require wrapping at a narrow width.";
        let text = render_at_width(input, 30);
        let content = text_content(&text);
        // Should produce multiple lines.
        assert!(content.len() > 1, "Expected wrapping, got: {content:?}");
        // Every line should fit within the specified width.
        assert!(
            max_line_width(&text) <= 30,
            "Line exceeded width 30: {content:?}",
        );
    }

    #[test]
    fn heading_wraps_with_style() {
        let input = "# A very long heading that should definitely wrap at a narrow width";
        let text = render_at_width(input, 20);
        let content = text_content(&text);
        // Should produce multiple lines.
        assert!(content.len() > 1, "Expected heading to wrap, got: {content:?}");
        // All lines containing text should have bold modifier (heading style).
        for line in &text.lines {
            for span in &line.spans {
                if !span.content.trim().is_empty() {
                    assert!(
                        span.style.add_modifier.contains(Modifier::BOLD),
                        "Heading style missing on wrapped line span: {:?}",
                        span.content,
                    );
                }
            }
        }
    }

    #[test]
    fn blockquote_bars_on_all_wrapped_lines() {
        let input = "> This is a blockquote with enough text to wrap to multiple lines easily.";
        let text = render_at_width(input, 25);
        let content = text_content(&text);
        assert!(content.len() > 1, "Expected wrapping, got: {content:?}");
        // Every line must start with the blockquote bar.
        for (i, line) in content.iter().enumerate() {
            assert!(
                line.starts_with("\u{258e} "),
                "Line {i} missing blockquote bar: {line:?}",
            );
        }
    }

    #[test]
    fn nested_blockquote_bars_on_wrapped() {
        let input = "> > This is a nested blockquote that should wrap with double bars on every line.";
        let text = render_at_width(input, 25);
        let content = text_content(&text);
        assert!(content.len() > 1, "Expected wrapping, got: {content:?}");
        // Every line must start with double blockquote bars.
        for (i, line) in content.iter().enumerate() {
            assert!(
                line.starts_with("\u{258e} \u{258e} "),
                "Line {i} missing nested blockquote bars: {line:?}",
            );
        }
    }

    #[test]
    fn list_hanging_indent() {
        let input = "- This is a long list item that should wrap with a hanging indent on continuation lines.";
        let text = render_at_width(input, 25);
        let content = text_content(&text);
        assert!(content.len() > 1, "Expected wrapping, got: {content:?}");
        // First line should have the bullet marker.
        assert!(
            content[0].contains('\u{2022}'),
            "First line missing bullet: {:?}",
            content[0],
        );
        // Continuation lines should start with whitespace (hanging indent), not bullet.
        for (i, line) in content.iter().enumerate().skip(1) {
            assert!(
                !line.contains('\u{2022}'),
                "Continuation line {i} should not have bullet: {line:?}",
            );
            // The hanging indent: continuation starts with spaces matching marker width.
            let trimmed = line.trim_start();
            assert!(
                line.len() > trimmed.len(),
                "Continuation line {i} missing hanging indent: {line:?}",
            );
        }
    }

    #[test]
    fn ordered_list_hanging_indent() {
        // pulldown-cmark normalizes numbers, so "10." in markdown becomes "2." as second item.
        let input = "1. First item text that should wrap around\n2. Second item text here";
        let text = render_at_width(input, 20);
        let content = text_content(&text);
        // Find lines with ordered markers.
        let has_one = content.iter().any(|l| l.contains("1. "));
        let has_two = content.iter().any(|l| l.contains("2. "));
        assert!(has_one, "Missing '1.' marker in: {content:?}");
        assert!(has_two, "Missing '2.' marker in: {content:?}");
        // All lines fit within width.
        assert!(
            max_line_width(&text) <= 20,
            "Line exceeded width 20: {content:?}",
        );
        // Continuation lines of the first item should have hanging indent.
        // First item starts with "1. ", continuation lines should start with spaces.
        let first_item_lines: Vec<_> = content.iter().take_while(|l| !l.contains("2. ")).collect();
        if first_item_lines.len() > 1 {
            for cont_line in &first_item_lines[1..] {
                let trimmed = cont_line.trim_start();
                assert!(
                    cont_line.len() > trimmed.len(),
                    "Continuation line missing hanging indent: {cont_line:?}",
                );
            }
        }
    }

    // ── Width-aware rendering tests ────────────────────────────────────

    #[test]
    fn hr_fills_available_width() {
        let text = render_at_width("---", 60);
        let content = text_content(&text);
        let hr_line = content.iter().find(|l| l.contains('─')).expect("HR line missing");
        // Count the number of ─ characters
        let dash_count = hr_line.chars().filter(|&c| c == '─').count();
        assert_eq!(dash_count, 60, "HR should be 60 chars wide, got {dash_count}");
    }

    #[test]
    fn hr_default_width_without_constraint() {
        let text = render_markdown("---\n", None);
        let content = text_content(&text);
        let hr_line = content.iter().find(|l| l.contains('─')).expect("HR line missing");
        let dash_count = hr_line.chars().filter(|&c| c == '─').count();
        assert_eq!(dash_count, 40, "Default HR should be 40 chars wide, got {dash_count}");
    }

    #[test]
    fn code_block_truncated_at_width() {
        let input = "```\nvery long line of code that definitely exceeds the width limit we set\n```";
        let text = render_at_width(input, 30);
        let content = text_content(&text);
        // All lines must fit within 30 columns.
        assert!(
            max_line_width(&text) <= 30,
            "Code block line exceeded width 30: {content:?}",
        );
        // Border chars must be intact.
        let joined = content.join("\n");
        assert!(joined.contains('┌'), "Missing code block header");
        assert!(joined.contains('└'), "Missing code block footer");
        assert!(joined.contains('│'), "Missing code block side borders");
        // Truncated line must have ellipsis.
        assert!(joined.contains('…'), "Missing truncation indicator '…'");
    }

    #[test]
    fn code_block_fits_no_truncation() {
        let input = "```\nshort\n```";
        let text = render_at_width(input, 40);
        let content = text_content(&text);
        let joined = content.join("\n");
        // Borders intact.
        assert!(joined.contains('┌'), "Missing header");
        assert!(joined.contains('└'), "Missing footer");
        // No truncation indicator.
        assert!(!joined.contains('…'), "Unexpected truncation in short code block");
        // Content present.
        assert!(joined.contains("short"), "Code content missing");
    }

    #[test]
    fn table_truncated_at_width() {
        let input = "| Very Long Header Name | Another Long Column Header |\n|---|---|\n| cell content here | more cell content here |\n";
        let text = render_at_width(input, 30);
        let content = text_content(&text);
        // All lines must fit within 30 columns.
        assert!(
            max_line_width(&text) <= 30,
            "Table line exceeded width 30: {content:?}",
        );
        // Table borders must be intact.
        let joined = content.join("\n");
        assert!(joined.contains('┌'), "Missing table top border");
        assert!(joined.contains('┘'), "Missing table bottom border");
        assert!(joined.contains('│'), "Missing table cell border");
    }

    #[test]
    fn table_fits_no_truncation() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let text = render_at_width(input, 80);
        let content = text_content(&text);
        let joined = content.join("\n");
        // No truncation indicator.
        assert!(!joined.contains('…'), "Unexpected truncation in narrow table");
        // Content fully present.
        assert!(joined.contains('A'), "Header content missing");
        assert!(joined.contains('B'), "Header content missing");
        assert!(joined.contains('1'), "Cell content missing");
        assert!(joined.contains('2'), "Cell content missing");
    }

    #[test]
    fn table_cell_content_wraps_instead_of_truncating() {
        let input = "| VeryLongCellContentThatExceedsWidth | Short |\n|---|---|\n| AnotherLongCellContent | X |\n";
        let text = render_at_width(input, 25);
        let content = text_content(&text);
        let joined = content.join("\n");
        // Content should wrap, NOT truncate — no ellipsis.
        assert!(!joined.contains('\u{2026}'), "Content should wrap, not truncate with ellipsis");
        // Extract per-column text by collecting span content from each Line, skipping border spans.
        // Since columns interleave on visual lines, we verify no content is lost by checking
        // that no ellipsis exists and that wrapping produced extra lines (below).
        // Wrapping means more visual lines than a non-wrapped table.
        // A 2-row table (header + 1 data) with no wrapping = 5 lines (top border, header, separator, data, bottom border).
        // With wrapping it must be more.
        assert!(text.lines.len() > 5, "Table should have extra lines from wrapping, got {}", text.lines.len());
        // Borders intact.
        assert!(joined.contains('┌'), "Missing table top border");
        assert!(joined.contains('┘'), "Missing table bottom border");
    }

    #[test]
    fn width_change_produces_different_line_count() {
        let input = "This is a paragraph with enough words to demonstrate that narrower width produces more lines";
        let wide = render_at_width(input, 80);
        let narrow = render_at_width(input, 20);
        assert!(narrow.lines.len() > wide.lines.len(), "Narrow should have more lines");
    }

    #[test]
    fn comprehensive_width_integration() {
        let input = "\
# A Heading That Is Fairly Long

This is a paragraph with enough words to need wrapping at narrow widths.

> A blockquote that also has enough text to require wrapping at this width.

- A list item with enough words to demonstrate hanging indent on continuation lines
- Second item

1. First ordered item with text that wraps
2. Second ordered item

```rust
fn example() { let very_long_variable_name = \"some value\"; }
```

| Key | Description |
|---|---|
| j/k | Navigate up and down |

---
";
        let width = 40;
        let text = render_at_width(input, width);
        let content = text_content(&text);

        // 1. No line exceeds the width
        assert!(
            max_line_width(&text) <= width,
            "Line exceeded width {width}: max was {}",
            max_line_width(&text),
        );

        // 2. Has multiple lines (wrapping occurred)
        assert!(content.len() > 10, "Expected many lines, got {}", content.len());

        // 3. Code block borders present and intact
        let joined = content.join("\n");
        assert!(joined.contains('┌'), "Missing code block header");
        assert!(joined.contains('└'), "Missing code block footer");
        assert!(joined.contains('│'), "Missing code block borders");

        // 4. Table borders present
        assert!(joined.contains("Key"), "Table header missing");
        assert!(joined.contains("Navigate"), "Table content missing");

        // 5. Blockquote bars present
        assert!(joined.contains('▎'), "Blockquote bar missing");

        // 6. List bullets present
        assert!(joined.contains('•'), "List bullet missing");

        // 7. HR present
        assert!(joined.contains('─'), "HR missing");

        // 8. Heading text present
        assert!(joined.contains("Heading"), "Heading text missing");
    }
}
