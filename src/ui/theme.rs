//! Style constants for modal overlays and the OpenCode-style modal system.

use ratatui::style::{Color, Modifier, Style};

// ── Existing modal styles (preserved) ──────────────────────────────────

/// Subtle border for the modal frame.
pub(crate) const MODAL_BORDER: Style = Style::new().fg(Color::DarkGray);

/// Title text: bold white, used in the top-left of the title bar.
pub(crate) const MODAL_TITLE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);

/// Hint/muted text (footer, secondary info).
pub(crate) const MODAL_HINT: Style = Style::new().fg(Color::Gray);

/// Selected/highlighted list item: blue background, black text (matches file tree).
pub(crate) const MODAL_SELECTED: Style =
    Style::new().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD);

// ── New: OpenCode-style modal zone styles ──────────────────────────────

/// Top-right "esc" dismiss hint in the title bar.
pub(crate) const MODAL_ESC_HINT: Style = Style::new().fg(Color::Gray);

/// Shortcut action label in the bottom bar (e.g. "delete", "rename").
pub(crate) const MODAL_SHORTCUT_LABEL: Style =
    Style::new().fg(Color::White).add_modifier(Modifier::BOLD);

/// Shortcut key combo in the bottom bar (e.g. "ctrl+d", "ctrl+r").
pub(crate) const MODAL_SHORTCUT_KEY: Style = Style::new().fg(Color::Gray);

/// Search input text the user has typed.
pub(crate) const MODAL_SEARCH_TEXT: Style = Style::new().fg(Color::White);

/// Search placeholder text when the input is empty.
pub(crate) const MODAL_SEARCH_PLACEHOLDER: Style = Style::new().fg(Color::DarkGray);

/// Block cursor — inverted colors so the character underneath stays visible.
/// Blink is driven by software toggle in `App::tick_cursor`.
pub(crate) const MODAL_CURSOR: Style = Style::new().fg(Color::Black).bg(Color::White);

// ── Help overlay styles ────────────────────────────────────────────────

/// Key binding label in the help overlay (bright cyan, bold — stands out).
pub(crate) const HELP_KEY_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

/// Description text in the help overlay (muted gray — clearly secondary).
pub(crate) const HELP_DESC_STYLE: Style = Style::new().fg(Color::Gray);
