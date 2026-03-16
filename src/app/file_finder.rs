//! File finder methods on `App`.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::types::Overlay;
use super::App;

impl App {
    /// Open the file finder overlay, populating results with all files.
    pub(crate) fn open_file_finder(&mut self) {
        self.file_finder.query.clear();
        self.file_finder.selected = 0;
        self.file_finder.results = self.collect_all_files();
        self.overlay = Overlay::FileFinder;
    }

    /// Re-filter file finder results based on the current query.
    pub(crate) fn update_file_finder_results(&mut self) {
        if self.file_finder.query.is_empty() {
            self.file_finder.results = self.collect_all_files();
            return;
        }

        let matcher = SkimMatcherV2::default();
        let query = &self.file_finder.query;
        let mut scored: Vec<(i64, String, PathBuf)> = self
            .tree
            .path_map
            .iter()
            .filter(|(_, (_, is_dir))| !is_dir)
            .filter_map(|(rel, (abs, _))| {
                matcher.fuzzy_match(rel, query).map(|score| (score, rel.clone(), abs.clone()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.file_finder.results = scored.into_iter().map(|(_, rel, abs)| (rel, abs)).collect();
    }

    /// Handle a key event while the file finder overlay is active.
    pub(crate) fn handle_file_finder_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down => {
                let len = self.file_finder.results.len();
                if len > 0 {
                    self.file_finder.selected = (self.file_finder.selected + 1) % len;
                }
            }
            KeyCode::Up => {
                let len = self.file_finder.results.len();
                if len > 0 {
                    self.file_finder.selected = if self.file_finder.selected == 0 {
                        len.saturating_sub(1)
                    } else {
                        self.file_finder.selected - 1
                    };
                }
            }
            KeyCode::Enter => {
                if let Some((_, path)) =
                    self.file_finder.results.get(self.file_finder.selected).cloned()
                {
                    self.overlay = Overlay::None;
                    self.open_file(&path);
                }
            }
            KeyCode::Backspace => {
                self.file_finder.query.pop();
                self.update_file_finder_results();
                self.file_finder.selected = 0;
            }
            KeyCode::Esc => {
                if self.file_finder.query.is_empty() {
                    self.overlay = Overlay::None;
                } else {
                    self.file_finder.query.clear();
                    self.update_file_finder_results();
                    self.file_finder.selected = 0;
                }
            }
            KeyCode::Char(c) => {
                self.file_finder.query.push(c);
                self.update_file_finder_results();
                self.file_finder.selected = 0;
            }
            _ => {}
        }
    }

    /// Collect all non-directory files sorted alphabetically.
    fn collect_all_files(&self) -> Vec<(String, PathBuf)> {
        let mut files: Vec<(String, PathBuf)> = self
            .tree
            .path_map
            .iter()
            .filter(|(_, (_, is_dir))| !is_dir)
            .map(|(rel, (abs, _))| (rel.clone(), abs.clone()))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }
}
