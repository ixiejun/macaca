use macaca_autonomy_evolution::{
    AutonomyEvolutionService, EvolutionRunState, EvolutionRunTransition, EvolutionScope,
    EvolutionTargetType, EvolutionTransitionCommand, InMemoryAutonomyEvolutionProvider,
};
use macaca_proto::TraceContext;

fn scope() -> EvolutionScope {
    EvolutionScope {
        application_id: Some("app.generic".into()),
        tenant_id: Some("tenant.generic".into()),
        session_id: Some("session.generic".into()),
        task_id: Some("task.generic".into()),
    }
}

fn command(
    run_id: &str,
    from_state: Option<EvolutionRunState>,
    transition: EvolutionRunTransition,
) -> EvolutionTransitionCommand {
    EvolutionTransitionCommand {
        run_id: run_id.into(),
        from_state,
        transition,
        target_type: EvolutionTargetType::SkillPackage,
        scope: scope(),
        actor_id: "agent.coordinator".into(),
        trace: TraceContext::new(format!("trace.{run_id}")),
        evidence_refs: vec![format!("eventlog://sessions/{run_id}/observer")],
        policy_decision_refs: Vec::new(),
        audit_refs: Vec::new(),
        rollback_refs: Vec::new(),
        diagnostics: Default::default(),
    }
}

#[tokio::test]
async fn control_plane_records_valid_forward_transitions() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let queued = service
        .transition(command(
            "run-forward",
            None,
            EvolutionRunTransition::QueueCandidate,
        ))
        .await
        .unwrap();
    assert!(queued.accepted);
    assert_eq!(queued.current_state, EvolutionRunState::CandidateQueued);

    let classified = service
        .transition(command(
            "run-forward",
            Some(EvolutionRunState::CandidateQueued),
            EvolutionRunTransition::ClassifyCandidate,
        ))
        .await
        .unwrap();
    assert!(classified.accepted);
    assert_eq!(
        classified.current_state,
        EvolutionRunState::CandidateClassified
    );
    assert_eq!(
        classified.previous_state,
        Some(EvolutionRunState::CandidateQueued)
    );
}

#[tokio::test]
async fn control_plane_denies_invalid_transition_without_mutating_state() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let created = service
        .transition(command(
            "run-invalid",
            None,
            EvolutionRunTransition::Observe,
        ))
        .await
        .unwrap();
    assert_eq!(created.current_state, EvolutionRunState::Observed);

    let denied = service
        .transition(command(
            "run-invalid",
            Some(EvolutionRunState::Observed),
            EvolutionRunTransition::Promote,
        ))
        .await
        .unwrap();
    assert!(!denied.accepted);
    assert_eq!(denied.current_state, EvolutionRunState::Observed);
    assert!(denied
        .denial_reason
        .as_deref()
        .unwrap_or_default()
        .contains("invalid transition"));
}

#[tokio::test]
async fn side_effecting_transition_requires_policy_and_audit_refs() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    for transition in [
        EvolutionRunTransition::Observe,
        EvolutionRunTransition::QueueCandidate,
        EvolutionRunTransition::ClassifyCandidate,
        EvolutionRunTransition::GenerateProposal,
        EvolutionRunTransition::StartAdmissionReview,
        EvolutionRunTransition::Quarantine,
        EvolutionRunTransition::PrepareBenchmark,
        EvolutionRunTransition::RecordBaseline,
        EvolutionRunTransition::RecordCandidateMeasurement,
        EvolutionRunTransition::StartCanary,
    ] {
        let current = service
            .transition(command("run-policy", None, transition))
            .await
            .unwrap()
            .current_state;
        let mut next = command(
            "run-policy",
            Some(current.clone()),
            EvolutionRunTransition::Promote,
        );
        next.from_state = Some(current);
        let denied = service.transition(next).await.unwrap();
        if denied.current_state == EvolutionRunState::CanaryRunning {
            assert!(!denied.accepted);
            assert!(denied
                .denial_reason
                .as_deref()
                .unwrap_or_default()
                .contains("policy decision"));
            break;
        }
    }
}

#[tokio::test]
async fn rollback_with_policy_evidence_is_accepted() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let mut state = EvolutionRunState::Observed;
    for transition in [
        EvolutionRunTransition::QueueCandidate,
        EvolutionRunTransition::ClassifyCandidate,
        EvolutionRunTransition::GenerateProposal,
        EvolutionRunTransition::StartAdmissionReview,
        EvolutionRunTransition::Quarantine,
        EvolutionRunTransition::PrepareBenchmark,
        EvolutionRunTransition::RecordBaseline,
        EvolutionRunTransition::RecordCandidateMeasurement,
        EvolutionRunTransition::StartCanary,
        EvolutionRunTransition::Promote,
        EvolutionRunTransition::StartActiveMonitoring,
    ] {
        let mut next = command("run-rollback", Some(state), transition);
        next.policy_decision_refs = vec!["policy://allow/evolution".into()];
        next.audit_refs = vec!["audit://evolution/run-rollback".into()];
        let result = service.transition(next).await.unwrap();
        assert!(result.accepted, "{:?}", result.denial_reason);
        state = result.current_state;
    }

    let mut rollback = command(
        "run-rollback",
        Some(EvolutionRunState::ActiveMonitoring),
        EvolutionRunTransition::Rollback,
    );
    rollback.policy_decision_refs = vec!["policy://allow/rollback".into()];
    rollback.audit_refs = vec!["audit://evolution/run-rollback".into()];
    rollback.rollback_refs = vec!["memento://evolution/run-rollback/before".into()];

    let result = service.transition(rollback).await.unwrap();
    assert!(result.accepted);
    assert_eq!(result.current_state, EvolutionRunState::RolledBack);
    assert!(result.adapter_dispatch_required);
}
