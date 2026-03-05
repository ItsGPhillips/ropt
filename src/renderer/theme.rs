//! Colour and style constants for all renderers.
//!
//! Centralising style here means changing the look-and-feel is a one-file
//! edit rather than a search-and-replace across the codebase.

use crossterm::style::{Attribute, Color};

pub struct Theme;

impl Theme {
    // ── Prompt / question ─────────────────────────────────────────────────────
    pub const PROMPT_MARK: &'static str = "?";
    pub const PROMPT_COLOR: Color = Color::Cyan;
    pub const PROMPT_ATTR: Attribute = Attribute::Bold;

    // ── Selected item ─────────────────────────────────────────────────────────
    pub const SELECTED_COLOR: Color = Color::Green;
    pub const SELECTED_ATTR: Attribute = Attribute::Bold;
    pub const CURSOR: &'static str = "❯";

    // ── Disabled item ─────────────────────────────────────────────────────────
    pub const DISABLED_COLOR: Color = Color::DarkGrey;
    pub const DISABLED_MARK: &'static str = "(unavailable)";

    // ── Default marker ────────────────────────────────────────────────────────
    pub const DEFAULT_MARK: &'static str = " [default]";
    pub const DEFAULT_COLOR: Color = Color::DarkGrey;

    // ── Group header ──────────────────────────────────────────────────────────
    pub const GROUP_COLOR: Color = Color::Blue;
    pub const GROUP_ATTR: Attribute = Attribute::Underlined;

    // ── Input field ───────────────────────────────────────────────────────────
    pub const INPUT_COLOR: Color = Color::Yellow;

    // ── Error / warning ───────────────────────────────────────────────────────
    pub const ERROR_COLOR: Color = Color::Red;

    // ── Filter / match highlight (reserved for future use) ───────────────────
}
