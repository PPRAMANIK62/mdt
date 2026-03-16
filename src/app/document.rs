//! Document and tree-view state structs.

use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::text::Line;
use tui_tree_widget::{TreeItem, TreeState};

use crate::markdown::{LinkInfo, RenderedBlock};

/// File tree view state.
pub(crate) struct TreeViewState {
    pub(crate) tree_state: TreeState<String>,
    pub(crate) tree_items: Vec<TreeItem<'static, String>>,
    pub(crate) path_map: HashMap<String, (PathBuf, bool)>,
    pub(crate) filtered_tree_items: Option<Vec<TreeItem<'static, String>>>,
    pub(crate) filtered_path_map: Option<HashMap<String, (PathBuf, bool)>>,
}

/// Current document / preview state.
pub(crate) struct DocumentState {
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) file_content: String,
    pub(crate) rendered_lines: Vec<Line<'static>>,
    pub(crate) rendered_lines_lower: Vec<String>,
    pub(crate) scroll_offset: usize,
    pub(crate) viewport_height: usize,
    pub(crate) viewport_width: usize,
    pub(crate) rendered_blocks: Vec<RenderedBlock>,
    pub(crate) links: Vec<LinkInfo>,
    pub(crate) heading_line_offsets: Vec<usize>,
    pub(crate) block_line_starts: Vec<usize>,
}

impl DocumentState {
    /// Rebuild the lowercase text cache from `rendered_lines`.
    pub(crate) fn rebuild_lower_cache(&mut self) {
        self.rendered_lines_lower = self
            .rendered_lines
            .iter()
            .map(|line| {
                let mut text = String::new();
                for s in &line.spans {
                    text.push_str(s.content.as_ref());
                }
                text.to_lowercase()
            })
            .collect();
    }

    pub(crate) fn scroll_down(&mut self) {
        if !self.rendered_lines.is_empty() {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.clamp_scroll();
        }
    }

    pub(crate) fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub(crate) fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    pub(crate) fn scroll_half_page_down(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_add(half.max(1));
        self.clamp_scroll();
    }

    pub(crate) fn scroll_half_page_up(&mut self) {
        let half = self.viewport_height / 2;
        self.scroll_offset = self.scroll_offset.saturating_sub(half.max(1));
    }

    pub(crate) fn max_scroll(&self) -> usize {
        self.rendered_lines.len().saturating_sub(self.viewport_height)
    }

    pub(crate) fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// Scroll so that `line` is visible near the top of the viewport.
    pub(crate) fn scroll_to_line(&mut self, line: usize) {
        self.scroll_offset = line.saturating_sub(2);
        self.clamp_scroll();
    }

    /// Rebuild the heading line-offset index from block_line_starts and rendered_blocks.
    pub(crate) fn rebuild_heading_index(&mut self) {
        self.heading_line_offsets = self
            .block_line_starts
            .iter()
            .enumerate()
            .filter_map(|(i, &line_start)| {
                if let RenderedBlock::StyledLine { heading_level: Some(_), .. } =
                    &self.rendered_blocks[i]
                {
                    Some(line_start)
                } else {
                    None
                }
            })
            .collect();
    }

    pub(crate) fn jump_to_next_heading(&mut self) {
        // Current heading sits at scroll_offset + 2 (due to scroll_to_line padding).
        let current_pos = self.scroll_offset + 2;
        if let Some(&target) = self.heading_line_offsets.iter().find(|&&o| o > current_pos) {
            self.scroll_to_line(target);
        }
    }

    pub(crate) fn jump_to_prev_heading(&mut self) {
        let current_pos = self.scroll_offset + 2;
        if let Some(&target) = self.heading_line_offsets.iter().rev().find(|&&o| o < current_pos) {
            self.scroll_to_line(target);
        }
    }

    /// Reset all document state (e.g. after deleting the current file).
    pub(crate) fn clear(&mut self) {
        self.current_file = None;
        self.file_content.clear();
        self.rendered_lines.clear();
        self.rendered_lines_lower.clear();
        self.rendered_blocks.clear();
        self.links.clear();
        self.heading_line_offsets.clear();
        self.block_line_starts.clear();
        self.scroll_offset = 0;
    }
}
