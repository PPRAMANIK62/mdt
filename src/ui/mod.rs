pub mod editor;
pub mod modal;
pub mod preview;
pub mod status_bar;
pub mod theme;
pub mod welcome;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;
use tui_tree_widget::Tree;

use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus};
use crate::markdown::LinkInfo;

const HELP_KEYS: &[(&str, &str)] = &[
    ("j/k", "Navigate / Scroll"),
    ("Enter", "Open file / Enter directory"),
    ("Tab", "Switch focus"),
    ("Spc+e", "Toggle file tree"),
    ("i/e", "Edit mode"),
    ("/", "Search"),
    ("n/N", "Next/Previous match"),
    ("gg/G", "Top/Bottom"),
    ("Ctrl+d/u", "Half page down/up"),
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
            app.bg_color,
            app.cursor_visible,
        );
    }

    if app.show_file_op {
        draw_file_op_overlay(frame, frame.area(), app);
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

fn draw_help_overlay(frame: &mut Frame, area: Rect, bg_color: Color) {
    let popup_area = modal::centered_rect(50, 26, area);
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

fn draw_links_overlay(
    frame: &mut Frame,
    area: Rect,
    links: &[&LinkInfo],
    selected: usize,
    search_query: &str,
    bg_color: Color,
    cursor_visible: bool,
) {
    let content_width = links
        .iter()
        .map(|l| UnicodeWidthStr::width(l.display_text.as_str()))
        .max()
        .unwrap_or(20)
        .max(search_query.len() + 15)
        .min(60);
    let popup_width = (content_width as u16 + 10).min(area.width.saturating_sub(4));
    let content_rows = links.len().max(1);
    let max_height = (area.height * 3 / 4).max(10);
    let popup_height =
        (content_rows as u16 + 10).min(max_height).min(area.height.saturating_sub(4));

    let popup_area = modal::centered_rect(popup_width, popup_height, area);

    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        "Links",
        Some((search_query, "Search")),
        &[("open", "enter"), ("navigate", "↕"), ("close", "esc")],
        bg_color,
        cursor_visible,
    );

    let mut text_lines: Vec<Line> = Vec::new();

    if links.is_empty() {
        text_lines.push(Line::from(Span::styled("No matching links", theme::MODAL_HINT)));
    } else {
        let visible_height = content_area.height as usize;
        let scroll_offset =
            if selected >= visible_height { selected - visible_height + 1 } else { 0 };

        for (i, link) in links.iter().enumerate().skip(scroll_offset).take(visible_height) {
            let display = link.display_text.clone();
            let max_text_width = content_area.width as usize;
            let display_width = UnicodeWidthStr::width(display.as_str());
            let truncated = if display_width > max_text_width {
                let mut width = 0;
                let mut end = 0;
                for (i, ch) in display.char_indices() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if width + ch_width > max_text_width.saturating_sub(1) {
                        break;
                    }
                    width += ch_width;
                    end = i + ch.len_utf8();
                }
                format!("{}…", &display[..end])
            } else {
                display
            };

            if i == selected {
                let padded = format!("{:<width$}", truncated, width = max_text_width);
                text_lines.push(Line::from(Span::styled(padded, theme::MODAL_SELECTED)));
            } else {
                text_lines.push(Line::from(truncated));
            }
        }
    }

    let links_content = Paragraph::new(text_lines);
    frame.render_widget(links_content, content_area);
}

fn draw_file_op_overlay(frame: &mut Frame, area: Rect, app: &App) {
    let Some(ref op) = app.file_op else {
        return;
    };

    let (title, hint, shortcuts, show_input, placeholder) = match op {
        crate::app::FileOp::CreateFile { parent_dir } => {
            let rel = parent_dir.strip_prefix(&app.root_path).unwrap_or(parent_dir);
            let dir_display = if rel.as_os_str().is_empty() {
                "./".to_string()
            } else {
                format!("{}/", rel.display())
            };
            ("New File", format!("in {dir_display}"), vec![("confirm", "enter")], true, "filename")
        }
        crate::app::FileOp::CreateDir { parent_dir } => {
            let rel = parent_dir.strip_prefix(&app.root_path).unwrap_or(parent_dir);
            let dir_display = if rel.as_os_str().is_empty() {
                "./".to_string()
            } else {
                format!("{}/", rel.display())
            };
            (
                "New Directory",
                format!("in {dir_display}"),
                vec![("confirm", "enter")],
                true,
                "dirname",
            )
        }
        crate::app::FileOp::Rename { target, .. } => {
            let name = target.file_name().unwrap_or_default().to_string_lossy();
            ("Rename", format!("current: {name}"), vec![("confirm", "enter")], true, "new name")
        }
        crate::app::FileOp::Delete { name, is_dir, .. } => {
            let what = if *is_dir { "directory" } else { "file" };
            (
                "Delete",
                format!("Delete {what} \"{name}\"?"),
                vec![("confirm", "enter"), ("cancel", "esc")],
                false,
                "",
            )
        }
        crate::app::FileOp::Move { source, .. } => {
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            ("Move", format!("moving: {name}"), vec![("confirm", "enter")], true, "destination")
        }
    };

    let popup_area = modal::centered_rect(50, 11, area);

    let input = if show_input { Some((app.file_op_input.as_str(), placeholder)) } else { None };
    let shortcut_refs: Vec<(&str, &str)> = shortcuts.iter().map(|(a, b)| (*a, *b)).collect();

    let content_area = modal::render_modal_frame(
        frame,
        popup_area,
        title,
        input,
        &shortcut_refs,
        app.bg_color,
        app.cursor_visible,
    );

    let hint_line = Line::from(Span::styled(&*hint, theme::MODAL_HINT));
    frame.render_widget(Paragraph::new(hint_line), content_area);
}
