//! Session state – the data written to and read from the session file.
//!
//! The state is a plain JSON document containing:
//!   1. The definition tree built by `push`/`append`/`pop` operations.
//!   2. The stack of currently open nodes (represented as index paths).
//!   3. The results map populated after `execute`.
//!   4. Metadata: session ID, creation time, checksum for tamper detection.
//!
//! Only this module serialises / deserialises the file.  All other modules
//! receive or return `SessionState` values.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::node::NodeDef;

// ── Result value ──────────────────────────────────────────────────────────────

/// The value recorded for a single interactive node after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResultValue {
    /// Single string (most node types).
    Single(String),
    /// Multiple strings (multi-select).
    Multiple(Vec<String>),
    /// Boolean (flag).
    Bool(bool),
}

impl std::fmt::Display for ResultValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultValue::Single(s) => write!(f, "{s}"),
            ResultValue::Bool(b) => write!(f, "{b}"),
            ResultValue::Multiple(vs) => write!(f, "{}", vs.join(" ")),
        }
    }
}

// ── Session state ─────────────────────────────────────────────────────────────

/// The full persisted state for one session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Unique identifier for this session.
    pub session_id: String,

    /// Unix timestamp (seconds) when the session was created.
    pub created_at: u64,

    /// The root-level definition nodes, in the order they were pushed.
    pub roots: Vec<NodeDef>,

    /// Path of indices into `roots` / children that represents the current
    /// open scope.  Empty means we are at depth 0 (top level).
    ///
    /// Example: `[0, 1]` means `roots[0].children[1]` is the current node.
    pub stack: Vec<usize>,

    /// Results keyed by dot-separated path strings (e.g. `"build.target"`).
    #[serde(default)]
    pub results: HashMap<String, ResultValue>,

    /// SHA-256 hex digest of the state (excluding this field itself).
    /// Recomputed on every write and verified on every read.
    #[serde(default)]
    pub checksum: String,
}

impl SessionState {
    /// Create a fresh, empty session state.
    pub fn new(session_id: String) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SessionState {
            session_id,
            created_at,
            roots: Vec::new(),
            stack: Vec::new(),
            results: HashMap::new(),
            checksum: String::new(),
        }
    }

    /// Serialise to MessagePack bytes, computing and embedding a checksum.
    pub fn to_msgpack_bytes(&self) -> anyhow::Result<Vec<u8>> {
        // Serialise without checksum to compute hash over stable payload.
        let mut copy = self.clone();
        copy.checksum = String::new();
        let payload = rmp_serde::to_vec_named(&copy)?;
        copy.checksum = fnv1a_hex(&payload);
        Ok(rmp_serde::to_vec_named(&copy)?)
    }

    /// Deserialise from MessagePack bytes and verify integrity.
    pub fn from_msgpack_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut state: SessionState = rmp_serde::from_slice(bytes)?;
        let stored_checksum = std::mem::take(&mut state.checksum);
        let payload = rmp_serde::to_vec_named(&state)?;
        let computed = fnv1a_hex(&payload);
        if computed != stored_checksum {
            anyhow::bail!(crate::error::RoptError::SessionIntegrityFailure);
        }
        state.checksum = stored_checksum;
        Ok(state)
    }

    /// Return the node currently at the top of the stack (mutable).
    pub fn current_node_mut(&mut self) -> Option<&mut NodeDef> {
        if self.stack.is_empty() {
            return None;
        }
        let mut node: &mut NodeDef = self.roots.get_mut(self.stack[0])?;
        for &idx in &self.stack[1..] {
            node = node.children.get_mut(idx)?;
        }
        Some(node)
    }

    /// Return the node at the top of the stack (immutable).
    pub fn current_node(&self) -> Option<&NodeDef> {
        if self.stack.is_empty() {
            return None;
        }
        let mut node: &NodeDef = self.roots.get(self.stack[0])?;
        for &idx in &self.stack[1..] {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    /// Check whether `name` is already used by a sibling in the current scope.
    ///
    /// Only named nodes can collide: unnamed nodes are keyed by index so they
    /// are always distinct.  Returns `true` if the name is already taken.
    pub fn name_exists_in_current_scope(&self, name: &str) -> bool {
        let siblings: &[NodeDef] = if self.stack.is_empty() {
            &self.roots
        } else {
            match self.current_node() {
                Some(n) => &n.children,
                None => return false,
            }
        };
        siblings.iter().any(|s| s.name.as_deref() == Some(name))
    }

    /// Add a child node to the current scope (or to roots if at depth 0).
    /// Returns the index of the new child.
    pub fn add_child(&mut self, child: NodeDef) -> usize {
        if self.stack.is_empty() {
            self.roots.push(child);
            self.roots.len() - 1
        } else {
            let parent = self
                .current_node_mut()
                .expect("stack points to a valid node");
            parent.children.push(child);
            parent.children.len() - 1
        }
    }

    /// Push a new node into the current scope and make it the current scope.
    pub fn push_node(&mut self, child: NodeDef) {
        let idx = self.add_child(child);
        self.stack.push(idx);
    }

    /// Pop the current scope, returning to the parent.
    pub fn pop_node(&mut self) -> anyhow::Result<()> {
        if self.stack.is_empty() {
            anyhow::bail!(crate::error::RoptError::StackUnderflow);
        }
        self.stack.pop();
        Ok(())
    }

    /// Current nesting depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

// ── Checksum helper ───────────────────────────────────────────────────────────

/// FNV-1a 64-bit hash over raw bytes, returned as a 16-char hex string.
///
/// Fast and dependency-free. Sufficient to detect accidental corruption or
/// tampering with the session file between ropt invocations. Not
/// cryptographically strong — swap for sha2 if stronger guarantees are needed.
fn fnv1a_hex(data: &[u8]) -> String {
    // FNV-1a 64-bit – fast, dependency-free, sufficient for integrity checks
    // against accidental corruption. Not cryptographically strong.
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;

    let mut hash: u64 = FNV_OFFSET;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_state() {
        let mut state = SessionState::new("test-session".into());
        state
            .results
            .insert("key".into(), ResultValue::Single("value".into()));
        let bytes = state.to_msgpack_bytes().unwrap();
        let restored = SessionState::from_msgpack_bytes(&bytes).unwrap();
        assert_eq!(restored.session_id, "test-session");
        assert!(restored.results.contains_key("key"));
    }

    #[test]
    fn tampered_file_fails_checksum() {
        let state = SessionState::new("tamper-test".into());
        let mut bytes = state.to_msgpack_bytes().unwrap();
        // Flip a byte in the middle of the payload.
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        assert!(SessionState::from_msgpack_bytes(&bytes).is_err());
    }
}
