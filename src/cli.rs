//! CLI argument definitions using `clap`.
//!
//! This module owns the complete argument schema for the `ropt` binary.
//! All subcommands and their flags are declared here; the rest of the
//! application works only with the parsed `Cli` / `Command` values.
//!
//! Design note: We use `clap` derive macros for clarity and to get free
//! `--help` generation.  Long flag names mirror the spec exactly.

use clap::{Args, Parser, Subcommand, ValueEnum};

// ── Top-level ─────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "ropt",
    version,
    about = "Interactive CLI option configuration tool",
    long_about = "ropt lets you define command-line options declaratively and \
                  drive interactive prompts from shell scripts."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

// ── Subcommands ───────────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialise a new session and print its ID to stdout.
    Begin,

    /// Delete a session and its state file.
    End(SessionArgs),

    /// Push a node onto the definition stack (opens a new scope).
    Push(PushArgs),

    /// Add a node at the current depth without opening a new scope.
    /// Equivalent to `push <type> [args]; pop`.
    Append(PushArgs),

    /// Close the current scope, returning to the parent.
    Pop(SessionArgs),

    /// Present interactive prompts and record results.
    Execute(ExecuteArgs),

    /// Read a single result value by its key path.
    Read(ReadArgs),

    /// Display the current definition structure (for debugging).
    Show(ShowArgs),
}

// ── Shared session argument ───────────────────────────────────────────────────

/// Arguments that carry the optional session override.
#[derive(Debug, Args)]
pub struct SessionArgs {
    /// Override the session ID (default: $ROPT_SESSION env var).
    #[arg(long, env = "ROPT_SESSION")]
    pub session: Option<String>,
}

// ── push / append ─────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct PushArgs {
    /// Node type to push (command, argument, select, option, group, flag, input).
    pub node_type: String,

    // ── Shared / common ───────────────────────────────────────────────────────
    /// Name used as a path segment in result keys.
    #[arg(long)]
    pub name: Option<String>,

    /// Human-readable description or label.
    #[arg(long)]
    pub description: Option<String>,

    // ── Select-specific ───────────────────────────────────────────────────────
    /// Prompt message shown above the option list (select).
    #[arg(long)]
    pub message: Option<String>,

    /// Rendering style for select nodes.
    #[arg(long, value_name = "STYLE")]
    pub render: Option<String>,

    /// Allow multiple selections (select).
    #[arg(long)]
    pub multiple: bool,

    // ── Option-specific ───────────────────────────────────────────────────────
    /// Value returned when this option is chosen.
    #[arg(long)]
    pub value: Option<String>,

    /// Display label for an option or group.
    #[arg(long)]
    pub label: Option<String>,

    /// Mark this option as pre-selected.
    #[arg(long)]
    pub default: bool,

    /// Option is visible but cannot be selected.
    #[arg(long)]
    pub disabled: bool,

    // ── Flag-specific ─────────────────────────────────────────────────────────
    /// Single-character short form for a flag.
    #[arg(long, value_name = "CHAR")]
    pub short: Option<char>,

    // ── Input-specific ────────────────────────────────────────────────────────
    /// Input type constraint (string, number, email, path, regex:<pattern>).
    #[arg(long = "type", value_name = "TYPE")]
    pub input_type: Option<String>,

    /// Regex pattern for custom validation.
    #[arg(long)]
    pub validate_regex: Option<String>,

    /// Minimum value or length.
    #[arg(long)]
    pub validate_min: Option<f64>,

    /// Maximum value or length.
    #[arg(long)]
    pub validate_max: Option<f64>,

    /// Default value when user presses Enter with no input.
    #[arg(long = "default-value")]
    pub default_value: Option<String>,

    /// Treat input as sensitive (no echo, no history).
    #[arg(long)]
    pub sensitive: bool,

    /// Session ID override.
    #[arg(long, env = "ROPT_SESSION")]
    pub session: Option<String>,
}

// ── execute ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum OutputFormat {
    /// Structured JSON.
    #[default]
    Json,
    /// Shell-sourceable KEY=VALUE pairs.
    Sh,
    /// Plain text, one value per line.
    Raw,
}

#[derive(Debug, Args)]
pub struct ExecuteArgs {
    /// Output format for the results.
    #[arg(long, value_enum, default_value = "json")]
    pub format: OutputFormat,

    /// Prefix prepended to every shell variable name (--format=sh only).
    /// Must be a valid shell identifier fragment (letters, digits, underscores).
    /// Example: --prefix=ropt_ produces ropt_action='deploy'.
    #[arg(long, default_value = "")]
    pub prefix: String,

    /// Session ID override.
    #[arg(long, env = "ROPT_SESSION")]
    pub session: Option<String>,
}

// ── read ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ReadArgs {
    /// Dot-separated key path (e.g. "build.target").
    #[arg(long)]
    pub key: String,

    /// Session ID override.
    #[arg(long, env = "ROPT_SESSION")]
    pub session: Option<String>,
}

// ── show ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum ShowFormat {
    #[default]
    Tree,
    Json,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Display format.
    #[arg(long, value_enum, default_value = "tree")]
    pub format: ShowFormat,

    /// Session ID override.
    #[arg(long, env = "ROPT_SESSION")]
    pub session: Option<String>,
}
