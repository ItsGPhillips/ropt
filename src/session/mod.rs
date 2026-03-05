//! Session management – create, resolve, read, write, and end sessions.
//!
//! A session is a MessagePack file on disk that holds the full `SessionState`.
//! The file path is derived from the session ID which is generated at
//! `ropt begin` time.
//!
//! File locations (in priority order):
//!   1. `$XDG_RUNTIME_DIR/ropt/<session-id>.ropt`   (Linux standard)
//!   2. `~/.ropt/tmp/<session-id>.ropt`             (fallback)
//!
//! Security:
//!   - Files are created with 0600 permissions.
//!   - Ownership is verified before every mutation.
//!   - Integrity checksums are validated on every read.
//!   - File size is capped at 1 MB.

pub mod lock;
pub mod state;

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::Context;
use uuid::Uuid;

use self::lock::SessionLock;
use self::state::SessionState;
use crate::error::RoptError;

const MAX_SESSION_FILE_BYTES: u64 = 1024 * 1024; // 1 MB
const SESSION_FILE_EXT: &str = "ropt";
const SESSION_ENV_VAR: &str = "ROPT_SESSION";

// ── Session directory ─────────────────────────────────────────────────────────

/// Return the directory where session files are stored, creating it if needed.
pub fn session_dir() -> anyhow::Result<PathBuf> {
    let dir = if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg).join("ropt")
    } else {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
            .join(".ropt")
            .join("tmp")
    };

    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create session directory: {}", dir.display()))?;

    // Ensure the directory itself is private on Unix.
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(&dir, perms).ok();
    }

    Ok(dir)
}

/// Return the path for a specific session ID.
fn session_path(session_id: &str) -> anyhow::Result<PathBuf> {
    // Basic ID validation to prevent path traversal.
    if session_id.contains('/') || session_id.contains("..") || session_id.is_empty() {
        anyhow::bail!("Invalid session ID: '{session_id}'");
    }
    let dir = session_dir()?;
    Ok(dir.join(format!("{session_id}.{SESSION_FILE_EXT}")))
}

// ── Session ID resolution ─────────────────────────────────────────────────────

/// Resolve the active session ID using the priority order defined in the spec:
///
/// 1. Explicit `--session` argument (if `Some`).
/// 2. `ROPT_SESSION` environment variable.
/// 3. Error: no session found.
pub fn resolve_session_id(explicit: Option<&str>) -> anyhow::Result<String> {
    if let Some(id) = explicit {
        return Ok(id.to_owned());
    }
    if let Ok(id) = std::env::var(SESSION_ENV_VAR)
        && !id.trim().is_empty()
    {
        return Ok(id.trim().to_owned());
    }
    Err(RoptError::NoSession.into())
}

// ── Public session operations ─────────────────────────────────────────────────

/// Begin a new session.
///
/// Generates a unique session ID, creates the state file with strict
/// permissions, and prints the session ID to stdout.
pub fn begin() -> anyhow::Result<String> {
    let session_id = Uuid::new_v4().to_string();
    let path = session_path(&session_id)?;

    let state = SessionState::new(session_id.clone());
    write_state_locked(&path, &state)?;

    Ok(session_id)
}

/// End (delete) an existing session.
pub fn end(session_id: &str) -> anyhow::Result<()> {
    let path = session_path(session_id)?;
    if !path.exists() {
        anyhow::bail!(RoptError::SessionNotFound(session_id.to_owned()));
    }
    let _lock = SessionLock::exclusive(&path)?;
    fs::remove_file(&path)
        .with_context(|| format!("Failed to remove session file: {}", path.display()))?;
    // Remove the lock file as well (best-effort).
    let lock_path = lock_path_for(&path);
    let _ = fs::remove_file(lock_path);
    Ok(())
}

/// Read session state (shared lock, integrity check, size limit).
pub fn read_state(session_id: &str) -> anyhow::Result<SessionState> {
    let path = session_path(session_id)?;
    if !path.exists() {
        anyhow::bail!(RoptError::SessionNotFound(session_id.to_owned()));
    }
    let _lock = SessionLock::shared(&path)?;
    read_state_unlocked(&path)
}

/// Mutate session state with a closure, then write the result back.
///
/// The exclusive lock is held for the entire duration of the read-modify-write
/// cycle to prevent concurrent corruption.
pub fn mutate_state<F>(session_id: &str, mutate: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut SessionState) -> anyhow::Result<()>,
{
    let path = session_path(session_id)?;
    if !path.exists() {
        anyhow::bail!(RoptError::SessionNotFound(session_id.to_owned()));
    }

    let _lock = SessionLock::exclusive(&path)?;

    let mut state = read_state_unlocked(&path)?;
    mutate(&mut state)?;
    write_state_raw(&path, &state)?;

    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn read_state_unlocked(path: &Path) -> anyhow::Result<SessionState> {
    let metadata = fs::metadata(path)?;

    // Size guard.
    if metadata.len() > MAX_SESSION_FILE_BYTES {
        anyhow::bail!(RoptError::SessionFileTooLarge);
    }

    // Ownership / permission check on Unix.
    #[cfg(unix)]
    {
        // Use std::process to get current UID without a libc dependency.
        // Safety: this is a standard Unix syscall with no failure modes.
        let uid = {
            unsafe extern "C" {
                fn getuid() -> u32;
            }
            unsafe { getuid() }
        };
        if metadata.uid() != uid {
            anyhow::bail!(RoptError::SessionPermissionDenied);
        }
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            // Group or other bits are set – reject.
            anyhow::bail!(RoptError::SessionPermissionDenied);
        }
    }

    let bytes = fs::read(path)?;
    SessionState::from_msgpack_bytes(&bytes)
        .with_context(|| format!("Failed to parse session file: {}", path.display()))
}

fn write_state_locked(path: &Path, state: &SessionState) -> anyhow::Result<()> {
    let _lock = SessionLock::exclusive(path)?;
    write_state_raw(path, state)
}

fn write_state_raw(path: &Path, state: &SessionState) -> anyhow::Result<()> {
    let bytes = state.to_msgpack_bytes()?;

    // Write atomically via a temp file.
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &bytes)
        .with_context(|| format!("Failed to write temp session file: {}", tmp_path.display()))?;

    // Set strict permissions before moving into place.
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&tmp_path, perms)?;
    }

    fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed to rename temp session file to: {}", path.display()))?;

    Ok(())
}

fn lock_path_for(state_path: &Path) -> PathBuf {
    let stem = state_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut p = state_path.to_path_buf();
    p.set_file_name(format!("{stem}.lock"));
    p
}
