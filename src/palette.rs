//! Named semantic colors shared across UI and markdown themes.
//!
//! Both `ui/theme.rs` and `markdown/theme.rs` reference these constants
//! to keep the color palette consistent and easy to change.

use ratatui::style::Color;

// ── Foreground colors ────────────────────────────────────────────────

pub const FG_PRIMARY: Color = Color::White;
pub const FG_SECONDARY: Color = Color::Gray;
pub const FG_MUTED: Color = Color::DarkGray;

// ── Accent colors ────────────────────────────────────────────────────

pub const ACCENT_CYAN: Color = Color::Cyan;
pub const ACCENT_GREEN: Color = Color::Green;
pub const ACCENT_YELLOW: Color = Color::Yellow;
pub const ACCENT_BLUE: Color = Color::Blue;
pub const ACCENT_RED: Color = Color::LightRed;
pub const ACCENT_LIGHT_CYAN: Color = Color::LightCyan;

// ── Structural colors ───────────────────────────────────────────────

pub const BORDER: Color = Color::DarkGray;
pub const SELECTION_BG: Color = Color::Blue;
pub const SELECTION_FG: Color = Color::Black;
pub const HIGHLIGHT_BG: Color = Color::White;
pub const HIGHLIGHT_FG: Color = Color::Black;
