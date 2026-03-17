use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

use crate::app::{App, AppMode, Focus, Overlay};
/// Timeout in milliseconds for composed key sequences (e.g., `gg`, `Space+e`).
const DOUBLE_KEY_TIMEOUT_MS: u128 = 500;

impl App {
    /// Handle key events in Normal mode.
    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) {
        // If we're in editor view (textarea is Some), handle editor normal-mode keys.
        if self.editor.textarea.is_some() {
            self.handle_editor_normal_key(key);
            return;
        }

        // Check for composed commands (e.g., gg) — works in both FileList and Preview.
        if let Some((pending_char, instant)) = self.pending_key.take() {
            if instant.elapsed().as_millis() < DOUBLE_KEY_TIMEOUT_MS {
                match (pending_char, key.code) {
                    ('g', KeyCode::Char('g')) => {
                        match self.focus {
                            Focus::Preview => self.document.scroll_to_top(),
                            Focus::FileList => {
                                self.tree.tree_state.select_first();
                            }
                        }
                        return;
                    }
                    (' ', KeyCode::Char('e')) => {
                        self.toggle_file_tree();
                        return;
                    }
                    ('f', KeyCode::Char('f')) => {
                        self.open_file_finder();
                        return;
                    }
                    _ => {} // expired or unrecognized — fall through
                }
            }
            // Pending key expired or didn't match — fall through to normal handling.
        }

        match key.code {
            // --- File operations (FileList only) ---
            KeyCode::Char('a') => {
                if self.focus == Focus::FileList {
                    self.start_create_file();
                }
            }
            KeyCode::Char('A') => {
                if self.focus == Focus::FileList {
                    self.start_create_dir();
                }
            }
            KeyCode::Char('d')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.focus == Focus::FileList =>
            {
                self.start_delete();
            }
            KeyCode::Char('r') => {
                if self.focus == Focus::FileList {
                    self.start_rename();
                }
            }
            KeyCode::Char('m') => {
                if self.focus == Focus::FileList {
                    self.start_move();
                }
            }

            // --- Navigation (focus-dependent) ---
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::FileList => {
                    self.tree.tree_state.key_down();
                }
                Focus::Preview => self.document.scroll_down(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::FileList => {
                    self.tree.tree_state.key_up();
                }
                Focus::Preview => self.document.scroll_up(),
            },
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Tab => self.toggle_focus(),

            // --- FileList-only navigation ---
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                if self.focus == Focus::FileList {
                    self.tree.tree_state.key_left();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.focus == Focus::FileList {
                    self.tree.tree_state.key_right();
                }
            }

            // --- G: last item (FileList) or scroll bottom (Preview) ---
            KeyCode::Char('G') => match self.focus {
                Focus::FileList => {
                    self.tree.tree_state.select_last();
                }
                Focus::Preview => self.document.scroll_to_bottom(),
            },

            // --- g: start pending key for gg (both focuses) ---
            KeyCode::Char('g') => {
                self.pending_key = Some(('g', Instant::now()));
            }

            // --- f: start pending key for ff (file finder) ---
            KeyCode::Char('f') => {
                self.pending_key = Some(('f', Instant::now()));
            }

            // --- Space: start pending key for Space+e (leader key) ---
            KeyCode::Char(' ') => {
                self.pending_key = Some((' ', Instant::now()));
            }

            // --- Preview-only scrolling ---
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == Focus::Preview {
                    self.document.scroll_half_page_down();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == Focus::Preview {
                    self.document.scroll_half_page_up();
                }
            }

            // --- Mode transitions ---
            KeyCode::Char(':') => {
                self.mode = AppMode::Command;
                self.command_buffer.clear();
            }
            KeyCode::Char('/') => {
                self.search.active = true;
                self.search.query.clear();
                self.search.matches.clear();
                self.search.current = 0;
                self.mode = AppMode::Search;
            }
            KeyCode::Char('n') => self.next_search_match(),
            KeyCode::Char('N') => self.prev_search_match(),
            KeyCode::Esc => {
                // Clear active search results
                self.clear_search();
            }
            KeyCode::Char('i' | 'e') => {
                if self.focus == Focus::Preview {
                    self.enter_editor();
                }
            }
            KeyCode::Char('o') => {
                if self.focus == Focus::Preview {
                    if self.document.links.is_empty() {
                        self.status_message = "No links in document".to_string();
                    } else {
                        self.overlay = Overlay::LinkPicker;
                        self.link_picker.selected = 0;
                        self.link_picker.search_query.clear();
                    }
                }
            }

            // --- Help ---
            KeyCode::Char('?') => {
                if self.editor.textarea.is_none() {
                    self.overlay = match self.overlay {
                        Overlay::Help => Overlay::None,
                        _ => Overlay::Help,
                    };
                }
            }

            KeyCode::Char(']') if self.focus == Focus::Preview => {
                self.document.jump_to_next_heading();
            }
            KeyCode::Char('[') if self.focus == Focus::Preview => {
                self.document.jump_to_prev_heading();
            }

            KeyCode::Char('q') => self.should_quit = true,

            _ => {}
        }
    }

    /// Open the selected file tree entry.
    pub(crate) fn handle_enter(&mut self) {
        if self.focus != Focus::FileList {
            return;
        }
        let selected = self.tree.tree_state.selected().to_vec();
        let Some(id) = selected.last() else {
            return;
        };
        let info = self.tree.path_map.get(id).cloned();
        if let Some((path, is_dir)) = info {
            if is_dir {
                self.tree.tree_state.toggle(selected);
            } else {
                self.open_file(&path);
            }
        }
    }

    pub(crate) fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileList => Focus::Preview,
            Focus::Preview if self.show_file_tree => Focus::FileList,
            Focus::Preview => Focus::Preview,
        };
    }

    pub(crate) fn toggle_file_tree(&mut self) {
        self.show_file_tree = !self.show_file_tree;
        if self.show_file_tree {
            self.focus = Focus::FileList;
        } else {
            self.focus = Focus::Preview;
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::app::{App, AppMode, Focus, Overlay};
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

    #[test]
    fn j_key_in_file_list_dispatches_tree_navigation() {
        let dir = TempTestDir::new("mdt-test-normal-j");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert_eq!(app.focus, Focus::FileList);
        assert_eq!(app.mode, AppMode::Normal);

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_normal_key(key);

        // Dispatch went to FileList branch — mode and focus unchanged.
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.focus, Focus::FileList);
    }

    #[test]
    fn colon_key_enters_command_mode() {
        let dir = TempTestDir::new("mdt-test-normal-colon");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert_eq!(app.mode, AppMode::Normal);

        let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
        app.handle_normal_key(key);

        assert_eq!(app.mode, AppMode::Command);
        assert!(app.command_buffer.is_empty());
    }

    #[test]
    fn slash_enters_search_mode() {
        let dir = TempTestDir::new("mdt-test-normal-slash");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Search);
        assert!(app.search.active);
        assert!(app.search.query.is_empty());
    }

    #[test]
    fn q_key_quits() {
        let dir = TempTestDir::new("mdt-test-normal-q");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        assert!(app.should_quit);
    }

    #[test]
    fn question_mark_toggles_help() {
        let dir = TempTestDir::new("mdt-test-normal-help");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(matches!(app.overlay, Overlay::None));

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(matches!(app.overlay, Overlay::Help));

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn tab_toggles_focus() {
        let dir = TempTestDir::new("mdt-test-normal-tab");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.show_file_tree = true;
        assert_eq!(app.focus, Focus::FileList);

        app.handle_normal_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn j_key_in_preview_scrolls_down() {
        let dir = TempTestDir::new("mdt-test-normal-j-preview");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 10;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.document.scroll_offset, 1);
    }

    #[test]
    fn k_key_in_preview_scrolls_up() {
        let dir = TempTestDir::new("mdt-test-normal-k-preview");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 10;
        app.document.scroll_offset = 5;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.document.scroll_offset, 4);
    }

    #[test]
    fn g_key_sets_pending() {
        let dir = TempTestDir::new("mdt-test-normal-g-pending");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

        assert!(app.pending_key.is_some());
        assert_eq!(app.pending_key.unwrap().0, 'g');
    }

    #[test]
    fn gg_scrolls_to_top() {
        let dir = TempTestDir::new("mdt-test-normal-gg");
        let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 10;
        app.document.scroll_offset = 15;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

        assert_eq!(app.document.scroll_offset, 0);
    }

    #[test]
    fn big_g_scrolls_to_bottom() {
        let dir = TempTestDir::new("mdt-test-normal-big-g");
        let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 10;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE));

        assert_eq!(app.document.scroll_offset, app.document.max_scroll());
    }

    #[test]
    fn ctrl_d_half_page_down() {
        let dir = TempTestDir::new("mdt-test-normal-ctrlD");
        let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 20;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));

        assert_eq!(app.document.scroll_offset, 10);
    }

    #[test]
    fn ctrl_u_half_page_up() {
        let dir = TempTestDir::new("mdt-test-normal-ctrlU");
        let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
        dir.create_file("test.md", &content);

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.document.viewport_height = 20;
        app.document.scroll_offset = 15;
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));

        assert_eq!(app.document.scroll_offset, 5);
    }

    #[test]
    fn toggle_file_tree_changes_visibility() {
        let dir = TempTestDir::new("mdt-test-normal-toggle-tree");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        assert!(!app.show_file_tree);

        app.toggle_file_tree();
        assert!(app.show_file_tree);
        assert_eq!(app.focus, Focus::FileList);

        app.toggle_file_tree();
        assert!(!app.show_file_tree);
        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn toggle_focus_stays_preview_when_tree_hidden() {
        let dir = TempTestDir::new("mdt-test-normal-toggle-focus-no-tree");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.show_file_tree = false;
        app.focus = Focus::Preview;

        app.toggle_focus();
        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn a_key_in_file_list_starts_create_file() {
        let dir = TempTestDir::new("mdt-test-normal-a-create");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.focus = Focus::FileList;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        assert!(matches!(app.overlay, Overlay::FileOp(crate::app::FileOp::CreateFile { .. })));
    }

    #[test]
    fn a_key_in_preview_does_nothing() {
        let dir = TempTestDir::new("mdt-test-normal-a-preview");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn o_key_opens_link_picker_with_links() {
        let dir = TempTestDir::new("mdt-test-normal-o-links");
        dir.create_file("test.md", "[link](https://example.com)");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        assert!(matches!(app.overlay, Overlay::LinkPicker));
    }

    #[test]
    fn o_key_no_links_shows_message() {
        let dir = TempTestDir::new("mdt-test-normal-o-nolinks");
        dir.create_file("test.md", "# No links here");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("test.md"));
        app.focus = Focus::Preview;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.status_message.contains("No links"));
    }

    #[test]
    fn esc_clears_search() {
        let dir = TempTestDir::new("mdt-test-normal-esc");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.search.active = true;
        app.search.query = "test".to_string();

        app.handle_normal_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(!app.search.active);
        assert!(app.search.query.is_empty());
    }

    #[test]
    fn ff_opens_file_finder() {
        let dir = TempTestDir::new("mdt-test-normal-ff");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        app.handle_normal_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

        assert!(matches!(app.overlay, Overlay::FileFinder));
    }
}
