pub mod editor;
pub mod modal;
pub mod preview;
pub mod status_bar;
pub mod theme;
pub mod welcome;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use crate::app::{App, Focus};
use crate::markdown::LinkInfo;

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
        draw_help_overlay(frame, frame.area());
    }

    if app.show_links {
        let filtered_indices = app.filtered_link_indices();
        let filtered_links: Vec<&LinkInfo> =
            filtered_indices.iter().filter_map(|&i| app.document.links.get(i)).collect();
        draw_links_overlay(
            frame,
            frame.area(),
            &filtered_links,
            app.link_picker_selected,
            &app.link_search_query,
        );
    }
}

fn draw_file_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Use filtered tree items if file search is active, otherwise full tree.
    let items = if let Some(ref filtered) = app.tree.filtered_tree_items {
        filtered
    } else {
        &app.tree.tree_items
    };

    // Show empty vault message if no items.
    if items.is_empty() {
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::new(0, 0, 1, 0));
        let msg = Paragraph::new("No markdown files found")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let selected_is_dir = app
        .tree
        .tree_state
        .selected()
        .last()
        .and_then(|id| app.tree.path_map.get(id))
        .is_some_and(|(_, is_dir)| *is_dir);

    let highlight = if app.focus != Focus::FileList {
        Style::default()
    } else if selected_is_dir {
        Style::default().bg(Color::Blue).fg(Color::Black).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)
    };

    let tree_widget = match Tree::new(items) {
        Ok(tree) => tree
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .padding(Padding::new(0, 0, 1, 0)),
            )
            .highlight_style(highlight)
            .node_open_symbol("▾ ")
            .node_closed_symbol("▸ "),
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
fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    modal::dim_background(frame, area);
    let popup_area = centered_rect(50, 22, area);
    modal::render_shadow(frame, popup_area);
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
        Line::from("  o         Open links"),
        Line::from("  ?         This help"),
        Line::from("  Esc       Close / Clear"),
        Line::from(""),
    ]);

    let popup =
        Paragraph::new(help_text).block(modal::popup_block_with_footer("Help", "Esc to close"));

    frame.render_widget(popup, popup_area);
}

fn draw_links_overlay(
    frame: &mut Frame,
    area: Rect,
    links: &[&LinkInfo],
    selected: usize,
    search_query: &str,
) {
    modal::dim_background(frame, area);

    let content_width = links
        .iter()
        .map(|l| l.display_text.len() + l.url.len() + 4)
        .max()
        .unwrap_or(20)
        .max(search_query.len() + 15)
        .min(60);
    let popup_width = (content_width as u16 + 6).min(area.width.saturating_sub(4));
    let content_rows = links.len().max(1);
    let popup_height = (content_rows as u16 + 6).min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);
    modal::render_shadow(frame, popup_area);
    frame.render_widget(Clear, popup_area);

    let mut text_lines: Vec<Line> = Vec::new();
    text_lines.push(Line::from(""));

    if links.is_empty() {
        text_lines.push(Line::from(Span::styled(
            " No matching links",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let visible_height = popup_height.saturating_sub(6) as usize;
        let scroll_offset =
            if selected >= visible_height { selected - visible_height + 1 } else { 0 };

        for (i, link) in links.iter().enumerate().skip(scroll_offset).take(visible_height) {
            let display = if link.display_text == link.url {
                link.url.clone()
            } else {
                format!("{} → {}", link.display_text, link.url)
            };
            let max_text_width = popup_width.saturating_sub(4) as usize;
            let truncated = if display.len() > max_text_width {
                format!("{}…", &display[..max_text_width.saturating_sub(1)])
            } else {
                display
            };

            if i == selected {
                text_lines.push(Line::from(Span::styled(
                    format!(" {} ", truncated),
                    theme::MODAL_SELECTED,
                )));
            } else {
                text_lines.push(Line::from(format!(" {} ", truncated)));
            }
        }
    }

    let title = if search_query.is_empty() {
        "Links".to_string()
    } else {
        format!("Links: {search_query}█")
    };

    let footer = if search_query.is_empty() {
        "↕ navigate · ↵ open · type to filter · Esc close"
    } else {
        "↕ navigate · ↵ open · Backspace delete · Esc clear"
    };

    let popup = Paragraph::new(Text::from(text_lines))
        .block(modal::popup_block_with_footer(&title, footer));

    frame.render_widget(popup, popup_area);
}
