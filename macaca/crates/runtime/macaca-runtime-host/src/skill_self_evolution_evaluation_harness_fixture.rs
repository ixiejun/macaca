use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SelfEvolutionEvaluationCheckpoint, SelfEvolutionEvaluationCheckpointKind,
    SelfEvolutionEvaluationLifecycle, SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs,
    SelfEvolutionRunMetrics, SelfEvolutionWhiteBoxEvidence, SkillAuthorKind,
    SkillCurationRunCommand, SkillCurationRunResult, SkillCurationSnapshotCommand,
    SkillCurationSnapshotResult, SkillEvaluationCheckpointAppendCommand,
    SkillEvaluationCheckpointAppendResult, SkillEvolutionCandidateClassification,
    SkillEvolutionPromoteDraftCommand, SkillEvolutionPromoteDraftResult,
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand,
    SkillExperienceProposalResult, SkillGovernanceRecordUsageCommand, SkillServicePolicyHints,
    SkillServiceScope, SkillUsageEventKind, SkillUsageObservation, SKILL_CURATION_RUN_COMMAND,
    SKILL_CURATION_SNAPSHOT_COMMAND, SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
};

use crate::SkillSystemServiceProvider;

/// Service-owned evidence produced by the governed self-evolution chain.
///
/// Tests use these refs as the only source for evaluation checkpoints, which
/// prevents the harness from passing with hand-written application fixtures.
pub(crate) struct ServiceOwnedEvolutionEvidence {
    pub(crate) proposed: SkillExperienceProposalResult,
    pub(crate) curation: SkillCurationRunResult,
    pub(crate) promoted: SkillEvolutionPromoteDraftResult,
    pub(crate) snapshot: SkillCurationSnapshotResult,
    pub(crate) activation_ref: String,
}

/// Build a traced service command exactly as SDK and shell adapters do.
pub(crate) fn command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("service test payload must serialize"),
        trace,
    )
}

pub(crate) fn scope() -> SkillServiceScope {
    SkillServiceScope {
        application_id: None,
        session_id: Some("session-evaluation-harness".into()),
        tenant_id: Some("tenant-evaluation-harness".into()),
        agent_name: Some("agent".into()),
    }
}

fn verified_candidate() -> SkillExperienceCandidate {
    SkillExperienceCandidate {
        task_id: "task-evaluation-harness".into(),
        session_id: scope().session_id,
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "Verified task completion exposed a reusable procedure.".into(),
        trace_digest: Some("trace-digest://task/evaluation-harness".into()),
        memory_digest_refs: vec!["memory://digest/evaluation-harness".into()],
        reusable_procedure:
            "Reuse service-owned evidence refs to evaluate governed Skill self-evolution.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some("evaluation-harness-reusable-procedure".into()),
        evidence_ids: vec!["event://task/evaluation-harness/verified-completion".into()],
        metadata: BTreeMap::new(),
    }
}

fn policy_hints() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.evolution.promote".into()],
        entitlement_ready: Some(true),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

pub(crate) fn initial_record() -> SelfEvolutionEvaluationRecord {
    SelfEvolutionEvaluationRecord {
        evaluation_id: "eval-service-owned-self-evolution".into(),
        trace_id: "trace-service-owned-self-evolution".into(),
        task_family_id: "runtime_verification_loop".into(),
        lifecycle: SelfEvolutionEvaluationLifecycle::Prepared,
        white_box: SelfEvolutionWhiteBoxEvidence::default(),
        baseline: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 3,
            elapsed_seconds: Some(120),
            tool_call_count: 20,
            retry_count: 2,
            policy_violation_count: 0,
            skill_activation_count: 0,
            accepted_proposal_count: 0,
            total_proposal_count: 1,
            reuse_score: 0,
            regression_count: 0,
        },
        evolved: SelfEvolutionRunMetrics {
            completion_success: true,
            verified_artifact_count: 2,
            human_intervention_count: 1,
            elapsed_seconds: Some(90),
            tool_call_count: 16,
            retry_count: 1,
            policy_violation_count: 0,
            skill_activation_count: 1,
            accepted_proposal_count: 1,
            total_proposal_count: 1,
            reuse_score: 1,
            regression_count: 0,
        },
        report_refs: SelfEvolutionReportRefs::default(),
    }
}

pub(crate) async fn produce_service_owned_evidence(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
) -> ServiceOwnedEvolutionEvidence {
    let proposed = provider
        .call(command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: scope(),
                candidate: verified_candidate(),
            },
            trace.clone(),
        ))
        .await
        .expect("verified task evidence should create a durable proposal");
    let proposed: SkillExperienceProposalResult =
        serde_json::from_value(proposed.output).expect("proposal result should decode");

    let curation = provider
        .call(command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: scope(),
                dry_run: true,
                stale_after_days: 0,
                narrow_use_threshold: 0,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://curation/dry-run".into()],
                policy: SkillServicePolicyHints::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("curation dry-run should record a service-owned run id");
    let curation: SkillCurationRunResult =
        serde_json::from_value(curation.output).expect("curation result should decode");

    let promoted = provider
        .call(command(
            SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND,
            SkillEvolutionPromoteDraftCommand {
                trace: trace.clone(),
                scope: scope(),
                proposal_id: proposed.proposal.proposal_id.clone(),
                reason: "approved reusable evidence for evaluation harness verification".into(),
                evidence_ids: vec!["event://promotion/evaluation-harness".into()],
                policy_decision_refs: vec!["policy://decision/evaluation-harness".into()],
                policy: policy_hints(),
            },
            trace.clone(),
        ))
        .await
        .expect("promotion should produce active governance metadata");
    let promoted: SkillEvolutionPromoteDraftResult =
        serde_json::from_value(promoted.output).expect("promotion result should decode");

    let activation_ref = "event://activation/evaluation-harness".to_string();
    provider
        .call(command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: scope(),
                observation: SkillUsageObservation {
                    skill_id: promoted.record.provenance.skill_id.clone(),
                    name: promoted.record.provenance.name.clone(),
                    source: "service-owned-evaluation".into(),
                    source_scope: "workspace".into(),
                    event: SkillUsageEventKind::Activated,
                    author_kind: SkillAuthorKind::Agent,
                    created_by: Some("agent".into()),
                    pinned: None,
                    evidence_id: Some(activation_ref.clone()),
                    metadata: BTreeMap::new(),
                },
            },
            trace.clone(),
        ))
        .await
        .expect("later evolved skill activation should update governance telemetry");

    let snapshot = provider
        .call(command(
            SKILL_CURATION_SNAPSHOT_COMMAND,
            SkillCurationSnapshotCommand {
                trace: trace.clone(),
                scope: scope(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
                include_package_mementos: true,
            },
            trace.clone(),
        ))
        .await
        .expect("catalog visibility should be captured as a snapshot ref");
    let snapshot: SkillCurationSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");

    ServiceOwnedEvolutionEvidence {
        proposed,
        curation,
        promoted,
        snapshot,
        activation_ref,
    }
}

pub(crate) fn checkpoint(
    kind: SelfEvolutionEvaluationCheckpointKind,
) -> SelfEvolutionEvaluationCheckpoint {
    SelfEvolutionEvaluationCheckpoint {
        kind,
        ..SelfEvolutionEvaluationCheckpoint::default()
    }
}

/// Append one evaluation checkpoint through the Skill service command boundary.
pub(crate) async fn append_checkpoint(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    record: SelfEvolutionEvaluationRecord,
    checkpoint: SelfEvolutionEvaluationCheckpoint,
) -> SelfEvolutionEvaluationRecord {
    let result = provider
        .call(command(
            SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND,
            SkillEvaluationCheckpointAppendCommand {
                trace: trace.clone(),
                scope: scope(),
                record,
                checkpoint,
            },
            trace.clone(),
        ))
        .await
        .expect("checkpoint append should stay behind Skill service");
    let result: SkillEvaluationCheckpointAppendResult =
        serde_json::from_value(result.output).expect("checkpoint result should decode");
    result.record
}
