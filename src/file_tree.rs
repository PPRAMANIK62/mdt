//! File tree navigation — scans directories for `.md` files and provides
//! keyboard-driven selection and traversal.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use tui_tree_widget::TreeItem;

/// Return type for [`build_tree_items`]: tree items + ID-to-path lookup map.
pub type TreeBuildResult = (Vec<TreeItem<'static, String>>, HashMap<String, (PathBuf, bool)>);

/// Maximum directory depth when scanning for `.md` files.
/// Higher values find more files but slow startup for deeply nested trees.
/// 5 covers most project structures (e.g., `docs/api/v2/guides/intro.md`).
const DIR_SCAN_MAX_DEPTH: u32 = 5;

/// Build a recursive tree of [`TreeItem`]s from a root directory.
///
/// Returns the tree items and a map from tree ID (relative path from root)
/// to `(absolute_path, is_directory)`.
pub fn build_tree_items(root: &Path) -> Result<TreeBuildResult> {
    let canonical = fs::canonicalize(root)?;
    let mut path_map = HashMap::new();
    let items = build_items_recursive(&canonical, &canonical, &mut path_map, DIR_SCAN_MAX_DEPTH)?;
    Ok((items, path_map))
}

fn build_items_recursive(
    dir: &Path,
    root: &Path,
    path_map: &mut HashMap<String, (PathBuf, bool)>,
    remaining_depth: u32,
) -> Result<Vec<TreeItem<'static, String>>> {
    if remaining_depth == 0 {
        return Ok(Vec::new());
    }

    let mut raw: Vec<(String, PathBuf, bool)> = Vec::new();

    for result in fs::read_dir(dir)? {
        let de = result?;
        let name = de.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = de.path();
        let ft = de.file_type()?;

        if ft.is_dir() {
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
            let children = build_items_recursive(&abs_path, root, path_map, remaining_depth - 1)?;
            if children.is_empty() {
                path_map.remove(&rel);
                continue;
            }
            items.push(
                TreeItem::new(
                    rel,
                    Line::from(Span::styled(
                        format!("{name}/"),
                        Style::new().fg(Color::Indexed(74)).add_modifier(Modifier::BOLD),
                    )),
                    children,
                )
                .map_err(|e| anyhow::anyhow!("tree build error: {e}"))?,
            );
        } else {
            items.push(TreeItem::new_leaf(
                rel,
                Line::from(Span::styled(name, Style::new().fg(Color::Indexed(253)))),
            ));
        }
    }

    Ok(items)
}


/// Check if a filename ends with `.md` (case-insensitive).
fn has_md_extension(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempTestDir;

    // ── has_md_extension ─────────────────────────────────────────

    #[test]
    fn has_md_extension_ascii_lowercase() {
        assert!(has_md_extension("readme.md"));
    }

    #[test]
    fn has_md_extension_ascii_uppercase() {
        assert!(has_md_extension("README.MD"));
    }

    #[test]
    fn has_md_extension_mixed_case() {
        assert!(has_md_extension("Notes.Md"));
    }

    #[test]
    fn has_md_extension_cjk_filename() {
        assert!(has_md_extension("日本語.md"));
    }

    #[test]
    fn has_md_extension_emoji_filename() {
        assert!(has_md_extension("📝notes.md"));
    }

    #[test]
    fn has_md_extension_no_extension() {
        assert!(!has_md_extension("justtext"));
    }

    #[test]
    fn has_md_extension_dot_only() {
        assert!(!has_md_extension(".md"));
    }

    #[test]
    fn has_md_extension_empty_string() {
        assert!(!has_md_extension(""));
    }

    #[test]
    fn has_md_extension_other_extension() {
        assert!(!has_md_extension("file.txt"));
    }

    #[test]
    fn has_md_extension_double_extension() {
        assert!(has_md_extension("archive.tar.md"));
    }

    // ── build_tree_items ─────────────────────────────────────────

    #[test]
    fn build_tree_items_empty_dir() {
        let dir = TempTestDir::new("mdt-test-ft-empty");

        let (items, map) = build_tree_items(dir.path()).unwrap();
        assert!(items.is_empty());
        assert!(map.is_empty());
    }

    #[test]
    fn build_tree_items_with_md_files() {
        let dir = TempTestDir::new("mdt-test-ft-md");
        dir.create_file("hello.md", "# Hello");
        dir.create_file("world.md", "# World");

        let (items, map) = build_tree_items(dir.path()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn build_tree_items_excludes_non_md() {
        let dir = TempTestDir::new("mdt-test-ft-nonmd");
        dir.create_file("notes.md", "# Notes");
        dir.create_file("image.png", "fake png");
        dir.create_file("data.json", "{}");

        let (items, map) = build_tree_items(dir.path()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(map.len(), 1);
    }
}
