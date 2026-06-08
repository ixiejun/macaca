//! Content digest and mutation metadata Value Object helpers.
//!
//! These helpers keep hashing, YAML escaping, and audit metadata assembly out
//! of the orchestration Strategy.  They are deterministic and side-effect free
//! so materialization previews and applied writes share identical digest refs.

use std::collections::BTreeMap;

use macaca_skill::SkillExperienceProposalRecord;

use super::draft_materialization_builder::SkillDraftMaterialization;

/// Build mutation metadata refs that tie the write back to proposal provenance.
///
/// The mutation Strategy forwards this map unchanged into rollback mementos and
/// audit events, giving operators a stable cross-reference without logging body
/// content.
pub(crate) fn materialization_mutation_metadata(
    proposal: &SkillExperienceProposalRecord,
    draft: &SkillDraftMaterialization,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("proposal_id".into(), proposal.proposal_id.clone());
    metadata.insert("task_id".into(), proposal.task_id.clone());
    metadata.insert("content_digest_ref".into(), draft.content_digest.clone());
    metadata
}

/// Escape a string for safe inclusion inside YAML double-quoted scalars.
pub(super) fn yaml_quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Compute a stable FNV-1a digest ref for bounded draft bytes.
///
/// The digest is intentionally lightweight rather than cryptographic because it
/// only deduplicates previews and audit refs inside the Skill service boundary.
pub(super) fn stable_content_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("skill-draft-digest://fnv1a64:{hash:016x}")
}
