//! `ropt show` – display the definition tree for debugging.
//!
//! Outputs either a coloured tree representation or raw JSON.
//! All output goes to stdout.

use std::io::{self, Write};

use crossterm::style::Stylize;

use crate::cli::ShowFormat;
use crate::node::{NodeDef, NodeKind};
use crate::renderer::theme::Theme;
use crate::session;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn show(session_id: &str, format: &ShowFormat) -> anyhow::Result<()> {
    let state = session::read_state(session_id)?;
    let mut stdout = io::stdout();

    match format {
        ShowFormat::Json => {
            let json = serde_json::to_string_pretty(&state.roots)?;
            writeln!(stdout, "{json}")?;
        }
        ShowFormat::Tree => {
            write_tree_header(&mut stdout, session_id, state.depth())?;
            for (idx, root) in state.roots.iter().enumerate() {
                write_node(&mut stdout, root, idx, "", true)?;
            }
        }
    }

    stdout.flush()?;
    Ok(())
}

// ── Tree renderer ─────────────────────────────────────────────────────────────

fn write_tree_header(out: &mut impl Write, session_id: &str, depth: usize) -> anyhow::Result<()> {
    writeln!(
        out,
        "{}",
        format!("Session: {session_id}  (stack depth: {depth})")
            .with(Theme::PROMPT_COLOR)
            .attribute(Theme::PROMPT_ATTR)
    )?;
    Ok(())
}

fn write_node(
    out: &mut impl Write,
    node: &NodeDef,
    index: usize,
    prefix: &str,
    is_last: bool,
) -> anyhow::Result<()> {
    let connector = if is_last { "└── " } else { "├── " };
    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

    let label = format_node_label(node, index);
    writeln!(out, "{prefix}{connector}{label}")?;

    let children = &node.children;
    for (i, child) in children.iter().enumerate() {
        let last = i == children.len() - 1;
        write_node(out, child, i, &child_prefix, last)?;
    }

    Ok(())
}

fn format_node_label(node: &NodeDef, index: usize) -> String {
    let key = node.key_segment(index);
    let kind_str = format!("[{}]", node.kind);

    let mut parts = vec![
        format!(
            "{}",
            kind_str.clone().with(kind_color(&node.kind)).to_string()
        ),
        format!("{}", key.clone().with(Theme::SELECTED_COLOR).to_string()),
    ];

    // Append type-specific detail.
    match node.kind {
        NodeKind::Select => {
            if let Some(ref msg) = node.message {
                parts.push(format!("\"{}\"", msg.clone().with(Theme::INPUT_COLOR)));
            }
            let opt_count = node.total_option_count();
            parts.push(format!("({opt_count} options)"));
        }
        NodeKind::Option => {
            if let Some(ref v) = node.value {
                parts.push(format!("value={}", v.clone().with(Theme::INPUT_COLOR)));
            }
            if node.disabled {
                parts.push(Theme::DISABLED_MARK.with(Theme::DISABLED_COLOR).to_string());
            }
            if node.default_selected {
                parts.push(Theme::DEFAULT_MARK.with(Theme::DEFAULT_COLOR).to_string());
            }
        }
        NodeKind::Flag => {
            if let Some(s) = node.short {
                parts.push(format!("(-{s})"));
            }
        }
        NodeKind::Input => {
            if let Some(ref t) = node.input_type {
                parts.push(format!("type={t:?}"));
            }
            if let Some(ref d) = node.default_value {
                parts.push(format!("default={d}"));
            }
        }
        _ => {}
    }

    if let Some(ref desc) = node.description {
        parts.push(format!("  # {desc}").with(Theme::DEFAULT_COLOR).to_string());
    }

    parts.join(" ")
}

fn kind_color(kind: &NodeKind) -> crossterm::style::Color {
    match kind {
        NodeKind::Command => crossterm::style::Color::Magenta,
        NodeKind::Argument => crossterm::style::Color::Blue,
        NodeKind::Select => crossterm::style::Color::Cyan,
        NodeKind::Option => crossterm::style::Color::Green,
        NodeKind::Group => crossterm::style::Color::Blue,
        NodeKind::Flag => crossterm::style::Color::Yellow,
        NodeKind::Input => crossterm::style::Color::Yellow,
    }
}
