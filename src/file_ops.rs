//! Pure filesystem operations for file tree management.
//!
//! All functions are stateless — they take paths, perform filesystem work,
//! and return results. No dependency on application state.
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};

/// Reject path inputs that contain `..` components (directory traversal).
fn reject_dotdot(input: &str) -> Result<()> {
    let path = Path::new(input);
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            bail!("path must not contain '..' components: {input}");
        }
    }
    Ok(())
}

/// Verify that `resolved` is inside `root` using canonical paths.
fn ensure_within_root(root: &Path, resolved: &Path) -> Result<()> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_resolved = fs::canonicalize(resolved)?;
    if !canonical_resolved.starts_with(&canonical_root) {
        bail!(
            "path escapes root directory: {} is not within {}",
            canonical_resolved.display(),
            canonical_root.display()
        );
    }
    Ok(())
}

/// Create a markdown file at `base/input` within `root`.
///
/// - Auto-appends `.md` if the input doesn't end with `.md` (case-insensitive).
/// - Creates intermediate directories as needed.
/// - Fails if the file already exists or the path escapes `root`.
pub fn create_file(root: &Path, base: &Path, input: &str) -> Result<PathBuf> {
    reject_dotdot(input)?;

    let mut target = base.join(input);

    // Auto-append .md if missing.
    let needs_md = target.extension().map_or(true, |ext| !ext.eq_ignore_ascii_case("md"));
    if needs_md {
        let mut name = target.file_name().unwrap_or_default().to_os_string();
        name.push(".md");
        target.set_file_name(name);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    // Verify the parent is within root (target itself doesn't exist yet).
    if let Some(parent) = target.parent() {
        ensure_within_root(root, parent)?;
    }

    // Atomically create — fails if the file already exists (no TOCTOU race).
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow::anyhow!("file already exists: {}", target.display())
            } else {
                e.into()
            }
        })?;
    fs::canonicalize(&target).map_err(Into::into)
}

/// Create a directory at `base/input` within `root`.
///
/// - Creates intermediate directories as needed.
/// - Fails if the path escapes `root`.
pub fn create_dir(root: &Path, base: &Path, input: &str) -> Result<PathBuf> {
    reject_dotdot(input)?;

    let target = base.join(input);
    fs::create_dir_all(&target)?;
    ensure_within_root(root, &target)?;

    fs::canonicalize(&target).map_err(Into::into)
}

/// Delete a file or directory (recursive for directories).
pub fn delete_entry(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Rename a file or directory. `new_name` is a bare name, not a path.
///
/// Fails if the new path already exists or escapes `root`.
pub fn rename_entry(root: &Path, path: &Path, new_name: &str) -> Result<PathBuf> {
    reject_dotdot(new_name)?;

    let parent = path.parent().ok_or_else(|| anyhow::anyhow!("path has no parent"))?;
    let new_path = parent.join(new_name);

    if new_path.exists() {
        bail!("destination already exists: {}", new_path.display());
    }

    // Validate destination is within root *before* performing the rename.
    // The destination doesn't exist yet, so check its parent.
    if let Some(parent) = new_path.parent() {
        ensure_within_root(root, parent)?;
    }

    fs::rename(path, &new_path)?;

    fs::canonicalize(&new_path).map_err(Into::into)
}

/// Move a file or directory. `dest_input` is a relative path from `root`
/// (e.g. `docs/guides/`). Creates intermediate directories as needed.
///
/// Returns the new absolute path.
pub fn move_entry(root: &Path, source: &Path, dest_input: &str) -> Result<PathBuf> {
    reject_dotdot(dest_input)?;

    let dest_dir = root.join(dest_input);
    fs::create_dir_all(&dest_dir)?;

    // Validate destination is within root *before* performing the rename.
    ensure_within_root(root, &dest_dir)?;

    let file_name = source.file_name().ok_or_else(|| anyhow::anyhow!("source has no file name"))?;
    let new_path = dest_dir.join(file_name);

    if new_path.exists() {
        bail!("destination already exists: {}", new_path.display());
    }

    fs::rename(source, &new_path)?;

    fs::canonicalize(&new_path).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempTestDir;

    // ── create_file ──────────────────────────────────────────────

    #[test]
    fn create_file_simple_name() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-simple");
        let result = create_file(dir.path(), dir.path(), "hello.md").unwrap();
        assert!(result.exists());
        assert!(result.ends_with("hello.md"));
    }

    #[test]
    fn create_file_nested_path() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-nested");
        let result = create_file(dir.path(), dir.path(), "abc/def/ghi.md").unwrap();
        assert!(result.exists());
        assert!(result.ends_with("abc/def/ghi.md"));
    }

    #[test]
    fn create_file_auto_md_append() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-automd");
        let result = create_file(dir.path(), dir.path(), "notes").unwrap();
        assert!(result.exists());
        assert!(result.ends_with("notes.md"));
    }

    #[test]
    fn create_file_already_has_md_uppercase() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-mdcase");
        let result = create_file(dir.path(), dir.path(), "README.MD").unwrap();
        assert!(result.exists());
        assert!(result.ends_with("README.MD"));
    }

    #[test]
    fn create_file_existing_file_error() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-exists");
        dir.create_file("exists.md", "# Exists");
        let err = create_file(dir.path(), dir.path(), "exists.md").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn create_file_path_escape_attempt() {
        let dir = TempTestDir::new("mdt-test-fileops-cf-escape");
        let err = create_file(dir.path(), dir.path(), "../escape.md").unwrap_err();
        assert!(err.to_string().contains(".."));
    }

    // ── create_dir ───────────────────────────────────────────────

    #[test]
    fn create_dir_simple_name() {
        let dir = TempTestDir::new("mdt-test-fileops-cd-simple");
        let result = create_dir(dir.path(), dir.path(), "new_dir").unwrap();
        assert!(result.is_dir());
    }

    #[test]
    fn create_dir_nested_path() {
        let dir = TempTestDir::new("mdt-test-fileops-cd-nested");
        let result = create_dir(dir.path(), dir.path(), "a/b/c").unwrap();
        assert!(result.is_dir());
        assert!(result.ends_with("a/b/c"));
    }

    // ── delete_entry ─────────────────────────────────────────────

    #[test]
    fn delete_entry_file() {
        let dir = TempTestDir::new("mdt-test-fileops-del-file");
        dir.create_file("doomed.md", "bye");
        let target = dir.path().join("doomed.md");
        assert!(target.exists());
        delete_entry(&target).unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn delete_entry_directory() {
        let dir = TempTestDir::new("mdt-test-fileops-del-dir");
        let sub = dir.path().join("subdir");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("inner.md"), "inner").unwrap();
        assert!(sub.exists());
        delete_entry(&sub).unwrap();
        assert!(!sub.exists());
    }

    // ── rename_entry ─────────────────────────────────────────────

    #[test]
    fn rename_entry_happy_path() {
        let dir = TempTestDir::new("mdt-test-fileops-ren-ok");
        dir.create_file("old.md", "# Old");
        let old = dir.path().join("old.md");
        let result = rename_entry(dir.path(), &old, "new.md").unwrap();
        assert!(!old.exists());
        assert!(result.exists());
        assert!(result.ends_with("new.md"));
    }

    #[test]
    fn rename_entry_collision() {
        let dir = TempTestDir::new("mdt-test-fileops-ren-coll");
        dir.create_file("a.md", "");
        dir.create_file("b.md", "");
        let a = dir.path().join("a.md");
        let err = rename_entry(dir.path(), &a, "b.md").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    // ── move_entry ───────────────────────────────────────────────

    #[test]
    fn move_entry_happy_path() {
        let dir = TempTestDir::new("mdt-test-fileops-mv-ok");
        dir.create_file("readme.md", "# Hi");
        let dest = dir.path().join("docs");
        fs::create_dir_all(&dest).unwrap();
        let source = dir.path().join("readme.md");
        let result = move_entry(dir.path(), &source, "docs").unwrap();
        assert!(!source.exists());
        assert!(result.exists());
        assert!(result.ends_with("docs/readme.md"));
    }

    #[test]
    fn move_entry_creates_intermediate_dirs() {
        let dir = TempTestDir::new("mdt-test-fileops-mv-mkdir");
        dir.create_file("file.md", "content");
        let source = dir.path().join("file.md");
        let result = move_entry(dir.path(), &source, "deep/nested/dir").unwrap();
        assert!(!source.exists());
        assert!(result.exists());
        assert!(result.ends_with("deep/nested/dir/file.md"));
    }
}
