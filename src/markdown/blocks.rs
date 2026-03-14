//! Width-independent intermediate representation for rendered markdown.
//!
//! Produced once per file open by [`super::render_markdown_blocks`]; re-wrapped
//! cheaply on width change by [`rewrap_blocks`].

use pulldown_cmark::Alignment;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::theme::*;
use super::wrap::wrap_spans;

/// Width-independent intermediate representation of rendered markdown.
/// Produced once per file open; re-wrapped cheaply on width change.
#[derive(Clone)]
#[allow(dead_code)] // alignments stored for potential future use
pub enum RenderedBlock {
    /// A single line of styled inline spans (paragraph text, heading, list item, etc.)
    /// These get word-wrapped to the target width.
    StyledLine { spans: Vec<Span<'static>>, blockquote_depth: usize, list_marker_width: usize },
    /// Pre-highlighted code block lines. These get truncated (not wrapped) to width.
    CodeBlock { lang: String, highlighted_lines: Vec<Vec<Span<'static>>>, blockquote_depth: usize },
    /// A horizontal rule that fills available width.
    HorizontalRule { blockquote_depth: usize },
    /// A blank line (respects blockquote depth for bars).
    BlankLine { blockquote_depth: usize },
    /// A complete table with pre-styled cells.
    Table {
        rows: Vec<Vec<Vec<Span<'static>>>>,
        alignments: Vec<Alignment>,
        blockquote_depth: usize,
    },
}

/// Re-wrap cached [`RenderedBlock`]s to a new width, producing final display lines.
///
/// This is the cheap "phase 2" of the split pipeline — no parsing or syntax
/// highlighting, just text wrapping and border drawing.
pub fn rewrap_blocks(
    blocks: &[RenderedBlock],
    available_width: Option<usize>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for block in blocks {
        match block {
            RenderedBlock::StyledLine { spans, blockquote_depth, list_marker_width } => {
                rewrap_styled_line(
                    &mut lines,
                    spans,
                    *blockquote_depth,
                    *list_marker_width,
                    available_width,
                );
            }
            RenderedBlock::CodeBlock { lang, highlighted_lines, blockquote_depth } => {
                rewrap_code_block(
                    &mut lines,
                    lang,
                    highlighted_lines,
                    *blockquote_depth,
                    available_width,
                );
            }
            RenderedBlock::HorizontalRule { blockquote_depth } => {
                rewrap_hr(&mut lines, *blockquote_depth, available_width);
            }
            RenderedBlock::BlankLine { blockquote_depth } => {
                rewrap_blank_line(&mut lines, *blockquote_depth);
            }
            RenderedBlock::Table { rows, alignments: _, blockquote_depth } => {
                rewrap_table(&mut lines, rows, *blockquote_depth, available_width);
            }
        }
    }
    lines
}

// ── Styled line (paragraphs, headings, list items, etc.) ────────────────

fn rewrap_styled_line(
    lines: &mut Vec<Line<'static>>,
    spans: &[Span<'static>],
    blockquote_depth: usize,
    list_marker_width: usize,
    available_width: Option<usize>,
) {
    if let Some(width) = available_width {
        let bq_prefix_width = blockquote_depth * BLOCKQUOTE_INDENT_COLS;
        let effective_width = width.saturating_sub(bq_prefix_width);
        let wrapped_lines = wrap_spans(spans, effective_width);

        for (i, line_spans) in wrapped_lines.into_iter().enumerate() {
            let mut final_spans = Vec::new();
            for _ in 0..blockquote_depth {
                final_spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
            }
            if i > 0 && list_marker_width > 0 {
                final_spans.push(Span::styled(" ".repeat(list_marker_width), Style::default()));
            }
            final_spans.extend(line_spans);
            lines.push(Line::from(final_spans));
        }
    } else {
        // No wrapping — original behavior.
        if blockquote_depth > 0 {
            let mut final_spans = Vec::new();
            for _ in 0..blockquote_depth {
                final_spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
            }
            final_spans.extend(spans.iter().cloned());
            lines.push(Line::from(final_spans));
        } else {
            lines.push(Line::from(spans.to_vec()));
        }
    }
}

// ── Code block ──────────────────────────────────────────────────────────

/// Minimum display width for code block boxes.
const CODE_BLOCK_MIN_WIDTH: usize = 20;
/// Padding added to each side of code block content (left + right spaces).
const CODE_BLOCK_BORDER_PAD: usize = 2;

fn spans_display_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

fn rewrap_code_block(
    lines: &mut Vec<Line<'static>>,
    lang: &str,
    highlighted_lines: &[Vec<Span<'static>>],
    blockquote_depth: usize,
    available_width: Option<usize>,
) {
    let effective =
        available_width.map(|w| w.saturating_sub(blockquote_depth * BLOCKQUOTE_INDENT_COLS));

    let content_max = highlighted_lines
        .iter()
        .map(|spans| spans_display_width(spans))
        .max()
        .unwrap_or(CODE_BLOCK_MIN_WIDTH)
        .max(CODE_BLOCK_MIN_WIDTH);

    let max_width = match effective {
        Some(aw) if aw > CODE_BLOCK_BORDER_PAD + 2 => {
            content_max.min(aw - CODE_BLOCK_BORDER_PAD - 2)
        }
        _ => content_max,
    };

    let inner = max_width + CODE_BLOCK_BORDER_PAD;

    // Header: ┌─ lang ─...─┐
    let header_text = if lang.is_empty() {
        format!("┌{}┐", "─".repeat(inner))
    } else {
        let label = format!("─ {} ─", lang);
        let label_width = label.width();
        let remaining = inner.saturating_sub(label_width);
        format!("┌{}{}┐", label, "─".repeat(remaining))
    };
    lines.push(Line::from(Span::styled(header_text, CODE_BORDER_STYLE)));

    // Code lines with borders.
    for line_spans in highlighted_lines {
        let content_width = spans_display_width(line_spans);
        let truncated_spans = if content_width > max_width {
            truncate_code_line(line_spans, max_width)
        } else {
            line_spans.clone()
        };
        let truncated_width = spans_display_width(&truncated_spans);
        let padding = max_width.saturating_sub(truncated_width);
        let mut spans = vec![Span::styled("│ ", CODE_BORDER_STYLE)];
        spans.extend(truncated_spans);
        spans.push(Span::styled(format!("{} │", " ".repeat(padding)), CODE_BORDER_STYLE));
        lines.push(Line::from(spans));
    }

    // Footer: └─...─┘
    lines.push(Line::from(Span::styled(format!("└{}┘", "─".repeat(inner)), CODE_BORDER_STYLE)));
}

/// Truncate a code line to `max_width`, appending "…" if truncated.
fn truncate_code_line(line_spans: &[Span<'static>], max_width: usize) -> Vec<Span<'static>> {
    let budget = max_width.saturating_sub(1); // 1 col for "…"
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for span in line_spans {
        let sw = span.content.width();
        if used + sw <= budget {
            result.push(span.clone());
            used += sw;
        } else {
            // Partial span: take graphemes that fit.
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
}

// ── Horizontal rule ─────────────────────────────────────────────────────

fn rewrap_hr(
    lines: &mut Vec<Line<'static>>,
    blockquote_depth: usize,
    available_width: Option<usize>,
) {
    let bq_offset = blockquote_depth * BLOCKQUOTE_INDENT_COLS;
    let width = available_width.map(|w| w.saturating_sub(bq_offset)).unwrap_or(40);
    let rule = "─".repeat(width);
    lines.push(Line::from(Span::styled(rule, HR_STYLE)));
}

// ── Blank line ──────────────────────────────────────────────────────────

fn rewrap_blank_line(lines: &mut Vec<Line<'static>>, blockquote_depth: usize) {
    if blockquote_depth > 0 {
        let mut spans = Vec::new();
        for _ in 0..blockquote_depth {
            spans.push(Span::styled("▎ ", BLOCKQUOTE_STYLE));
        }
        lines.push(Line::from(spans));
    } else {
        lines.push(Line::default());
    }
}

// ── Table ───────────────────────────────────────────────────────────────

/// Minimum display width for table columns.
const TABLE_MIN_COL_WIDTH: usize = 3;
/// Padding added to each side of a table cell.
const TABLE_CELL_PAD: usize = 2;

fn cell_display_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

fn rewrap_table(
    lines: &mut Vec<Line<'static>>,
    rows: &[Vec<Vec<Span<'static>>>],
    blockquote_depth: usize,
    available_width: Option<usize>,
) {
    if rows.is_empty() {
        return;
    }

    let num_cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    if num_cols == 0 {
        return;
    }

    // Calculate column widths (max content width per column).
    let mut col_widths = vec![0usize; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let width: usize = cell.iter().map(|s| s.content.width()).sum();
            col_widths[i] = col_widths[i].max(width);
        }
    }
    // Minimum column width of 3.
    for w in &mut col_widths {
        *w = (*w).max(TABLE_MIN_COL_WIDTH);
    }

    // Clamp to available width if provided.
    let effective_width =
        available_width.map(|w| w.saturating_sub(blockquote_depth * BLOCKQUOTE_INDENT_COLS));
    if let Some(aw) = effective_width {
        let total: usize =
            col_widths.iter().sum::<usize>() + (num_cols * TABLE_CELL_PAD) + num_cols + 1;
        if total > aw && num_cols > 0 {
            let border_overhead = (num_cols * TABLE_CELL_PAD) + num_cols + 1;
            let available_content = aw.saturating_sub(border_overhead);
            let current_content: usize = col_widths.iter().sum();
            for w in &mut col_widths {
                let shrunk = (*w * available_content) / current_content.max(1);
                *w = shrunk.max(TABLE_MIN_COL_WIDTH);
            }
        }
    }

    // Helper to build a horizontal border line.
    let build_border = |left: &str, mid: &str, right: &str, fill: &str| -> Line<'static> {
        let mut s = left.to_string();
        for (i, &w) in col_widths.iter().enumerate() {
            s.push_str(&fill.repeat(w + TABLE_CELL_PAD));
            if i < num_cols - 1 {
                s.push_str(mid);
            }
        }
        s.push_str(right);
        Line::from(Span::styled(s, TABLE_BORDER_STYLE))
    };

    // Top border: ┌───┬───┐
    lines.push(build_border("┌", "┬", "┐", "─"));

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
                let cell_line_spans = wrapped_cells.get(col_idx).and_then(|wc| wc.get(sub_line));

                let (line_spans, content_width) = match cell_line_spans {
                    Some(ls) => {
                        let w = cell_display_width(ls);
                        (ls.clone(), w)
                    }
                    None => (vec![], 0),
                };

                let padding = col_width.saturating_sub(content_width);
                spans.extend(line_spans);
                spans.push(Span::styled(format!("{} │ ", " ".repeat(padding)), TABLE_BORDER_STYLE));
            }

            lines.push(Line::from(spans));
        }

        // After the first row (header), add separator: ├───┼───┤
        if row_idx == 0 && rows.len() > 1 {
            lines.push(build_border("├", "┼", "┤", "─"));
        }
    }

    // Bottom border: └───┴───┘
    lines.push(build_border("└", "┴", "┘", "─"));
}
