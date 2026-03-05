//! Unified error types for ropt.
//!
//! All fallible operations return `RoptError` or wrap it via `anyhow::Error`.
//! Using `thiserror` to derive `Display` and `Error` keeps error messages
//! consistent and avoids ad-hoc string formatting throughout the codebase.

use thiserror::Error;

/// Top-level error type for all ropt operations.
#[derive(Debug, Error)]
pub enum RoptError {
    // ── Session errors ────────────────────────────────────────────────────────
    #[error("No active session found. Run `ropt begin` first, or set ROPT_SESSION.")]
    NoSession,

    #[error("Session '{0}' not found or state file missing.")]
    SessionNotFound(String),

    #[error("Session state file is too large (limit: 1 MB).")]
    SessionFileTooLarge,

    #[error("Session state file failed integrity check (tampered or corrupted).")]
    SessionIntegrityFailure,

    #[error("Session file has incorrect ownership or permissions.")]
    SessionPermissionDenied,

    // ── Stack / structure errors ──────────────────────────────────────────────
    #[error("Cannot pop: already at depth 0.")]
    StackUnderflow,

    #[error("Maximum nesting depth of {0} exceeded.")]
    MaxDepthExceeded(usize),

    #[error("Maximum options per select ({0}) exceeded.")]
    MaxOptionsExceeded(usize),

    #[error("Maximum groups per select ({0}) exceeded.")]
    MaxGroupsExceeded(usize),

    #[error("Node type '{0}' is not valid in this context.")]
    InvalidNodeContext(String),

    #[error("A node named '{0}' already exists at this level.")]
    DuplicateName(String),

    // ── Execution errors ──────────────────────────────────────────────────────
    #[error("Prompt timed out after {0} seconds.")]
    PromptTimeout(u64),

    #[error("Interactive prompt requires a TTY.")]
    NotATty,

    #[error("No result found for key '{0}'.")]
    ResultNotFound(String),

    // ── Validation errors ─────────────────────────────────────────────────────
    #[error("Invalid input: {0}")]
    ValidationError(String),

    #[error("Input exceeds maximum length of {0} bytes.")]
    InputTooLong(usize),

    // ── I/O and system errors ─────────────────────────────────────────────────
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;
