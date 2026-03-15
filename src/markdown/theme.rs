//! Style constants for markdown rendering.

use ratatui::style::{Modifier, Style};

use crate::palette;

pub(super) const H1_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(palette::ACCENT_CYAN);
pub(super) const H2_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(palette::ACCENT_GREEN);
pub(super) const H3_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(palette::ACCENT_YELLOW);
pub(super) const H4_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(palette::FG_MUTED);
pub(super) const BOLD_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(palette::ACCENT_RED);
pub(super) const ITALIC_STYLE: Style = Style::new().add_modifier(Modifier::ITALIC);
pub(super) const STRIKETHROUGH_STYLE: Style = Style::new().add_modifier(Modifier::CROSSED_OUT);
pub(super) const INLINE_CODE_STYLE: Style = Style::new().fg(palette::ACCENT_LIGHT_CYAN);
pub(super) const LINK_STYLE: Style =
    Style::new().add_modifier(Modifier::UNDERLINED).fg(palette::ACCENT_BLUE);
pub(super) const BLOCKQUOTE_STYLE: Style = Style::new().fg(palette::ACCENT_CYAN);
pub(super) const CODE_BORDER_STYLE: Style = Style::new().fg(palette::BORDER);
pub(super) const CODE_DEFAULT_STYLE: Style = Style::new();
pub(super) const HR_STYLE: Style = Style::new().fg(palette::BORDER);
pub(super) const BLOCKQUOTE_INDENT_COLS: usize = 2;
pub(super) const TABLE_HEADER_STYLE: Style =
    Style::new().add_modifier(Modifier::BOLD).fg(palette::ACCENT_CYAN);
pub(super) const TABLE_BORDER_STYLE: Style = Style::new().fg(palette::BORDER);
