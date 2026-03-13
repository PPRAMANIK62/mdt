use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use tui_tree_widget::TreeItem;

use crate::app::{App, AppMode, Focus};

impl App {
    /// Handle key events in Search mode (`/` prefix).
    pub(crate) fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // Cancel search, restore full list.
                self.clear_search();
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                // Confirm search.
                self.search.active = false;
                if self.focus == Focus::Preview {
                    self.perform_document_search();
                }
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.search.query.pop();
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            KeyCode::Char(c) => {
                self.search.query.push(c);
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            _ => {}
        }
    }

    /// Rebuild filtered tree items based on current search query (file search).
    pub(crate) fn update_file_search_filter(&mut self) {
        if self.search.query.is_empty() {
            self.tree.filtered_tree_items = None;
            self.tree.filtered_path_map = None;
            return;
        }

        let query_lower = self.search.query.to_lowercase();
        let mut filtered_items = Vec::new();
        let mut filtered_map = HashMap::new();

        for (id, (path, is_dir)) in &self.tree.path_map {
            if *is_dir {
                continue;
            }
            let name =
                path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if name.contains(&query_lower) {
                let display_name =
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let item = TreeItem::new_leaf(
                    id.clone(),
                    Line::from(Span::styled(
                        display_name,
                        Style::new().fg(Color::Indexed(253)),
                    )),
                );
                filtered_items.push(item);
                filtered_map.insert(id.clone(), (path.clone(), *is_dir));
            }
        }

        // Sort filtered items alphabetically by their identifier.
        filtered_items.sort_by(|a, b| a.identifier().cmp(b.identifier()));

        self.tree.filtered_tree_items = Some(filtered_items);
        self.tree.filtered_path_map = Some(filtered_map);
    }

    /// Perform in-document search: find all lines containing the query.
    pub(crate) fn perform_document_search(&mut self) {
        self.search.matches.clear();
        self.search.current = 0;

        if self.search.query.is_empty() || self.document.rendered_lines.is_empty() {
            return;
        }

        let query_lower = self.search.query.to_lowercase();
        for (i, line) in self.document.rendered_lines.iter().enumerate() {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if text.to_lowercase().contains(&query_lower) {
                self.search.matches.push(i);
            }
        }

        // Scroll to first match.
        if let Some(&line_num) = self.search.matches.first() {
            self.document.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message =
                format!("/{} [{}/{}]", self.search.query, 1, self.search.matches.len());
        } else {
            self.status_message = format!("Pattern not found: {}", self.search.query);
        }
    }

    /// Navigate to the next search match.
    pub(crate) fn next_search_match(&mut self) {
        if self.search.matches.is_empty() {
            // For file search, just keep filter active.
            return;
        }
        if self.search.current + 1 < self.search.matches.len() {
            self.search.current += 1;
        } else {
            self.search.current = 0; // Wrap around.
        }
        if let Some(&line_num) = self.search.matches.get(self.search.current) {
            self.document.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search.query,
                self.search.current + 1,
                self.search.matches.len()
            );
        }
    }

    /// Navigate to the previous search match.
    pub(crate) fn prev_search_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        if self.search.current > 0 {
            self.search.current -= 1;
        } else {
            self.search.current = self.search.matches.len().saturating_sub(1); // Wrap.
        }
        if let Some(&line_num) = self.search.matches.get(self.search.current) {
            self.document.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search.query,
                self.search.current + 1,
                self.search.matches.len()
            );
        }
    }

    /// Clear all search state.
    pub(crate) fn clear_search(&mut self) {
        self.search.active = false;
        self.search.query.clear();
        self.search.matches.clear();
        self.search.current = 0;
        self.tree.filtered_tree_items = None;
        self.tree.filtered_path_map = None;
        self.status_message.clear();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, AppMode};
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

    #[test]
    fn esc_in_search_clears_and_returns_to_normal() {
        let dir = TempTestDir::new("mdt-test-search-esc");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Search;
        app.search.active = true;
        app.search.query = "hello".to_string();

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_search_key(key);

        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.search.active);
        assert!(app.search.query.is_empty());
    }

    #[test]
    fn char_keys_append_to_search_query() {
        let dir = TempTestDir::new("mdt-test-search-char");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.mode = AppMode::Search;

        app.handle_search_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.handle_search_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

        assert_eq!(app.search.query, "ab");
    }
}
