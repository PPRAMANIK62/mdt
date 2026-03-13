pub mod editor;
pub mod preview;
pub mod status_bar;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use crate::app::{App, AppMode, Focus};

/// Draw the full UI: file list | preview, plus status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    if app.show_file_tree {
        let main_chunks = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(main_area);
        draw_file_list(frame, app, main_chunks[0]);
        let content_area = main_chunks[1];
        if let Some(ref textarea) = app.textarea {
            editor::draw_editor(frame, textarea, content_area);
        } else {
            preview::draw_preview(frame, app, content_area);
        }
    } else if let Some(ref textarea) = app.textarea {
            editor::draw_editor(frame, textarea, main_area);
        } else {
            preview::draw_preview(frame, app, main_area);
        }

    // --- Status bar ---
    draw_status_bar(frame, app, status_area);

    // --- Help overlay (rendered last so it's on top) ---
    if app.show_help {
        draw_help_overlay(frame, frame.area());
    }
}

fn draw_file_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let border_style = if app.focus == Focus::FileList {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Use filtered tree items if file search is active, otherwise full tree.
    let items = if let Some(ref filtered) = app.filtered_tree_items {
        filtered
    } else {
        &app.tree_items
    };

    // Show empty vault message if no items.
    if items.is_empty() {
        let block = Block::default()
            .title(" Files ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let msg = Paragraph::new("No markdown files found")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let tree_widget = match Tree::new(items) {
        Ok(tree) => tree
            .block(
                Block::default()
                    .title(" Files ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .node_open_symbol("\u{25bc} ")
            .node_closed_symbol("\u{25b6} "),
        Err(_) => return,
    };

    frame.render_stateful_widget(tree_widget, area, &mut app.tree_state);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // In Command mode, show ":" + command_buffer as the full status bar.
    if app.mode == AppMode::Command {
        let line = Line::from(vec![Span::raw(format!(":{}█", app.command_buffer))]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
        return;
    }

    // In Search mode, show "/" + search_query as the full status bar.
    if app.mode == AppMode::Search {
        let line = Line::from(vec![Span::raw(format!("/{}█", app.search_query))]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
        return;
    }

    // Left: mode indicator (reversed style).
    let mode = format!(" {} ", app.mode);

    // Center: file path + dirty indicator + status message.
    let file_info: String = if let Some(ref path) = app.current_file {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let dirty_indicator = if app.is_dirty { " [+]" } else { "" };

    let center = if !app.status_message.is_empty() {
        if file_info.is_empty() {
            app.status_message.clone()
        } else {
            format!("{}{} {}", file_info, dirty_indicator, app.status_message)
        }
    } else if !file_info.is_empty() {
        format!("{}{}", file_info, dirty_indicator)
    } else {
        String::new()
    };

    // Right: line position when a file is open.
    let right = if app.current_file.is_some() && !app.rendered_lines.is_empty() {
        format!(
            "Ln {}/{} ",
            app.scroll_offset.saturating_add(1),
            app.rendered_lines.len()
        )
    } else {
        String::new()
    };

    // Calculate padding to right-align the line position.
    let mode_len = mode.len();
    let center_len = center.len() + 1; // +1 for space after mode
    let right_len = right.len();
    let used = mode_len + center_len + right_len;
    let padding = if area.width as usize > used {
        area.width as usize - used
    } else {
        1
    };

    let line = Line::from(vec![
        Span::styled(mode, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(" "),
        Span::raw(center),
        Span::raw(" ".repeat(padding)),
        Span::raw(right),
    ]);

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
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
fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(40, 20, area);

    // Clear the area behind the popup.
    frame.render_widget(Clear, popup_area);

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
            .border_style(Style::default().fg(Color::White)),
    );

    frame.render_widget(popup, popup_area);
}
