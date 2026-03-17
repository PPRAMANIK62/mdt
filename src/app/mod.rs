//! Application state and logic.

mod document;
mod event;
mod file_finder;
mod link_picker;
mod state;
mod tree;
mod types;
mod watcher_handlers;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tui_tree_widget::TreeState;

use crate::file_tree;
use crate::markdown::{
    deduplicate_links, render_markdown_blocks, render_markdown_blocks_with_source_map,
    rewrap_blocks,
};

pub use types::{AppMode, Focus};
pub(crate) use types::{FileOp, Overlay, SplitOrientation};

pub(crate) use document::{DocumentState, TreeViewState};
pub(crate) use state::{
    CursorState, EditorState, FileFinderState, LinkPickerState, LivePreviewState, SearchState,
};

/// Top-level application state.
pub struct App {
    pub(crate) search: SearchState,
    pub(crate) editor: EditorState,
    pub(crate) tree: TreeViewState,
    pub(crate) document: DocumentState,
    pub(crate) link_picker: LinkPickerState,
    pub(crate) file_finder: FileFinderState,
    pub(crate) cursor: CursorState,
    pub(crate) mode: AppMode,
    pub(crate) focus: Focus,
    pub(crate) should_quit: bool,
    pub(crate) status_message: String,
    pub(crate) pending_key: Option<(char, Instant)>,
    pub(crate) command_buffer: String,
    pub(crate) overlay: Overlay,
    pub(crate) file_op_input: String,
    pub(crate) show_file_tree: bool,
    pub(crate) bg_color: ratatui::style::Color,
    pub(crate) root_path: PathBuf,
    pub(crate) max_file_size: u64,
    pub(crate) preview_area: Option<ratatui::layout::Rect>,
    pub(crate) file_list_area: Option<ratatui::layout::Rect>,
    pub(crate) live_preview: LivePreviewState,
}

impl App {
    /// Default maximum file size (5 MB).
    pub const DEFAULT_MAX_FILE_SIZE: u64 = 5_000_000;

    /// Create a new `App` rooted at `path`.
    pub fn new(path: &Path, bg_color: ratatui::style::Color) -> anyhow::Result<Self> {
        let (tree_items, path_map) = file_tree::build_tree_items(path)?;
        let root_path = std::fs::canonicalize(path)?;
        let mut tree_state = TreeState::default();
        if let Some(first_item) = tree_items.first() {
            tree_state.select(vec![first_item.identifier().clone()]);
        }

        // Initialize viewport dimensions from terminal size so the first file
        // open can wrap to the correct width instead of wrapping with None
        // (which forces an immediate re-wrap on the next draw frame).
        let (init_width, init_height) = crossterm::terminal::size().unwrap_or((80, 24));

        Ok(Self {
            tree: TreeViewState {
                tree_state,
                tree_items,
                path_map,
                filtered_tree_items: None,
                filtered_path_map: None,
            },
            document: DocumentState {
                current_file: None,
                file_content: String::new(),
                rendered_lines: Vec::new(),
                rendered_lines_lower: Vec::new(),
                rendered_blocks: Vec::new(),
                links: Vec::new(),
                heading_line_offsets: Vec::new(),
                block_line_starts: Vec::new(),
                scroll_offset: 0,
                viewport_height: init_height as usize,
                viewport_width: init_width as usize,
            },
            search: SearchState::default(),
            editor: EditorState::default(),
            link_picker: LinkPickerState::default(),
            file_finder: FileFinderState::default(),
            cursor: CursorState { visible: true, last_toggle: Instant::now() },
            mode: AppMode::Normal,
            focus: Focus::FileList,
            should_quit: false,
            status_message: String::new(),
            pending_key: None,
            command_buffer: String::new(),
            overlay: Overlay::None,
            file_op_input: String::new(),
            show_file_tree: false,
            bg_color,
            root_path,
            max_file_size: Self::DEFAULT_MAX_FILE_SIZE,
            preview_area: None,
            file_list_area: None,
            live_preview: LivePreviewState::default(),
        })
    }

    /// Toggle the cursor blink state every ~530ms.
    ///
    /// Advances by the fixed interval rather than resetting to `Instant::now()`
    /// to prevent drift from event-loop latency.
    pub fn tick_cursor(&mut self) {
        let interval = Duration::from_millis(530);
        if self.cursor.last_toggle.elapsed() >= interval {
            self.cursor.visible = !self.cursor.visible;
            self.cursor.last_toggle += interval;
        }
    }

    /// Get the display path for the current file (relative to root).
    pub(crate) fn display_file_path(&self) -> String {
        self.document
            .current_file
            .as_ref()
            .map(|p| p.strip_prefix(&self.root_path).unwrap_or(p).to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Toggle live preview on/off.
    pub(crate) fn toggle_live_preview(&mut self) {
        self.live_preview.enabled = !self.live_preview.enabled;
        if self.live_preview.enabled {
            self.update_live_preview();
            self.status_message = "Live preview ON".to_string();
        } else {
            self.status_message = "Live preview OFF".to_string();
        }
    }

    /// Swap split orientation between horizontal and vertical.
    pub(crate) fn toggle_split_orientation(&mut self) {
        self.live_preview.orientation = match self.live_preview.orientation {
            SplitOrientation::Horizontal => {
                self.status_message = "Preview split: vertical".to_string();
                SplitOrientation::Vertical
            }
            SplitOrientation::Vertical => {
                self.status_message = "Preview split: horizontal".to_string();
                SplitOrientation::Horizontal
            }
        };
    }

    /// Re-render live preview from editor buffer content.
    pub(crate) fn update_live_preview(&mut self) {
        let Some(ref textarea) = self.editor.textarea else {
            return;
        };
        let content = textarea.lines().join("\n");
        let (blocks, _links, source_lines) = render_markdown_blocks_with_source_map(&content);
        let width = if self.live_preview.viewport_width > 0 {
            Some(self.live_preview.viewport_width)
        } else if self.document.viewport_width > 0 {
            Some(self.document.viewport_width)
        } else {
            None
        };
        let (rendered, block_line_starts) = rewrap_blocks(&blocks, width);
        self.live_preview.rendered_lines = rendered;
        self.live_preview.rendered_blocks = blocks;
        self.live_preview.block_line_starts = block_line_starts;
        self.live_preview.block_source_lines = source_lines;
        self.live_preview.debounce = None;
    }

    /// Read a file, render its markdown, and store the result.
    pub(crate) fn open_file(&mut self, path: &Path) {
        let limit = self.max_file_size;
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > limit {
                let mb = limit / 1_000_000;
                self.status_message = format!("File too large (>{mb}MB)");
                return;
            }
        }

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status_message = format!("Error: {e}");
                return;
            }
        };

        let Ok(content) = String::from_utf8(bytes) else {
            self.status_message = "Binary file, cannot preview".to_string();
            return;
        };

        let (blocks, links) = render_markdown_blocks(&content);
        let links = deduplicate_links(links);
        let width = if self.document.viewport_width > 0 {
            Some(self.document.viewport_width)
        } else {
            None
        };
        let (rendered, block_line_starts) = rewrap_blocks(&blocks, width);
        self.document.rendered_lines = rendered;
        self.document.rebuild_lower_cache();
        self.document.block_line_starts = block_line_starts;
        self.document.rendered_blocks = blocks;
        self.document.links = links;
        self.document.file_content = content;
        self.document.current_file = Some(path.to_path_buf());
        self.document.scroll_offset = 0;
        self.document.rebuild_heading_index();
        self.status_message.clear();

        if !self.show_file_tree {
            self.focus = Focus::Preview;
        }
    }
}
