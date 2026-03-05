//! Text input prompt with optional type-to-filter (autocomplete) behaviour.
//!
//! Two modes:
//!   1. **Plain input** – used for `Flag` (y/n) and `Input` nodes.
//!      Reads a single line from the user with optional masking (sensitive).
//!
//!   2. **Filter input** – used for `Select` nodes rendered as `input`.
//!      Shows a list of options that narrows as the user types.
//!
//! Writes to stderr; returns the chosen value as a `String`.

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
use crate::node::MAX_INPUT_BYTES;

// ── Plain line input ──────────────────────────────────────────────────────────

/// Prompt the user for a single line of text.
///
/// * `message`       – text shown before the input field.
/// * `default`       – pre-filled / suggested value (shown, accepted on Enter).
/// * `sensitive`     – if true, input characters are not echoed.
/// * `timeout_secs`  – maximum wait time.
pub fn read_line(
    message: &str,
    default: Option<&str>,
    sensitive: bool,
    timeout_secs: u64,
) -> anyhow::Result<String> {
    let mut stderr = io::stderr();
    terminal::enable_raw_mode()?;
    let result = read_line_inner(&mut stderr, message, default, sensitive, timeout_secs);
    terminal::disable_raw_mode()?;
    execute!(stderr, cursor::Show)?;
    result
}

fn read_line_inner(
    out: &mut impl Write,
    message: &str,
    default: Option<&str>,
    sensitive: bool,
    timeout_secs: u64,
) -> anyhow::Result<String> {
    let mut buffer = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    // Print prompt.
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
    )?;
    if let Some(d) = default {
        queue!(
            out,
            style::PrintStyledContent(format!(" (default: {d})").with(Theme::DEFAULT_COLOR))
        )?;
    }
    queue!(out, style::Print(": "))?;
    out.flush()?;

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            queue!(out, style::Print("\r\n"))?;
            out.flush()?;
            return Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if !event::poll(remaining)? {
            queue!(out, style::Print("\r\n"))?;
            out.flush()?;
            return Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                    queue!(out, style::Print("\r\n"))?;
                    out.flush()?;
                    return Err(anyhow::anyhow!("Prompt cancelled by user."));
                }

                (KeyCode::Enter, _) => {
                    queue!(out, style::Print("\r\n"))?;
                    out.flush()?;
                    if buffer.is_empty() {
                        return Ok(default.unwrap_or("").to_owned());
                    }
                    return Ok(buffer);
                }

                (KeyCode::Backspace, _) => {
                    if !buffer.is_empty() {
                        buffer.pop();
                        // Move cursor back and clear character.
                        queue!(
                            out,
                            cursor::MoveLeft(1),
                            terminal::Clear(ClearType::UntilNewLine)
                        )?;
                        out.flush()?;
                    }
                }

                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    let new_len = buffer.len() + c.len_utf8();
                    if new_len > MAX_INPUT_BYTES {
                        continue; // silently ignore; user sees nothing added
                    }
                    buffer.push(c);
                    if sensitive {
                        queue!(out, style::Print('*'))?;
                    } else {
                        queue!(out, style::Print(c))?;
                    }
                    out.flush()?;
                }

                _ => {}
            }
        }
    }
}

// ── Filter / autocomplete input ───────────────────────────────────────────────

/// Option entry for the filter prompt.
#[derive(Debug, Clone)]
pub struct FilterOption {
    pub label: String,
    pub value: String,
    pub disabled: bool,
}

/// Show a type-to-filter input that narrows a list of options.
///
/// Returns the value of the selected option.
pub fn filter_select(
    message: &str,
    options: &[FilterOption],
    timeout_secs: u64,
) -> anyhow::Result<String> {
    let mut stderr = io::stderr();
    terminal::enable_raw_mode()?;
    let result = filter_select_inner(&mut stderr, message, options, timeout_secs);
    terminal::disable_raw_mode()?;
    execute!(stderr, cursor::Show)?;
    result
}

fn filter_select_inner(
    out: &mut impl Write,
    message: &str,
    options: &[FilterOption],
    timeout_secs: u64,
) -> anyhow::Result<String> {
    let mut query = String::new();
    let mut cursor_pos: usize = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    // How many option lines we rendered last time (for clearing).
    let mut prev_rendered_lines: usize = 0;

    execute!(out, cursor::Hide)?;

    loop {
        // Filter options.
        let filtered: Vec<&FilterOption> = options
            .iter()
            .filter(|o| !o.disabled && o.label.to_lowercase().contains(&query.to_lowercase()))
            .collect();

        // Clamp cursor.
        if cursor_pos >= filtered.len() && !filtered.is_empty() {
            cursor_pos = filtered.len() - 1;
        }

        // Render.
        prev_rendered_lines = render_filter(
            out,
            message,
            &query,
            &filtered,
            cursor_pos,
            prev_rendered_lines,
        )?;

        // Event loop.
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            clear_filter(out, prev_rendered_lines)?;
            return Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if !event::poll(remaining)? {
            clear_filter(out, prev_rendered_lines)?;
            return Err(anyhow::anyhow!(RoptError::PromptTimeout(timeout_secs)));
        }

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Esc, _) => {
                    clear_filter(out, prev_rendered_lines)?;
                    return Err(anyhow::anyhow!("Prompt cancelled by user."));
                }

                (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                    if !filtered.is_empty() {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        } else {
                            cursor_pos = filtered.len() - 1;
                        }
                    }
                }

                (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                    if !filtered.is_empty() {
                        cursor_pos = (cursor_pos + 1) % filtered.len();
                    }
                }

                (KeyCode::Enter, _) => {
                    clear_filter(out, prev_rendered_lines)?;
                    if filtered.is_empty() {
                        return Err(anyhow::anyhow!("No matching options."));
                    }
                    return Ok(filtered[cursor_pos].value.clone());
                }

                (KeyCode::Backspace, _) => {
                    query.pop();
                    cursor_pos = 0;
                }

                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    if query.len() < MAX_INPUT_BYTES {
                        query.push(c);
                        cursor_pos = 0;
                    }
                }

                _ => {}
            }
        }
    }
}

fn render_filter(
    out: &mut impl Write,
    message: &str,
    query: &str,
    filtered: &[&FilterOption],
    cursor_pos: usize,
    prev_lines: usize,
) -> anyhow::Result<usize> {
    // Clear previous render.
    for _ in 0..prev_lines {
        queue!(
            out,
            terminal::Clear(ClearType::CurrentLine),
            cursor::MoveDown(1),
        )?;
    }
    if prev_lines > 0 {
        queue!(out, cursor::MoveToPreviousLine(prev_lines as u16))?;
    }
    queue!(out, cursor::MoveToColumn(0))?;

    // Prompt line.
    queue!(
        out,
        terminal::Clear(ClearType::CurrentLine),
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
        style::Print(": "),
        style::PrintStyledContent(query.to_string().with(Theme::INPUT_COLOR)),
        style::Print("\r\n"),
    )?;

    let mut lines_rendered = 1usize;

    for (i, option) in filtered.iter().enumerate() {
        let is_cursor = i == cursor_pos;
        let prefix = if is_cursor { Theme::CURSOR } else { " " };
        if is_cursor {
            queue!(
                out,
                style::PrintStyledContent(
                    format!("{} {}\r\n", prefix, option.label)
                        .with(Theme::SELECTED_COLOR)
                        .attribute(Theme::SELECTED_ATTR)
                )
            )?;
        } else {
            queue!(
                out,
                style::Print(format!("{} {}\r\n", prefix, option.label))
            )?;
        }
        lines_rendered += 1;
    }

    if filtered.is_empty() {
        queue!(
            out,
            style::PrintStyledContent("  (no matches)\r\n".to_string().with(Theme::ERROR_COLOR))
        )?;
        lines_rendered += 1;
    }

    // Move back to top.
    queue!(out, cursor::MoveToPreviousLine(lines_rendered as u16))?;

    out.flush()?;
    Ok(lines_rendered)
}

fn clear_filter(out: &mut impl Write, lines: usize) -> anyhow::Result<()> {
    for _ in 0..lines {
        queue!(
            out,
            terminal::Clear(ClearType::CurrentLine),
            cursor::MoveDown(1),
        )?;
    }
    if lines > 0 {
        queue!(out, cursor::MoveToPreviousLine(lines as u16))?;
    }
    queue!(out, terminal::Clear(ClearType::CurrentLine))?;
    out.flush()?;
    Ok(())
}
