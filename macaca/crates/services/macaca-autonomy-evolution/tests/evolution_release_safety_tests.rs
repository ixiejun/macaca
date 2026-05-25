use macaca_autonomy_evolution::{
    AutonomyEvolutionService, BenchmarkDecision, EvolutionReleaseAction, EvolutionReleaseCommand,
    EvolutionReleasePolicyInput, EvolutionReleaseStatus, EvolutionRollbackMemento, EvolutionScope,
    EvolutionTargetType, EvolutionTrustLevel, InMemoryAutonomyEvolutionProvider,
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

fn memento(label: &str) -> EvolutionRollbackMemento {
    EvolutionRollbackMemento {
        memento_id: format!("memento-{label}"),
        target_type: EvolutionTargetType::SkillPackage,
        scope: scope(),
        state_ref: format!("memento://release/{label}/state"),
        evidence_refs: vec![format!("eventlog://release/{label}/memento")],
    }
}

fn safe_policy() -> EvolutionReleasePolicyInput {
    EvolutionReleasePolicyInput {
        capability_diff_refs: vec!["diff://release/generic-capability".into()],
        package_ownership_refs: vec!["ownership://release/generic-package".into()],
        resource_permission_refs: vec!["permission://release/generic-resource".into()],
        benchmark_decision: BenchmarkDecision::Passed,
        benchmark_refs: vec!["benchmark://release/generic".into()],
        rollback_mementos: vec![memento("safe")],
        trust_level: EvolutionTrustLevel::Trusted,
        executable_change: false,
        blast_radius_score: 20,
        canary_failure_reason_codes: Vec::new(),
    }
}

fn command(action: EvolutionReleaseAction) -> EvolutionReleaseCommand {
    EvolutionReleaseCommand {
        release_id: format!("release-{action:?}"),
        run_id: "run-release".into(),
        actor_id: "agent.release".into(),
        target_type: EvolutionTargetType::SkillPackage,
        action,
        trace: TraceContext::new("trace-release"),
        scope: scope(),
        policy_decision_refs: vec!["policy://release/allow".into()],
        audit_refs: vec!["audit://release/decision".into()],
        evidence_refs: vec!["eventlog://release/evidence".into()],
        policy: safe_policy(),
        dry_run: false,
    }
}

#[tokio::test]
async fn dry_run_accepts_without_side_effect_status() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut release = command(EvolutionReleaseAction::Promote);
    release.dry_run = true;

    let result = service.evaluate_release(release).await.unwrap();

    assert!(result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::DryRunAccepted);
    assert!(result.adapter_dispatch_required.is_none());
}

#[tokio::test]
async fn canary_release_passes_with_policy_refs_and_memento() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service
        .evaluate_release(command(EvolutionReleaseAction::StartCanary))
        .await
        .unwrap();

    assert!(result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::CanaryRunning);
    assert_eq!(result.rollback_refs, vec!["memento://release/safe/state"]);
    assert!(result.adapter_dispatch_required.unwrap_or(false));
}

#[tokio::test]
async fn canary_failure_rolls_back_with_memento() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut release = command(EvolutionReleaseAction::Rollback);
    release.policy.canary_failure_reason_codes = vec!["canary_error_rate_regressed".into()];

    let result = service.evaluate_release(release).await.unwrap();

    assert!(result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::RolledBack);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "rollback_memento_ready"));
}

#[tokio::test]
async fn rollback_without_memento_is_denied() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut release = command(EvolutionReleaseAction::Rollback);
    release.policy.rollback_mementos.clear();

    let result = service.evaluate_release(release).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "rollback_memento_missing"));
}

#[tokio::test]
async fn executable_high_blast_radius_promotion_is_denied() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut release = command(EvolutionReleaseAction::Promote);
    release.policy.executable_change = true;
    release.policy.blast_radius_score = 95;

    let result = service.evaluate_release(release).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "blast_radius_exceeds_release_threshold"));
}
