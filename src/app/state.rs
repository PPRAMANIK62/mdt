//! Small state structs: search, editor, link picker, cursor.

use std::time::Instant;

use ratatui_textarea::TextArea;

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
