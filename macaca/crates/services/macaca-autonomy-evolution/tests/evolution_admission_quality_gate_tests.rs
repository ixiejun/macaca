use macaca_autonomy_evolution::{
    AdmissionDecision, AutonomyEvolutionService, EvolutionAdmissionCandidate,
    EvolutionAdmissionCommand, EvolutionScope, EvolutionTargetType,
    InMemoryAutonomyEvolutionProvider,
};
use macaca_proto::TraceContext;

fn base_candidate(name: &str) -> EvolutionAdmissionCandidate {
    EvolutionAdmissionCandidate {
        candidate_id: format!("candidate-{name}"),
        target_type: EvolutionTargetType::SkillPackage,
        package_name: name.into(),
        trigger_descriptions: vec![
            "Use when repeated task evidence shows this generic skill improves workflow reuse."
                .into(),
        ],
        skill_summary:
            "A focused, reusable skill candidate derived from bounded execution evidence.".into(),
        resource_categories: vec!["SKILL.md".into(), "references".into()],
        quick_validation_refs: vec!["validation://skills/generic/quick".into()],
        forward_test_refs: vec!["forward-test://skills/generic/task-family".into()],
        duplicate_candidate_refs: Vec::new(),
        metadata_stale: false,
    }
}

fn command(candidate: EvolutionAdmissionCandidate) -> EvolutionAdmissionCommand {
    EvolutionAdmissionCommand {
        actor_id: "agent.admission".into(),
        trace: TraceContext::new(format!("trace-admission-{}", candidate.candidate_id)),
        scope: EvolutionScope {
            application_id: Some("app.generic".into()),
            tenant_id: Some("tenant.generic".into()),
            session_id: Some("session.generic".into()),
            task_id: Some("task.generic".into()),
        },
        candidate,
        evidence_refs: vec!["eventlog://sessions/generic/evolution-candidate".into()],
        policy_decision_refs: vec!["policy://autonomy/admission/allow".into()],
    }
}

#[tokio::test]
async fn semantic_skill_candidate_is_accepted() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service
        .admit_candidate(command(base_candidate("reusable-task-evidence-summarizer")))
        .await
        .unwrap();

    assert_eq!(result.decision, AdmissionDecision::Accepted);
    assert!(result
        .findings
        .iter()
        .any(|finding| finding.gate == "semantic_package_name"));
    assert_eq!(
        result.trace.trace_id,
        "trace-admission-candidate-reusable-task-evidence-summarizer"
    );
}

#[tokio::test]
async fn meaningless_experiment_name_is_denied_with_sanitized_reason() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service
        .admit_candidate(command(base_candidate("skill-exp-12345")))
        .await
        .unwrap();

    assert_eq!(result.decision, AdmissionDecision::Denied);
    assert!(result
        .summary_reason
        .as_deref()
        .unwrap_or_default()
        .contains("semantic_package_name"));
    assert!(!result
        .summary_reason
        .as_deref()
        .unwrap_or_default()
        .contains("Use when repeated task evidence"));
}

#[tokio::test]
async fn missing_trigger_quality_needs_evidence() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut candidate = base_candidate("reusable-task-evidence-summarizer");
    candidate.trigger_descriptions.clear();

    let result = service.admit_candidate(command(candidate)).await.unwrap();

    assert_eq!(result.decision, AdmissionDecision::NeedsEvidence);
    assert!(result
        .missing_evidence
        .iter()
        .any(|item| item == "trigger_quality"));
}

#[tokio::test]
async fn duplicate_candidate_is_denied() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut candidate = base_candidate("reusable-task-evidence-summarizer");
    candidate.duplicate_candidate_refs = vec!["candidate://skills/existing-equivalent".into()];

    let result = service.admit_candidate(command(candidate)).await.unwrap();

    assert_eq!(result.decision, AdmissionDecision::Denied);
    assert!(result
        .findings
        .iter()
        .any(|finding| finding.gate == "duplicate_suppression"));
}

#[tokio::test]
async fn stale_metadata_requires_regeneration_evidence() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut candidate = base_candidate("reusable-task-evidence-summarizer");
    candidate.metadata_stale = true;

    let result = service.admit_candidate(command(candidate)).await.unwrap();

    assert_eq!(result.decision, AdmissionDecision::NeedsEvidence);
    assert!(result
        .missing_evidence
        .iter()
        .any(|item| item == "metadata_regeneration"));
}
