//! Shared filesystem helpers for serviceization escape-hatch gates.
//!
//! These helpers intentionally stay small and deterministic. The boundary gates
//! use them as a support layer so policy tests can focus on service ownership
//! assertions instead of repeating workspace discovery and optional frontend
//! checkout handling.

use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

/// Return the repository root that contains both `macaca/` and optional
/// `frontend/`.
///
/// The Macaca Cargo workspace lives under `macaca/`, while the Next.js shell is a
/// sibling checkout. Backend-only CI agents may omit `frontend/`; gates that
/// reference presentation sources must tolerate that absence.
pub fn repository_root() -> PathBuf {
    workspace_root()
        .parent()
        .expect("Macaca workspace should live under the repository root")
        .to_path_buf()
}

/// Read a presentation source file when the checkout includes it.
///
/// Returning `None` for absent frontend files preserves deterministic backend
/// CI while still validating shell adapters in full monorepo checkouts.
pub fn read_optional_presentation_source(path: &Path) -> Option<String> {
    if !path.exists() {
        eprintln!(
            "serviceization_escape_hatches event=skip_missing_presentation_source path={}",
            path.display()
        );
        return None;
    }
    Some(
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display())),
    )
}
