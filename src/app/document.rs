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

#[cfg(test)]
mod tests {
    use ratatui::text::{Line, Span};

    use super::*;

    fn make_doc(num_lines: usize, viewport_height: usize) -> DocumentState {
        let rendered_lines: Vec<Line<'static>> =
            (0..num_lines).map(|i| Line::from(format!("line {i}"))).collect();
        let rendered_lines_lower =
            rendered_lines.iter().map(|l| l.to_string().to_lowercase()).collect();
        DocumentState {
            current_file: None,
            file_content: String::new(),
            rendered_lines,
            rendered_lines_lower,
            rendered_blocks: Vec::new(),
            links: Vec::new(),
            heading_line_offsets: Vec::new(),
            block_line_starts: Vec::new(),
            scroll_offset: 0,
            viewport_height,
            viewport_width: 80,
        }
    }

    #[test]
    fn scroll_down_empty_document() {
        let mut doc = make_doc(0, 10);
        doc.scroll_down();
        assert_eq!(doc.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_at_zero_stays_zero() {
        let mut doc = make_doc(20, 10);
        doc.scroll_up();
        assert_eq!(doc.scroll_offset, 0);
    }

    #[test]
    fn scroll_down_clamps_to_max() {
        let mut doc = make_doc(15, 10);
        for _ in 0..20 {
            doc.scroll_down();
        }
        assert_eq!(doc.scroll_offset, 5); // 15 - 10
    }

    #[test]
    fn scroll_half_page_down_at_least_one() {
        let mut doc = make_doc(20, 1); // viewport=1, half=0, max(0,1)=1
        doc.scroll_half_page_down();
        assert_eq!(doc.scroll_offset, 1);
    }

    #[test]
    fn scroll_half_page_up_at_least_one() {
        let mut doc = make_doc(20, 1);
        doc.scroll_offset = 5;
        doc.scroll_half_page_up();
        assert_eq!(doc.scroll_offset, 4);
    }

    #[test]
    fn max_scroll_with_fewer_lines_than_viewport() {
        let doc = make_doc(5, 10);
        assert_eq!(doc.max_scroll(), 0);
    }

    #[test]
    fn clamp_scroll_reduces_to_max() {
        let mut doc = make_doc(15, 10);
        doc.scroll_offset = 100;
        doc.clamp_scroll();
        assert_eq!(doc.scroll_offset, 5);
    }

    #[test]
    fn scroll_to_line_positions_with_padding() {
        let mut doc = make_doc(30, 10);
        doc.scroll_to_line(10);
        assert_eq!(doc.scroll_offset, 8); // 10 - 2
    }

    #[test]
    fn scroll_to_line_zero_clamps_to_zero() {
        let mut doc = make_doc(30, 10);
        doc.scroll_to_line(0);
        assert_eq!(doc.scroll_offset, 0);
    }

    #[test]
    fn rebuild_heading_index_filters_headings() {
        let mut doc = make_doc(0, 10);
        doc.rendered_blocks = vec![
            RenderedBlock::StyledLine {
                spans: vec![Span::raw("heading")],
                blockquote_depth: 0,
                list_marker_width: 0,
                heading_level: Some(1),
            },
            RenderedBlock::StyledLine {
                spans: vec![Span::raw("paragraph")],
                blockquote_depth: 0,
                list_marker_width: 0,
                heading_level: None,
            },
            RenderedBlock::StyledLine {
                spans: vec![Span::raw("heading2")],
                blockquote_depth: 0,
                list_marker_width: 0,
                heading_level: Some(2),
            },
        ];
        doc.block_line_starts = vec![0, 2, 4];
        doc.rebuild_heading_index();
        assert_eq!(doc.heading_line_offsets, vec![0, 4]);
    }

    #[test]
    fn jump_to_next_heading() {
        let mut doc = make_doc(30, 10);
        doc.heading_line_offsets = vec![5, 15, 25];
        doc.scroll_offset = 0;
        doc.jump_to_next_heading();
        // current_pos = 0 + 2 = 2, first heading > 2 is 5, scroll_to_line(5) = 3
        assert_eq!(doc.scroll_offset, 3);
    }

    #[test]
    fn jump_to_next_heading_no_more_headings() {
        let mut doc = make_doc(30, 10);
        doc.heading_line_offsets = vec![5];
        doc.scroll_offset = 10;
        let before = doc.scroll_offset;
        doc.jump_to_next_heading();
        assert_eq!(doc.scroll_offset, before);
    }

    #[test]
    fn jump_to_prev_heading() {
        let mut doc = make_doc(30, 10);
        doc.heading_line_offsets = vec![5, 15, 25];
        doc.scroll_offset = 15;
        doc.jump_to_prev_heading();
        // current_pos = 15 + 2 = 17, last heading < 17 is 15, scroll_to_line(15) = 13
        assert_eq!(doc.scroll_offset, 13);
    }

    #[test]
    fn jump_to_prev_heading_no_earlier() {
        let mut doc = make_doc(30, 10);
        doc.heading_line_offsets = vec![5];
        doc.scroll_offset = 0;
        doc.jump_to_prev_heading();
        assert_eq!(doc.scroll_offset, 0);
    }

    #[test]
    fn clear_resets_all_state() {
        let mut doc = make_doc(10, 5);
        doc.current_file = Some(std::path::PathBuf::from("/tmp/test.md"));
        doc.file_content = "hello".to_string();
        doc.scroll_offset = 5;
        doc.heading_line_offsets = vec![1, 2];

        doc.clear();

        assert!(doc.current_file.is_none());
        assert!(doc.file_content.is_empty());
        assert!(doc.rendered_lines.is_empty());
        assert!(doc.rendered_lines_lower.is_empty());
        assert!(doc.rendered_blocks.is_empty());
        assert!(doc.links.is_empty());
        assert!(doc.heading_line_offsets.is_empty());
        assert!(doc.block_line_starts.is_empty());
        assert_eq!(doc.scroll_offset, 0);
    }

    #[test]
    fn rebuild_lower_cache_lowercases_spans() {
        let mut doc = make_doc(0, 10);
        doc.rendered_lines = vec![
            Line::from(vec![Span::raw("Hello"), Span::raw(" WORLD")]),
            Line::from(vec![Span::raw("FoO")]),
        ];
        doc.rebuild_lower_cache();
        assert_eq!(doc.rendered_lines_lower, vec!["hello world", "foo"]);
    }
}
