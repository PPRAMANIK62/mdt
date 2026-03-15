//! End-to-end integration tests using `TestBackend`.
//!
//! These tests create an `App`, feed multi-key sequences via `handle_event`,
//! render to a `TestBackend`, and assert on both app state and terminal output.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::Color;
use ratatui::Terminal;

use crate::app::{App, AppMode, Focus};
use crate::test_util::TempTestDir;
use crate::ui;

/// Create a plain key press event.
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Create a key press with modifiers.
fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent { code, modifiers, kind: KeyEventKind::Press, state: KeyEventState::NONE }
}

/// Extract all text from a terminal buffer into a single string.
fn buffer_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .filter_map(|(x, y)| buf.cell(Position::new(x, y)))
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

/// Render the app and return the buffer text.
fn render(terminal: &mut Terminal<TestBackend>, app: &mut App) -> String {
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    buffer_text(terminal.backend().buffer())
}

// ── gg (go to top) sequence ─────────────────────────────────────────────

#[test]
fn gg_sequence_scrolls_preview_to_top() {
    let dir = TempTestDir::new("mdt-integ-gg-preview");
    let content = (0..100)
        .map(|i| format!("## Heading {i}\n\nParagraph {i}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 20;

    // Scroll down first
    app.document.scroll_offset = 50;
    app.focus = Focus::Preview;

    // Send gg
    app.handle_event(key(KeyCode::Char('g')));
    app.handle_event(key(KeyCode::Char('g')));

    assert_eq!(app.document.scroll_offset, 0);
}

#[test]
fn gg_sequence_selects_first_in_file_list() {
    let dir = TempTestDir::new("mdt-integ-gg-filelist");
    dir.create_file("a.md", "# A");
    dir.create_file("b.md", "# B");
    dir.create_file("c.md", "# C");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert_eq!(app.focus, Focus::FileList);

    // Render once so the tree widget initializes internal state
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    app.show_file_tree = true;
    render(&mut terminal, &mut app);

    let first_selected = app.tree.tree_state.selected().to_vec();

    // Navigate down
    app.handle_event(key(KeyCode::Char('j')));
    app.handle_event(key(KeyCode::Char('j')));
    let moved_selected = app.tree.tree_state.selected().to_vec();
    assert_ne!(first_selected, moved_selected, "j should move selection");

    // Now gg to go back to first
    app.handle_event(key(KeyCode::Char('g')));
    app.handle_event(key(KeyCode::Char('g')));

    // Should be back at first item
    let after_gg = app.tree.tree_state.selected().to_vec();
    assert_eq!(after_gg, first_selected);
}

// ── Space+e (toggle file tree) ─────────────────────────────────────────

#[test]
fn space_e_toggles_file_tree() {
    let dir = TempTestDir::new("mdt-integ-space-e");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(!app.show_file_tree);

    // Space+e toggles on
    app.handle_event(key(KeyCode::Char(' ')));
    app.handle_event(key(KeyCode::Char('e')));
    assert!(app.show_file_tree);

    // Space+e toggles off
    app.handle_event(key(KeyCode::Char(' ')));
    app.handle_event(key(KeyCode::Char('e')));
    assert!(!app.show_file_tree);
}

// ── Search flow: /, type, Enter, n, N ──────────────────────────────────

#[test]
fn search_flow_enter_query_and_navigate_matches() {
    let dir = TempTestDir::new("mdt-integ-search");
    let content = "# Title\n\nfoo bar\n\nsome foo text\n\nanother foo line";
    dir.create_file("test.md", content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("test.md"));
    app.document.viewport_height = 20;
    app.focus = Focus::Preview;

    // Enter search mode
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Search);

    // Type search query
    app.handle_event(key(KeyCode::Char('f')));
    app.handle_event(key(KeyCode::Char('o')));
    app.handle_event(key(KeyCode::Char('o')));
    assert_eq!(app.search.query, "foo");

    // Confirm search
    app.handle_event(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::Normal);

    // Navigate matches with n/N
    if !app.search.matches.is_empty() {
        let first = app.search.current;
        app.handle_event(key(KeyCode::Char('n')));
        let second = app.search.current;
        // n should advance (or wrap)
        if app.search.matches.len() > 1 {
            assert_ne!(first, second);
        }
        app.handle_event(key(KeyCode::Char('N')));
        assert_eq!(app.search.current, first);
    }
}

// ── Command mode: :q quits ─────────────────────────────────────────────

#[test]
fn command_mode_quit() {
    let dir = TempTestDir::new("mdt-integ-cmd-quit");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(!app.should_quit);

    // :q
    app.handle_event(key(KeyCode::Char(':')));
    assert_eq!(app.mode, AppMode::Command);
    app.handle_event(key(KeyCode::Char('q')));
    app.handle_event(key(KeyCode::Enter));

    assert!(app.should_quit);
}

// ── Help overlay renders correctly ─────────────────────────────────────

#[test]
fn help_overlay_renders_and_dismisses() {
    let dir = TempTestDir::new("mdt-integ-help");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // Open help
    app.handle_event(key(KeyCode::Char('?')));
    assert!(app.show_help);

    let text = render(&mut terminal, &mut app);
    assert!(text.contains("Help"));
    assert!(text.contains("Navigate"));

    // Esc dismisses
    app.handle_event(key(KeyCode::Esc));
    assert!(!app.show_help);
}

// ── Full render with file tree ─────────────────────────────────────────

#[test]
fn file_tree_renders_filenames() {
    let dir = TempTestDir::new("mdt-integ-tree-render");
    dir.create_file("alpha.md", "# Alpha");
    dir.create_file("beta.md", "# Beta");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.show_file_tree = true;
    app.focus = Focus::FileList;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let text = render(&mut terminal, &mut app);

    assert!(text.contains("alpha.md"));
    assert!(text.contains("beta.md"));
}

// ── Navigate file list and open file ───────────────────────────────────

#[test]
fn navigate_and_open_file_shows_content() {
    let dir = TempTestDir::new("mdt-integ-nav-open");
    dir.create_file("hello.md", "# Hello World");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.show_file_tree = true;
    assert_eq!(app.focus, Focus::FileList);

    // Select and open the file
    app.handle_event(key(KeyCode::Enter));

    // File should be open
    assert!(app.document.current_file.is_some());

    // Render and check content
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let text = render(&mut terminal, &mut app);
    assert!(text.contains("Hello World"));
}

// ── Ctrl+C quits from any state ────────────────────────────────────────

#[test]
fn ctrl_c_quits_from_search_mode() {
    let dir = TempTestDir::new("mdt-integ-ctrl-c");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();

    // Enter search mode
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Search);

    // Ctrl+C quits
    app.handle_event(key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(app.should_quit);
}

// ── Tab focus cycling with render ──────────────────────────────────────

#[test]
fn tab_cycling_updates_render() {
    let dir = TempTestDir::new("mdt-integ-tab-cycle");
    dir.create_file("test.md", "# Content");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.show_file_tree = true;
    app.open_file(&dir.path().join("test.md"));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    assert_eq!(app.focus, Focus::FileList);
    render(&mut terminal, &mut app);

    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Preview);
    render(&mut terminal, &mut app);

    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::FileList);
}

// ── File search filtering ──────────────────────────────────────────────

#[test]
fn file_search_filters_tree() {
    let dir = TempTestDir::new("mdt-integ-file-search");
    dir.create_file("readme.md", "# Readme");
    dir.create_file("notes.md", "# Notes");
    dir.create_file("todo.md", "# Todo");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert_eq!(app.focus, Focus::FileList);

    // Enter search mode (from file list, this does file filtering)
    app.handle_event(key(KeyCode::Char('/')));
    assert_eq!(app.mode, AppMode::Search);

    // Type "read"
    app.handle_event(key(KeyCode::Char('r')));
    app.handle_event(key(KeyCode::Char('e')));
    app.handle_event(key(KeyCode::Char('a')));
    app.handle_event(key(KeyCode::Char('d')));

    // Filtered tree should be present
    assert!(app.tree.filtered_tree_items.is_some());

    // Confirm and return to normal
    app.handle_event(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::Normal);
}

// ── G goes to bottom ───────────────────────────────────────────────────

#[test]
fn capital_g_scrolls_to_bottom() {
    let dir = TempTestDir::new("mdt-integ-G-bottom");
    let content = (0..50).map(|i| format!("## H{i}\n\nText {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 10;
    app.focus = Focus::Preview;

    app.handle_event(key(KeyCode::Char('G')));

    assert_eq!(app.document.scroll_offset, app.document.max_scroll());
    assert!(app.document.scroll_offset > 0);
}

// ── Welcome screen renders when no file is open ────────────────────────

#[test]
fn welcome_screen_shows_when_no_file_open() {
    let dir = TempTestDir::new("mdt-integ-welcome");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    // Don't open any file — welcome screen should show

    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let text = render(&mut terminal, &mut app);

    assert!(text.contains("Terminal Markdown Viewer"));
}

// ── Ctrl+d / Ctrl+u half-page scroll ──────────────────────────────────

#[test]
fn ctrl_d_u_half_page_scroll() {
    let dir = TempTestDir::new("mdt-integ-half-page");
    let content = (0..100).map(|i| format!("## H{i}\n\nText {i}")).collect::<Vec<_>>().join("\n\n");
    dir.create_file("long.md", &content);

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&dir.path().join("long.md"));
    app.document.viewport_height = 20;
    app.focus = Focus::Preview;

    // Ctrl+d scrolls down half page
    app.handle_event(key_mod(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(app.document.scroll_offset, 10);

    // Ctrl+u scrolls back up
    app.handle_event(key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(app.document.scroll_offset, 0);
}
