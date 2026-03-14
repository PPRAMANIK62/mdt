//! Style constants for modal overlays.

use ratatui::style::{Color, Modifier, Style};

pub(crate) const MODAL_BORDER: Style = Style::new().fg(Color::Gray);
pub(crate) const MODAL_TITLE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
pub(crate) const MODAL_HINT: Style = Style::new().fg(Color::DarkGray);
pub(crate) const MODAL_SELECTED: Style =
    Style::new().bg(Color::Indexed(236)).add_modifier(Modifier::BOLD);
pub(crate) const MODAL_SHADOW: Color = Color::Indexed(233);
pub(crate) const MODAL_DIM_BG: Color = Color::Indexed(232);
