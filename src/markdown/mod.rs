//! Custom markdown-to-ratatui renderer using pulldown-cmark.
//!
//! Produces styled [`Text`] with all syntax markers stripped — headings, bold, italic,
//! strikethrough, inline code, code blocks, lists, blockquotes, links, horizontal rules,
//! and task lists are all rendered as properly styled text.
//!
//! The rendering pipeline is split into two phases:
//! 1. **`render_markdown_blocks`** — parses markdown + syntax highlights code blocks → cached blocks
//! 2. **`rewrap_blocks`** — re-wraps cached blocks to a given width → `Vec<Line<'static>>`

use pulldown_cmark::{Options, Parser};
use ratatui::text::Span;

pub(crate) mod blocks;
pub(crate) mod syntax;
mod wrap;
use syntax::no_color;
mod renderer;
mod theme;
use renderer::Renderer;
use theme::*;

pub(crate) use blocks::{rewrap_blocks, RenderedBlock};
pub(crate) use renderer::LinkInfo;

#[cfg(test)]
mod test_helpers;

/// Render markdown input into styled ratatui [`Text`].
///
/// - Pre-expands tabs to 4 spaces (ratatui `Paragraph` silently drops tabs).
/// - Respects the `NO_COLOR` environment variable: when set, returns plain unstyled text.
/// - All markdown syntax markers are stripped; styling is applied via ratatui modifiers/colors.
#[cfg(test)]
pub fn render_markdown(
    input: &str,
    available_width: Option<usize>,
) -> ratatui::text::Text<'static> {
    use ratatui::text::Text;
    let cleaned = input.replace('\t', "    ");

    if no_color() {
        return Text::raw(cleaned);
    }

    let (blocks, _links) = render_markdown_blocks(input);
    let lines = rewrap_blocks(&blocks, available_width);
    Text::from(lines)
}

/// Render markdown to width-independent intermediate blocks.
///
/// This is the expensive "phase 1" of the split pipeline — parses markdown and
/// syntax-highlights all code blocks. The result can be cached and cheaply re-wrapped
/// to different widths via [`rewrap_blocks`].
pub(crate) fn render_markdown_blocks(input: &str) -> (Vec<RenderedBlock>, Vec<LinkInfo>) {
    let cleaned = input.replace('\t', "    ");

    if no_color() {
        return (
            cleaned
                .lines()
                .map(|l| RenderedBlock::StyledLine {
                    spans: vec![Span::raw(l.to_string())],
                    blockquote_depth: 0,
                    list_marker_width: 0,
                })
                .collect(),
            Vec::new(),
        );
    }

    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(&cleaned, options);

    let mut renderer = Renderer::new();
    renderer.run(parser);
    renderer.into_blocks()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};
    use test_helpers::*;

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
        assert!(
            !joined.contains("https://example.com"),
            "URL should be hidden when display text differs"
        );
        assert!(!joined.contains('['), "Link bracket visible");
    }

    #[test]
    fn autolink_shows_url() {
        let text = render_markdown("<https://example.com>\n", None);
        let content = text_content(&text);
        let joined = content.join(" ");
        assert!(joined.contains("https://example.com"), "Autolink URL should be visible");
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

    // ── Width-aware wrapping tests ──────────────────────────────────────

    #[test]
    fn paragraph_wraps_to_width() {
        let input = "This is a paragraph with enough words to require wrapping at a narrow width.";
        let text = render_at_width(input, 30);
        let content = text_content(&text);
        // Should produce multiple lines.
        assert!(content.len() > 1, "Expected wrapping, got: {content:?}");
        // Every line should fit within the specified width.
        assert!(max_line_width(&text) <= 30, "Line exceeded width 30: {content:?}",);
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
            assert!(line.starts_with("\u{258e} "), "Line {i} missing blockquote bar: {line:?}",);
        }
    }

    #[test]
    fn nested_blockquote_bars_on_wrapped() {
        let input =
            "> > This is a nested blockquote that should wrap with double bars on every line.";
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
        assert!(content[0].contains('\u{2022}'), "First line missing bullet: {:?}", content[0],);
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
        assert!(max_line_width(&text) <= 20, "Line exceeded width 20: {content:?}",);
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
        let input =
            "```\nvery long line of code that definitely exceeds the width limit we set\n```";
        let text = render_at_width(input, 30);
        let content = text_content(&text);
        // All lines must fit within 30 columns.
        assert!(max_line_width(&text) <= 30, "Code block line exceeded width 30: {content:?}",);
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
        assert!(max_line_width(&text) <= 30, "Table line exceeded width 30: {content:?}",);
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
        assert!(
            text.lines.len() > 5,
            "Table should have extra lines from wrapping, got {}",
            text.lines.len()
        );
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
