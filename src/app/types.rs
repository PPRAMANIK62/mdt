//! Application enums: modes, focus, overlays, and file operations.

use std::path::PathBuf;

/// Active file operation (overlay, not a mode).
#[derive(Debug, Clone)]
pub(crate) enum FileOp {
    CreateFile { parent_dir: PathBuf },
    CreateDir { parent_dir: PathBuf },
    Rename { target: PathBuf, is_dir: bool },
    Delete { target: PathBuf, is_dir: bool, name: String },
    Move { source: PathBuf, is_dir: bool },
}

/// Current input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Insert,
    Command,
    Search,
}

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Command => write!(f, "COMMAND"),
            Self::Search => write!(f, "SEARCH"),
        }
    }
}

/// Which pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileList,
    Preview,
}

/// Which overlay (if any) is currently displayed.
///
/// These overlays are mutually exclusive — only one can be active at a time.
#[derive(Debug, Clone)]
pub(crate) enum Overlay {
    None,
    Help,
    LinkPicker,
    FileOp(FileOp),
    FileFinder,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_mode_display_normal() {
        assert_eq!(format!("{}", AppMode::Normal), "NORMAL");
    }

    #[test]
    fn app_mode_display_insert() {
        assert_eq!(format!("{}", AppMode::Insert), "INSERT");
    }

    #[test]
    fn app_mode_display_command() {
        assert_eq!(format!("{}", AppMode::Command), "COMMAND");
    }

    #[test]
    fn app_mode_display_search() {
        assert_eq!(format!("{}", AppMode::Search), "SEARCH");
    }

    #[test]
    fn app_mode_equality() {
        assert_eq!(AppMode::Normal, AppMode::Normal);
        assert_ne!(AppMode::Normal, AppMode::Insert);
    }

    #[test]
    fn focus_equality() {
        assert_eq!(Focus::FileList, Focus::FileList);
        assert_eq!(Focus::Preview, Focus::Preview);
        assert_ne!(Focus::FileList, Focus::Preview);
    }
}
