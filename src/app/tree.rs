//! Tree management methods on `App`.

use std::path::Path;

use crate::file_tree;

use super::App;

impl App {
    /// Refresh the tree after adding a new file or directory.
    ///
    /// Updates `path_map` in-place and rebuilds the tree structure without
    /// any filesystem access (the expensive `read_dir` calls are skipped).
    pub(crate) fn refresh_tree_add(
        &mut self,
        abs_path: &Path,
        is_dir: bool,
        select_id: Option<&str>,
    ) {
        let rel = self.relative_path_str(abs_path);
        self.ensure_parent_dirs_in_map(abs_path);

        self.tree.path_map.insert(rel, (abs_path.to_path_buf(), is_dir));
        self.finish_targeted_refresh(select_id);
    }

    /// Refresh the tree after deleting a file or directory.
    pub(crate) fn refresh_tree_remove(&mut self, abs_path: &Path, select_id: Option<&str>) {
        let rel = self.relative_path_str(abs_path);
        self.tree.path_map.remove(&rel);
        let prefix = format!("{rel}/");
        self.tree.path_map.retain(|k, _| !k.starts_with(&prefix));

        self.finish_targeted_refresh(select_id);
    }

    /// Refresh the tree after moving or renaming a file or directory.
    pub(crate) fn refresh_tree_move(
        &mut self,
        old_abs: &Path,
        new_abs: &Path,
        is_dir: bool,
        select_id: Option<&str>,
    ) {
        let old_rel = self.relative_path_str(old_abs);
        let new_rel = self.relative_path_str(new_abs);

        self.tree.path_map.remove(&old_rel);

        if is_dir {
            let old_prefix = format!("{old_rel}/");
            let updates: Vec<(String, std::path::PathBuf, bool)> = self
                .tree
                .path_map
                .iter()
                .filter(|(k, _)| k.starts_with(&old_prefix))
                .map(|(k, (p, d))| {
                    let suffix = &k[old_rel.len()..];
                    let child_rel = format!("{new_rel}{suffix}");
                    let child_abs = new_abs.join(p.strip_prefix(old_abs).unwrap_or(p));
                    (child_rel, child_abs, *d)
                })
                .collect();

            self.tree.path_map.retain(|k, _| !k.starts_with(&old_prefix));
            for (r, a, d) in updates {
                self.tree.path_map.insert(r, (a, d));
            }
        }

        self.ensure_parent_dirs_in_map(new_abs);
        self.tree.path_map.insert(new_rel, (new_abs.to_path_buf(), is_dir));
        self.finish_targeted_refresh(select_id);
    }

    fn finish_targeted_refresh(&mut self, select_id: Option<&str>) {
        self.tree.tree_items = file_tree::rebuild_tree_from_map(&mut self.tree.path_map);
        self.tree.filtered_tree_items = None;
        self.tree.filtered_path_map = None;
        if let Some(id) = select_id {
            self.tree.tree_state.select(vec![id.to_string()]);
        }
    }

    pub(crate) fn relative_path_str(&self, abs_path: &Path) -> String {
        abs_path
            .strip_prefix(&self.root_path)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .into_owned()
            .replace('\\', "/")
    }

    fn ensure_parent_dirs_in_map(&mut self, abs_path: &Path) {
        let mut current = abs_path.parent();
        while let Some(parent) = current {
            if parent == self.root_path || !parent.starts_with(&self.root_path) {
                break;
            }
            let parent_rel = self.relative_path_str(parent);
            self.tree.path_map.entry(parent_rel).or_insert_with(|| (parent.to_path_buf(), true));
            current = parent.parent();
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use crate::app::App;
    use crate::test_util::TempTestDir;

    #[test]
    fn relative_path_str_strips_root() {
        let dir = TempTestDir::new("mdt-test-tree-rel");
        dir.create_file("test.md", "# T");
        let app = App::new(dir.path(), Color::Reset).unwrap();
        let abs = dir.path().join("test.md");
        assert_eq!(app.relative_path_str(&abs), "test.md");
    }

    #[test]
    fn relative_path_str_nested() {
        let dir = TempTestDir::new("mdt-test-tree-rel-nested");
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        dir.create_file("sub/note.md", "# N");
        let app = App::new(dir.path(), Color::Reset).unwrap();
        let abs = dir.path().join("sub/note.md");
        assert_eq!(app.relative_path_str(&abs), "sub/note.md");
    }

    #[test]
    fn refresh_tree_add_inserts_file() {
        let dir = TempTestDir::new("mdt-test-tree-add");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let new_path = dir.path().join("b.md");
        app.refresh_tree_add(&new_path, false, Some("b.md"));

        assert!(app.tree.path_map.contains_key("b.md"));
        let (stored_path, is_dir) = &app.tree.path_map["b.md"];
        assert_eq!(stored_path, &new_path);
        assert!(!is_dir);
    }

    #[test]
    fn refresh_tree_add_creates_parent_dirs() {
        let dir = TempTestDir::new("mdt-test-tree-add-parents");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let new_path = dir.path().join("sub/deep/file.md");
        app.refresh_tree_add(&new_path, false, None);

        assert!(app.tree.path_map.contains_key("sub/deep/file.md"));
        assert!(app.tree.path_map.contains_key("sub/deep"));
        assert!(app.tree.path_map.contains_key("sub"));
        // Parent dirs should be marked as directories
        assert!(app.tree.path_map["sub"].1);
        assert!(app.tree.path_map["sub/deep"].1);
    }

    #[test]
    fn refresh_tree_remove_deletes_entry() {
        let dir = TempTestDir::new("mdt-test-tree-remove");
        dir.create_file("a.md", "# A");
        dir.create_file("b.md", "# B");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let path = dir.path().join("a.md");
        app.refresh_tree_remove(&path, None);

        assert!(!app.tree.path_map.contains_key("a.md"));
        assert!(app.tree.path_map.contains_key("b.md"));
    }

    #[test]
    fn refresh_tree_remove_deletes_children() {
        let dir = TempTestDir::new("mdt-test-tree-remove-children");
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        dir.create_file("sub/a.md", "# A");
        dir.create_file("sub/b.md", "# B");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let sub = dir.path().join("sub");
        app.refresh_tree_remove(&sub, None);

        assert!(!app.tree.path_map.contains_key("sub"));
        assert!(!app.tree.path_map.contains_key("sub/a.md"));
        assert!(!app.tree.path_map.contains_key("sub/b.md"));
    }

    #[test]
    fn refresh_tree_move_file() {
        let dir = TempTestDir::new("mdt-test-tree-move");
        dir.create_file("old.md", "# Old");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let old = dir.path().join("old.md");
        let new = dir.path().join("new.md");
        app.refresh_tree_move(&old, &new, false, Some("new.md"));

        assert!(!app.tree.path_map.contains_key("old.md"));
        assert!(app.tree.path_map.contains_key("new.md"));
    }

    #[test]
    fn refresh_tree_move_directory_updates_children() {
        let dir = TempTestDir::new("mdt-test-tree-move-dir");
        std::fs::create_dir_all(dir.path().join("old")).unwrap();
        dir.create_file("old/a.md", "# A");
        dir.create_file("old/b.md", "# B");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();

        let old = dir.path().join("old");
        let new = dir.path().join("new");
        app.refresh_tree_move(&old, &new, true, Some("new"));

        assert!(!app.tree.path_map.contains_key("old"));
        assert!(!app.tree.path_map.contains_key("old/a.md"));
        assert!(!app.tree.path_map.contains_key("old/b.md"));
        assert!(app.tree.path_map.contains_key("new"));
        assert!(app.tree.path_map.contains_key("new/a.md"));
        assert!(app.tree.path_map.contains_key("new/b.md"));
    }

    #[test]
    fn refresh_tree_add_clears_filtered_state() {
        let dir = TempTestDir::new("mdt-test-tree-add-filter");
        dir.create_file("a.md", "# A");
        let mut app = App::new(dir.path(), Color::Reset).unwrap();
        // Simulate having filtered state
        app.tree.filtered_tree_items = Some(Vec::new());
        app.tree.filtered_path_map = Some(std::collections::HashMap::new());

        let new_path = dir.path().join("b.md");
        app.refresh_tree_add(&new_path, false, None);

        assert!(app.tree.filtered_tree_items.is_none());
        assert!(app.tree.filtered_path_map.is_none());
    }
}
