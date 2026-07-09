//! Path membership checks against a frozen skill snapshot (file-policy seam).
//!
//! Used by tool/file guards to verify reads stay within projected or canonical
//! skill directories for the active agent session.
//!
//! Security note (2026-07-08 audit P0-2): the previous implementation fell back
//! to the *raw, unresolved* path when `canonicalize` failed (which happens for
//! any non-existent path). Because `Path::starts_with` compares whole
//! components, a crafted path such as `<base>/../../etc/passwd` — whose first two
//! components are `<base>` — was accepted as "inside" the skill directory,
//! allowing traversal escape. This module now resolves `..` before comparing and
//! refuses any path it cannot prove is contained.

use std::path::{Component, Path, PathBuf};

use super::types::SkillSnapshot;

/// Resolve a path for containment policy, returning `None` when it cannot be
/// proven safe.
///
/// Resolution strategy:
/// 1. Prefer [`std::fs::canonicalize`], which resolves symlinks and `..` for
///    paths that exist on disk — the strongest guarantee.
/// 2. When canonicalization fails (typically a not-yet-created path), fall back
///    to [`normalize_lexical`], which resolves `.`/`..` purely syntactically.
/// 3. If neither can produce a normalized path (e.g. `..` escapes above the
///    root), return `None` so the caller treats the path as non-matching.
///
/// Returning `None` is fail-closed: an unprovable path is never treated as
/// contained.
fn resolve_for_policy(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some(canonical);
    }
    normalize_lexical(path)
}

/// Resolve `.` and `..` components without touching the filesystem.
///
/// Returns `None` when a `..` component would pop above the root of the path,
/// which is the signature of a traversal-escape attempt on an absolute path.
/// This is the panic-free, existence-independent complement to `canonicalize`.
fn normalize_lexical(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            // `..` unwinds one level; if there is nothing to unwind, the path is
            // escaping its own root and must be rejected.
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            // `.` contributes nothing to the resolved path.
            Component::CurDir => {}
            // Root, prefix (Windows), and normal names are retained verbatim.
            other => normalized.push(other.as_os_str()),
        }
    }
    Some(normalized)
}

/// Return true when `path` is inside any skill in a snapshot (projected or source tree).
///
/// Both the candidate path and each skill base are resolved through
/// [`resolve_for_policy`] before an inclusion test, so `..` traversal cannot slip
/// past component-wise `starts_with`. A rejected traversal attempt is logged at a
/// key node for audit without leaking the raw path content.
pub fn path_belongs_to_snapshot_skill(snapshot: &SkillSnapshot, path: &Path) -> bool {
    // Fail-closed: if the candidate cannot be resolved to a normalized form, we
    // cannot prove containment, so it does not belong to any skill.
    let Some(canonical) = resolve_for_policy(path) else {
        tracing::warn!(
            target = "macaca_skill::path_policy",
            event = "path_membership_rejected",
            reason_code = "unresolvable_or_traversal"
        );
        return false;
    };

    snapshot.skills.iter().any(|skill| {
        // A base that cannot be resolved is skipped rather than matched loosely.
        let matches_projected = resolve_for_policy(&skill.base_dir)
            .map(|base| canonical.starts_with(base))
            .unwrap_or(false);

        let matches_source = if skill.source_base_dir.as_os_str().is_empty() {
            false
        } else {
            resolve_for_policy(&skill.source_base_dir)
                .map(|source_base| canonical.starts_with(source_base))
                .unwrap_or(false)
        };

        matches_projected || matches_source
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lexical_resolves_interior_parent() {
        // `/base/sub/../file` resolves to `/base/file` (still inside `/base`).
        let resolved = normalize_lexical(Path::new("/base/sub/../file")).unwrap();
        assert_eq!(resolved, PathBuf::from("/base/file"));
    }

    #[test]
    fn normalize_lexical_rejects_root_escape() {
        // `/base/../../etc/passwd` escapes above root and must be rejected.
        assert!(normalize_lexical(Path::new("/base/../../etc/passwd")).is_none());
    }

    #[test]
    fn normalize_lexical_drops_current_dir() {
        let resolved = normalize_lexical(Path::new("/base/./skills/./foo")).unwrap();
        assert_eq!(resolved, PathBuf::from("/base/skills/foo"));
    }

    #[test]
    fn traversal_path_is_not_contained_by_prefix() {
        // Regression for the audit finding: a component-wise prefix of the base
        // must not be treated as containment once `..` is resolved.
        let base = Path::new("/base/skills");
        let escaping = normalize_lexical(Path::new("/base/skills/../../etc/passwd"));
        // The escaping path resolves to `/etc/passwd`, which does not start with base.
        match escaping {
            Some(resolved) => assert!(!resolved.starts_with(base)),
            None => {} // Also acceptable: normalization rejected it outright.
        }
    }
}
