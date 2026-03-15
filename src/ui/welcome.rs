//! Welcome screen shown when no file is selected.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};
use ratatui::Frame;

const LOGO: &[&str] = &[
    "███╗   ███╗██████╗ ████████╗",
    "████╗ ████║██╔══██╗╚══██╔══╝",
    "██╔████╔██║██║  ██║   ██║   ",
    "██║╚██╔╝██║██║  ██║   ██║   ",
    "██║ ╚═╝ ██║██████╔╝   ██║   ",
    "╚═╝     ╚═╝╚═════╝    ╚═╝   ",
];

const KEYBINDINGS: &[(&str, &str)] = &[
    ("Spc+e", "Open file tree"),
    ("Tab", "Toggle focus"),
    ("j/k", "Navigate files"),
    ("Enter", "Open file"),
    ("/", "Search"),
    ("?", "Help"),
    (":q", "Quit"),
];

/// Draw a centered welcome screen, degrading gracefully for small terminals.
pub fn draw_welcome(frame: &mut Frame, area: Rect, bg_color: Color) {
    let block =
        Block::default().padding(Padding::new(2, 2, 1, 0)).style(Style::default().bg(bg_color));
    let inner = block.inner(area);

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(32);
    let available = inner.height as usize;

    let logo_h = LOGO.len();
    let subtitle_h = 2;
    let gap_h = 2;
    let version_h = 2;
    let total_h = logo_h + subtitle_h + gap_h + KEYBINDINGS.len() + version_h;

    let show_logo = available >= logo_h;
    let show_subtitle = available >= logo_h + subtitle_h;
    let show_keybindings = available >= logo_h + subtitle_h + gap_h + 3;
    let show_version = available >= total_h;

    let mut content_h: usize = 0;
    if show_logo {
        content_h += logo_h;
    }
    if show_subtitle {
        content_h += subtitle_h;
    }
    if show_keybindings {
        content_h += gap_h;
        let remaining =
            available.saturating_sub(content_h + if show_version { version_h } else { 0 });
        content_h += KEYBINDINGS.len().min(remaining);
    }
    if show_version {
        content_h += version_h;
    }

    let top_pad = available.saturating_sub(content_h) / 2;
    for _ in 0..top_pad {
        lines.push(Line::from(""));
    }

    if show_logo {
        let style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        for logo_line in LOGO {
            lines.push(Line::from(Span::styled(*logo_line, style)));
        }
    }

    if show_subtitle {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Terminal Markdown Viewer",
            Style::default().fg(Color::Gray),
        )));
    }

    if show_keybindings {
        lines.push(Line::from(""));
        lines.push(Line::from(""));

        let space_for_kb = available
            .saturating_sub(top_pad)
            .saturating_sub(if show_logo { logo_h } else { 0 })
            .saturating_sub(if show_subtitle { subtitle_h } else { 0 })
            .saturating_sub(gap_h)
            .saturating_sub(if show_version { version_h } else { 0 });
        let kb_count = KEYBINDINGS.len().min(space_for_kb);

        let key_style = Style::default().fg(Color::Yellow);
        let desc_style = Style::default().fg(Color::White);

        for &(key, desc) in KEYBINDINGS.iter().take(kb_count) {
            lines.push(Line::from(vec![
                Span::styled(format!("{key:>7}"), key_style),
                Span::styled("   ", desc_style),
                Span::styled(format!("{desc:<15}"), desc_style),
            ]));
        }
    }

    if show_version {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("v0.1.0", Style::default().fg(Color::DarkGray))));
    }

    if !show_logo {
        lines.push(Line::from(Span::styled(
            "mdt",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Position;
    use ratatui::Terminal;

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .filter_map(|(x, y)| buf.cell(Position::new(x, y)))
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn welcome_screen_renders_logo() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                draw_welcome(f, f.area(), Color::Reset);
            })
            .unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("Terminal Markdown Viewer"));
    }

    #[test]
    fn welcome_screen_renders_keybindings() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                draw_welcome(f, f.area(), Color::Reset);
            })
            .unwrap();
        let text = buffer_text(terminal.backend().buffer());
        assert!(text.contains("Navigate files"));
        assert!(text.contains("Quit"));
    }

    #[test]
    fn welcome_screen_small_terminal_no_panic() {
        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                draw_welcome(f, f.area(), Color::Reset);
            })
            .unwrap();
        assert!(buffer_text(terminal.backend().buffer()).contains("mdt"));
    }
}
