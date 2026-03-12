//! File tree navigation — scans directories for `.md` files and provides
//! keyboard-driven selection and traversal.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// A single entry in the file tree (file or directory).
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Display name (file/dir name only, not full path).
    pub name: String,
    /// Absolute path to the entry.
    pub path: PathBuf,
    /// `true` if this entry is a directory.
    pub is_dir: bool,
    /// Nesting depth relative to the tree root (0 = top-level).
    pub depth: usize,
}

/// Navigable file tree rooted at a directory, showing only `.md` files and
/// directories that (recursively) contain `.md` files.
#[derive(Debug)]
pub struct FileTree {
    /// Visible entries in the current directory.
    pub entries: Vec<FileEntry>,
    /// Index of the currently selected entry.
    pub selected: usize,
    /// The original root path the tree was opened with.
    pub root_path: PathBuf,
    /// The directory currently being displayed.
    pub current_dir: PathBuf,
}

impl FileTree {
    /// Scan `path` and build a new `FileTree`.
    ///
    /// Only `.md` files and directories that contain `.md` files (checked up to
    /// 3 levels deep) are included. Dotfiles are skipped. Entries are sorted
    /// directories-first, then case-insensitive alphabetical.
    pub fn scan(path: &Path) -> Result<Self> {
        let canonical = fs::canonicalize(path)?;
        let entries = scan_dir(&canonical, 0)?;
        Ok(Self {
            entries,
            selected: 0,
            root_path: canonical.clone(),
            current_dir: canonical,
        })
    }

    /// Move selection up by one (clamped at 0).
    pub fn navigate_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Move selection down by one (clamped at last entry).
    pub fn navigate_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }

    /// Enter the selected entry.
    ///
    /// - **Directory**: changes `current_dir`, rescans, returns `None`.
    /// - **`.md` file**: returns `Some(path)` so the caller can open it.
    /// - **No selection**: returns `None`.
    pub fn enter(&mut self) -> Option<PathBuf> {
        let entry = self.entries.get(self.selected)?.clone();
        if entry.is_dir {
            self.current_dir = entry.path;
            // Best-effort rescan; keep empty list on error.
            self.entries = scan_dir(&self.current_dir, 0).unwrap_or_default();
            self.selected = 0;
            None
        } else {
            Some(entry.path)
        }
    }

    /// Navigate to the parent directory (no-op if already at `root_path`).
    pub fn go_back(&mut self) {
        if self.current_dir == self.root_path {
            return;
        }
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.entries = scan_dir(&self.current_dir, 0).unwrap_or_default();
            self.selected = 0;
        }
    }

    /// Path of the currently selected entry, if any.
    pub fn selected_path(&self) -> Option<&Path> {
        self.entries.get(self.selected).map(|e| e.path.as_path())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read a single directory level and return sorted, filtered entries.
fn scan_dir(dir: &Path, depth: usize) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    let read = fs::read_dir(dir)?;
    for result in read {
        let de = result?;
        let name = de.file_name().to_string_lossy().into_owned();

        // Skip dotfiles.
        if name.starts_with('.') {
            continue;
        }

        let path = de.path();
        let ft = de.file_type()?;

        if ft.is_dir() {
            if dir_contains_md(&path, 3) {
                entries.push(FileEntry {
                    name,
                    path,
                    is_dir: true,
                    depth,
                });
            }
        } else if ft.is_file() && has_md_extension(&name) {
            entries.push(FileEntry {
                name,
                path,
                is_dir: false,
                depth,
            });
        }
    }

    // Sort: directories first, then case-insensitive alphabetical.
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    /// Helper: create a temp directory tree for testing.
    /// Each test must pass a unique `label` to avoid collisions.
    fn setup_tree(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mdt_test_{}_{}", std::process::id(), label));
        // Clean up any previous run.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        File::create(dir.join("hello.md")).unwrap();
        File::create(dir.join("notes.md")).unwrap();
        File::create(dir.join(".hidden.md")).unwrap();

        fs::create_dir(dir.join("docs")).unwrap();
        File::create(dir.join("docs/guide.md")).unwrap();

        fs::create_dir(dir.join("empty_dir")).unwrap();

        fs::create_dir(dir.join(".secret")).unwrap();
        File::create(dir.join(".secret/secret.md")).unwrap();

        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scan_filters_md_and_dirs() {
        let dir = setup_tree("filter");
        let tree = FileTree::scan(&dir).unwrap();

        let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"docs"), "dirs with .md should appear");
        assert!(!names.contains(&"empty_dir"), "empty dirs filtered");
        assert!(!names.contains(&".secret"), "dotdirs filtered");
        assert!(!names.contains(&".hidden.md"), "dotfiles filtered");
        assert!(names.contains(&"hello.md"));
        assert!(names.contains(&"notes.md"));

        cleanup(&dir);
    }

    #[test]
    fn scan_sorts_dirs_first_then_alpha() {
        let dir = setup_tree("sort");
        let tree = FileTree::scan(&dir).unwrap();

        assert!(tree.entries[0].is_dir, "dirs come first");
        assert_eq!(tree.entries[0].name, "docs");

        let file_names: Vec<&str> = tree
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(file_names, vec!["hello.md", "notes.md"]);

        cleanup(&dir);
    }

    #[test]
    fn navigate_up_down_clamps() {
        let dir = setup_tree("nav");
        let mut tree = FileTree::scan(&dir).unwrap();

        assert_eq!(tree.selected, 0);
        tree.navigate_up();
        assert_eq!(tree.selected, 0, "clamped at 0");

        let max = tree.entries.len() - 1;
        for _ in 0..100 {
            tree.navigate_down();
        }
        assert_eq!(tree.selected, max, "clamped at last");

        cleanup(&dir);
    }

    #[test]
    fn enter_file_returns_path() {
        let dir = setup_tree("enter_file");
        let mut tree = FileTree::scan(&dir).unwrap();

        // Navigate past the directory to the first file
        tree.navigate_down();
        let result = tree.enter();
        assert!(result.is_some(), "entering a file returns Some(path)");
        let p = result.unwrap();
        assert!(p.to_string_lossy().ends_with(".md"));

        cleanup(&dir);
    }

    #[test]
    fn enter_dir_rescans() {
        let dir = setup_tree("enter_dir");
        let mut tree = FileTree::scan(&dir).unwrap();

        assert_eq!(tree.selected, 0);
        assert!(tree.entries[0].is_dir);

        let result = tree.enter();
        assert!(result.is_none(), "entering a dir returns None");
        assert!(tree.current_dir.ends_with("docs"));
        assert_eq!(tree.entries.len(), 1);
        assert_eq!(tree.entries[0].name, "guide.md");

        cleanup(&dir);
    }

    #[test]
    fn go_back_to_parent() {
        let dir = setup_tree("go_back");
        let mut tree = FileTree::scan(&dir).unwrap();

        tree.enter();
        assert!(tree.current_dir.ends_with("docs"));

        tree.go_back();
        assert_eq!(tree.current_dir, tree.root_path);

        cleanup(&dir);
    }

    #[test]
    fn go_back_noop_at_root() {
        let dir = setup_tree("noop");
        let mut tree = FileTree::scan(&dir).unwrap();
        let before = tree.current_dir.clone();
        tree.go_back();
        assert_eq!(tree.current_dir, before, "no-op at root");

        cleanup(&dir);
    }

    #[test]
    fn selected_path_returns_correct_entry() {
        let dir = setup_tree("sel_path");
        let tree = FileTree::scan(&dir).unwrap();
        let sp = tree.selected_path().unwrap();
        assert_eq!(sp, tree.entries[0].path.as_path());

        cleanup(&dir);
    }
}
