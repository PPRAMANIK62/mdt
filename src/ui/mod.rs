pub mod editor;
pub mod preview;
pub mod status_bar;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus};

/// Draw the full UI: file list | preview, plus status bar.
pub fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let main_area = outer[0];
    let status_area = outer[1];

    let main_chunks = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_area);

    // --- File list ---
    draw_file_list(frame, app, main_chunks[0]);

    // --- Preview ---
    draw_preview(frame, app, main_chunks[1]);

    // --- Status bar ---
    draw_status_bar(frame, app, status_area);
}

fn draw_file_list(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let border_style = if app.focus == Focus::FileList {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = app
        .file_tree
        .entries
        .iter()
        .map(|entry| {
            let prefix = if entry.is_dir { "📁 " } else { "  " };
            ListItem::new(format!("{prefix}{}", entry.name))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Files ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    if !app.file_tree.entries.is_empty() {
        state.select(Some(app.file_tree.selected));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let border_style = if app.focus == Focus::Preview {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let title = app
        .current_file
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| format!(" {} ", n.to_string_lossy()))
        .unwrap_or_else(|| " Preview ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.rendered_lines.is_empty() {
        let placeholder = Paragraph::new("Select a file to preview").block(block);
        frame.render_widget(placeholder, area);
    } else {
        let text = ratatui::text::Text::from(app.rendered_lines.clone());
        // Paragraph::scroll takes (y, x).
        let paragraph = Paragraph::new(text)
            .block(block)
            .scroll((app.scroll_offset as u16, 0));
        frame.render_widget(paragraph, area);
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
