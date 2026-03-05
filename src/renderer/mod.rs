//! Terminal renderer module.
//!
//! Sub-modules:
//!   - `picklist`     – keyboard-navigable menu (↑↓ + Enter)
//!   - `input_prompt` – text input with optional type-to-filter behaviour
//!   - `theme`        – colour / style constants used across renderers

pub mod input_prompt;
pub mod picklist;
pub mod theme;
