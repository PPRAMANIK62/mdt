//! File list (tree) panel drawing.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use crate::app::{App, Focus};

pub(super) fn draw_file_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
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
