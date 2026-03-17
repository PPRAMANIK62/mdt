//! Small state structs: search, editor, link picker, cursor.

use std::time::Instant;

use ratatui::text::Line;
use ratatui_textarea::TextArea;

use crate::app::types::SplitOrientation;
use crate::markdown::RenderedBlock;

/// Search-related state (both file search and in-document search).
#[derive(Default)]
pub(crate) struct SearchState {
    pub(crate) active: bool,
    pub(crate) query: String,
    pub(crate) matches: Vec<usize>,
    pub(crate) current: usize,
}

/// Editor (TextArea) state.
#[derive(Default)]
pub(crate) struct EditorState {
    pub(crate) textarea: Option<TextArea<'static>>,
    pub(crate) is_dirty: bool,
    pub(crate) external_change_detected: bool,
}

/// Link picker overlay state.
#[derive(Default)]
pub(crate) struct LinkPickerState {
    pub(crate) selected: usize,
    pub(crate) search_query: String,
    pub(crate) cached_indices: Vec<usize>,
    pub(crate) cached_query: String,
    pub(crate) cached_count: usize,
}

/// File finder overlay state.
#[derive(Default)]
pub(crate) struct FileFinderState {
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) results: Vec<(String, std::path::PathBuf)>,
}

/// Cursor blink state for overlays with text input.
pub(crate) struct CursorState {
    pub(crate) visible: bool,
    pub(crate) last_toggle: Instant,
}

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
