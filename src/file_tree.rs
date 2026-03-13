//! File tree navigation — scans directories for `.md` files and provides
//! keyboard-driven selection and traversal.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use std::collections::HashMap;
use tui_tree_widget::TreeItem;

/// Return type for [`build_tree_items`]: tree items + ID-to-path lookup map.
pub type TreeBuildResult = (Vec<TreeItem<'static, String>>, HashMap<String, (PathBuf, bool)>);

/// Build a recursive tree of [`TreeItem`]s from a root directory.
///
/// Returns the tree items and a map from tree ID (relative path from root)
/// to `(absolute_path, is_directory)`.
pub fn build_tree_items(root: &Path) -> Result<TreeBuildResult> {
    let canonical = fs::canonicalize(root)?;
    let mut path_map = HashMap::new();
    let items = build_items_recursive(&canonical, &canonical, &mut path_map)?;
    Ok((items, path_map))
}

fn build_items_recursive(
    dir: &Path,
    root: &Path,
    path_map: &mut HashMap<String, (PathBuf, bool)>,
) -> Result<Vec<TreeItem<'static, String>>> {
    let mut raw: Vec<(String, PathBuf, bool)> = Vec::new();

    for result in fs::read_dir(dir)? {
        let de = result?;
        let name = de.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = de.path();
        let ft = de.file_type()?;

        if ft.is_dir() && dir_contains_md(&path, 3) {
            raw.push((name, path, true));
        } else if ft.is_file() && has_md_extension(&name) {
            raw.push((name, path, false));
        }
    }

    // Sort: directories first, then case-insensitive alphabetical.
    raw.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));

    let mut items = Vec::new();
    for (name, abs_path, is_dir) in raw {
        let rel = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_string_lossy()
            .into_owned()
            .replace('\\', "/");

        path_map.insert(rel.clone(), (abs_path.clone(), is_dir));

        if is_dir {
            let children = build_items_recursive(&abs_path, root, path_map)?;
            items.push(
                TreeItem::new(rel, format!("\u{1f4c1} {name}"), children)
                    .map_err(|e| anyhow::anyhow!("tree build error: {e}"))?,
            );
        } else {
            items.push(TreeItem::new_leaf(rel, name));
        }
    }

    Ok(items)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively check whether `dir` contains at least one `.md` file.
/// `max_depth` limits how deep we look (0 means only check direct children).
fn dir_contains_md(dir: &Path, max_depth: u32) -> bool {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return false,
    };

    for result in read {
        let de = match result {
            Ok(d) => d,
            Err(_) => continue,
        };

        let name = de.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let ft = match de.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        if ft.is_file() && has_md_extension(&name) {
            return true;
        }

        if ft.is_dir() && max_depth > 0 && dir_contains_md(&de.path(), max_depth - 1) {
            return true;
        }
    }

    false
}

/// Check if a filename ends with `.md` (case-insensitive).
fn has_md_extension(name: &str) -> bool {
    name.len() > 3 && name[name.len() - 3..].eq_ignore_ascii_case(".md")
}
