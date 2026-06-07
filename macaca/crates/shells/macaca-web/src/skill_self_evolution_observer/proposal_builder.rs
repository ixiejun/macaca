//! Builder: assemble provider-neutral `SkillExperienceProposalCommand` payloads.

use std::collections::BTreeMap;

use macaca_proto::{ApplicationId, TraceContext};
use macaca_runtime_host::executor::TaskResult;
use macaca_sdk::skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceEvidenceGateStatus,
    SkillExperienceProposalCommand, SkillServiceScope,
};

use super::projection::bounded_artifact_ref;
use super::semantic_signal::SemanticSkillCreatorSignal;
use super::types::MAX_ARTIFACT_REFS;

/// Build the provider-neutral command sent to the Skill service.
///
/// The command uses only stable identifiers, bounded counts, and event-log
/// references.  It deliberately avoids raw prompts, raw model payloads, raw task
/// output, and full artifact contents so the proposal remains safe to display,
/// score, and replay through governance tooling.
pub(crate) fn build_skill_experience_proposal_command(
    app_id: &ApplicationId,
    session_id: &str,
    agent: &str,
    result: &TaskResult,
    agent_execution_trace_id: &str,
) -> Option<SkillExperienceProposalCommand> {
    let task_id = result.task_id.to_string();

    if result.output.trim().is_empty() && result.artifacts.is_empty() {
        tracing::debug!(
            task_id = %result.task_id,
            "Skipping skill self-evolution proposal without output or artifact evidence"
        );
        return None;
    }

    let trace = proposal_trace(app_id, session_id, agent, result);
    let scope = SkillServiceScope {
        application_id: Some(*app_id),
        session_id: Some(session_id.to_string()),
        tenant_id: None,
        agent_name: Some(agent.to_string()),
    };
    let semantic_signal = SemanticSkillCreatorSignal::from_task_result(result);

    let evidence_ids = vec![
        format!(
            "eventlog://sessions/{}/skill_self_evolution_observer/agent_execution_completed_seen/{}",
            session_id, result.task_id
        ),
        format!("trace://service.agent_execution/{}", agent_execution_trace_id),
    ];

    let mut metadata = BTreeMap::new();
    metadata.insert("origin".into(), "agent_execution_result_observer".into());
    metadata.insert("evidence_ref.event_log".into(), evidence_ids[0].clone());
    metadata.insert(
        "evidence_ref.agent_execution_trace".into(),
        evidence_ids[1].clone(),
    );
    metadata.insert(
        "output_char_count".into(),
        result.output.chars().count().to_string(),
    );
    metadata.insert("artifact_count".into(), result.artifacts.len().to_string());
    metadata.insert("completed_at".into(), result.completed_at.to_rfc3339());
    if let Some(tokens) = &result.tokens_used {
        metadata.insert("total_tokens".into(), tokens.total_tokens.to_string());
    }
    for (index, artifact) in result.artifacts.iter().take(MAX_ARTIFACT_REFS).enumerate() {
        metadata.insert(
            format!("evidence_ref.artifact_{}", index),
            bounded_artifact_ref(artifact),
        );
    }

    tracing::info!(
        task_id = %result.task_id,
        semantic_target = semantic_signal.target_skill_name.as_deref().unwrap_or(""),
        semantic_phrase_count = semantic_signal.trigger_phrases.len(),
        semantic_signal_fallback = semantic_signal.target_skill_name.is_none(),
        "Skill self-evolution observer built Skill Creator-compatible semantic trigger signal"
    );

    Some(SkillExperienceProposalCommand {
        trace,
        scope,
        candidate: SkillExperienceCandidate {
            task_id,
            session_id: Some(session_id.to_string()),
            application_id: Some(*app_id),
            agent_name: Some(agent.to_string()),
            verified_terminal_success: true,
            evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
            bounded_summary: semantic_signal.bounded_summary(format!(
                "Verified terminal task completion observed through service.agent_execution; output_chars={}, artifact_count={}, token_total={}.",
                result.output.chars().count(),
                result.artifacts.len(),
                result
                    .tokens_used
                    .as_ref()
                    .map(|tokens| tokens.total_tokens.to_string())
                    .unwrap_or_else(|| "unavailable".to_string())
            )),
            trace_digest: Some(format!(
                "session={},task={},agent={},agent_execution_trace={},completed_at={}",
                session_id,
                result.task_id,
                agent,
                agent_execution_trace_id,
                result.completed_at.to_rfc3339()
            )),
            memory_digest_refs: Vec::new(),
            reusable_procedure: semantic_signal.reusable_procedure(),
            classification: SkillEvolutionCandidateClassification::ReusableProcedure,
            destination: SkillExperienceCandidateDestination::NewSkillDraft,
            recommended_action: SkillEvolutionProposalAction::CreateDraft,
            target_skill_id: None,
            target_skill_name: semantic_signal.target_skill_name,
            evidence_ids,
            metadata,
        },
    })
}

/// Create the trace context attached to the Skill service command.
fn proposal_trace(
    app_id: &ApplicationId,
    session_id: &str,
    agent: &str,
    result: &TaskResult,
) -> TraceContext {
    let mut trace = TraceContext::new(format!(
        "skill-self-evolution:{}:{}:{}:{}",
        app_id.0, session_id, agent, result.task_id
    ));
    trace.session_id = Some(session_id.to_string());
    trace.task_id = Some(result.task_id.to_string());
    trace.agent = Some(agent.to_string());
    trace
}
