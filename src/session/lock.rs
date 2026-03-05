//! File locking for session state files.
//!
//! We use `fs2::FileExt` which provides portable advisory file locks.
//! Mutations acquire an exclusive lock; reads acquire a shared lock.
//! The lock is held for the duration of the operation and released when
//! the `SessionLock` guard is dropped.

use std::fs::{File, OpenOptions};
use std::path::Path;

use anyhow::Context;
use fs2::FileExt;

/// RAII guard that holds a lock on the session state file.
///
/// The underlying `File` keeps the advisory lock alive.  When the guard
/// is dropped the OS releases the lock automatically.
pub struct SessionLock {
    _file: File,
}

impl SessionLock {
    /// Acquire an **exclusive** (write) lock on the lock file alongside the
    /// state file.  Blocks until the lock is available.
    pub fn exclusive(state_path: &Path) -> anyhow::Result<Self> {
        let lock_path = lock_path(state_path);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to open lock file: {}", lock_path.display()))?;
        file.lock_exclusive()
            .with_context(|| "Failed to acquire exclusive lock on session")?;
        Ok(SessionLock { _file: file })
    }

    /// Acquire a **shared** (read) lock on the lock file.  Blocks until no
    /// exclusive lock is held.
    pub fn shared(state_path: &Path) -> anyhow::Result<Self> {
        let lock_path = lock_path(state_path);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to open lock file: {}", lock_path.display()))?;
        file.lock_shared()
            .with_context(|| "Failed to acquire shared lock on session")?;
        Ok(SessionLock { _file: file })
    }
}

fn lock_path(state_path: &Path) -> std::path::PathBuf {
    let mut p = state_path.to_path_buf();
    let stem = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    p.set_file_name(format!("{stem}.lock"));
    p
}
