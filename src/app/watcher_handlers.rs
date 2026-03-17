//! Handlers for filesystem watcher events.

use std::path::Path;

use crate::file_tree;
use crate::markdown::{deduplicate_links, render_markdown_blocks, rewrap_blocks};
use crate::watcher::FsEvent;

use super::App;

impl App {
    /// Dispatch a filesystem event to the appropriate handler.
    pub(crate) fn handle_fs_event(&mut self, event: FsEvent) {
        match event {
            FsEvent::FileModified(path) => self.handle_file_modified(&path),
            FsEvent::EntryCreated { path, is_dir } => self.handle_entry_created(&path, is_dir),
            FsEvent::EntryRemoved(path) => self.handle_entry_removed(&path),
            FsEvent::EntryRenamed { from, to } => self.handle_entry_renamed(&from, &to),
        }
    }

    fn handle_file_modified(&mut self, path: &Path) {
        let Some(ref current) = self.document.current_file else {
            return;
        };
        if current != path {
            return;
        }

        let Ok(new_content) = std::fs::read_to_string(path) else {
            return;
        };

        // Content equality check: skip if nothing actually changed (e.g. our own save triggered this).
        if new_content == self.document.file_content {
            return;
        }

        if self.editor.textarea.is_some() {
            // Editor is active — warn the user instead of reloading.
            self.editor.external_change_detected = true;
            self.status_message = "File changed on disk! :e to reload, :w to overwrite".to_string();
        } else {
            // Auto-reload in preview mode.
            self.reload_preview_content(&new_content);
            self.status_message = "reloaded".to_string();
        }
    }

    fn handle_entry_created(&mut self, path: &Path, is_dir: bool) {
        // Must be under root_path.
        if !path.starts_with(&self.root_path) {
            return;
        }

        // Non-directories must be markdown files.
        if !is_dir {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !file_tree::has_md_extension(name) {
                return;
            }
        }

        // Vim save pattern: delete + rename → Create event for the file we already have open.
        if self.document.current_file.as_deref() == Some(path) {
            self.handle_file_modified(path);
            return;
        }

        self.refresh_tree_add(path, is_dir, None);
    }

    fn handle_entry_removed(&mut self, path: &Path) {
        self.refresh_tree_remove(path, None);

        if self.document.current_file.as_deref() == Some(path) {
            self.document.clear();
            self.status_message = "File deleted".to_string();
        }
    }

    fn handle_entry_renamed(&mut self, from: &Path, to: &Path) {
        let is_dir = to.is_dir();

        if is_dir {
            self.refresh_tree_move(from, to, true, None);
        } else {
            // Non-directories must be markdown files to appear in the tree.
            let to_name = to.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let from_name = from.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let to_is_md = file_tree::has_md_extension(to_name);
            let from_is_md = file_tree::has_md_extension(from_name);

            if from_is_md && to_is_md {
                self.refresh_tree_move(from, to, false, None);
            } else if from_is_md {
                self.refresh_tree_remove(from, None);
            } else if to_is_md {
                self.refresh_tree_add(to, false, None);
            }
            // Neither is .md — ignore entirely.
        }

        if self.document.current_file.as_deref() == Some(from) {
            self.document.current_file = Some(to.to_path_buf());
            self.status_message = format!(
                "File renamed → {}",
                to.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            );
        }
    }

    /// Re-render preview from new file content, preserving scroll position.
    fn reload_preview_content(&mut self, new_content: &str) {
        let (blocks, links) = render_markdown_blocks(new_content);
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
        self.document.file_content = new_content.to_string();
        self.document.rebuild_heading_index();

        // Preserve scroll position, clamped to new content length.
        self.document.clamp_scroll();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::test_util::TempTestDir;
    use crate::watcher::FsEvent;
    use ratatui::style::Color;

    // ── handle_file_modified ─────────────────────────────────────

    #[test]
    fn modified_auto_reloads_preview() {
        let dir = TempTestDir::new("mdt-wh-reload");
        dir.create_file("test.md", "# Old");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        assert_eq!(app.document.file_content, "# Old");

        // Overwrite on disk.
        std::fs::write(&file, "# New").unwrap();
        app.handle_fs_event(FsEvent::FileModified(file));

        assert_eq!(app.document.file_content, "# New");
        assert_eq!(app.status_message, "reloaded");
    }

    #[test]
    fn modified_warns_in_editor() {
        let dir = TempTestDir::new("mdt-wh-editor-warn");
        dir.create_file("test.md", "# Old");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.enter_editor();

        // Overwrite on disk while editor is open.
        std::fs::write(&file, "# New").unwrap();
        app.handle_fs_event(FsEvent::FileModified(file));

        assert!(app.editor.external_change_detected);
        assert!(app.status_message.contains("changed on disk"));
        // Content should NOT be updated — editor is active.
        assert_eq!(app.document.file_content, "# Old");
    }

    #[test]
    fn modified_skips_different_file() {
        let dir = TempTestDir::new("mdt-wh-skip-diff");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&dir.path().join("a.md"));
        app.status_message.clear();

        app.handle_fs_event(FsEvent::FileModified(dir.path().join("b.md")));
        assert!(app.status_message.is_empty());
    }

    #[test]
    fn modified_skips_unchanged() {
        let dir = TempTestDir::new("mdt-wh-skip-same");
        dir.create_file("test.md", "# Same");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);
        app.status_message.clear();

        // File content on disk is identical — should be a no-op.
        app.handle_fs_event(FsEvent::FileModified(file));
        assert!(app.status_message.is_empty());
    }

    // ── handle_entry_created ─────────────────────────────────────

    #[test]
    fn created_md_refreshes_tree() {
        let dir = TempTestDir::new("mdt-wh-create-md");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        // Create a new file on disk, then send the event.
        let new_file = dir.path().join("new.md");
        std::fs::write(&new_file, "# New").unwrap();
        app.handle_fs_event(FsEvent::EntryCreated { path: new_file, is_dir: false });

        assert!(app.tree.path_map.contains_key("new.md"));
    }

    #[test]
    fn created_non_md_ignored() {
        let dir = TempTestDir::new("mdt-wh-create-txt");
        dir.create_file("test.md", "# Test");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let txt_file = dir.path().join("file.txt");
        std::fs::write(&txt_file, "hello").unwrap();
        app.handle_fs_event(FsEvent::EntryCreated { path: txt_file, is_dir: false });

        assert!(!app.tree.path_map.contains_key("file.txt"));
    }

    #[test]
    fn created_current_file_triggers_reload() {
        let dir = TempTestDir::new("mdt-wh-create-vim");
        dir.create_file("test.md", "# Original");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);

        // Overwrite to simulate vim's delete+rename pattern.
        std::fs::write(&file, "# Updated").unwrap();
        app.handle_fs_event(FsEvent::EntryCreated { path: file, is_dir: false });

        assert_eq!(app.document.file_content, "# Updated");
        assert_eq!(app.status_message, "reloaded");
    }

    // ── handle_entry_removed ─────────────────────────────────────

    #[test]
    fn removed_current_clears_document() {
        let dir = TempTestDir::new("mdt-wh-remove");
        dir.create_file("test.md", "# Test");
        let file = dir.path().join("test.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&file);

        app.handle_fs_event(FsEvent::EntryRemoved(file));

        assert!(app.document.current_file.is_none());
        assert!(app.document.file_content.is_empty());
        assert_eq!(app.status_message, "File deleted");
    }

    // ── handle_entry_renamed ─────────────────────────────────────

    #[test]
    fn renamed_current_updates_path() {
        let dir = TempTestDir::new("mdt-wh-rename");
        dir.create_file("old.md", "# Old");

        let old_path = dir.path().join("old.md");
        let new_path = dir.path().join("new.md");

        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        app.open_file(&old_path);

        // Create new file on disk (the rename target).
        std::fs::rename(&old_path, &new_path).unwrap();
        app.handle_fs_event(FsEvent::EntryRenamed { from: old_path, to: new_path.clone() });

        assert_eq!(app.document.current_file, Some(new_path));
        assert!(app.status_message.contains("renamed"));
    }
}
