//! Result output formatters.
//!
//! After `ropt execute` stores results in the session, the same execute
//! command (or a later `ropt read`) retrieves them and formats the output.
//!
//! Supported formats:
//!   - **json** – structured JSON object, suitable for `jq` processing.
//!   - **sh**   – shell-sourceable `KEY=VALUE` pairs, safe for `eval`.
//!   - **raw**  – one `value` per line (no keys), useful for piping.
//!
//! Shell variable name escaping:
//!   Dot-separated keys are converted to `snake_case` variable names by
//!   replacing `.` and `-` with `_`.  Values are single-quoted to prevent
//!   injection.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::cli::OutputFormat;
use crate::session::state::ResultValue;

/// Write formatted results to stdout.
pub fn write_results(
    results: &HashMap<String, ResultValue>,
    format: &OutputFormat,
    prefix: &str,
) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    match format {
        OutputFormat::Json => write_json(&mut stdout, results)?,
        OutputFormat::Sh => write_sh(&mut stdout, results, prefix)?,
        OutputFormat::Raw => write_raw(&mut stdout, results)?,
    }
    stdout.flush()?;
    Ok(())
}

// ── JSON ──────────────────────────────────────────────────────────────────────

fn write_json(out: &mut impl Write, results: &HashMap<String, ResultValue>) -> anyhow::Result<()> {
    // Build a nested JSON object from dot-separated keys.
    let mut root = serde_json::Map::new();

    for (key, value) in results {
        let segments: Vec<&str> = key.split('.').collect();
        insert_nested(&mut root, &segments, value);
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))?;
    writeln!(out, "{json}")?;
    Ok(())
}

/// Recursively insert a value at a nested path in a JSON map.
fn insert_nested(
    map: &mut serde_json::Map<String, serde_json::Value>,
    segments: &[&str],
    value: &ResultValue,
) {
    if segments.is_empty() {
        return;
    }

    let key = segments[0].to_owned();

    if segments.len() == 1 {
        map.insert(key, result_to_json(value));
        return;
    }

    let child = map
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(child_map) = child {
        insert_nested(child_map, &segments[1..], value);
    }
}

fn result_to_json(value: &ResultValue) -> serde_json::Value {
    match value {
        ResultValue::Single(s) => serde_json::Value::String(s.clone()),
        ResultValue::Bool(b) => serde_json::Value::Bool(*b),
        ResultValue::Multiple(vs) => serde_json::Value::Array(
            vs.iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        ),
    }
}

// ── Shell ─────────────────────────────────────────────────────────────────────

fn write_sh(
    out: &mut impl Write,
    results: &HashMap<String, ResultValue>,
    prefix: &str,
) -> anyhow::Result<()> {
    // Sort for deterministic output.
    let mut pairs: Vec<_> = results.iter().collect();
    pairs.sort_by_key(|(k, _)| k.as_str());

    for (key, value) in pairs {
        let var_name = format!("{}{}", prefix, key_to_sh_var(key));
        match value {
            ResultValue::Single(s) => {
                writeln!(out, "{}={}", var_name, sh_quote(s))?;
            }
            ResultValue::Bool(b) => {
                writeln!(out, "{}={}", var_name, if *b { "true" } else { "false" })?;
            }
            ResultValue::Multiple(vs) => {
                // Bash array syntax.
                let elements: Vec<String> = vs.iter().map(|s| sh_quote(s)).collect();
                writeln!(out, "{}=({})", var_name, elements.join(" "))?;
            }
        }
    }
    Ok(())
}

/// Convert a dot-separated key to a valid shell variable name.
///
/// Rules:
/// - Non-alphanumeric, non-underscore characters (including `.` and `-`) are
///   replaced with `_`.
/// - If the result starts with a digit (e.g. an unnamed node at index 0
///   produces key `"0"`), a leading underscore is prepended so the name is
///   always a legal bash identifier.
fn key_to_sh_var(key: &str) -> String {
    let sanitised: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase();

    // Bash identifiers must not start with a digit.
    if sanitised.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{sanitised}")
    } else {
        sanitised
    }
}

/// Single-quote a string for safe shell embedding.
/// Single quotes in the value are escaped as `'\''`.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ── Raw ───────────────────────────────────────────────────────────────────────

fn write_raw(out: &mut impl Write, results: &HashMap<String, ResultValue>) -> anyhow::Result<()> {
    let mut pairs: Vec<_> = results.iter().collect();
    pairs.sort_by_key(|(k, _)| k.as_str());

    for (_, value) in pairs {
        writeln!(out, "{value}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_var_replaces_dots_and_dashes() {
        assert_eq!(key_to_sh_var("build.output-dir"), "build_output_dir");
    }

    #[test]
    fn sh_var_prepends_underscore_for_digit_start() {
        assert_eq!(key_to_sh_var("0"), "_0");
        assert_eq!(key_to_sh_var("1"), "_1");
        assert_eq!(key_to_sh_var("42"), "_42");
    }

    #[test]
    fn sh_var_does_not_prepend_underscore_for_letter_start() {
        assert_eq!(key_to_sh_var("action"), "action");
        assert_eq!(key_to_sh_var("_private"), "_private");
    }

    #[test]
    fn sh_quote_escapes_single_quotes() {
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }
}
