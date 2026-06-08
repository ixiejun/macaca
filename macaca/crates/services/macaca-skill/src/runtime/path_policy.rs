//! Path membership checks against a frozen skill snapshot (file-policy seam).
//!
//! Used by tool/file guards to verify reads stay within projected or canonical
//! skill directories for the active agent session.

use std::path::Path;

use super::types::SkillSnapshot;

/// Return true when `path` is inside any skill in a snapshot (projected or source tree).
pub fn path_belongs_to_snapshot_skill(snapshot: &SkillSnapshot, path: &Path) -> bool {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    snapshot.skills.iter().any(|skill| {
        let base =
            std::fs::canonicalize(&skill.base_dir).unwrap_or_else(|_| skill.base_dir.clone());
        let matches_projected = canonical.starts_with(base);
        let matches_source = if skill.source_base_dir.as_os_str().is_empty() {
            false
        } else {
            let source_base = std::fs::canonicalize(&skill.source_base_dir)
                .unwrap_or_else(|_| skill.source_base_dir.clone());
            canonical.starts_with(source_base)
        };
        matches_projected || matches_source
    })
}
