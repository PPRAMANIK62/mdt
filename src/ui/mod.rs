pub mod editor;
pub mod preview;
pub mod status_bar;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use crate::app::{App, Focus};

/// Draw the full UI: file list | preview, plus status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    // Fill entire frame with solid terminal background color to prevent transparency.
    frame.render_widget(Block::default().style(Style::default().bg(app.bg_color)), frame.area());

    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    if app.show_file_tree {
        let main_chunks =
            Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(main_area);
        draw_file_list(frame, app, main_chunks[0]);
        let content_area = main_chunks[1];
        if let Some(ref textarea) = app.editor.textarea {
            editor::draw_editor(frame, textarea, content_area);
        } else {
            preview::draw_preview(frame, app, content_area);
        }
    } else if let Some(ref textarea) = app.editor.textarea {
        editor::draw_editor(frame, textarea, main_area);
    } else {
        preview::draw_preview(frame, app, main_area);
    }

    // --- Status bar ---
    status_bar::draw_status_bar(frame, app, status_area);

    // --- Help overlay (rendered last so it's on top) ---
    if app.show_help {
        draw_help_overlay(frame, frame.area(), app.bg_color);
    }
}

fn draw_file_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let border_style = if app.focus == Focus::FileList {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Use filtered tree items if file search is active, otherwise full tree.
    let items = if let Some(ref filtered) = app.tree.filtered_tree_items {
        filtered
    } else {
        &app.tree.tree_items
    };

    // Show empty vault message if no items.
    if items.is_empty() {
        let block =
            Block::default().title(" Files ").borders(Borders::ALL).border_style(border_style);
        let msg = Paragraph::new("No markdown files found")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let tree_widget = match Tree::new(items) {
        Ok(tree) => tree
            .block(
                Block::default().title(" Files ").borders(Borders::ALL).border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .node_open_symbol("\u{25bc} ")
            .node_closed_symbol("\u{25b6} "),
        Err(_) => return,
    };

    frame.render_stateful_widget(tree_widget, area, &mut app.tree.tree_state);
}

/// Compute a centered rectangle of the given size, clamped to the area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}

/// Draw the help overlay popup.
fn draw_help_overlay(frame: &mut Frame, area: Rect, bg_color: Color) {
    let popup_area = centered_rect(40, 20, area);

    // Clear the area behind the popup.
    frame.render_widget(Clear, popup_area);
    frame.render_widget(Block::default().style(Style::default().bg(bg_color)), popup_area);

    let help_text = Text::from(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            " Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("  j/k       Navigate / Scroll"),
        Line::from("  Enter     Open file / Enter directory"),
        Line::from("  Tab       Switch focus"),
        Line::from("  Space+e   Toggle file tree"),
        Line::from("  i/e       Edit mode"),
        Line::from("  /         Search"),
        Line::from("  n/N       Next/Previous match"),
        Line::from("  gg/G      Top/Bottom"),
        Line::from("  Ctrl+d/u  Half page down/up"),
        Line::from("  :w        Save"),
        Line::from("  :q        Quit"),
        Line::from("  ?         This help"),
        Line::from("  Esc       Close / Clear"),
        Line::from(""),
    ]);

    let popup = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(bg_color)),
    );

    frame.render_widget(popup, popup_area);
}
