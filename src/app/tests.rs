use crossterm::event::{KeyCode, KeyEvent};

use super::*;
use crate::test_util::TempTestDir;
use ratatui::style::Color;

// ── App::new ─────────────────────────────────────────────────────

#[test]
fn app_new_with_temp_dir() {
    let dir = TempTestDir::new("mdt-test-app-new");
    dir.create_file("test.md", "# Test");

    let app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(!app.tree.tree_items.is_empty());
    assert!(!app.tree.path_map.is_empty());
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.focus, Focus::FileList);
    assert!(!app.should_quit);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn app_new_empty_dir() {
    let dir = TempTestDir::new("mdt-test-empty-dir");

    let app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(app.tree.tree_items.is_empty());
    assert!(app.tree.path_map.is_empty());
}

// ── Search state ─────────────────────────────────────────────────

#[test]
fn clear_search_resets_all_fields() {
    let dir = TempTestDir::new("mdt-test-clear-search");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.search.active = true;
    app.search.query = "test".to_string();
    app.search.matches = vec![1, 5, 10];
    app.search.current = 2;
    app.status_message = "something".to_string();

    app.clear_search();

    assert!(!app.search.active);
    assert!(app.search.query.is_empty());
    assert!(app.search.matches.is_empty());
    assert_eq!(app.search.current, 0);
    assert!(app.tree.filtered_tree_items.is_none());
    assert!(app.tree.filtered_path_map.is_none());
    assert!(app.status_message.is_empty());
}

// ── State machine transitions ──────────────────────────────────

/// Helper: create a key press `KeyEvent` for use with `handle_event`.
fn key_event(code: KeyCode) -> KeyEvent {
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn transition_normal_to_command_and_back() {
    let dir = TempTestDir::new("mdt-test-cmd-transition");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert_eq!(app.mode, AppMode::Normal);

    // ':' enters Command mode.
    app.handle_event(key_event(KeyCode::Char(':')));
    assert_eq!(app.mode, AppMode::Command);

    // Esc returns to Normal.
    app.handle_event(key_event(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn transition_normal_to_search_and_back() {
    let dir = TempTestDir::new("mdt-test-search-transition");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert_eq!(app.mode, AppMode::Normal);

    // '/' enters Search mode and activates search.
    app.handle_event(key_event(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Search);
    assert!(app.search.active);

    // Esc returns to Normal and deactivates search.
    app.handle_event(key_event(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Normal);
    assert!(!app.search.active);
}

#[test]
fn help_toggle_on_and_off() {
    let dir = TempTestDir::new("mdt-test-help-toggle");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(matches!(app.overlay, Overlay::None));

    // '?' toggles help on.
    app.handle_event(key_event(KeyCode::Char('?')));
    assert!(matches!(app.overlay, Overlay::Help));

    // '?' again toggles help off (while help is showing, '?' dismisses it).
    app.handle_event(key_event(KeyCode::Char('?')));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn focus_toggle_cycles_between_panels() {
    let dir = TempTestDir::new("mdt-test-focus-toggle");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.show_file_tree = true;
    assert_eq!(app.focus, Focus::FileList);

    // Tab switches to Preview.
    app.handle_event(key_event(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Preview);

    // Tab switches back to FileList when file tree is visible.
    app.handle_event(key_event(KeyCode::Tab));
    assert_eq!(app.focus, Focus::FileList);

    // Tab stays on Preview when file tree is collapsed.
    app.handle_event(key_event(KeyCode::Tab)); // go to Preview
    app.show_file_tree = false;
    app.handle_event(key_event(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Preview);
}

#[test]
fn ctrl_c_quits_from_any_mode() {
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    let dir = TempTestDir::new("mdt-test-ctrl-c-quit");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();

    // Enter Command mode first.
    app.handle_event(key_event(KeyCode::Char(':')));
    assert_eq!(app.mode, AppMode::Command);

    // Ctrl+C quits even from Command mode.
    let ctrl_c = KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    app.handle_event(ctrl_c);
    assert!(app.should_quit);
}

// ── Scroll (DocumentState) ──────────────────────────────────

#[test]
fn scroll_down_increments_offset() {
    let dir = TempTestDir::new("mdt-test-scroll-down");
    let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 10;
    assert_eq!(app.document.scroll_offset, 0);

    app.document.scroll_down();

    assert_eq!(app.document.scroll_offset, 1);
}

#[test]
fn scroll_half_page_down_moves_half_viewport() {
    let dir = TempTestDir::new("mdt-test-scroll-half-down");
    let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 20;

    app.document.scroll_half_page_down();

    assert_eq!(app.document.scroll_offset, 10);
}

#[test]
fn scroll_to_top_resets_to_zero() {
    let dir = TempTestDir::new("mdt-test-scroll-top");
    let content = (0..30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 10;
    app.document.scroll_offset = 15;

    app.document.scroll_to_top();

    assert_eq!(app.document.scroll_offset, 0);
}

#[test]
fn scroll_to_bottom_sets_max_scroll() {
    let dir = TempTestDir::new("mdt-test-scroll-bottom");
    let content = (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 10;

    app.document.scroll_to_bottom();

    let expected = app.document.rendered_lines.len().saturating_sub(10);
    assert_eq!(app.document.scroll_offset, expected);
    assert!(app.document.scroll_offset > 0);
}

// ── open_file ──────────────────────────────────────────────────

#[test]
fn open_file_rejects_large_files() {
    let dir = TempTestDir::new("mdt-test-open-file-large");
    // Create a file just over 5MB
    let big_path = dir.path().join("big.md");
    let data = vec![b'x'; 5_000_001];
    std::fs::write(&big_path, &data).unwrap();

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&big_path);

    assert!(app.status_message.contains("File too large"));
    assert!(app.document.current_file.is_none());
}

#[test]
fn open_file_succeeds_for_small_file() {
    let dir = TempTestDir::new("mdt-test-open-file-small");
    dir.create_file("hello.md", "# Hello");
    let md_path = dir.path().join("hello.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&md_path);

    assert!(app.status_message.is_empty());
    assert_eq!(app.document.current_file, Some(md_path));
}

// ── Live preview ──────────────────────────────────────────────────

#[test]
fn toggle_live_preview_flips_enabled() {
    let dir = TempTestDir::new("mdt-test-lp-toggle");
    dir.create_file("test.md", "# Test");
    let mut app = App::new(dir.path(), Color::Reset).unwrap();

    assert!(!app.live_preview.enabled);
    app.toggle_live_preview();
    assert!(app.live_preview.enabled);
    app.toggle_live_preview();
    assert!(!app.live_preview.enabled);
}

#[test]
fn toggle_split_orientation_swaps() {
    let dir = TempTestDir::new("mdt-test-lp-orientation");
    dir.create_file("test.md", "# Test");
    let mut app = App::new(dir.path(), Color::Reset).unwrap();

    assert_eq!(app.live_preview.orientation, SplitOrientation::Horizontal);
    app.toggle_split_orientation();
    assert_eq!(app.live_preview.orientation, SplitOrientation::Vertical);
    app.toggle_split_orientation();
    assert_eq!(app.live_preview.orientation, SplitOrientation::Horizontal);
}

#[test]
fn update_live_preview_renders_editor_content() {
    let dir = TempTestDir::new("mdt-test-lp-update");
    dir.create_file("test.md", "# Hello");
    let file = dir.path().join("test.md");
    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.live_preview.enabled = true;

    app.update_live_preview();

    assert!(!app.live_preview.rendered_lines.is_empty());
}

/// End-to-end test: full user flow through handle_event() for Space+p in editor.
#[test]
fn e2e_space_p_toggles_preview_in_editor_via_handle_event() {
    let dir = TempTestDir::new("mdt-test-e2e-space-p");
    dir.create_file("test.md", "# Hello\nWorld");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.focus = Focus::Preview;

    // Step 1: Press 'i' to enter editor (sets Insert mode).
    app.handle_event(key_event(KeyCode::Char('i')));
    assert_eq!(app.mode, AppMode::Insert);
    assert!(app.editor.textarea.is_some());

    // Step 2: Press Esc to go to Normal mode (still in editor).
    app.handle_event(key_event(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Normal);
    assert!(app.editor.textarea.is_some()); // still in editor

    // Step 3: Press Space then 'p' to toggle live preview.
    assert!(!app.live_preview.enabled);
    app.handle_event(key_event(KeyCode::Char(' ')));
    app.handle_event(key_event(KeyCode::Char('p')));
    assert!(app.live_preview.enabled);
    assert_eq!(app.status_message, "Live preview ON");

    // Step 4: Toggle off.
    app.handle_event(key_event(KeyCode::Char(' ')));
    app.handle_event(key_event(KeyCode::Char('p')));
    assert!(!app.live_preview.enabled);
    assert_eq!(app.status_message, "Live preview OFF");
}

/// Verify Space+p in Insert mode does NOT toggle preview (types into editor instead).
#[test]
fn space_p_in_insert_mode_does_not_toggle_preview() {
    let dir = TempTestDir::new("mdt-test-insert-space-p");
    dir.create_file("test.md", "# Test");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.focus = Focus::Preview;

    // Enter editor (Insert mode).
    app.handle_event(key_event(KeyCode::Char('i')));
    assert_eq!(app.mode, AppMode::Insert);

    // Space+p in Insert mode should NOT toggle preview.
    app.handle_event(key_event(KeyCode::Char(' ')));
    app.handle_event(key_event(KeyCode::Char('p')));
    assert!(!app.live_preview.enabled);
}
