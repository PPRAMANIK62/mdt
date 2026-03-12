pub mod editor;
pub mod preview;
pub mod status_bar;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use crate::app::{App, AppMode, Focus};

/// Draw the full UI: file list | preview, plus status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    let main_chunks = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_area);

    // --- File list ---
    draw_file_list(frame, app, main_chunks[0]);

    // --- Preview ---
    preview::draw_preview(frame, app, main_chunks[1]);

    // --- Status bar ---
    draw_status_bar(frame, app, status_area);
}

fn draw_file_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let border_style = if app.focus == Focus::FileList {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let tree_widget = match Tree::new(&app.tree_items) {
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
        let line = Line::from(vec![
            Span::raw(format!(":{}▌", app.command_buffer)),
        ]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
        return;
    }

    let mode = format!(" {} ", app.mode);
    let file_info = if let Some(ref path) = app.current_file {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let status = if app.status_message.is_empty() {
        file_info
    } else {
        app.status_message.clone()
    };

    let line = Line::from(vec![
        Span::styled(mode, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(" "),
        Span::raw(status),
    ]);

    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}
