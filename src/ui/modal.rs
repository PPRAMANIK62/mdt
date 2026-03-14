//! Zone-based modal rendering framework.
//!
//! Provides `render_modal_frame` which draws the modal chrome (title bar,
//! optional search bar, shortcuts bar) and returns the content `Rect` for the
//! caller to fill with their own widgets.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph};
use ratatui::Frame;

use super::theme;

/// Compute a centered rectangle of the given size, clamped to the area.
pub(crate) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}

/// Render the modal frame (title bar, optional search bar, shortcuts bar) and
/// return the content area `Rect` for the caller to render into.
///
/// # Zone layout (top to bottom)
///
/// | Zone | Height | Description |
/// |------|--------|-------------|
/// | 1 | 1 | Title bar: left-aligned title + right-aligned "esc" |
/// | gap | 1 | Empty separator |
/// | 2 | 1 (optional) | Search input with placeholder |
/// | gap | 1 (optional) | Empty separator after search |
/// | 3 | remaining | Content area — returned to caller |
/// | gap | 1 (optional) | Empty separator before shortcuts |
/// | 4 | 1 (optional) | Shortcuts bar with action labels + key combos |
pub(crate) fn render_modal_frame(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    search_query: Option<&str>,
    shortcuts: &[(&str, &str)],
    bg_color: Color,
) -> Rect {
    frame.render_widget(Clear, area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::MODAL_BORDER)
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(bg_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let has_search = search_query.is_some();
    let has_shortcuts = !shortcuts.is_empty();

    let mut constraints: Vec<Constraint> = Vec::new();

    // Zone 1: title bar
    constraints.push(Constraint::Length(1));
    // Gap after title
    constraints.push(Constraint::Length(1));

    // Zone 2: search bar (optional)
    if has_search {
        constraints.push(Constraint::Length(1));
        // Gap after search
        constraints.push(Constraint::Length(1));
    }

    // Zone 3: content area
    constraints.push(Constraint::Min(0));

    // Gap + Zone 4: shortcuts bar (optional)
    if has_shortcuts {
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
    }

    let chunks = Layout::vertical(constraints).split(inner);

    let mut idx = 0;

    // ── Zone 1: Title bar ──────────────────────────────────────────────
    let title_area = chunks[idx];
    idx += 1;
    // Skip gap
    idx += 1;

    let title_width = title_area.width as usize;
    let title_len = title.len();
    let esc_label = "esc";
    let esc_len = esc_label.len();
    let padding_len = title_width.saturating_sub(title_len + esc_len);

    let title_line = Line::from(vec![
        Span::styled(title, theme::MODAL_TITLE),
        Span::raw(" ".repeat(padding_len)),
        Span::styled(esc_label, theme::MODAL_ESC_HINT),
    ]);
    frame.render_widget(Paragraph::new(title_line), title_area);

    // ── Zone 2: Search bar (optional) ──────────────────────────────────
    if let Some(query) = search_query {
        let search_area = chunks[idx];
        idx += 1;
        // Skip gap
        idx += 1;

        let search_line = if query.is_empty() {
            Line::from(Span::styled("Search", theme::MODAL_SEARCH_PLACEHOLDER))
        } else {
            Line::from(vec![
                Span::styled(query, theme::MODAL_SEARCH_TEXT),
                Span::styled("█", theme::MODAL_SEARCH_TEXT),
            ])
        };
        frame.render_widget(Paragraph::new(search_line), search_area);
    }

    // ── Zone 3: Content area ───────────────────────────────────────────
    let content_area = chunks[idx];
    idx += 1;

    // ── Zone 4: Shortcuts bar (optional) ───────────────────────────────
    if has_shortcuts {
        // Skip gap
        idx += 1;
        let shortcuts_area = chunks[idx];

        let mut spans: Vec<Span> = Vec::new();
        for (i, (action, key)) in shortcuts.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(*action, theme::MODAL_SHORTCUT_LABEL));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(*key, theme::MODAL_SHORTCUT_KEY));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), shortcuts_area);
    }

    content_area
}
