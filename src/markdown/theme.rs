//! Style constants for markdown rendering.

use ratatui::style::{Color, Modifier, Style};

pub(super) const H1_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Cyan);
pub(super) const H2_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Green);
pub(super) const H3_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::Yellow);
pub(super) const H4_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::DarkGray);
pub(super) const BOLD_STYLE: Style = Style::new().add_modifier(Modifier::BOLD).fg(Color::LightRed);
pub(super) const ITALIC_STYLE: Style = Style::new().add_modifier(Modifier::ITALIC);
pub(super) const STRIKETHROUGH_STYLE: Style = Style::new().add_modifier(Modifier::CROSSED_OUT);
pub(super) const INLINE_CODE_STYLE: Style = Style::new().fg(Color::LightCyan);
pub(super) const LINK_STYLE: Style =
    Style::new().add_modifier(Modifier::UNDERLINED).fg(Color::Blue);
pub(super) const BLOCKQUOTE_STYLE: Style = Style::new().fg(Color::Cyan);
pub(super) const CODE_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
pub(super) const CODE_DEFAULT_STYLE: Style = Style::new();
pub(super) const HR_STYLE: Style = Style::new().fg(Color::DarkGray);
pub(super) const BLOCKQUOTE_INDENT_COLS: usize = 2;
pub(super) const TABLE_HEADER_STYLE: Style =
    Style::new().add_modifier(Modifier::BOLD).fg(Color::Cyan);
pub(super) const TABLE_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
