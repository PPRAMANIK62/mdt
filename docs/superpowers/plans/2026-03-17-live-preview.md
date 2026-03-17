# Live Preview Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional, debounced, split-pane live markdown preview alongside the editor with toggleable orientation.

**Architecture:** New `SplitOrientation` enum and live-preview fields on `App`. The `ui::draw()` layout splits the editor area 50/50 when preview is active. A 150ms debounce in the event loop triggers re-rendering of editor buffer content through the existing two-phase markdown pipeline. Keybindings (`Space+p`, `Space+s`) and command (`:preview`) control the feature.

**Tech Stack:** Rust, ratatui, crossterm, pulldown-cmark, syntect, ratatui-textarea

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/app/types.rs` | Modify | Add `SplitOrientation` enum |
| `src/app/state.rs` | Modify | Add `LivePreviewState` struct |
| `src/app/mod.rs` | Modify | Add `live_preview: LivePreviewState` field to `App`, add `update_live_preview()` and `toggle_live_preview()` methods |
| `src/input/normal.rs` | Modify | Add `Space+p` and `Space+s` leader-key bindings |
| `src/input/editor.rs` | Modify | Set debounce timer on insert-mode keystrokes, add `Space+p`/`Space+s` to editor normal mode |
| `src/input/command.rs` | Modify | Add `:preview` command |
| `src/ui/mod.rs` | Modify | Split editor area when live preview is active |
| `src/ui/preview.rs` | Modify | Add `draw_live_preview()` function |
| `src/ui/status_bar.rs` | Modify | Add `[Preview H]`/`[Preview V]` indicator |
| `src/ui/help.rs` | Modify | Add live preview keybindings to help |
| `src/main.rs` | Modify | Add debounce check in event loop |

---

### Task 1: Add `SplitOrientation` Enum and `LivePreviewState`

**Files:**
- Modify: `src/app/types.rs`
- Modify: `src/app/state.rs`

- [ ] **Step 1: Add `SplitOrientation` enum to `types.rs`**

Add after the existing `Focus` enum:

```rust
/// Split orientation for live preview alongside editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// Editor left, preview right.
    Horizontal,
    /// Editor top, preview bottom.
    Vertical,
}

impl Default for SplitOrientation {
    fn default() -> Self {
        Self::Horizontal
    }
}
```

Also add the pub export in `app/mod.rs`:

```rust
pub(crate) use types::{FileOp, Overlay, SplitOrientation};
```

- [ ] **Step 2: Add `LivePreviewState` struct to `state.rs`**

Add after `CursorState`:

```rust
use ratatui::text::Line;
use crate::app::types::SplitOrientation;
use crate::markdown::RenderedBlock;

/// Live preview state for split-pane editing.
pub(crate) struct LivePreviewState {
    pub(crate) enabled: bool,
    pub(crate) orientation: SplitOrientation,
    pub(crate) debounce: Option<Instant>,
    pub(crate) rendered_lines: Vec<Line<'static>>,
    pub(crate) rendered_blocks: Vec<RenderedBlock>,
    pub(crate) scroll_offset: usize,
    pub(crate) viewport_width: usize,
}

impl Default for LivePreviewState {
    fn default() -> Self {
        Self {
            enabled: false,
            orientation: SplitOrientation::default(),
            debounce: None,
            rendered_lines: Vec::new(),
            rendered_blocks: Vec::new(),
            scroll_offset: 0,
            viewport_width: 0,
        }
    }
}
```

Export it from `app/mod.rs`:

```rust
pub(crate) use state::{CursorState, EditorState, FileFinderState, LinkPickerState, LivePreviewState, SearchState};
```

- [ ] **Step 3: Add `live_preview` field to `App` struct in `app/mod.rs`**

Add the field to the struct:

```rust
pub(crate) live_preview: LivePreviewState,
```

Initialize in `App::new()`:

```rust
live_preview: LivePreviewState::default(),
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors (unused warnings are OK at this stage)

- [ ] **Step 5: Commit**

```bash
git add src/app/types.rs src/app/state.rs src/app/mod.rs
git commit -m "feat: add SplitOrientation enum and LivePreviewState struct"
```

---

### Task 2: Add Toggle and Update Methods on `App`

**Files:**
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Write tests for toggle and update methods**

Add to `src/app/tests.rs` (or a new test block in `mod.rs`):

```rust
#[cfg(test)]
mod live_preview_tests {
    use super::*;
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test live_preview_tests`
Expected: FAIL — methods don't exist yet

- [ ] **Step 3: Implement the methods**

Add to `App` impl in `app/mod.rs`:

```rust
/// Toggle live preview on/off.
pub(crate) fn toggle_live_preview(&mut self) {
    self.live_preview.enabled = !self.live_preview.enabled;
    if self.live_preview.enabled {
        self.update_live_preview();
    }
}

/// Swap split orientation between horizontal and vertical.
pub(crate) fn toggle_split_orientation(&mut self) {
    self.live_preview.orientation = match self.live_preview.orientation {
        SplitOrientation::Horizontal => SplitOrientation::Vertical,
        SplitOrientation::Vertical => SplitOrientation::Horizontal,
    };
}

/// Re-render live preview from editor buffer content.
pub(crate) fn update_live_preview(&mut self) {
    let Some(ref textarea) = self.editor.textarea else {
        return;
    };
    let content = textarea.lines().join("\n");
    let (blocks, _links) = render_markdown_blocks(&content);
    let width = if self.live_preview.viewport_width > 0 {
        Some(self.live_preview.viewport_width)
    } else if self.document.viewport_width > 0 {
        Some(self.document.viewport_width)
    } else {
        None
    };
    let (rendered, _block_line_starts) = rewrap_blocks(&blocks, width);
    self.live_preview.rendered_lines = rendered;
    self.live_preview.rendered_blocks = blocks;
    self.live_preview.debounce = None;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test live_preview_tests`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: add toggle and update methods for live preview"
```

---

### Task 3: Add Keybindings (`Space+p`, `Space+s`)

**Files:**
- Modify: `src/input/normal.rs`
- Modify: `src/input/editor.rs`

- [ ] **Step 1: Write tests for keybindings**

Add to the existing `tests` module in `src/input/normal.rs`:

```rust
#[test]
fn space_p_toggles_live_preview() {
    let dir = TempTestDir::new("mdt-test-normal-space-p");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(!app.live_preview.enabled);

    app.handle_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_normal_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(app.live_preview.enabled);
}

#[test]
fn space_s_toggles_split_orientation() {
    use crate::app::SplitOrientation;
    let dir = TempTestDir::new("mdt-test-normal-space-s");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert_eq!(app.live_preview.orientation, SplitOrientation::Horizontal);

    app.handle_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_normal_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert_eq!(app.live_preview.orientation, SplitOrientation::Vertical);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test space_p_toggles -- && cargo test space_s_toggles`
Expected: FAIL

- [ ] **Step 3: Add `Space+p` and `Space+s` to the leader-key match in `normal.rs`**

In `handle_normal_key()`, add to the composed-commands match block (after the `(' ', KeyCode::Char('e'))` arm):

```rust
(' ', KeyCode::Char('p')) => {
    self.toggle_live_preview();
    return;
}
(' ', KeyCode::Char('s')) => {
    self.toggle_split_orientation();
    return;
}
```

- [ ] **Step 4: Add `Space+p` and `Space+s` to editor normal mode in `editor.rs`**

In `handle_editor_normal_key()`, add before the catch-all `_` arm. Need to add pending_key support to editor normal mode. Replace the existing `handle_editor_normal_key` method:

Add pending key check at the top of `handle_editor_normal_key()`:

```rust
// Check for composed commands (Space+p, Space+s).
if let Some((pending_char, instant)) = self.pending_key.take() {
    if instant.elapsed().as_millis() < 500 {
        match (pending_char, key.code) {
            (' ', KeyCode::Char('p')) => {
                self.toggle_live_preview();
                return;
            }
            (' ', KeyCode::Char('s')) => {
                self.toggle_split_orientation();
                return;
            }
            _ => {} // fall through
        }
    }
}
```

And add a `KeyCode::Char(' ')` arm to the match:

```rust
KeyCode::Char(' ') => {
    self.pending_key = Some((' ', std::time::Instant::now()));
}
```

- [ ] **Step 5: Write tests for editor normal mode keybindings**

Add to the existing `tests` module in `src/input/editor.rs`:

```rust
#[test]
fn space_p_toggles_live_preview_in_editor_normal() {
    let dir = TempTestDir::new("mdt-test-editor-space-p");
    dir.create_file("test.md", "# Test");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.mode = AppMode::Normal;
    assert!(!app.live_preview.enabled);

    app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(app.live_preview.enabled);
}

#[test]
fn space_s_toggles_orientation_in_editor_normal() {
    use crate::app::SplitOrientation;
    let dir = TempTestDir::new("mdt-test-editor-space-s");
    dir.create_file("test.md", "# Test");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.mode = AppMode::Normal;
    assert_eq!(app.live_preview.orientation, SplitOrientation::Horizontal);

    app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_editor_normal_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert_eq!(app.live_preview.orientation, SplitOrientation::Vertical);
}
```

- [ ] **Step 6: Run all keybinding tests to verify they pass**

Run: `cargo test space_p_toggles -- && cargo test space_s_toggles`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/input/normal.rs src/input/editor.rs
git commit -m "feat: add Space+p and Space+s keybindings for live preview"
```

---

### Task 4: Add `:preview` Command

**Files:**
- Modify: `src/input/command.rs`

- [ ] **Step 1: Write test**

Add to the existing `tests` module in `command.rs`:

```rust
#[test]
fn preview_command_toggles_live_preview() {
    let dir = TempTestDir::new("mdt-test-cmd-preview");
    dir.create_file("test.md", "# Test");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    assert!(!app.live_preview.enabled);

    app.execute_command("preview");
    assert!(app.live_preview.enabled);

    app.execute_command("preview");
    assert!(!app.live_preview.enabled);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test preview_command_toggles`
Expected: FAIL — "Unknown command: :preview"

- [ ] **Step 3: Add `:preview` arm to `execute_command()`**

Add before the `other =>` catch-all arm in `execute_command()`:

```rust
"preview" => {
    self.toggle_live_preview();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test preview_command_toggles`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/input/command.rs
git commit -m "feat: add :preview command to toggle live preview"
```

---

### Task 5: Add Debounce Trigger in Insert Mode

**Files:**
- Modify: `src/input/editor.rs`

- [ ] **Step 1: Write test**

Add to the existing `tests` module in `editor.rs`:

```rust
#[test]
fn insert_keystroke_sets_debounce_when_preview_enabled() {
    let dir = TempTestDir::new("mdt-test-editor-debounce");
    dir.create_file("test.md", "# Test");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.live_preview.enabled = true;

    assert!(app.live_preview.debounce.is_none());

    app.handle_insert_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert!(app.live_preview.debounce.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test insert_keystroke_sets_debounce`
Expected: FAIL

- [ ] **Step 3: Add debounce trigger to `handle_insert_key()`**

In `handle_insert_key()`, after the `if modified { self.editor.is_dirty = true; }` block, add:

```rust
if modified && self.live_preview.enabled {
    self.live_preview.debounce = Some(std::time::Instant::now());
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test insert_keystroke_sets_debounce`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/input/editor.rs
git commit -m "feat: set debounce timer on insert-mode keystrokes for live preview"
```

---

### Task 6: Add Debounce Check in Event Loop

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add debounce check to `run_loop()`**

In `run_loop()`, after the filesystem watcher drain block and before the cursor tick, add:

```rust
// Check live preview debounce timer.
if let Some(debounce_time) = app.live_preview.debounce {
    if debounce_time.elapsed() >= Duration::from_millis(150) {
        app.update_live_preview();
        needs_redraw = true;
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add debounce check for live preview in event loop"
```

---

### Task 7: Add `draw_live_preview()` Function

**Files:**
- Modify: `src/ui/preview.rs`

- [ ] **Step 1: Write test**

Add to existing `tests` module in `preview.rs`:

```rust
#[test]
fn draw_live_preview_renders_content() {
    let dir = TempTestDir::new("mdt-test-live-preview-draw");
    dir.create_file("test.md", "# Hello World");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.live_preview.enabled = true;
    app.update_live_preview();

    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            draw_live_preview(f, &mut app, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer();
    let text: String = (0..buf.area.height)
        .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .filter_map(|(x, y)| buf.cell(Position::new(x, y)))
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(text.contains("Hello World"));
}

#[test]
fn draw_live_preview_empty_renders_blank() {
    let dir = TempTestDir::new("mdt-test-live-preview-empty");
    dir.create_file("test.md", "");
    let file = dir.path().join("test.md");

    let mut app = App::new(dir.path(), Color::Reset).unwrap();
    app.open_file(&file);
    app.enter_editor();
    app.live_preview.enabled = true;
    // Don't call update_live_preview — rendered_lines is empty

    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            draw_live_preview(f, &mut app, area);
        })
        .unwrap();
    // Should not panic
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test draw_live_preview`
Expected: FAIL — function doesn't exist

- [ ] **Step 3: Implement `draw_live_preview()`**

Add to `src/ui/preview.rs`:

```rust
/// Draw the live preview pane alongside the editor.
///
/// Similar to `draw_preview()` but renders from `app.live_preview.rendered_lines`
/// (editor buffer content) instead of `app.document.rendered_lines` (on-disk content).
/// Scroll position tracks the editor cursor.
pub fn draw_live_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Preview ")
        .borders(ratatui::widgets::Borders::ALL)
        .padding(Padding::new(1, 1, 0, 0));

    let inner = block.inner(area);

    // Re-wrap if viewport width changed.
    let new_width = inner.width as usize;
    if new_width != app.live_preview.viewport_width && !app.live_preview.rendered_blocks.is_empty() {
        let (lines, _block_line_starts) =
            rewrap_blocks(&app.live_preview.rendered_blocks, Some(new_width));
        app.live_preview.rendered_lines = lines;
        app.live_preview.viewport_width = new_width;
    }

    if app.live_preview.rendered_lines.is_empty() {
        frame.render_widget(block, area);
        return;
    }

    // Scroll sync: map editor cursor line to approximate preview line.
    let viewport_height = inner.height as usize;
    if let Some(ref textarea) = app.editor.textarea {
        let cursor_row = textarea.cursor().0;
        let total_editor_lines = textarea.lines().len().max(1);
        let total_preview_lines = app.live_preview.rendered_lines.len();
        // Proportional mapping: cursor position in editor → position in preview
        let target_line = (cursor_row as f64 / total_editor_lines as f64
            * total_preview_lines as f64) as usize;
        // Center the target line in the viewport
        app.live_preview.scroll_offset = target_line.saturating_sub(viewport_height / 2);
    }

    let max_scroll = app.live_preview.rendered_lines.len().saturating_sub(viewport_height);
    if app.live_preview.scroll_offset > max_scroll {
        app.live_preview.scroll_offset = max_scroll;
    }

    let end = (app.live_preview.scroll_offset + viewport_height)
        .min(app.live_preview.rendered_lines.len());
    let visible_slice = &app.live_preview.rendered_lines[app.live_preview.scroll_offset..end];

    let lines: Vec<Line<'_>> = visible_slice
        .iter()
        .map(|line| {
            let spans: Vec<Span<'_>> =
                line.spans.iter().map(|s| Span::styled(s.content.as_ref(), s.style)).collect();
            Line { spans, style: line.style, alignment: line.alignment }
        })
        .collect();
    let text = Text::from(lines);

    let paragraph = Paragraph::new(text).block(block).scroll((0, 0));
    frame.render_widget(paragraph, area);

    // Scrollbar
    let total_lines = app.live_preview.rendered_lines.len();
    if total_lines > viewport_height {
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll).position(app.live_preview.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .thumb_symbol("┃")
            .thumb_style(Style::default().fg(Color::DarkGray))
            .track_symbol(Some(" "))
            .track_style(Style::default());
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
```

Also add the `Borders` import at the top of the file if not present (it's used by `Block::default().borders(...)`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test draw_live_preview`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat: add draw_live_preview() for split-pane rendering"
```

---

### Task 8: Update Layout in `ui::draw()`

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write test**

Add a test to verify the layout splits correctly. Add to a new `tests` module in `ui/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::test_util::TempTestDir;
    use ratatui::style::Color;

    #[test]
    fn draw_with_live_preview_does_not_panic() {
        let dir = TempTestDir::new("mdt-test-ui-live-preview");
        dir.create_file("test.md", "# Hello\n\nSome content here.");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();
        app.live_preview.enabled = true;
        app.update_live_preview();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        // Should not panic and should render both editor and preview
    }
}
```

- [ ] **Step 2: Run test to verify it fails (or passes with no split)**

Run: `cargo test draw_with_live_preview`
Expected: May pass but without the split — we'll verify visually after implementation.

- [ ] **Step 3: Update `draw()` to split editor area when live preview is active**

Replace the editor rendering logic in `draw()`. The key change: when `app.live_preview.enabled && app.editor.textarea.is_some()`, split the content area instead of giving it entirely to the editor.

Add a helper function at the top of `ui/mod.rs`:

```rust
use crate::app::SplitOrientation;

/// Minimum editor area width (columns) for horizontal preview split.
const MIN_HORIZONTAL_WIDTH: u16 = 40;
/// Minimum editor area height (rows) for vertical preview split.
const MIN_VERTICAL_HEIGHT: u16 = 10;

/// Render editor + optional live preview in the given content area.
///
/// Uses `app.editor.textarea.is_some()` checks and scoped borrows to avoid
/// holding an immutable borrow of `app.editor.textarea` while passing `&mut app`
/// to `draw_live_preview`.
fn draw_editor_area(frame: &mut Frame, app: &mut App, content_area: Rect) {
    let has_editor = app.editor.textarea.is_some();

    if has_editor {
        let wants_split = app.live_preview.enabled;
        let can_split = if wants_split {
            match app.live_preview.orientation {
                SplitOrientation::Horizontal => content_area.width >= MIN_HORIZONTAL_WIDTH,
                SplitOrientation::Vertical => content_area.height >= MIN_VERTICAL_HEIGHT,
            }
        } else {
            false
        };

        if wants_split && can_split {
            let chunks = match app.live_preview.orientation {
                SplitOrientation::Horizontal => {
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(content_area)
                }
                SplitOrientation::Vertical => {
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(content_area)
                }
            };
            // Scoped borrow: draw editor first, borrow drops before draw_live_preview.
            {
                let textarea = app.editor.textarea.as_ref().unwrap();
                editor::draw_editor(frame, textarea, chunks[0]);
            }
            preview::draw_live_preview(frame, app, chunks[1]);
            app.preview_area = None;
        } else {
            // Fallback: editor only (preview disabled, or terminal too small)
            let textarea = app.editor.textarea.as_ref().unwrap();
            editor::draw_editor(frame, textarea, content_area);
            app.preview_area = None;
        }
    } else {
        app.preview_area = Some(content_area);
        preview::draw_preview(frame, app, content_area);
    }
}
```

Then simplify the `draw()` function body to use `draw_editor_area()`:

```rust
pub fn draw(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Block::default().style(Style::default().bg(app.bg_color)), frame.area());

    let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    let main_area = outer[0];
    let status_area = outer[1];

    if app.show_file_tree {
        let main_chunks =
            Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(main_area);
        file_list::draw_file_list(frame, app, main_chunks[0]);
        app.file_list_area = Some(main_chunks[0]);
        draw_editor_area(frame, app, main_chunks[1]);
    } else {
        app.file_list_area = None;
        draw_editor_area(frame, app, main_area);
    }

    status_bar::draw_status_bar(frame, app, status_area);

    // --- Overlays (rendered last so they're on top) ---
    match app.overlay {
        Overlay::Help => {
            help::draw_help_overlay(frame, frame.area(), app.bg_color);
        }
        Overlay::LinkPicker => {
            let filtered_indices: Vec<usize> = app.filtered_link_indices().to_vec();
            let filtered_links: Vec<&LinkInfo> =
                filtered_indices.iter().filter_map(|&i| app.document.links.get(i)).collect();
            link_picker::draw_links_overlay(
                frame,
                frame.area(),
                &filtered_links,
                app.link_picker.selected,
                &app.link_picker.search_query,
                app.bg_color,
                app.cursor.visible,
            );
        }
        Overlay::FileFinder => {
            file_finder::draw_file_finder_overlay(frame, frame.area(), app);
        }
        Overlay::FileOp(_) => {
            file_op::draw_file_op_overlay(frame, frame.area(), app);
        }
        Overlay::None => {}
    }
}
```

Note: `draw_editor_area` uses scoped borrows to avoid borrow conflicts. The `app.editor.textarea` immutable borrow is confined to a block scope that ends before `draw_live_preview(frame, app, ...)` takes `&mut App`. This ensures the borrow checker is satisfied.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test draw_with_live_preview && cargo test`
Expected: PASS (all existing tests should still pass)

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat: split editor area for live preview in draw()"
```

---

### Task 9: Update Status Bar

**Files:**
- Modify: `src/ui/status_bar.rs`

- [ ] **Step 1: Add `[Preview H]`/`[Preview V]` indicator**

In `draw_status_bar()`, after the `dirty_indicator` calculation, add:

```rust
use crate::app::SplitOrientation;

let preview_indicator = if app.live_preview.enabled && app.editor.textarea.is_some() {
    match app.live_preview.orientation {
        SplitOrientation::Horizontal => "[Preview H]",
        SplitOrientation::Vertical => "[Preview V]",
    }
} else {
    ""
};
```

Then append `preview_indicator` to the `center` string. Change the format strings that include `dirty_indicator` to also include `preview_indicator`:

```rust
let preview_sep = if preview_indicator.is_empty() { "" } else { " " };

let center = if !app.status_message.is_empty() {
    if file_info.is_empty() {
        app.status_message.clone()
    } else {
        format!("{}{}{}{} {}", file_info, dirty_indicator, preview_sep, preview_indicator, app.status_message)
    }
} else if !file_info.is_empty() {
    format!("{}{}{}{}", file_info, dirty_indicator, preview_sep, preview_indicator)
} else {
    String::new()
};
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui/status_bar.rs
git commit -m "feat: show preview indicator in status bar"
```

---

### Task 10: Update Help Overlay

**Files:**
- Modify: `src/ui/help.rs`

- [ ] **Step 1: Add live preview keybindings to `HELP_KEYS`**

Add before the `("?", "This help")` entry:

```rust
("Spc+p", "Toggle live preview"),
("Spc+s", "Swap preview split"),
```

- [ ] **Step 2: Update the `centered_rect` height**

The help overlay uses `centered_rect(50, 27, area)`. Adding 2 new entries means increasing the height. Change `27` to `29`:

```rust
let popup_area = modal::centered_rect(50, 29, area);
```

- [ ] **Step 3: Verify it compiles and help test passes**

Run: `cargo check && cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/help.rs
git commit -m "feat: add live preview keybindings to help overlay"
```

---

### Task 11: Initial Live Preview Render on Enter Editor

**Files:**
- Modify: `src/input/editor.rs`

- [ ] **Step 1: Add initial preview render when entering editor**

At the end of `enter_editor()`, after `self.status_message = "-- INSERT --".to_string();`, add:

```rust
if self.live_preview.enabled {
    self.update_live_preview();
}
```

- [ ] **Step 2: Clear live preview cached data on editor exit**

In `exit_editor()`, after `self.mode = AppMode::Normal;`, add:

```rust
// Free cached live preview data to avoid holding large allocations
// when not editing. The `enabled` flag persists for next edit session.
self.live_preview.rendered_lines.clear();
self.live_preview.rendered_blocks.clear();
self.live_preview.scroll_offset = 0;
self.live_preview.debounce = None;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: compiles

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src/input/editor.rs
git commit -m "feat: render live preview on entering editor"
```

---

### Task 12: Full Integration Test

**Files:**
- Test only — no new files needed

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Manual verification checklist**

Run `cargo run` and verify:
1. Open a markdown file, press `i` to edit, then `Esc` → `Space+p` → `i` — preview split appears
2. Type in editor — preview updates after ~150ms delay
3. Press `Esc` → `Space+s` → `i` — orientation swaps to vertical
4. Press `Esc` → `Space+e` — file tree appears alongside editor + preview (3 panes)
5. Press `Esc` → `Space+p` — preview disappears, editor goes full-width
6. Type `:preview` in command mode — preview toggles on
7. Press `?` — help overlay shows new keybindings
8. Status bar shows `[Preview H]` or `[Preview V]` when active
9. Resize terminal — preview adapts, suppresses when too narrow

- [ ] **Step 4: Commit any fixups**

```bash
git add -A
git commit -m "fix: integration test fixups for live preview"
```
