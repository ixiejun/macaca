//! Builder that converts sanitized proposal records into bounded Skill drafts.
//!
//! The Builder pattern isolates markdown rendering and identity derivation from
//! readiness checks, mutation orchestration, and promotion.  It consumes only
//! provider-neutral proposal fields and emits UTF-8 bytes that the content
//! mutation Strategy validates again before any filesystem write occurs.

use std::path::PathBuf;

use macaca_skill::SkillExperienceProposalRecord;

use super::content_digest_vocabulary::{stable_content_digest, yaml_quoted};
use super::identity_vocabulary::{bounded_block, bounded_line, slugify};

/// Bounded materialized draft produced by the Builder.
///
/// This value object captures everything the mutation Strategy needs without
/// exposing the full markdown body through logs or service responses.
pub(crate) struct SkillDraftMaterialization {
    pub(crate) skill_id: String,
    pub(crate) name: String,
    pub(crate) identity_source: &'static str,
    pub(crate) identity_used_fallback: bool,
    pub(crate) relative_path: PathBuf,
    pub(crate) content: Vec<u8>,
    pub(crate) content_digest: String,
    pub(crate) materialization_ref: String,
}

/// Model-facing identity derived for a materialized Skill package.
///
/// Proposal ids remain the immutable audit identity, but the frontmatter name is
/// the handle the model sees when deciding whether a future task should activate
/// the Skill.  Keeping this small value object inside the Builder preserves the
/// service boundary: identity derivation is part of package construction, while
/// mutation, policy, rollback, and promotion stay in their existing strategies.
struct MaterializedSkillIdentity {
    name: String,
    source: &'static str,
    used_fallback: bool,
}

/// Builder that isolates proposal-to-`SKILL.md` content construction.
///
/// Keeping this as a dedicated Builder prevents the provider Strategy from
/// mixing readiness checks, mutation orchestration, and markdown rendering. The
/// Builder consumes only sanitized proposal fields and emits bounded UTF-8
/// bytes for the existing mutation Strategy to validate again.
pub(crate) struct SkillDraftMaterializationBuilder<'a> {
    proposal: &'a SkillExperienceProposalRecord,
}

impl<'a> SkillDraftMaterializationBuilder<'a> {
    /// Bind the Builder to one sanitized proposal record.
    pub(crate) fn new(proposal: &'a SkillExperienceProposalRecord) -> Self {
        Self { proposal }
    }

    /// Render bounded AgentSkills-compatible markdown and derive stable digests.
    ///
    /// Identity selection follows a deterministic precedence chain documented on
    /// `materialized_skill_identity`.  Content sections are hard-capped so
    /// unattended materialization cannot emit unbounded packages.
    pub(crate) fn build(&self) -> SkillDraftMaterialization {
        let identity = materialized_skill_identity(self.proposal);
        let name = identity.name;
        let skill_id = self
            .proposal
            .target_skill_id
            .clone()
            .unwrap_or_else(|| format!("skill://agent/{name}"));
        let description = yaml_quoted(&skill_creator_description(self.proposal));
        let procedure = bounded_block(&self.proposal.reusable_procedure, 4_096);
        let when_to_use = bounded_line(&self.proposal.reusable_procedure, 240);
        let content = format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n\n## When To Use\nUse this skill when a future task matches this bounded procedure: {when_to_use}\n\n## Procedure\n{procedure}\n\n## Verification\n- Confirm terminal task evidence is still present before relying on this skill.\n- Record new evidence refs when the procedure changes.\n\n## Provenance\n- Proposal ref: `{}`\n- Task ref: `{}`\n- Trace ref: `{}`\n",
            self.proposal.proposal_id, self.proposal.task_id, self.proposal.trace_id
        )
        .into_bytes();
        let content_digest = stable_content_digest(&content);
        SkillDraftMaterialization {
            skill_id,
            name,
            identity_source: identity.source,
            identity_used_fallback: identity.used_fallback,
            relative_path: PathBuf::from("SKILL.md"),
            materialization_ref: format!(
                "skill.materialization://{}",
                content_digest
                    .strip_prefix("skill-draft-digest://")
                    .unwrap_or(&content_digest)
            ),
            content,
            content_digest,
        }
    }
}

/// Build the only model-visible trigger field guaranteed to be loaded before a
/// Skill body is selected.
///
/// Skill Creator guidance treats `description` as the primary discovery surface.
/// The materialization Builder therefore writes a concise "Use when..." trigger
/// sentence and leaves task ids, proposal ids, and trace refs in provenance
/// fields rather than the selector text.
fn skill_creator_description(proposal: &SkillExperienceProposalRecord) -> String {
    let semantic_context = proposal
        .target_skill_name
        .as_deref()
        .map(slugify)
        .filter(|name| !name.is_empty())
        .or_else(|| semantic_skill_name_from_text(&proposal.reusable_procedure))
        .or_else(|| semantic_skill_name_from_text(&proposal.bounded_summary))
        .unwrap_or_else(|| "governed-skill-procedure".into());
    bounded_line(
        &format!(
            "Use when a future task needs the governed reusable procedure for {semantic_context}; rely on linked evidence refs, registry visibility, telemetry, curation, approval, and rollback before reuse."
        ),
        320,
    )
}

/// Fallback slug derived from explicit target name or proposal id.
fn proposal_skill_name(proposal: &SkillExperienceProposalRecord) -> String {
    proposal
        .target_skill_name
        .as_deref()
        .map(slugify)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| slugify(&proposal.proposal_id))
}

/// Resolve model-facing identity using a fixed precedence chain.
///
/// Explicit `target_skill_name` wins, then semantic tokens from procedure text,
/// then bounded summary text, and finally a deterministic proposal-id fallback.
fn materialized_skill_identity(
    proposal: &SkillExperienceProposalRecord,
) -> MaterializedSkillIdentity {
    if let Some(name) = proposal
        .target_skill_name
        .as_deref()
        .map(slugify)
        .filter(|name| !name.is_empty())
    {
        return MaterializedSkillIdentity {
            name,
            source: "target_skill_name",
            used_fallback: false,
        };
    }

    if let Some(name) = semantic_skill_name_from_text(&proposal.reusable_procedure) {
        return MaterializedSkillIdentity {
            name,
            source: "reusable_procedure",
            used_fallback: false,
        };
    }

    if let Some(name) = semantic_skill_name_from_text(&proposal.bounded_summary) {
        return MaterializedSkillIdentity {
            name,
            source: "bounded_summary",
            used_fallback: false,
        };
    }

    MaterializedSkillIdentity {
        name: proposal_skill_name(proposal),
        source: "proposal_id_fallback",
        used_fallback: true,
    }
}

/// Extract a short deterministic slug from sanitized proposal evidence.
///
/// This is intentionally a local Specification rather than an LLM call.  The
/// materialization service must be reproducible, provider-neutral, and safe to
/// run during unattended autonomous operation.
fn semantic_skill_name_from_text(value: &str) -> Option<String> {
    super::identity_vocabulary::semantic_skill_name_from_text(value)
}
