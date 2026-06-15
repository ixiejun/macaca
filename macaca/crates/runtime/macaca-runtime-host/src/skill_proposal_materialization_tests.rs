use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceEvidenceGateStatus,
    SkillExperienceProposalCommand, SkillExperienceProposalSnapshotCommand,
    SkillExperienceProposalSnapshotResult, SkillGovernanceSnapshotCommand,
    SkillGovernanceSnapshotResult, SkillPackageOwnershipClass, SkillProposalMaterializationCommand,
    SkillProposalMaterializationResult, SkillProposalMaterializationStatus,
    SkillProposalProcessingRunCommand, SkillProposalProcessingRunResult,
    SkillProposalProcessingState, SkillServicePolicyHints, SkillServiceScope,
    SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND, SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
    SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND, SKILL_EVOLUTION_SNAPSHOT_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
};

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test payload should serialize"),
        trace,
    )
}

fn candidate(task_id: &str, evidence_id: &str) -> SkillExperienceCandidate {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "evidence_ref.artifact_0".into(),
        format!("tool:file_write:{task_id}"),
    );
    SkillExperienceCandidate {
        task_id: task_id.into(),
        session_id: Some("session-materialization".into()),
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "Verified terminal task completion produced a reusable maintenance procedure.".into(),
        trace_digest: Some(format!("trace-digest://{task_id}")),
        memory_digest_refs: Vec::new(),
        reusable_procedure: "Inspect the service-owned evidence, identify the reusable workflow, verify the output artifact, and record the governed follow-up as a skill draft.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some("governed-materialization-skill".into()),
        evidence_ids: vec![evidence_id.into()],
        metadata,
    }
}

fn semantic_identity_candidate(task_id: &str, evidence_id: &str) -> SkillExperienceCandidate {
    let mut candidate = candidate(task_id, evidence_id);
    candidate.target_skill_name = None;
    candidate.bounded_summary =
        "Verified materialization registry load-path and usage telemetry proof.".into();
    candidate.reusable_procedure = "Verify materialization output, inspect registry load path projection, confirm usage telemetry increments, and record bounded governance evidence.".into();
    candidate
}

fn approved_policy() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec![
            "skill.proposal_processing".into(),
            "skill.proposal_materialization".into(),
            "skill.mutation".into(),
        ],
        entitlement_ready: Some(true),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

async fn create_proposal(
    provider: &SkillSystemServiceProvider,
    task_id: &str,
    evidence_id: &str,
) -> String {
    let trace = TraceContext::new(format!("trace-{task_id}"));
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: candidate(task_id, evidence_id),
            },
            trace,
        ))
        .await
        .expect("verified task evidence should create a proposal");
    let snapshot = provider
        .call(traced_command(
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            SkillExperienceProposalSnapshotCommand {
                trace: TraceContext::new(format!("trace-{task_id}-snapshot")),
                scope: SkillServiceScope::default(),
                include_discarded: false,
            },
            TraceContext::new(format!("trace-{task_id}-snapshot")),
        ))
        .await
        .expect("proposal snapshot should decode after creation");
    let snapshot: SkillExperienceProposalSnapshotResult =
        serde_json::from_value(snapshot.output).expect("proposal snapshot should decode");
    let _ = result;
    snapshot
        .proposals
        .into_iter()
        .find(|proposal| proposal.task_id == task_id)
        .expect("created proposal should appear in snapshot")
        .proposal_id
}

async fn create_semantic_identity_proposal(
    provider: &SkillSystemServiceProvider,
    task_id: &str,
    evidence_id: &str,
) -> String {
    let trace = TraceContext::new(format!("trace-{task_id}"));
    provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: semantic_identity_candidate(task_id, evidence_id),
            },
            trace,
        ))
        .await
        .expect("verified semantic task evidence should create a proposal");
    let snapshot = provider
        .call(traced_command(
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            SkillExperienceProposalSnapshotCommand {
                trace: TraceContext::new(format!("trace-{task_id}-snapshot")),
                scope: SkillServiceScope::default(),
                include_discarded: false,
            },
            TraceContext::new(format!("trace-{task_id}-snapshot")),
        ))
        .await
        .expect("proposal snapshot should decode after semantic proposal creation");
    let snapshot: SkillExperienceProposalSnapshotResult =
        serde_json::from_value(snapshot.output).expect("proposal snapshot should decode");
    snapshot
        .proposals
        .into_iter()
        .find(|proposal| proposal.task_id == task_id)
        .expect("created semantic proposal should appear in snapshot")
        .proposal_id
}

async fn mark_ready(provider: &SkillSystemServiceProvider) -> String {
    let trace = TraceContext::new("trace-materialization-processing");
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROCESSING_RUN_COMMAND,
            SkillProposalProcessingRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                min_ready_score: 40,
                evidence_ids: vec!["evidence://processing/materialization".into()],
                policy_decision_refs: vec!["policy://processing/approved".into()],
                audit_event_ids: vec!["audit://processing/materialization".into()],
                policy: approved_policy(),
            },
            trace,
        ))
        .await
        .expect("proposal processing apply should be accepted");
    let processed: SkillProposalProcessingRunResult =
        serde_json::from_value(result.output).expect("processing result should decode");
    processed
        .records
        .into_iter()
        .find(|record| record.state == SkillProposalProcessingState::ReadyForMaterialization)
        .expect("one proposal should be ready")
        .proposal_id
}

fn materialization_command(
    proposal_id: String,
    package_root: std::path::PathBuf,
    dry_run: bool,
) -> SkillProposalMaterializationCommand {
    SkillProposalMaterializationCommand {
        trace: TraceContext::new("trace-materialization-apply"),
        scope: SkillServiceScope::default(),
        proposal_id,
        package_root,
        dry_run,
        ownership: SkillPackageOwnershipClass::AgentPrivate,
        reason: "approved materialization from ready proposal".into(),
        evidence_ids: vec!["evidence://materialization/apply".into()],
        policy_decision_refs: vec!["policy://materialization/approved".into()],
        audit_event_ids: vec!["audit://materialization/apply".into()],
        policy: approved_policy(),
    }
}

#[tokio::test]
async fn materialization_denies_proposal_without_ready_processing_record() {
    let provider = SkillSystemServiceProvider::new();
    let proposal_id = create_proposal(
        &provider,
        "task-materialization-denied",
        "evidence://task/materialization-denied",
    )
    .await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let package_root = temp.path().join("denied-skill");
    tokio::fs::create_dir_all(&package_root)
        .await
        .expect("test package root should be created");
    let trace = TraceContext::new("trace-materialization-denied");

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
            materialization_command(proposal_id, package_root.clone(), false),
            trace,
        ))
        .await
        .expect("non-ready materialization should return structured denial");
    let result: SkillProposalMaterializationResult =
        serde_json::from_value(result.output).expect("materialization result should decode");

    assert_eq!(result.status, SkillProposalMaterializationStatus::Denied);
    assert!(!result.mutated);
    assert!(!package_root.join("SKILL.md").exists());
}

#[tokio::test]
async fn materialization_preview_and_apply_write_skill_md_then_promote_governance() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-materialization-ready",
        "evidence://task/materialization-ready",
    )
    .await;
    let proposal_id = mark_ready(&provider).await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let package_root = temp.path().join("ready-skill");
    tokio::fs::create_dir_all(&package_root)
        .await
        .expect("test package root should be created");

    let preview_trace = TraceContext::new("trace-materialization-preview");
    let preview = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
            materialization_command(proposal_id.clone(), package_root.clone(), true),
            preview_trace,
        ))
        .await
        .expect("ready materialization preview should be accepted");
    let preview: SkillProposalMaterializationResult =
        serde_json::from_value(preview.output).expect("preview result should decode");

    assert_eq!(
        preview.status,
        SkillProposalMaterializationStatus::Previewed
    );
    assert!(!preview.mutated);
    assert!(preview.content_digest.is_some());
    assert!(!package_root.join("SKILL.md").exists());

    let apply_trace = TraceContext::new("trace-materialization-apply");
    let applied = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
            materialization_command(proposal_id, package_root.clone(), false),
            apply_trace.clone(),
        ))
        .await
        .expect("ready materialization apply should be accepted");
    let applied: SkillProposalMaterializationResult =
        serde_json::from_value(applied.output).expect("apply result should decode");

    assert_eq!(applied.status, SkillProposalMaterializationStatus::Applied);
    assert!(applied.mutated);
    assert!(applied.promoted);
    assert!(applied.rollback_memento_ref.is_some());
    let skill_body = tokio::fs::read_to_string(package_root.join("SKILL.md"))
        .await
        .expect("materialization should write SKILL.md");
    assert!(skill_body.contains("name: governed-materialization-skill"));
    assert!(skill_body
        .contains("description: \"Use when a future task needs the governed reusable procedure"));
    assert!(skill_body.contains("Inspect the service-owned evidence"));
    assert!(
        !skill_body.contains("README.md"),
        "Skill Creator packages should avoid auxiliary documentation clutter"
    );
    assert!(
        !serde_json::to_string(&applied)
            .expect("materialization result should serialize")
            .contains("Inspect the service-owned evidence"),
        "result must not echo the generated SKILL.md body"
    );

    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: apply_trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            apply_trace,
        ))
        .await
        .expect("governance snapshot should reflect promoted skill");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("governance snapshot should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(
        snapshot.records[0].provenance.name,
        "governed-materialization-skill"
    );
}

#[tokio::test]
async fn semantic_materialized_skill_identity_prefers_reusable_procedure_over_proposal_id() {
    let provider = SkillSystemServiceProvider::new();
    create_semantic_identity_proposal(
        &provider,
        "1590b4d7-0aa7-4d87-ad40-14e222b2394b",
        "evidence://task/semantic-materialization",
    )
    .await;
    let proposal_id = mark_ready(&provider).await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let package_root = temp.path().join("semantic-skill");
    tokio::fs::create_dir_all(&package_root)
        .await
        .expect("test package root should be created");

    let preview_trace = TraceContext::new("trace-semantic-materialization-preview");
    let preview = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
            materialization_command(proposal_id, package_root, true),
            preview_trace,
        ))
        .await
        .expect("semantic materialization preview should be accepted");
    let preview: SkillProposalMaterializationResult =
        serde_json::from_value(preview.output).expect("preview result should decode");

    assert_eq!(
        preview.status,
        SkillProposalMaterializationStatus::Previewed
    );
    assert!(
        preview.skill_id.contains("materialization"),
        "semantic skill id should expose the reusable task domain"
    );
    assert!(
        preview.skill_id.contains("registry"),
        "semantic skill id should expose registry/load-path relevance"
    );
    assert!(
        preview.skill_id.contains("telemetry"),
        "semantic skill id should expose telemetry relevance"
    );
    assert!(
        !preview
            .skill_id
            .contains("1590b4d7-0aa7-4d87-ad40-14e222b2394b"),
        "proposal task UUID should remain provenance, not the model-facing trigger"
    );
}

#[tokio::test]
async fn restart_recovers_active_governance_record_from_materialized_package() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-restart-governance-recovery",
        "evidence://task/restart-governance-recovery",
    )
    .await;
    let proposal_id = mark_ready(&provider).await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let skills_root = temp.path().join("skills");
    let package_root = skills_root.join("restart-recovered-skill");
    tokio::fs::create_dir_all(&package_root)
        .await
        .expect("test package root should be created");
    let apply_trace = TraceContext::new("trace-restart-recovery-apply");

    provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND,
            materialization_command(proposal_id, package_root.clone(), false),
            apply_trace,
        ))
        .await
        .expect("ready materialization apply should be accepted");

    let restarted_provider =
        SkillSystemServiceProvider::new().with_materialized_skill_roots(vec![skills_root]);
    restarted_provider
        .start()
        .await
        .expect("provider startup recovery should complete");
    let snapshot_trace = TraceContext::new("trace-restart-recovery-snapshot");
    let snapshot = restarted_provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: snapshot_trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            snapshot_trace,
        ))
        .await
        .expect("governance snapshot should recover package-backed record");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("governance snapshot should decode");

    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(
        snapshot.records[0].provenance.name,
        "governed-materialization-skill"
    );
    assert_eq!(snapshot.telemetry_aggregate.record_count, 1);
    assert!(
        snapshot.records[0]
            .evidence_ids
            .iter()
            .any(|id| id.contains("skill-exp-")),
        "recovered governance record should retain proposal-linked evidence"
    );
}
