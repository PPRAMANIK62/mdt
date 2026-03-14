//! Reusable modal overlay utilities.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding};
use ratatui::Frame;

use super::theme;

pub(crate) fn popup_block<'a>(title: &str) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::MODAL_BORDER)
        .title(
            Line::from(Span::styled(format!(" {title} "), theme::MODAL_TITLE))
                .alignment(Alignment::Center),
        )
        .padding(Padding::new(1, 1, 1, 0))
}

pub(crate) fn popup_block_with_footer<'a>(title: &str, footer: &str) -> Block<'a> {
    popup_block(title).title_bottom(
        Line::from(Span::styled(format!(" {footer} "), theme::MODAL_HINT))
            .alignment(Alignment::Center),
    )
}

pub(crate) fn dim_background(frame: &mut Frame, area: Rect) {
    frame.render_widget(Block::default().style(Style::default().bg(theme::MODAL_DIM_BG)), area);
}

pub(crate) fn render_shadow(frame: &mut Frame, popup_area: Rect) {
    let shadow_area = Rect {
        x: popup_area.x.saturating_add(1),
        y: popup_area.y.saturating_add(1),
        width: popup_area.width,
        height: popup_area.height,
    }
    .intersection(frame.area());

    frame.render_widget(
        Block::default().style(Style::default().bg(theme::MODAL_SHADOW)),
        shadow_area,
    );
}
