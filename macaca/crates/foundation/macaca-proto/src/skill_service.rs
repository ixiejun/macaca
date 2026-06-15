//! Provider-neutral Skill service DTOs.
//!
//! Skill providers and runtime-host bootstraps may discover rich local Skill
//! metadata, but shells need only bounded display rows for diagnostics and
//! operator-visible catalogs. Keeping this row in proto prevents SDK callers
//! from depending on runtime-host view structs just to render a Skill list.

use serde::{Deserialize, Serialize};

/// Sanitized catalog entry exposed to shells for read-only rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCatalogEntryView {
    /// Stable Skill name loaded from package metadata.
    pub name: String,
    /// Short Skill description suitable for operator-facing lists.
    pub description: String,
}
