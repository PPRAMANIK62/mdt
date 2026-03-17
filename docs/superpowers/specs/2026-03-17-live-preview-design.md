# Live Preview While Editing

**Date:** 2026-03-17
**Status:** Approved

## Overview

Add an optional split-pane live preview that shows rendered markdown alongside the editor, updating on each keystroke (debounced). The user can toggle the preview, choose between horizontal and vertical split orientations, and use it independently of the file tree.

## Requirements

- Side-by-side (horizontal) or stacked (vertical) editor + rendered preview split
- Toggle preview with keybinding (`Space+p`) and command (`:preview`)
- Default horizontal orientation; `Space+s` swaps between horizontal and vertical
- Debounced updates (150ms) to avoid performance issues
- File tree toggle (`Space+e`) works independently — user can have 2 or 3 panes

## State & Data Model

New types and fields:

```rust
// in app/types.rs
pub enum SplitOrientation {
    Horizontal,  // editor left, preview right (default)
    Vertical,    // editor top, preview bottom
}
```

New fields on `App`:

```rust
pub show_live_preview: bool,              // toggle for the preview split
pub split_orientation: SplitOrientation,  // default: Horizontal
pub preview_debounce: Option<Instant>,    // when last edit happened
pub preview_content: Vec<Line<'static>>,  // cached rendered lines for live preview
```

`show_live_preview` persists across editor open/close — reopening the editor brings preview back without re-toggling.

## Layout System

The existing layout divides the terminal into `[File Tree (25%)] [Preview/Editor (75%)] [Status Bar]`.

With live preview active during editing, the editor area (whatever remains after file tree + status bar) is subdivided 50/50:

**Horizontal split (default):**
```
[File Tree 25%] [Editor 50%] [Preview 50%]
[Status Bar]
```

**Vertical split:**
```
[File Tree 25%] [Editor (top 50%)  ]
                [Preview (bottom 50%)]
[Status Bar]
```

**Without file tree:**
```
[Editor 50%] [Preview 50%]       -- horizontal
[Status Bar]

[Editor (top 50%)  ]             -- vertical
[Preview (bottom 50%)]
[Status Bar]
```

The existing `draw_preview()` function is reused for the live preview pane, rendering from editor buffer content instead of the on-disk file.

## Debounce & Rendering Pipeline

**Debounce mechanism:**
- On each keystroke in insert mode, set `preview_debounce = Some(Instant::now())`
- In the main event loop, after processing events, check: if `preview_debounce` is `Some(t)` and `t.elapsed() >= 150ms`, re-render the preview content and clear the debounce timer

**Rendering:**
- Reuses the existing two-phase pipeline: `render_markdown_blocks()` on the editor buffer text, then `rewrap_blocks()` for the preview pane width, stored in `preview_content`
- Parses the editor's current `TextArea` content (`textarea.lines().join("\n")`) — not the on-disk file
- The existing `DocumentState` rendering is untouched — live preview uses its own `preview_content` cache
- On preview pane resize (terminal resize or orientation swap), only `rewrap_blocks()` runs (cheap phase 2)

**Scroll sync:**
- The live preview scroll position tracks the editor cursor — auto-scrolls to keep the edited region visible
- No manual scroll for the preview pane

## Keybindings & Commands

**Normal mode:**
| Key | Action |
|-----|--------|
| `Space+p` | Toggle live preview on/off (works in both editor and non-editor view; state persists) |
| `Space+s` | Swap split orientation (horizontal/vertical). Can be toggled anytime, but only has visible effect when editor is active and `show_live_preview` is true |

**Command mode:**
| Command | Action |
|---------|--------|
| `:preview` | Toggle live preview (same as `Space+p`) |

**Help overlay:**
- Add both keybindings under a "Live Preview" section

**Status bar:**
- When live preview is active during editing, show `[Preview]` indicator and `[H]`/`[V]` for orientation

## Edge Cases

**Small terminals:**
- If editor area is less than 40 columns wide (horizontal) or 10 rows tall (vertical), suppress the preview pane and show editor full-width. Preview reappears when terminal is resized larger.

**Large files:**
- The 150ms debounce prevents lag. The existing `render_markdown_blocks()` is already efficient. The 5MB file size limit already exists.

**Empty editor:**
- Show a blank preview pane.

**Closing the editor:**
- Layout returns to normal preview mode. `show_live_preview` persists in state for next edit session.

**File tree interaction:**
- `Space+e` works independently. If all three panes are visible and terminal is narrow, minimum-width check suppresses preview gracefully.

**Syntax highlighting:**
- The existing syntect pre-warming on startup covers live preview. Same syntax set is used.

## Implementation Notes

Key integration points in the existing codebase:

- **State**: New fields go on `App` struct (`app/mod.rs`). `SplitOrientation` enum goes in `app/types.rs`.
- **Keybindings**: `Space+p` and `Space+s` extend the existing leader-key pattern in `input/normal.rs` (the `(' ', KeyCode::Char('e'))` match arm). `Space+s` also needs handling in `handle_editor_normal_key()` in `input/editor.rs`.
- **Command**: `:preview` adds a new arm in `execute_command()` in `input/command.rs`.
- **Layout**: `ui/mod.rs` `draw()` function needs to subdivide the content area when `show_live_preview && editor.textarea.is_some()`.
- **Debounce trigger**: `handle_insert_key()` in `input/editor.rs` already sets `is_dirty` — set `preview_debounce` there too.
- **Debounce check**: `run_loop()` in `main.rs` — add a check after event processing, before the cursor tick.
- **Preview rendering**: New function (e.g., `draw_live_preview()`) in `ui/preview.rs` that renders `preview_content` with virtual scrolling, similar to existing `draw_preview()`.
- **Status bar**: `ui/status_bar.rs` — add `[Preview H]` / `[Preview V]` indicator.
- **Help**: `ui/help.rs` — add Live Preview section.
