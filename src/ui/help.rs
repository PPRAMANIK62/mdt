//! Help overlay drawing.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{modal, theme};

pub(crate) const HELP_KEYS: &[(&str, &str)] = &[
    ("j/k", "Navigate / Scroll"),
    ("Enter", "Open file / Enter directory"),
    ("Tab", "Switch focus"),
    ("Spc+e", "Toggle file tree"),
    ("i/e", "Edit mode"),
    ("/", "Search"),
    ("ff", "Find file"),
    ("n/N", "Next/Previous match"),
    ("gg/G", "Top/Bottom"),
    ("Ctrl+d/u", "Half page down/up"),
    ("[/]", "Previous/Next heading"),
    (":w", "Save"),
    (":q", "Quit"),
    ("o", "Open links"),
    ("a", "New file"),
    ("A", "New directory"),
    ("d", "Delete"),
    ("r", "Rename"),
    ("m", "Move"),
    ("?", "This help"),
];

pub(super) fn draw_help_overlay(frame: &mut Frame, area: Rect, bg_color: Color) {
    let popup_area = modal::centered_rect(50, 27, area);
    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        "Help",
        None,
        &[("close", "esc")],
        bg_color,
        false,
    );

    let help_lines: Vec<Line> = HELP_KEYS
        .iter()
        .map(|&(key, desc)| {
            Line::from(vec![
                Span::styled(format!("{key:>12}"), theme::HELP_KEY_STYLE),
                Span::styled("  ", Style::default()),
                Span::styled(desc, theme::HELP_DESC_STYLE),
            ])
        })
        .collect();

    let help_content = Paragraph::new(help_lines);
    frame.render_widget(help_content, content_area);
}
