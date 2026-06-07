//! Bounded observer DTOs and proposal metadata limits.

/// Maximum number of artifact references carried in metadata.
///
/// Artifact paths can be useful for audit replay, but the observer must keep
/// proposal metadata bounded because the Skill operations surface can be shown
/// in shells and reports.
pub(crate) const MAX_ARTIFACT_REFS: usize = 8;

/// Maximum number of characters copied from one artifact reference.
pub(crate) const MAX_ARTIFACT_REF_CHARS: usize = 256;

/// Maximum number of semantic trigger phrases used for an autonomous Skill name.
///
/// The frontmatter `name` is a model-facing selector, not a provenance id.  A
/// short phrase budget keeps generated Skills easy to trigger while avoiding
/// overfitted task transcripts or application-specific wording.
pub(crate) const MAX_SEMANTIC_TRIGGER_PHRASES: usize = 4;

/// Bounded observer result written to EventLog by the decorator.
///
/// This is intentionally small and stringly for audit display only.  The Skill
/// service result remains the typed source of truth; the shell event lets live
/// black-box tests prove that the Agent Execution boundary observer actually ran.
#[derive(Debug, Clone)]
pub(crate) struct SkillSelfEvolutionObservation {
    pub status: &'static str,
    pub task_id: Option<String>,
    pub proposal_id: Option<String>,
    pub reason: Option<String>,
}
