//! **Template method** for reading Markdown profile files with shared safety rails.
//!
//! Steps (unchanging skeleton, customizable via policy layer):
//! 1. Require the profile root directory to exist.
//! 2. Canonicalise the root once — every file must remain under that prefix after canonicalisation.
//! 3. Open the candidate path, enforce a byte cap per [`macaca_proto::config::AgentProfileContextConfig::max_file_bytes`].
//! 4. Decode UTF-8 with explicit handling for lossy replacement.

use std::path::{Path, PathBuf};

use macaca_proto::config::AgentProfileContextConfig;
use tokio::io::AsyncReadExt;

use super::kinds::AgentProfileFileKind;

/// Output of the template-method load: what to hand to [`super::ProfileFileContextProvider`].
#[derive(Debug, Clone)]
pub struct ProfileLoadOutput {
    /// Normalised Markdown body (trimmed for stable empty detection upstream).
    pub text: String,
    /// True when the on-disk file exceeded the byte cap and was truncated at a char boundary.
    pub truncated_by_cap: bool,
    /// Number of raw bytes read from disk before UTF-8 interpretation.
    pub raw_bytes: u64,
    /// Canonical absolute path to the file that was read (helpful for audits).
    pub resolved_path: PathBuf,
    /// True when UTF-8 decoding replaced invalid bytes (never silent in diagnostics).
    pub utf8_lossy: bool,
}

/// Reasons a profile file might be skipped without treating the run as fatal.
#[derive(Debug, Clone)]
pub enum ProfileSkipReason {
    /// Symlink / hardlink escape, or other canonicalisation mismatch.
    PathNotConfinedToRoot,
    /// Root directory itself cannot be canonicalised (permissions / missing).
    RootResolutionFailed(String),
    /// Explicit `std::io::Error` mid-flight.
    Io(String),
}

/// Template-method entry: load `kind` if present and permitted by policy/config.
///
/// The `skip_due_to_policy` flag lets the provider skip heartbeat / memory **without** touching
/// the filesystem when operators disable those classes globally for the agent.
pub async fn load_profile_file(
    root: &Path,
    kind: AgentProfileFileKind,
    config: &AgentProfileContextConfig,
    skip_due_to_policy: bool,
) -> Result<Option<ProfileLoadOutput>, ProfileSkipReason> {
    if skip_due_to_policy {
        return Ok(None);
    }

    let root_ok = canonical_root(root)?;
    load_profile_file_at_canonical_root(&root_ok, kind, config, skip_due_to_policy).await
}

/// Loads a single file when the caller has already canonicalised the profile directory.
///
/// This avoids repeating `canonicalize` syscalls for every [`AgentProfileFileKind`].
pub async fn load_profile_file_at_canonical_root(
    root_ok: &Path,
    kind: AgentProfileFileKind,
    config: &AgentProfileContextConfig,
    skip_due_to_policy: bool,
) -> Result<Option<ProfileLoadOutput>, ProfileSkipReason> {
    if skip_due_to_policy {
        return Ok(None);
    }

    let candidate = root_ok.join(kind.file_name());
    if tokio::fs::metadata(&candidate).await.is_err() {
        return Ok(None);
    }

    enforce_containment(root_ok, &candidate).await?;

    read_limited_utf8(&candidate, config.max_file_bytes).await
}

/// Canonicalise and validate `root` before issuing per-file loads.
pub fn canonical_root(root: &Path) -> Result<PathBuf, ProfileSkipReason> {
    if !root.exists() {
        return Err(ProfileSkipReason::RootResolutionFailed(format!(
            "profile root does not exist: {}",
            root.display()
        )));
    }
    std::fs::canonicalize(root).map_err(|e| ProfileSkipReason::RootResolutionFailed(e.to_string()))
}

/// Security gate: both paths are canonicalised and the file must be a child of `root_canon`.
async fn enforce_containment(root_canon: &Path, candidate: &Path) -> Result<(), ProfileSkipReason> {
    let file_canon = match tokio::fs::canonicalize(candidate).await {
        Ok(p) => p,
        Err(e) => {
            return Err(ProfileSkipReason::Io(format!(
                "canonicalize {}: {e}",
                candidate.display()
            )))
        }
    };
    if !file_canon.starts_with(root_canon) {
        return Err(ProfileSkipReason::PathNotConfinedToRoot);
    }
    Ok(())
}

/// Reads up to `max + 1` bytes to detect oversize files, then truncates to `max` UTF-8 safe.
async fn read_limited_utf8(
    path: &Path,
    max: u64,
) -> Result<Option<ProfileLoadOutput>, ProfileSkipReason> {
    let max_usize = usize::try_from(max).unwrap_or(usize::MAX);
    // `AsyncReadExt::take` requires `&mut self`; `tokio::fs::File` must stay mutable until the read finishes.
    #[allow(unused_mut)]
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| ProfileSkipReason::Io(format!("open {}: {e}", path.display())))?;

    // `AsyncReadExt::take` is in bytes; read one extra byte so we can flag truncation.
    let per_read_cap = max.saturating_add(1);
    let mut buf = Vec::new();
    let mut chunk = file.take(per_read_cap);
    chunk
        .read_to_end(&mut buf)
        .await
        .map_err(|e| ProfileSkipReason::Io(format!("read {}: {e}", path.display())))?;
    let read_len = buf.len();
    let truncated_by_cap = read_len > max_usize;
    let original_byte_len = read_len as u64;
    if truncated_by_cap {
        buf.truncate(max_usize);
        // Guarantee truncation stays on a UTF-8 boundary for human-readable diagnostics.
        while std::str::from_utf8(&buf).is_err() {
            buf.pop();
        }
    }

    let raw_bytes = original_byte_len;
    let utf8_lossy = std::str::from_utf8(&buf).is_err();
    let text = String::from_utf8_lossy(&buf).to_string();

    let resolved_path = tokio::fs::canonicalize(path)
        .await
        .map_err(|e| ProfileSkipReason::Io(format!("canonicalize {}: {e}", path.display())))?;

    Ok(Some(ProfileLoadOutput {
        text,
        truncated_by_cap,
        raw_bytes,
        resolved_path,
        utf8_lossy,
    }))
}

impl std::fmt::Display for ProfileSkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileSkipReason::PathNotConfinedToRoot => {
                write!(f, "resolved path escapes profile root (symlink attack?)")
            }
            ProfileSkipReason::RootResolutionFailed(m) => write!(f, "{m}"),
            ProfileSkipReason::Io(m) => write!(f, "{m}"),
        }
    }
}
