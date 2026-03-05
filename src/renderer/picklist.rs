//! Keyboard-navigable picklist renderer.
//!
//! Displays a list of items that the user navigates with ↑/↓ arrow keys and
//! selects with Enter.  Supports:
//!   - Single and multiple selection modes.
//!   - Disabled items (skipped during navigation).
//!   - Group headers (not selectable).
//!   - Coloured output via `crossterm`.
//!
//! The renderer writes to stderr so that stdout remains clean for machine-
//! readable output.

use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{self, Stylize},
    terminal::{self, ClearType},
};

use super::theme::Theme;
use crate::error::RoptError;

// ── Public entry points ───────────────────────────────────────────────────────

/// A flat item shown in the picklist.
#[derive(Debug, Clone)]
pub struct PickItem {
    /// Display label.
    pub label: String,
    /// Underlying value returned on selection.
    pub value: String,
    /// Non-selectable decorative header (group label).
    pub is_group_header: bool,
    /// Shown but not selectable.
    pub disabled: bool,
    /// Pre-selected in multi-select mode.
    pub preselected: bool,
}

impl PickItem {
    pub fn option(label: impl Into<String>, value: impl Into<String>) -> Self {
        PickItem {
            label: label.into(),
            value: value.into(),
            is_group_header: false,
            disabled: false,
            preselected: false,
        }
    }

    pub fn group_header(label: impl Into<String>) -> Self {
        PickItem {
            label: label.into(),
            value: String::new(),
            is_group_header: true,
            disabled: false,
            preselected: false,
        }
    }
}

/// Run the picklist and return the selected value(s).
///
/// `multiple` – if true, Space toggles selection and Enter confirms; if false,
/// Enter immediately selects the highlighted item.
pub fn run(
    message: &str,
    items: &[PickItem],
    multiple: bool,
    timeout_secs: u64,
) -> anyhow::Result<Vec<String>> {
    if !crossterm::tty::IsTty::is_tty(&io::stderr()) {
        anyhow::bail!(RoptError::NotATty);
    }

    let mut stderr = io::stderr();
    terminal::enable_raw_mode()?;

    let result = run_inner(&mut stderr, message, items, multiple, timeout_secs);

    terminal::disable_raw_mode()?;
    // Ensure cursor is shown even on error.
    execute!(stderr, cursor::Show)?;

    result
}

// ── Internal implementation ───────────────────────────────────────────────────

fn run_inner(
    out: &mut impl Write,
    message: &str,
    items: &[PickItem],
    multiple: bool,
    timeout_secs: u64,
) -> anyhow::Result<Vec<String>> {
    // Selectable indices (not group headers, not disabled).
    let selectable: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.is_group_header && !item.disabled)
        .map(|(i, _)| i)
        .collect();

    if selectable.is_empty() {
        anyhow::bail!("No selectable options available.");
    }

    // Cursor within `selectable` (not raw item index).
    let mut cursor_pos: usize = 0;

    // For multi-select: which raw indices are ticked.
    let mut selected: std::collections::HashSet<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.preselected)
        .map(|(i, _)| i)
        .collect();

    execute!(out, cursor::Hide)?;

    // Save the cursor position once before the first render.  Every subsequent
    // render and the final clear both restore to this position.
    queue!(out, cursor::SavePosition)?;
    out.flush()?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    loop {
        render_picklist(
            out,
            message,
            items,
            &selectable,
            cursor_pos,
            &selected,
            multiple,
        )?;

        // Wait for an event with timeout.
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if !event::poll(remaining)? {
            break Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                // Quit on Ctrl-C / Escape.
                (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                    clear_picklist(out)?;
                    break Err(anyhow::anyhow!("Prompt cancelled by user."));
                }

                // Move up.
                (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                    if cursor_pos > 0 {
                        cursor_pos -= 1;
                    } else {
                        cursor_pos = selectable.len().saturating_sub(1);
                    }
                }

                // Move down.
                (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                    cursor_pos = (cursor_pos + 1) % selectable.len();
                }

                // Space: toggle in multi-select mode.
                (KeyCode::Char(' '), KeyModifiers::NONE) if multiple => {
                    let raw_idx = selectable[cursor_pos];
                    if selected.contains(&raw_idx) {
                        selected.remove(&raw_idx);
                    } else {
                        selected.insert(raw_idx);
                    }
                }

                // Enter: confirm selection.
                (KeyCode::Enter, _) => {
                    clear_picklist(out)?;
                    if multiple {
                        let values: Vec<String> = selectable
                            .iter()
                            .filter(|&&i| selected.contains(&i))
                            .map(|&i| items[i].value.clone())
                            .collect();
                        break Ok(values);
                    } else {
                        let raw_idx = selectable[cursor_pos];
                        break Ok(vec![items[raw_idx].value.clone()]);
                    }
                }

                _ => {}
            }
        }
    }
}

fn render_picklist(
    out: &mut impl Write,
    message: &str,
    items: &[PickItem],
    selectable: &[usize],
    cursor_sel_pos: usize,
    selected: &std::collections::HashSet<usize>,
    multiple: bool,
) -> anyhow::Result<()> {
    let cursor_raw = selectable.get(cursor_sel_pos).copied();

    // Jump back to the saved position so we overwrite the previous frame.
    queue!(out, cursor::RestorePosition)?;

    // Print prompt message.
    queue!(
        out,
        style::PrintStyledContent(
            format!("{} ", Theme::PROMPT_MARK)
                .with(Theme::PROMPT_COLOR)
                .attribute(Theme::PROMPT_ATTR)
        ),
        style::PrintStyledContent(
            message
                .to_string()
                .with(Theme::PROMPT_COLOR)
                .attribute(Theme::PROMPT_ATTR)
        ),
        style::Print("\r\n"),
    )?;

    if multiple {
        queue!(
            out,
            style::PrintStyledContent(
                "  (Space to toggle, Enter to confirm)\r\n"
                    .to_string()
                    .with(Theme::DEFAULT_COLOR)
            )
        )?;
    }

    for (raw_idx, item) in items.iter().enumerate() {
        let is_cursor = Some(raw_idx) == cursor_raw;

        if item.is_group_header {
            queue!(
                out,
                style::PrintStyledContent(
                    format!("  {}\r\n", item.label)
                        .with(Theme::GROUP_COLOR)
                        .attribute(Theme::GROUP_ATTR)
                )
            )?;
            continue;
        }

        // Cursor indicator.
        let prefix = if is_cursor { Theme::CURSOR } else { " " };

        // Checkbox for multi-select.
        let checkbox = if multiple {
            if selected.contains(&raw_idx) {
                "[x] "
            } else {
                "[ ] "
            }
        } else {
            ""
        };

        if item.disabled {
            queue!(
                out,
                style::PrintStyledContent(
                    format!(
                        "{} {}{} {}\r\n",
                        prefix,
                        checkbox,
                        item.label,
                        Theme::DISABLED_MARK
                    )
                    .with(Theme::DISABLED_COLOR)
                )
            )?;
        } else if is_cursor {
            queue!(
                out,
                style::PrintStyledContent(
                    format!("{} {}{}\r\n", prefix, checkbox, item.label)
                        .with(Theme::SELECTED_COLOR)
                        .attribute(Theme::SELECTED_ATTR)
                )
            )?;
        } else {
            queue!(
                out,
                style::Print(format!("{} {}{}\r\n", prefix, checkbox, item.label))
            )?;
        }
    }

    out.flush()?;

    Ok(())
}

fn clear_picklist(out: &mut impl Write) -> anyhow::Result<()> {
    queue!(
        out,
        cursor::RestorePosition,
        terminal::Clear(ClearType::FromCursorDown),
    )?;
    out.flush()?;
    Ok(())
}
