use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ApplicationId, ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillAutonomousMaterializationRunCommand, SkillAutonomousMaterializationRunResult,
    SkillAutonomousMaterializationStatus, SkillEvolutionCandidateClassification,
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand, SkillPackageOwnershipClass,
    SkillProposalMaterializationStatus, SkillServicePolicyHints, SkillServiceScope,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
    SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
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

fn candidate(
    task_id: &str,
    evidence_id: &str,
    target_skill_name: &str,
) -> SkillExperienceCandidate {
    candidate_for_application(task_id, evidence_id, target_skill_name, None)
}

fn candidate_for_application(
    task_id: &str,
    evidence_id: &str,
    target_skill_name: &str,
    application_id: Option<ApplicationId>,
) -> SkillExperienceCandidate {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "evidence_ref.artifact_0".into(),
        format!("tool:file_write:{task_id}"),
    );
    SkillExperienceCandidate {
        task_id: task_id.into(),
        session_id: Some("session-materialization-operator".into()),
        application_id,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary:
            "Verified terminal task completion produced a reusable maintenance procedure.".into(),
        trace_digest: Some(format!("trace-digest://{task_id}")),
        memory_digest_refs: Vec::new(),
        reusable_procedure: "Inspect service-owned evidence, extract the reusable procedure, verify artifact refs, and record governed follow-up as a reusable skill draft.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some(target_skill_name.into()),
        evidence_ids: vec![evidence_id.into()],
        metadata,
    }
}

fn approved_policy() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec![
            "skill.proposal_processing".into(),
            "skill.proposal_materialization_operator".into(),
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
    target_skill_name: &str,
) {
    let trace = TraceContext::new(format!("trace-{task_id}"));
    provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                candidate: candidate(task_id, evidence_id, target_skill_name),
            },
            trace,
        ))
        .await
        .expect("verified task evidence should create a proposal");
}

async fn create_app_proposal(
    provider: &SkillSystemServiceProvider,
    application_id: ApplicationId,
    task_id: &str,
    evidence_id: &str,
    target_skill_name: &str,
) {
    let trace = TraceContext::new(format!("trace-{task_id}"));
    provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope: SkillServiceScope {
                    application_id: Some(application_id),
                    session_id: None,
                    tenant_id: None,
                    agent_name: None,
                },
                candidate: candidate_for_application(
                    task_id,
                    evidence_id,
                    target_skill_name,
                    Some(application_id),
                ),
            },
            trace,
        ))
        .await
        .expect("app-scoped task evidence should create a proposal");
}

fn operator_command(
    package_collection_root: std::path::PathBuf,
    dry_run: bool,
    batch_limit: u16,
) -> SkillAutonomousMaterializationRunCommand {
    SkillAutonomousMaterializationRunCommand {
        trace: TraceContext::new("trace-materialization-operator"),
        scope: SkillServiceScope::default(),
        dry_run,
        batch_limit,
        min_ready_score: 40,
        package_collection_root,
        ownership: SkillPackageOwnershipClass::AgentPrivate,
        reason: "approved autonomous materialization operator run".into(),
        evidence_ids: vec!["evidence://operator/run".into()],
        policy_decision_refs: vec!["policy://operator/approved".into()],
        audit_event_ids: vec!["audit://operator/run".into()],
        policy: approved_policy(),
    }
}

fn app_operator_command(
    application_id: ApplicationId,
    package_collection_root: std::path::PathBuf,
    dry_run: bool,
    batch_limit: u16,
) -> SkillAutonomousMaterializationRunCommand {
    let mut command = operator_command(package_collection_root, dry_run, batch_limit);
    command.scope.application_id = Some(application_id);
    command
}

#[tokio::test]
async fn autonomous_operator_dry_run_previews_ready_materialization_without_file_mutation() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-operator-preview",
        "evidence://task/operator-preview",
        "operator-preview-skill",
    )
    .await;
    let temp = tempfile::tempdir().expect("temp dir should be available");

    let trace = TraceContext::new("trace-materialization-operator-preview");
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            operator_command(temp.path().join("skills"), true, 1),
            trace,
        ))
        .await
        .expect("dry-run operator should return a structured preview");
    let result: SkillAutonomousMaterializationRunResult =
        serde_json::from_value(result.output).expect("operator result should decode");

    assert_eq!(
        result.status,
        SkillAutonomousMaterializationStatus::Previewed
    );
    assert!(!result.mutated);
    assert_eq!(result.processing.state_counts.ready_for_materialization, 1);
    assert_eq!(result.materializations.len(), 1);
    assert_eq!(
        result.materializations[0].status,
        SkillProposalMaterializationStatus::Previewed
    );
    assert!(!temp
        .path()
        .join("skills/operator-preview-skill/SKILL.md")
        .exists());
}

#[tokio::test]
async fn autonomous_operator_apply_materializes_ready_proposal_after_processing() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-operator-apply",
        "evidence://task/operator-apply",
        "operator-applied-skill",
    )
    .await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let collection_root = temp.path().join("skills");

    let trace = TraceContext::new("trace-materialization-operator-apply");
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            operator_command(collection_root.clone(), false, 1),
            trace,
        ))
        .await
        .expect("apply operator should materialize a ready proposal");
    let result: SkillAutonomousMaterializationRunResult =
        serde_json::from_value(result.output).expect("operator result should decode");

    assert_eq!(result.status, SkillAutonomousMaterializationStatus::Applied);
    assert!(result.mutated);
    assert_eq!(result.materializations.len(), 1);
    assert_eq!(
        result.materializations[0].status,
        SkillProposalMaterializationStatus::Applied
    );
    assert!(
        collection_root
            .join("operator-applied-skill")
            .join("SKILL.md")
            .exists(),
        "operator should resolve a proposal-scoped package root and write SKILL.md"
    );
}

#[tokio::test]
async fn autonomous_operator_filters_processing_backlog_by_application_scope() {
    let provider = SkillSystemServiceProvider::new();
    let target_app = ApplicationId(
        uuid::Uuid::parse_str("aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa")
            .expect("test app id should parse"),
    );
    let other_app = ApplicationId(
        uuid::Uuid::parse_str("bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb")
            .expect("test app id should parse"),
    );
    create_app_proposal(
        &provider,
        other_app,
        "task-other-app",
        "evidence://task/other-app",
        "other-app-skill",
    )
    .await;
    create_app_proposal(
        &provider,
        target_app,
        "task-target-app",
        "evidence://task/target-app",
        "target-app-skill",
    )
    .await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let collection_root = temp.path().join("skills");

    let trace = TraceContext::new("trace-materialization-operator-target-app");
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            app_operator_command(target_app, collection_root.clone(), false, 5),
            trace,
        ))
        .await
        .expect("app-scoped operator should run");
    let result: SkillAutonomousMaterializationRunResult =
        serde_json::from_value(result.output).expect("operator result should decode");

    assert_eq!(result.status, SkillAutonomousMaterializationStatus::Applied);
    assert_eq!(
        result.selected_proposal_ids.len(),
        1,
        "operator must not select proposals from another application scope"
    );
    assert!(
        result.selected_proposal_ids[0].contains("task-target-app"),
        "selected proposal should be linked to the target application"
    );
    assert!(
        collection_root
            .join("target-app-skill")
            .join("SKILL.md")
            .exists(),
        "target application proposal should be materialized"
    );
    assert!(
        !collection_root
            .join("other-app-skill")
            .join("SKILL.md")
            .exists(),
        "other application proposal must not be materialized into target app root"
    );
}

#[tokio::test]
async fn autonomous_operator_skips_promoted_ready_records_when_selecting_next_draft() {
    let provider = SkillSystemServiceProvider::new();
    create_proposal(
        &provider,
        "task-first-promoted",
        "evidence://task/first-promoted",
        "first-promoted-skill",
    )
    .await;
    let temp = tempfile::tempdir().expect("temp dir should be available");
    let collection_root = temp.path().join("skills");

    let first = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            operator_command(collection_root.clone(), false, 1),
            TraceContext::new("trace-materialization-operator-first-promoted"),
        ))
        .await
        .expect("first operator run should promote the first proposal");
    let first: SkillAutonomousMaterializationRunResult =
        serde_json::from_value(first.output).expect("first operator result should decode");
    assert_eq!(first.status, SkillAutonomousMaterializationStatus::Applied);

    create_proposal(
        &provider,
        "task-second-draft",
        "evidence://task/second-draft",
        "second-draft-skill",
    )
    .await;
    let second = provider
        .call(traced_command(
            SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
            operator_command(collection_root.clone(), false, 1),
            TraceContext::new("trace-materialization-operator-second-draft"),
        ))
        .await
        .expect("second operator run should skip promoted records");
    let second: SkillAutonomousMaterializationRunResult =
        serde_json::from_value(second.output).expect("second operator result should decode");

    assert_eq!(second.status, SkillAutonomousMaterializationStatus::Applied);
    assert_eq!(second.materializations.len(), 1);
    assert!(
        second.selected_proposal_ids[0].contains("task-second-draft"),
        "operator must select the fresh Draft proposal, not the already Promoted one"
    );
    assert!(
        collection_root
            .join("second-draft-skill")
            .join("SKILL.md")
            .exists(),
        "second Draft proposal should be materialized after promoted records are skipped"
    );
}
