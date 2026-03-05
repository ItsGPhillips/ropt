//! Execution engine – drives interactive prompts and records results.
//!
//! `execute` walks the definition tree depth-first, presenting the appropriate
//! prompt for each leaf node and storing results in the session state.
//!
//! Result key paths are built by concatenating node key segments with `.`.
//! Example: a `Command("build")` containing a `Select` at index 0 produces
//! the key `"build.0"` (or `"build.<name>"` if the select has a name).
//!
//! Design:
//! - The engine is pure logic; all terminal I/O is delegated to the
//!   `renderer` sub-modules.
//! - Timeout is read from the `ROPT_TIMEOUT` env var, defaulting to 60s.
//! - Results are written back to the session file after all prompts complete.

use crate::error::RoptError;
use crate::node::{InputType, NodeDef, NodeKind, SelectRender};
use crate::renderer::{
    input_prompt::{self, FilterOption},
    picklist::{self, PickItem},
};
use crate::session::{self, state::ResultValue};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const TIMEOUT_ENV_VAR: &str = "ROPT_TIMEOUT";

// ── Public entry point ────────────────────────────────────────────────────────

/// Execute all interactive prompts for `session_id` and store results.
pub fn execute(session_id: &str) -> anyhow::Result<()> {
    let timeout = read_timeout();

    // Read the current state (read-only for the prompt phase).
    let state = session::read_state(session_id)?;

    // Build results by walking the tree.
    let mut results = std::collections::HashMap::new();
    for (idx, root) in state.roots.iter().enumerate() {
        let path = root.key_segment(idx);
        collect_results(root, &path, &mut results, timeout)?;
    }

    // Write results back.
    session::mutate_state(session_id, |s| {
        s.results = results;
        Ok(())
    })
}

// ── Tree walker ───────────────────────────────────────────────────────────────

fn collect_results(
    node: &NodeDef,
    path: &str,
    results: &mut std::collections::HashMap<String, ResultValue>,
    timeout: u64,
) -> anyhow::Result<()> {
    match node.kind {
        // Container nodes: recurse into children.
        NodeKind::Command | NodeKind::Argument | NodeKind::Group => {
            for (idx, child) in node.children.iter().enumerate() {
                let child_path = format!("{}.{}", path, child.key_segment(idx));
                collect_results(child, &child_path, results, timeout)?;
            }
        }

        // Leaf nodes that produce interactive prompts.
        NodeKind::Select => {
            let value = prompt_select(node, timeout)?;
            results.insert(path.to_owned(), value);
        }

        NodeKind::Flag => {
            let value = prompt_flag(node, timeout)?;
            results.insert(path.to_owned(), value);
        }

        NodeKind::Input => {
            let value = prompt_input(node, timeout)?;
            results.insert(path.to_owned(), value);
        }

        // Option nodes are consumed by their parent Select prompt.
        NodeKind::Option => {}
    }

    Ok(())
}

// ── Prompt implementations ────────────────────────────────────────────────────

fn prompt_select(node: &NodeDef, timeout: u64) -> anyhow::Result<ResultValue> {
    let message = node
        .message
        .as_deref()
        .or(node.description.as_deref())
        .unwrap_or("Choose an option");

    // Flatten children into a list of PickItems (with group headers).
    let (items, filter_opts) = flatten_select_options(node);

    let total_selectable = items
        .iter()
        .filter(|i| !i.is_group_header && !i.disabled)
        .count();

    // Determine render mode.
    let render = node.render.clone().unwrap_or(SelectRender::Auto);
    let use_filter = match render {
        SelectRender::Picklist => false,
        SelectRender::Input => true,
        SelectRender::Auto => total_selectable >= 5,
    };

    let chosen: Vec<String> = if use_filter {
        let value = input_prompt::filter_select(message, &filter_opts, timeout)?;
        vec![value]
    } else {
        picklist::run(message, &items, node.multiple, timeout)?
    };

    if node.multiple {
        Ok(ResultValue::Multiple(chosen))
    } else {
        Ok(ResultValue::Single(
            chosen.into_iter().next().unwrap_or_default(),
        ))
    }
}

fn flatten_select_options(node: &NodeDef) -> (Vec<PickItem>, Vec<FilterOption>) {
    let mut pick_items = Vec::new();
    let mut filter_opts = Vec::new();

    for child in &node.children {
        match child.kind {
            NodeKind::Group => {
                let header_label = child
                    .label
                    .clone()
                    .or(child.name.clone())
                    .unwrap_or_else(|| "Group".to_owned());
                pick_items.push(PickItem::group_header(&header_label));

                for opt in &child.children {
                    if opt.kind == NodeKind::Option {
                        push_option(&mut pick_items, &mut filter_opts, opt);
                    }
                }
            }
            NodeKind::Option => {
                push_option(&mut pick_items, &mut filter_opts, child);
            }
            _ => {}
        }
    }

    (pick_items, filter_opts)
}

fn push_option(pick_items: &mut Vec<PickItem>, filter_opts: &mut Vec<FilterOption>, opt: &NodeDef) {
    let value = opt.value.clone().unwrap_or_default();
    let label = opt
        .label
        .clone()
        .or(opt.name.clone())
        .unwrap_or_else(|| value.clone());

    let mut pick = PickItem::option(&label, &value);
    pick.disabled = opt.disabled;
    pick.preselected = opt.default_selected;
    pick_items.push(pick);

    if !opt.disabled {
        filter_opts.push(FilterOption {
            label: label.clone(),
            value: value.clone(),
            disabled: false,
        });
    }
}

fn prompt_flag(node: &NodeDef, timeout: u64) -> anyhow::Result<ResultValue> {
    let name = node.display_name();
    let message = node.description.as_deref().unwrap_or(&name).to_owned();

    let prompt = format!("{} (y/n)", message);
    loop {
        let answer = input_prompt::read_line(&prompt, Some("n"), false, timeout)?;
        match answer.trim().to_lowercase().as_str() {
            "y" | "yes" | "true" | "1" => return Ok(ResultValue::Bool(true)),
            "n" | "no" | "false" | "0" | "" => return Ok(ResultValue::Bool(false)),
            _ => {
                eprintln!("Please enter y or n.");
            }
        }
    }
}

fn prompt_input(node: &NodeDef, timeout: u64) -> anyhow::Result<ResultValue> {
    let name = node.display_name();
    let description = node.description.as_deref().unwrap_or(&name);

    loop {
        let raw = input_prompt::read_line(
            description,
            node.default_value.as_deref(),
            node.sensitive,
            timeout,
        )?;

        // Validate the input.
        match validate_input(&raw, node) {
            Ok(()) => return Ok(ResultValue::Single(raw)),
            Err(e) => {
                eprintln!("Validation error: {e}  Please try again.");
            }
        }
    }
}

// ── Input validation ──────────────────────────────────────────────────────────

fn validate_input(value: &str, node: &NodeDef) -> anyhow::Result<()> {
    // Absolute length guard (applies to all types).
    if value.len() > crate::node::MAX_INPUT_BYTES {
        anyhow::bail!(RoptError::InputTooLong(crate::node::MAX_INPUT_BYTES));
    }

    // Type-specific validation.
    match node.input_type.as_ref().unwrap_or(&InputType::String) {
        InputType::Number => {
            // Parse and check numeric bounds; do NOT also apply string-length
            // bounds below — validate_min/max mean numeric range here.
            let n: f64 = value.parse().map_err(|_| {
                anyhow::anyhow!(RoptError::ValidationError(format!(
                    "'{value}' is not a valid number"
                )))
            })?;

            if let Some(min) = node.validate_min
                && n < min
            {
                anyhow::bail!(RoptError::ValidationError(format!(
                    "Value {n} is below minimum {min}"
                )));
            }
            if let Some(max) = node.validate_max
                && n > max
            {
                anyhow::bail!(RoptError::ValidationError(format!(
                    "Value {n} exceeds maximum {max}"
                )));
            }

            // Skip the generic string-length bounds for Number nodes.
            return validate_custom_regex(value, node);
        }

        InputType::Email => {
            // Heuristic: must have an `@` with non-empty text on both sides.
            if !value.contains('@') || value.starts_with('@') || value.ends_with('@') {
                anyhow::bail!(RoptError::ValidationError(format!(
                    "'{value}' is not a valid email address"
                )));
            }
        }

        InputType::Path => {
            // Accept any non-empty string; existence is not verified here.
            if value.is_empty() {
                anyhow::bail!(RoptError::ValidationError("Path cannot be empty".into()));
            }
        }

        InputType::Regex { pattern } => {
            // Compile and match the full pattern against the input.
            let re = regex::Regex::new(pattern).map_err(|e| {
                anyhow::anyhow!(RoptError::ValidationError(format!(
                    "Invalid regex pattern '{pattern}': {e}"
                )))
            })?;
            if !re.is_match(value) {
                anyhow::bail!(RoptError::ValidationError(format!(
                    "'{value}' does not match required pattern '{pattern}'"
                )));
            }
            // Regex nodes do not apply string length bounds.
            return Ok(());
        }

        InputType::String => {
            // String nodes apply length bounds below.
        }
    }

    // String-length bounds (String, Email, Path types only — Number and Regex
    // handle their own bounds above and return early).
    if let Some(min) = node.validate_min
        && (value.len() as f64) < min
    {
        anyhow::bail!(RoptError::ValidationError(format!(
            "Input is shorter than minimum length {min}"
        )));
    }
    if let Some(max) = node.validate_max
        && (value.len() as f64) > max
    {
        anyhow::bail!(RoptError::ValidationError(format!(
            "Input exceeds maximum length {max}"
        )));
    }

    validate_custom_regex(value, node)
}

/// Apply the `--validate-regex` custom regex (distinct from `InputType::Regex`).
/// This can be combined with any input type for additional validation.
fn validate_custom_regex(value: &str, node: &NodeDef) -> anyhow::Result<()> {
    if let Some(ref pattern) = node.validate_regex {
        let re = regex::Regex::new(pattern).map_err(|e| {
            anyhow::anyhow!(RoptError::ValidationError(format!(
                "Invalid validate-regex '{pattern}': {e}"
            )))
        })?;
        if !re.is_match(value) {
            anyhow::bail!(RoptError::ValidationError(format!(
                "'{value}' does not match required pattern '{pattern}'"
            )));
        }
    }
    Ok(())
}

// ── Timeout helper ────────────────────────────────────────────────────────────

fn read_timeout() -> u64 {
    std::env::var(TIMEOUT_ENV_VAR)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}
