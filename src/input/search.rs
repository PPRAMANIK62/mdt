use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
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
                self.search_active = false;
                if self.focus == Focus::Preview {
                    self.perform_document_search();
                }
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                if self.focus == Focus::FileList {
                    self.update_file_search_filter();
                }
            }
            _ => {}
        }
    }

    /// Rebuild filtered tree items based on current search query (file search).
    pub(crate) fn update_file_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_tree_items = None;
            self.filtered_path_map = None;
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        let mut filtered_items = Vec::new();
        let mut filtered_map = HashMap::new();

        for (id, (path, is_dir)) in &self.path_map {
            if *is_dir {
                continue;
            }
            let name =
                path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if name.contains(&query_lower) {
                let display_name =
                    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let item = TreeItem::new_leaf(id.clone(), display_name);
                filtered_items.push(item);
                filtered_map.insert(id.clone(), (path.clone(), *is_dir));
            }
        }

        // Sort filtered items alphabetically by their identifier.
        filtered_items.sort_by(|a, b| a.identifier().cmp(b.identifier()));

        self.filtered_tree_items = Some(filtered_items);
        self.filtered_path_map = Some(filtered_map);
    }

    /// Perform in-document search: find all lines containing the query.
    pub(crate) fn perform_document_search(&mut self) {
        self.search_matches.clear();
        self.search_current = 0;

        if self.search_query.is_empty() || self.file_content.is_empty() {
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        for (i, line) in self.file_content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                self.search_matches.push(i);
            }
        }

        // Scroll to first match.
        if let Some(&line_num) = self.search_matches.first() {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message =
                format!("/{} [{}/{}]", self.search_query, 1, self.search_matches.len());
        } else {
            self.status_message = format!("Pattern not found: {}", self.search_query);
        }
    }

    /// Navigate to the next search match.
    pub(crate) fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            // For file search, just keep filter active.
            return;
        }
        if self.search_current + 1 < self.search_matches.len() {
            self.search_current += 1;
        } else {
            self.search_current = 0; // Wrap around.
        }
        if let Some(&line_num) = self.search_matches.get(self.search_current) {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search_query,
                self.search_current + 1,
                self.search_matches.len()
            );
        }
    }

    /// Navigate to the previous search match.
    pub(crate) fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_current > 0 {
            self.search_current -= 1;
        } else {
            self.search_current = self.search_matches.len().saturating_sub(1); // Wrap.
        }
        if let Some(&line_num) = self.search_matches.get(self.search_current) {
            self.scroll_offset = line_num.saturating_sub(2);
            self.clamp_scroll();
            self.status_message = format!(
                "/{} [{}/{}]",
                self.search_query,
                self.search_current + 1,
                self.search_matches.len()
            );
        }
    }

    /// Clear all search state.
    pub(crate) fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
        self.filtered_tree_items = None;
        self.filtered_path_map = None;
        self.status_message.clear();
    }
}
