//! Built-in local Strategy for Skill proposal materialization.
//!
//! The Strategy is deliberately narrow: it accepts only proposals that the
//! proposal-processing lane has already marked `ReadyForMaterialization`, builds
//! bounded AgentSkills-compatible markdown, delegates the file write to the
//! existing content-mutation Strategy, and promotes governance metadata only
//! after the write succeeds.  It never branches on application or workflow
//! names, and it never logs or returns the generated `SKILL.md` body.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_proto::{ServiceCallResult, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{
    SkillContentMutationCommand, SkillContentMutationKind, SkillContentMutationStatus,
    SkillEvolutionPromoteDraftCommand, SkillEvolutionProposalLifecycle,
    SkillExperienceProposalRecord, SkillProposalMaterializationCommand,
    SkillProposalMaterializationDenied, SkillProposalMaterializationResult,
    SkillProposalMaterializationStatus, SkillProposalProcessingState,
};
use serde_json::Value;

use crate::skill_service_codec::{decode, service_result, to_value};
use crate::skill_service_content_mutation::LocalSkillContentMutationStrategy;
use crate::skill_service_provider_state::SkillProviderGovernanceState;

/// Decode and apply a traced proposal materialization command.
pub(crate) async fn apply_command(
    state: &Arc<SkillProviderGovernanceState>,
    content_mutation: &LocalSkillContentMutationStrategy,
    payload: Value,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    let typed: SkillProposalMaterializationCommand = decode(payload)?;
    let result = state
        .materialize_ready_proposal(typed.clone(), content_mutation)
        .await?;
    tracing::info!(
        trace_id = %typed.trace.trace_id,
        proposal_id = %typed.proposal_id,
        status = ?result.status,
        skill_id = %result.skill_id,
        planned_bytes = result.planned_bytes,
        bytes_written = result.bytes_written,
        rollback_ref_present = result.rollback_memento_ref.is_some(),
        mutated = result.mutated,
        promoted = result.promoted,
        evidence_refs = result.evidence_ids.len(),
        policy_decision_refs = result.policy_decision_refs.len(),
        audit_event_ids = result.audit_event_ids.len(),
        "skill proposal materialization command completed"
    );
    Ok(service_result(to_value(result)?, trace))
}

impl SkillProviderGovernanceState {
    /// Materialize one ready proposal through the local Builder and mutation Strategy.
    ///
    /// The method follows a strict side-effect order.  It first performs all
    /// service-owned readiness checks, then builds bounded content, then either
    /// returns a preview or writes through `SkillContentMutationCommand`.  Only
    /// after the mutation Strategy reports `Applied` does it promote proposal
    /// lifecycle into active governance metadata.
    pub(crate) async fn materialize_ready_proposal(
        &self,
        command: SkillProposalMaterializationCommand,
        content_mutation: &LocalSkillContentMutationStrategy,
    ) -> ServiceResult<SkillProposalMaterializationResult> {
        if let Err(denial) = command.validate() {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                proposal_id = %command.proposal_id,
                reason = %denial.reason,
                "skill proposal materialization denied by command validation"
            );
            return Ok(SkillProposalMaterializationResult::denied(&command, denial));
        }

        let proposal = match self.ready_materialization_proposal(&command).await {
            Ok(proposal) => proposal,
            Err(denial) => {
                tracing::warn!(
                    trace_id = %command.trace.trace_id,
                    proposal_id = %command.proposal_id,
                    reason = %denial.reason,
                    "skill proposal materialization denied by readiness gate"
                );
                return Ok(SkillProposalMaterializationResult::denied(&command, denial));
            }
        };
        let draft = SkillDraftMaterializationBuilder::new(&proposal).build();

        tracing::info!(
            trace_id = %command.trace.trace_id,
            proposal_id = %proposal.proposal_id,
            skill_id = %draft.skill_id,
            skill_name = %draft.name,
            identity_source = %draft.identity_source,
            identity_used_fallback = draft.identity_used_fallback,
            dry_run = command.dry_run,
            planned_bytes = draft.content.len(),
            "skill proposal materialization built bounded semantic draft identity"
        );

        if command.dry_run {
            return Ok(SkillProposalMaterializationResult {
                status: SkillProposalMaterializationStatus::Previewed,
                proposal_id: proposal.proposal_id,
                skill_id: draft.skill_id,
                relative_path: draft.relative_path,
                content_digest: Some(draft.content_digest),
                planned_bytes: draft.content.len() as u64,
                bytes_written: 0,
                rollback_memento_ref: None,
                denied_reason: None,
                mutated: false,
                promoted: false,
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                audit_event_ids: command.sanitized_audit_event_ids(),
                trace_id: command.trace.trace_id,
                captured_at: chrono::Utc::now(),
            });
        }

        let mutation = content_mutation
            .apply(SkillContentMutationCommand {
                trace: command.trace.clone(),
                scope: command.scope.clone(),
                skill_id: draft.skill_id.clone(),
                package_root: command.package_root.clone(),
                relative_path: draft.relative_path.clone(),
                kind: SkillContentMutationKind::CreateSkill,
                ownership: command.ownership.clone(),
                content: Some(draft.content.clone()),
                reason: command.reason.clone(),
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                allow_executable_script_mutation: false,
                policy: command.policy.clone(),
                metadata: materialization_mutation_metadata(&proposal, &draft),
            })
            .await?;

        if mutation.status != SkillContentMutationStatus::Applied {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                proposal_id = %proposal.proposal_id,
                skill_id = %draft.skill_id,
                mutation_status = ?mutation.status,
                denied_reason = mutation.denied_reason.as_deref().unwrap_or(""),
                "skill proposal materialization stopped after mutation denial"
            );
            return Ok(SkillProposalMaterializationResult {
                status: SkillProposalMaterializationStatus::Denied,
                proposal_id: proposal.proposal_id,
                skill_id: draft.skill_id,
                relative_path: draft.relative_path,
                content_digest: Some(draft.content_digest),
                planned_bytes: draft.content.len() as u64,
                bytes_written: 0,
                rollback_memento_ref: mutation.rollback_memento_ref,
                denied_reason: mutation.denied_reason,
                mutated: false,
                promoted: false,
                evidence_ids: mutation.evidence_ids,
                policy_decision_refs: mutation.policy_decision_refs,
                audit_event_ids: command.sanitized_audit_event_ids(),
                trace_id: command.trace.trace_id,
                captured_at: chrono::Utc::now(),
            });
        }

        self.prepare_proposal_materialization_target(&command, &draft)
            .await;
        let promotion = self
            .promote_draft(SkillEvolutionPromoteDraftCommand {
                trace: command.trace.clone(),
                scope: command.scope.clone(),
                proposal_id: command.proposal_id.clone(),
                reason: command.reason.clone(),
                evidence_ids: command.sanitized_evidence_ids(),
                policy_decision_refs: command.sanitized_policy_decision_refs(),
                policy: command.policy.clone(),
            })
            .await
            .map_err(ServiceError::InvalidArgument)?;

        tracing::info!(
            trace_id = %command.trace.trace_id,
            proposal_id = %promotion.proposal.proposal_id,
            skill_id = %promotion.record.provenance.skill_id,
            bytes_written = mutation.bytes_written,
            rollback_ref_present = mutation.rollback_memento_ref.is_some(),
            "skill proposal materialization promoted governance after write"
        );

        Ok(SkillProposalMaterializationResult {
            status: SkillProposalMaterializationStatus::Applied,
            proposal_id: promotion.proposal.proposal_id,
            skill_id: draft.skill_id,
            relative_path: draft.relative_path,
            content_digest: Some(draft.content_digest),
            planned_bytes: draft.content.len() as u64,
            bytes_written: mutation.bytes_written,
            rollback_memento_ref: mutation.rollback_memento_ref,
            denied_reason: None,
            mutated: true,
            promoted: true,
            evidence_ids: promotion.evidence_ids,
            policy_decision_refs: promotion.policy_decision_refs,
            audit_event_ids: command.sanitized_audit_event_ids(),
            trace_id: command.trace.trace_id,
            captured_at: chrono::Utc::now(),
        })
    }

    /// Return the proposal only if processing has explicitly marked it ready.
    async fn ready_materialization_proposal(
        &self,
        command: &SkillProposalMaterializationCommand,
    ) -> Result<SkillExperienceProposalRecord, SkillProposalMaterializationDenied> {
        let proposals = self.proposals.lock().await;
        let proposal = proposals
            .get(command.proposal_id.trim())
            .cloned()
            .ok_or_else(|| {
                SkillProposalMaterializationDenied::new(
                    "skill proposal materialization requires an existing proposal",
                    false,
                )
            })?;
        drop(proposals);

        if proposal.lifecycle != SkillEvolutionProposalLifecycle::Draft {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires draft proposal lifecycle",
                false,
            ));
        }

        let processing = self.proposal_processing.lock().await;
        let Some(record) = processing.get(command.proposal_id.trim()) else {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal materialization requires ready processing record",
                false,
            ));
        };
        if record.state != SkillProposalProcessingState::ReadyForMaterialization {
            return Err(SkillProposalMaterializationDenied::new(
                "skill proposal processing state is not ready for materialization",
                false,
            ));
        }
        Ok(proposal)
    }

    /// Store target identity refs on the proposal before promotion reads it.
    async fn prepare_proposal_materialization_target(
        &self,
        command: &SkillProposalMaterializationCommand,
        draft: &SkillDraftMaterialization,
    ) {
        let mut proposals = self.proposals.lock().await;
        if let Some(proposal) = proposals.get_mut(command.proposal_id.trim()) {
            proposal.target_skill_id = Some(draft.skill_id.clone());
            proposal.target_skill_name = Some(draft.name.clone());
            proposal.metadata.insert(
                "materialization_ref".into(),
                draft.materialization_ref.clone(),
            );
            proposal
                .metadata
                .insert("content_digest_ref".into(), draft.content_digest.clone());
        }
    }
}

/// Bounded materialized draft produced by the Builder.
struct SkillDraftMaterialization {
    skill_id: String,
    name: String,
    identity_source: &'static str,
    identity_used_fallback: bool,
    relative_path: PathBuf,
    content: Vec<u8>,
    content_digest: String,
    materialization_ref: String,
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
struct SkillDraftMaterializationBuilder<'a> {
    proposal: &'a SkillExperienceProposalRecord,
}

impl<'a> SkillDraftMaterializationBuilder<'a> {
    fn new(proposal: &'a SkillExperienceProposalRecord) -> Self {
        Self { proposal }
    }

    fn build(&self) -> SkillDraftMaterialization {
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

fn materialization_mutation_metadata(
    proposal: &SkillExperienceProposalRecord,
    draft: &SkillDraftMaterialization,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("proposal_id".into(), proposal.proposal_id.clone());
    metadata.insert("task_id".into(), proposal.task_id.clone());
    metadata.insert("content_digest_ref".into(), draft.content_digest.clone());
    metadata
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

fn proposal_skill_name(proposal: &SkillExperienceProposalRecord) -> String {
    proposal
        .target_skill_name
        .as_deref()
        .map(slugify)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| slugify(&proposal.proposal_id))
}

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
/// run during unattended autonomous operation.  Stop words remove generic
/// evidence language, while the remaining tokens preserve task concepts that
/// make model-facing Skill selection possible.
fn semantic_skill_name_from_text(value: &str) -> Option<String> {
    const MAX_TOKENS: usize = 8;
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_semantic_token(&mut tokens, &current, MAX_TOKENS);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_semantic_token(&mut tokens, &current, MAX_TOKENS);
    }

    if tokens.len() < 2 {
        return None;
    }
    Some(slugify(&tokens.join("-")))
}

fn push_semantic_token(tokens: &mut Vec<String>, token: &str, max_tokens: usize) {
    if tokens.len() >= max_tokens || token.len() < 4 || is_identity_stop_word(token) {
        return;
    }
    if token.chars().all(|character| character.is_ascii_digit()) {
        return;
    }
    if !tokens.iter().any(|existing| existing == token) {
        tokens.push(token.to_string());
    }
}

fn is_identity_stop_word(token: &str) -> bool {
    matches!(
        token,
        "this"
            | "that"
            | "when"
            | "with"
            | "from"
            | "into"
            | "future"
            | "task"
            | "tasks"
            | "skill"
            | "skills"
            | "agent"
            | "agents"
            | "verified"
            | "evidence"
            | "record"
            | "records"
            | "bounded"
            | "reusable"
            | "procedure"
            | "procedures"
            | "output"
            | "verify"
            | "inspect"
            | "confirm"
            | "artifact"
            | "artifacts"
            | "service"
            | "owned"
            | "follow"
            | "followup"
    )
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
        if slug.len() >= 80 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "materialized-skill".into()
    } else {
        slug
    }
}

fn bounded_line(value: &str, max_chars: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn bounded_block(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn yaml_quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn stable_content_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("skill-draft-digest://fnv1a64:{hash:016x}")
}
