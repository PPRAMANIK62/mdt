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
pub(crate) use renderer::{deduplicate_links, LinkInfo};

#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod tests;

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
    let (lines, _block_starts) = rewrap_blocks(&blocks, available_width);
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
        // Still extract links even when color is disabled.
        let options =
            Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
        let parser = Parser::new_ext(&cleaned, options);
        let mut renderer = Renderer::new();
        renderer.run(parser);
        let (_styled_blocks, links) = renderer.into_blocks();

        let plain_blocks = cleaned
            .lines()
            .map(|l| RenderedBlock::StyledLine {
                spans: vec![Span::raw(l.to_string())],
                blockquote_depth: 0,
                list_marker_width: 0,
                heading_level: None,
            })
            .collect();

        return (plain_blocks, links);
    }

    let options =
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(&cleaned, options);

    let mut renderer = Renderer::new();
    renderer.run(parser);
    renderer.into_blocks()
}
