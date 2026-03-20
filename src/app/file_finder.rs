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
        if self.stdin_mode {
            self.status_message = "Not available (stdin)".to_string();
            return;
        }
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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::style::Color;

    use crate::app::{App, Overlay};
    use crate::test_util::TempTestDir;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn open_file_finder_shows_all_files() {
        let dir = TempTestDir::new("mdt-test-finder-open");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");
        dir.create_file("c.md", "# C");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();

        assert!(matches!(app.overlay, Overlay::FileFinder));
        assert_eq!(app.file_finder.results.len(), 3);
        assert_eq!(app.file_finder.selected, 0);
        assert!(app.file_finder.query.is_empty());
        // Sorted alphabetically
        assert_eq!(app.file_finder.results[0].0, "a.md");
        assert_eq!(app.file_finder.results[1].0, "b.md");
        assert_eq!(app.file_finder.results[2].0, "c.md");
    }

    #[test]
    fn open_file_finder_excludes_directories() {
        let dir = TempTestDir::new("mdt-test-finder-no-dirs");
        dir.create_file("a.md", "# A");
        std::fs::create_dir_all(dir.path().join("subdir")).unwrap();
        dir.create_file("subdir/b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();

        // Only files, not directories
        for (rel, _) in &app.file_finder.results {
            assert!(rel.ends_with(".md"));
        }
    }

    #[test]
    fn update_file_finder_filters_by_query() {
        let dir = TempTestDir::new("mdt-test-finder-filter");
        dir.create_file("alpha.md", "# A");
        dir.create_file("beta.md", "# B");
        dir.create_file("gamma.md", "# G");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();

        app.file_finder.query = "alpha".to_string();
        app.update_file_finder_results();

        assert_eq!(app.file_finder.results.len(), 1);
        assert_eq!(app.file_finder.results[0].0, "alpha.md");
    }

    #[test]
    fn update_file_finder_empty_query_shows_all() {
        let dir = TempTestDir::new("mdt-test-finder-empty-q");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        app.file_finder.query.clear();
        app.update_file_finder_results();

        assert_eq!(app.file_finder.results.len(), 2);
    }

    #[test]
    fn file_finder_key_down_wraps_around() {
        let dir = TempTestDir::new("mdt-test-finder-down");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        assert_eq!(app.file_finder.selected, 0);

        app.handle_file_finder_key(key(KeyCode::Down));
        assert_eq!(app.file_finder.selected, 1);

        app.handle_file_finder_key(key(KeyCode::Down));
        assert_eq!(app.file_finder.selected, 0); // wrapped
    }

    #[test]
    fn file_finder_key_up_wraps_around() {
        let dir = TempTestDir::new("mdt-test-finder-up");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        assert_eq!(app.file_finder.selected, 0);

        app.handle_file_finder_key(key(KeyCode::Up));
        assert_eq!(app.file_finder.selected, 1); // wrapped to end
    }

    #[test]
    fn file_finder_char_adds_to_query_and_resets_selection() {
        let dir = TempTestDir::new("mdt-test-finder-char");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        app.file_finder.selected = 1;

        app.handle_file_finder_key(key(KeyCode::Char('a')));
        assert_eq!(app.file_finder.query, "a");
        assert_eq!(app.file_finder.selected, 0);
    }

    #[test]
    fn file_finder_backspace_pops_char() {
        let dir = TempTestDir::new("mdt-test-finder-bksp");
        dir.create_file("a.md", "# A");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        app.file_finder.query = "ab".to_string();

        app.handle_file_finder_key(key(KeyCode::Backspace));
        assert_eq!(app.file_finder.query, "a");
        assert_eq!(app.file_finder.selected, 0);
    }

    #[test]
    fn file_finder_esc_clears_query_first() {
        let dir = TempTestDir::new("mdt-test-finder-esc");
        dir.create_file("a.md", "# A");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();
        app.file_finder.query = "test".to_string();

        // First Esc clears query
        app.handle_file_finder_key(key(KeyCode::Esc));
        assert!(app.file_finder.query.is_empty());
        assert!(matches!(app.overlay, Overlay::FileFinder));

        // Second Esc closes overlay
        app.handle_file_finder_key(key(KeyCode::Esc));
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn file_finder_enter_opens_file() {
        let dir = TempTestDir::new("mdt-test-finder-enter");
        dir.create_file("a.md", "# Hello");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file_finder();

        app.handle_file_finder_key(key(KeyCode::Enter));

        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.document.current_file.is_some());
    }
}
