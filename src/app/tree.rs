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
