//! Stack-based definition operations: push, pop, append.
//!
//! Each operation loads the session state, applies a mutation, and writes it
//! back.  Validation (depth limits, parent-child constraints, duplicate names)
//! happens here before the state is modified.

use crate::cli::PushArgs;
use crate::error::RoptError;
use crate::node::{
    InputType, MAX_DEPTH, MAX_GROUPS, MAX_OPTIONS, NodeDef, NodeKind, SelectRender,
    validate_parent_child,
};
use crate::session::{self, state::SessionState};

// ── Public API ────────────────────────────────────────────────────────────────

/// Execute `ropt push <type> [args]`.
///
/// Adds a new node under the current scope and makes it the active scope.
pub fn push(args: &PushArgs) -> anyhow::Result<()> {
    let session_id = session::resolve_session_id(args.session.as_deref())?;
    session::mutate_state(&session_id, |state| {
        let node = build_node(args)?;
        push_node_into_state(state, node)
    })
}

/// Execute `ropt append <type> [args]`.
///
/// Equivalent to `push` followed immediately by `pop`: inserts a node at the
/// current depth without changing the active scope.
pub fn append(args: &PushArgs) -> anyhow::Result<()> {
    let session_id = session::resolve_session_id(args.session.as_deref())?;
    session::mutate_state(&session_id, |state| {
        let node = build_node(args)?;
        push_node_into_state(state, node)?;
        state.pop_node()
    })
}

/// Execute `ropt pop`.
///
/// Closes the current scope and returns to the parent.
pub fn pop(session_id: &str) -> anyhow::Result<()> {
    session::mutate_state(session_id, |state| state.pop_node())
}

// ── Node construction ─────────────────────────────────────────────────────────

/// Build a `NodeDef` from parsed CLI arguments.
fn build_node(args: &PushArgs) -> anyhow::Result<NodeDef> {
    let kind: NodeKind = args.node_type.parse()?;
    let mut node = NodeDef::new(kind.clone());

    node.name = args.name.clone();
    node.description = args.description.clone();

    match kind {
        NodeKind::Select => {
            node.message = args.message.clone();
            node.multiple = args.multiple;
            if let Some(ref r) = args.render {
                node.render = Some(r.parse::<SelectRender>()?);
            }
        }

        NodeKind::Option => {
            node.value = args.value.clone();
            node.label = args.label.clone();
            node.default_selected = args.default;
            node.disabled = args.disabled;
            if let Some(ref dv) = args.default_value {
                node.default_value = Some(dv.clone());
            }
        }

        NodeKind::Group => {
            node.label = args.label.clone();
        }

        NodeKind::Flag => {
            node.short = args.short;
        }

        NodeKind::Input => {
            if let Some(ref t) = args.input_type {
                node.input_type = Some(t.parse::<InputType>()?);
            }
            node.validate_regex = args.validate_regex.clone();
            node.validate_min = args.validate_min;
            node.validate_max = args.validate_max;
            node.default_value = args.default_value.clone();
            node.sensitive = args.sensitive;
        }

        NodeKind::Command | NodeKind::Argument => {
            // Name is the primary meaningful field; already set above.
        }
    }

    Ok(node)
}

// ── State mutation ────────────────────────────────────────────────────────────

/// Validate and insert a node into the state, then push it as the new scope.
fn push_node_into_state(state: &mut SessionState, node: NodeDef) -> anyhow::Result<()> {
    // Depth limit check.
    if state.depth() >= MAX_DEPTH {
        anyhow::bail!(RoptError::MaxDepthExceeded(MAX_DEPTH));
    }

    // Duplicate name check: named nodes must be unique within their scope.
    if let Some(ref name) = node.name
        && state.name_exists_in_current_scope(name)
    {
        anyhow::bail!(RoptError::DuplicateName(name.clone()));
    }

    // Parent-child compatibility check.
    if let Some(parent) = state.current_node() {
        validate_parent_child(&parent.kind.clone(), &node.kind)?;

        // Option/group count limits within a select parent.
        if parent.kind == NodeKind::Select {
            match node.kind {
                NodeKind::Option => {
                    let current = parent.total_option_count();
                    if current >= MAX_OPTIONS {
                        anyhow::bail!(RoptError::MaxOptionsExceeded(MAX_OPTIONS));
                    }
                }
                NodeKind::Group => {
                    let current = parent.group_count();
                    if current >= MAX_GROUPS {
                        anyhow::bail!(RoptError::MaxGroupsExceeded(MAX_GROUPS));
                    }
                }
                _ => {}
            }
        }
    }

    state.push_node(node);
    Ok(())
}
