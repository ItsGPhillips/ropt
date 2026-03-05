//! Node tree – the data model that describes a session's interactive structure.
//!
//! Each call to `ropt push` / `ropt append` adds a `NodeDef` into the tree.
//! The tree is serialised to the session state file and later interpreted by
//! the execution engine to drive interactive prompts.
//!
//! Design principles
//! -----------------
//! * Plain data types with full serde support so the whole tree round-trips
//!   through JSON without loss.
//! * No behaviour here – presentation and execution logic live in their own
//!   modules.  `NodeDef` is a dumb record.
//! * Hierarchical: children are owned `Vec<NodeDef>`, giving a proper tree
//!   rather than a flat list with parent pointers.

use serde::{Deserialize, Serialize};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const MAX_DEPTH: usize = 10;
pub const MAX_OPTIONS: usize = 1000;
pub const MAX_GROUPS: usize = 100;
pub const MAX_INPUT_BYTES: usize = 10 * 1024; // 10 KB

// ── Node type discriminant ────────────────────────────────────────────────────

/// Every node in the definition tree has one of these types.  The type
/// determines which fields are relevant and how execution handles the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Top-level container for a sub-command.
    Command,
    /// Container grouping related nodes under a named argument.
    Argument,
    /// Interactive selection prompt (contains `Option` / `Group` children).
    Select,
    /// A selectable choice inside a `Select` or `Group`.
    Option,
    /// A visual grouping of `Option` nodes inside a `Select`.
    Group,
    /// Boolean yes/no prompt.
    Flag,
    /// Free-text input prompt.
    Input,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            NodeKind::Command => "command",
            NodeKind::Argument => "argument",
            NodeKind::Select => "select",
            NodeKind::Option => "option",
            NodeKind::Group => "group",
            NodeKind::Flag => "flag",
            NodeKind::Input => "input",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for NodeKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "command" => Ok(NodeKind::Command),
            "argument" => Ok(NodeKind::Argument),
            "select" => Ok(NodeKind::Select),
            "option" => Ok(NodeKind::Option),
            "group" => Ok(NodeKind::Group),
            "flag" => Ok(NodeKind::Flag),
            "input" => Ok(NodeKind::Input),
            other => Err(anyhow::anyhow!("Unknown node type: '{other}'")),
        }
    }
}

// ── Select rendering hint ─────────────────────────────────────────────────────

/// How a `Select` node should be rendered at execution time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectRender {
    /// Choose automatically: < 5 options → picklist; >= 5 → input with filter.
    #[default]
    Auto,
    /// Keyboard-navigable list (arrow keys + Enter).
    Picklist,
    /// Type-to-filter input with autocomplete.
    Input,
}

impl std::str::FromStr for SelectRender {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "auto" => Ok(SelectRender::Auto),
            "picklist" => Ok(SelectRender::Picklist),
            "input" => Ok(SelectRender::Input),
            other => Err(anyhow::anyhow!("Unknown render mode: '{other}'")),
        }
    }
}

// ── Input type ────────────────────────────────────────────────────────────────

/// Validation constraint for an `Input` node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InputType {
    #[default]
    String,
    Number,
    Email,
    Path,
    /// Regex pattern stored as a plain string (compiled at execution time).
    Regex {
        pattern: String,
    },
}

impl std::str::FromStr for InputType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(pattern) = s.strip_prefix("regex:") {
            return Ok(InputType::Regex {
                pattern: pattern.to_owned(),
            });
        }
        match s {
            "string" => Ok(InputType::String),
            "number" => Ok(InputType::Number),
            "email" => Ok(InputType::Email),
            "path" => Ok(InputType::Path),
            other => Err(anyhow::anyhow!("Unknown input type: '{other}'")),
        }
    }
}

// ── Node definition ───────────────────────────────────────────────────────────

/// A single node in the definition tree.
///
/// The `kind` field acts as the discriminant.  Not all fields are meaningful
/// for all kinds; irrelevant fields are `None` / default.  This intentional
/// flat-ish structure (rather than an enum-of-structs) keeps JSON
/// serialisation straightforward and avoids verbose pattern matching in places
/// that only care about shared fields like `name` or `description`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    /// Logical type of this node.
    pub kind: NodeKind,

    /// Optional name used as a path segment in result keys.
    /// If absent the node's zero-based index among siblings is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description / label shown to the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // ── Select fields ─────────────────────────────────────────────────────────
    /// Prompt message shown above the option list (Select).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Rendering hint (Select).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render: Option<SelectRender>,

    /// Whether multiple selections are allowed (Select).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub multiple: bool,

    // ── Option fields ─────────────────────────────────────────────────────────
    /// Machine-readable value returned when this option is selected (Option).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Display label (Option / Group).  Falls back to `value` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Whether this option is pre-selected (Option).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default_selected: bool,

    /// Whether this option is visible but not selectable (Option).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,

    // ── Flag fields ───────────────────────────────────────────────────────────
    /// Single-character short form (Flag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,

    // ── Input fields ──────────────────────────────────────────────────────────
    /// Type constraint / validation (Input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<InputType>,

    /// Custom validation regex (Input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_regex: Option<String>,

    /// Minimum length or numeric value (Input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_min: Option<f64>,

    /// Maximum length or numeric value (Input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_max: Option<f64>,

    /// Default value when the user presses Enter without typing (Input / Option).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,

    /// Treat input as sensitive: disable history and echo (Input).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sensitive: bool,

    // ── Shared ────────────────────────────────────────────────────────────────
    /// Child nodes (Select → Options/Groups; Command/Argument → anything).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeDef>,
}

impl NodeDef {
    /// Create a minimal node with just a kind.
    pub fn new(kind: NodeKind) -> Self {
        NodeDef {
            kind,
            name: None,
            description: None,
            message: None,
            render: None,
            multiple: false,
            value: None,
            label: None,
            default_selected: false,
            disabled: false,
            short: None,
            input_type: None,
            validate_regex: None,
            validate_min: None,
            validate_max: None,
            default_value: None,
            sensitive: false,
            children: Vec::new(),
        }
    }

    /// The display name: `name`, then `label`, then `value`, then kind string.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.label.clone())
            .or_else(|| self.value.clone())
            .unwrap_or_else(|| self.kind.to_string())
    }

    /// Resolve the path key segment for this node given its index among siblings.
    pub fn key_segment(&self, index: usize) -> String {
        self.name.clone().unwrap_or_else(|| index.to_string())
    }

    /// Count `Option` children (direct only, not inside groups).
    pub fn direct_option_count(&self) -> usize {
        self.children
            .iter()
            .filter(|c| c.kind == NodeKind::Option)
            .count()
    }

    /// Count `Group` children.
    pub fn group_count(&self) -> usize {
        self.children
            .iter()
            .filter(|c| c.kind == NodeKind::Group)
            .count()
    }

    /// Total selectable options (direct + inside groups).
    pub fn total_option_count(&self) -> usize {
        self.children
            .iter()
            .map(|c| match c.kind {
                NodeKind::Option => 1,
                NodeKind::Group => c.direct_option_count(),
                _ => 0,
            })
            .sum()
    }
}

// ── Validation helpers ────────────────────────────────────────────────────────

/// Validate that a child `kind` is legal under a given parent `kind`.
pub fn validate_parent_child(parent: &NodeKind, child: &NodeKind) -> anyhow::Result<()> {
    use NodeKind::*;

    let valid = match parent {
        Command => matches!(child, Argument | Select | Flag | Input | Group),
        Argument => matches!(child, Select | Flag | Input),
        Select => matches!(child, Option | Group),
        Group => matches!(child, Option),
        // Flag and Input and Option are leaf nodes
        Flag | Input | Option => false,
    };

    if !valid {
        anyhow::bail!(crate::error::RoptError::InvalidNodeContext(format!(
            "'{child}' cannot be a child of '{parent}'"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── NodeKind::from_str ────────────────────────────────────────────────────

    #[test]
    fn node_kind_parses_all_valid_variants() {
        let cases = [
            ("command", NodeKind::Command),
            ("argument", NodeKind::Argument),
            ("select", NodeKind::Select),
            ("option", NodeKind::Option),
            ("group", NodeKind::Group),
            ("flag", NodeKind::Flag),
            ("input", NodeKind::Input),
        ];
        for (s, expected) in cases {
            assert_eq!(s.parse::<NodeKind>().unwrap(), expected, "parsing '{s}'");
        }
    }

    #[test]
    fn node_kind_rejects_unknown_string() {
        assert!("widget".parse::<NodeKind>().is_err());
    }

    #[test]
    fn node_kind_display_round_trips() {
        // Every variant's Display output should re-parse to itself.
        let variants = [
            NodeKind::Command,
            NodeKind::Argument,
            NodeKind::Select,
            NodeKind::Option,
            NodeKind::Group,
            NodeKind::Flag,
            NodeKind::Input,
        ];
        for kind in variants {
            let s = kind.to_string();
            assert_eq!(s.parse::<NodeKind>().unwrap(), kind);
        }
    }

    // ── SelectRender::from_str ────────────────────────────────────────────────

    #[test]
    fn select_render_parses_all_valid_variants() {
        assert_eq!("auto".parse::<SelectRender>().unwrap(), SelectRender::Auto);
        assert_eq!(
            "picklist".parse::<SelectRender>().unwrap(),
            SelectRender::Picklist
        );
        assert_eq!(
            "input".parse::<SelectRender>().unwrap(),
            SelectRender::Input
        );
    }

    #[test]
    fn select_render_rejects_unknown_string() {
        assert!("dropdown".parse::<SelectRender>().is_err());
    }

    // ── InputType::from_str ───────────────────────────────────────────────────

    #[test]
    fn input_type_parses_plain_variants() {
        assert!(matches!(
            "string".parse::<InputType>().unwrap(),
            InputType::String
        ));
        assert!(matches!(
            "number".parse::<InputType>().unwrap(),
            InputType::Number
        ));
        assert!(matches!(
            "email".parse::<InputType>().unwrap(),
            InputType::Email
        ));
        assert!(matches!(
            "path".parse::<InputType>().unwrap(),
            InputType::Path
        ));
    }

    #[test]
    fn input_type_parses_regex_prefix() {
        let t = "regex:^[a-z]+$".parse::<InputType>().unwrap();
        assert!(matches!(t, InputType::Regex { ref pattern } if pattern == "^[a-z]+$"));
    }

    #[test]
    fn input_type_regex_with_empty_pattern() {
        let t = "regex:".parse::<InputType>().unwrap();
        assert!(matches!(t, InputType::Regex { ref pattern } if pattern.is_empty()));
    }

    #[test]
    fn input_type_rejects_unknown_string() {
        assert!("textarea".parse::<InputType>().is_err());
    }

    // ── NodeDef::display_name ─────────────────────────────────────────────────

    #[test]
    fn node_key_segment_uses_name_over_index() {
        let mut node = NodeDef::new(NodeKind::Select);
        node.name = Some("myselect".into());
        assert_eq!(node.key_segment(0), "myselect");
    }

    #[test]
    fn node_key_segment_falls_back_to_index() {
        let node = NodeDef::new(NodeKind::Select);
        assert_eq!(node.key_segment(3), "3");
    }

    #[test]
    fn display_name_prefers_name_over_all() {
        let mut node = NodeDef::new(NodeKind::Option);
        node.name = Some("myname".into());
        node.label = Some("mylabel".into());
        node.value = Some("myvalue".into());
        assert_eq!(node.display_name(), "myname");
    }

    #[test]
    fn display_name_falls_back_to_label() {
        let mut node = NodeDef::new(NodeKind::Option);
        node.label = Some("mylabel".into());
        node.value = Some("myvalue".into());
        assert_eq!(node.display_name(), "mylabel");
    }

    #[test]
    fn display_name_falls_back_to_value() {
        let mut node = NodeDef::new(NodeKind::Option);
        node.value = Some("myvalue".into());
        assert_eq!(node.display_name(), "myvalue");
    }

    #[test]
    fn display_name_falls_back_to_kind_string() {
        let node = NodeDef::new(NodeKind::Select);
        assert_eq!(node.display_name(), "select");
    }

    // ── Option counting ───────────────────────────────────────────────────────

    fn make_option(value: &str) -> NodeDef {
        let mut n = NodeDef::new(NodeKind::Option);
        n.value = Some(value.into());
        n
    }

    fn make_group(options: &[&str]) -> NodeDef {
        let mut g = NodeDef::new(NodeKind::Group);
        for v in options {
            g.children.push(make_option(v));
        }
        g
    }

    #[test]
    fn direct_option_count_ignores_groups() {
        let mut select = NodeDef::new(NodeKind::Select);
        select.children.push(make_option("a"));
        select.children.push(make_option("b"));
        select.children.push(make_group(&["c", "d"])); // group, not counted directly
        assert_eq!(select.direct_option_count(), 2);
    }

    #[test]
    fn group_count_ignores_options() {
        let mut select = NodeDef::new(NodeKind::Select);
        select.children.push(make_option("a"));
        select.children.push(make_group(&["b"]));
        select.children.push(make_group(&["c"]));
        assert_eq!(select.group_count(), 2);
    }

    #[test]
    fn total_option_count_includes_options_inside_groups() {
        let mut select = NodeDef::new(NodeKind::Select);
        select.children.push(make_option("a")); // direct: 1
        select.children.push(make_group(&["b", "c"])); // inside group: 2
                                                       // Total = 3
        assert_eq!(select.total_option_count(), 3);
    }

    #[test]
    fn total_option_count_with_no_children_is_zero() {
        let select = NodeDef::new(NodeKind::Select);
        assert_eq!(select.total_option_count(), 0);
    }

    // ── validate_parent_child ─────────────────────────────────────────────────

    #[test]
    fn validate_parent_child_rejects_flag_under_select() {
        assert!(validate_parent_child(&NodeKind::Select, &NodeKind::Flag).is_err());
    }

    #[test]
    fn validate_parent_child_accepts_option_under_select() {
        assert!(validate_parent_child(&NodeKind::Select, &NodeKind::Option).is_ok());
    }

    #[test]
    fn validate_parent_child_accepts_group_under_select() {
        assert!(validate_parent_child(&NodeKind::Select, &NodeKind::Group).is_ok());
    }

    #[test]
    fn validate_parent_child_accepts_option_under_group() {
        assert!(validate_parent_child(&NodeKind::Group, &NodeKind::Option).is_ok());
    }

    #[test]
    fn validate_parent_child_rejects_select_under_group() {
        assert!(validate_parent_child(&NodeKind::Group, &NodeKind::Select).is_err());
    }

    #[test]
    fn validate_parent_child_accepts_valid_command_children() {
        for child in [
            NodeKind::Argument,
            NodeKind::Select,
            NodeKind::Flag,
            NodeKind::Input,
        ] {
            assert!(
                validate_parent_child(&NodeKind::Command, &child).is_ok(),
                "{child} should be valid under Command"
            );
        }
    }

    #[test]
    fn validate_parent_child_rejects_command_under_command() {
        assert!(validate_parent_child(&NodeKind::Command, &NodeKind::Command).is_err());
    }

    #[test]
    fn leaf_nodes_reject_all_children() {
        for parent in [NodeKind::Flag, NodeKind::Input, NodeKind::Option] {
            for child in [
                NodeKind::Select,
                NodeKind::Flag,
                NodeKind::Input,
                NodeKind::Option,
            ] {
                assert!(
                    validate_parent_child(&parent, &child).is_err(),
                    "{child} should be invalid under leaf node {parent}"
                );
            }
        }
    }
}
