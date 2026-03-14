//! Reusable modal overlay utilities.

use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding};

use super::theme;

pub(crate) fn popup_block<'a>(title: &str, bg_color: Color) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::MODAL_BORDER)
        .title(
            Line::from(Span::styled(format!(" {title} "), theme::MODAL_TITLE))
                .alignment(Alignment::Center),
        )
        .padding(Padding::new(1, 1, 1, 0))
        .style(Style::default().bg(bg_color))
}

pub(crate) fn popup_block_with_footer<'a>(title: &str, footer: &str, bg_color: Color) -> Block<'a> {
    popup_block(title, bg_color).title_bottom(
        Line::from(Span::styled(format!(" {footer} "), theme::MODAL_HINT))
            .alignment(Alignment::Center),
    )
}
