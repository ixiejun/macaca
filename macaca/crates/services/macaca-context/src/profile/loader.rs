//! **Template method** for reading Markdown profile files with shared safety rails.
//!
//! Steps (unchanging skeleton, customizable via policy layer):
//! 1. Require the profile root directory to exist.
//! 2. Canonicalise the root once — every file must remain under that prefix after canonicalisation.
//! 3. Open the candidate path, enforce a byte cap per [`macaca_proto::config::AgentProfileContextConfig::max_file_bytes`].
//! 4. Decode UTF-8 with explicit handling for lossy replacement.
//! 5. Strip optional YAML frontmatter; run lightweight content scans (NUL, line budgets).

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
    /// Loader notes (frontmatter stripping, scan outcomes) surfaced as provider diagnostics only.
    pub load_notes: Vec<String>,
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
    /// Lightweight scan rejects unsafe or oversized payloads.
    ContentRejected(String),
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

    let mut output = read_limited_utf8(&candidate, config.max_file_bytes).await?;
    finalize_loaded_profile_body(&mut output, config)?;
    Ok(Some(output))
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

fn finalize_loaded_profile_body(
    output: &mut ProfileLoadOutput,
    config: &AgentProfileContextConfig,
) -> Result<(), ProfileSkipReason> {
    let (stripped, mut notes) = strip_leading_yaml_frontmatter(&output.text);
    output.text = stripped;
    output.load_notes.append(&mut notes);
    scan_profile_markdown_after_frontmatter(&output.text, config)
}

fn strip_leading_yaml_frontmatter(text: &str) -> (String, Vec<String>) {
    let mut notes = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return (text.to_string(), notes);
    }
    let mut close: Option<usize> = None;
    for idx in 1..lines.len() {
        if lines[idx].trim() == "---" {
            close = Some(idx);
            break;
        }
    }
    let Some(close_idx) = close else {
        notes.push(
            "profile_yaml_frontmatter: unclosed delimiter (missing closing ---); kept full bytes"
                .into(),
        );
        return (text.to_string(), notes);
    };
    let body = lines[close_idx.saturating_add(1)..].join("\n");
    notes.push(format!(
        "profile_yaml_frontmatter: stripped {} preamble lines including delimiters",
        close_idx.saturating_add(1)
    ));
    (body, notes)
}

fn scan_profile_markdown_after_frontmatter(
    text: &str,
    config: &AgentProfileContextConfig,
) -> Result<(), ProfileSkipReason> {
    if text.contains('\0') {
        return Err(ProfileSkipReason::ContentRejected(
            "NUL byte detected in profile text".into(),
        ));
    }
    if config.profile_max_content_lines > 0 {
        let n = text.lines().count();
        let cap = usize::try_from(config.profile_max_content_lines).unwrap_or(usize::MAX);
        if n > cap {
            return Err(ProfileSkipReason::ContentRejected(format!(
                "profile exceeds profile_max_content_lines={cap} (saw {n} lines)",
            )));
        }
    }
    Ok(())
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
async fn read_limited_utf8(path: &Path, max: u64) -> Result<ProfileLoadOutput, ProfileSkipReason> {
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

    Ok(ProfileLoadOutput {
        text,
        truncated_by_cap,
        raw_bytes,
        resolved_path,
        utf8_lossy,
        load_notes: Vec::new(),
    })
}

impl std::fmt::Display for ProfileSkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileSkipReason::PathNotConfinedToRoot => {
                write!(f, "resolved path escapes profile root (symlink attack?)")
            }
            ProfileSkipReason::RootResolutionFailed(m) => write!(f, "{m}"),
            ProfileSkipReason::Io(m) => write!(f, "{m}"),
            ProfileSkipReason::ContentRejected(m) => write!(f, "{m}"),
        }
    }
}
