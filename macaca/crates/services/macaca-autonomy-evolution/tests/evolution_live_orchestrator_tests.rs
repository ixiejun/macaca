use macaca_autonomy_evolution::{
    AutonomyEvolutionService, BenchmarkDecision, EvolutionAdmissionCandidate,
    EvolutionBenchmarkMeasurement, EvolutionBenchmarkMetrics, EvolutionLiveAdapterDispatch,
    EvolutionLiveAuditCommand, EvolutionLivePhase, EvolutionLivePhaseStatus,
    EvolutionLiveTickCommand, EvolutionReleaseAction, EvolutionReleasePolicyInput,
    EvolutionRollbackMemento, EvolutionScope, EvolutionTargetType, EvolutionTrustLevel,
    InMemoryAutonomyEvolutionProvider,
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

fn candidate(target_type: EvolutionTargetType) -> EvolutionAdmissionCandidate {
    EvolutionAdmissionCandidate {
        candidate_id: "candidate-live-generic".into(),
        target_type,
        package_name: "generic-live-skill".into(),
        trigger_descriptions: vec![
            "Use when a generic repeated task family benefits from reuse".into(),
        ],
        skill_summary: "A focused generic skill candidate produced from bounded observer evidence."
            .into(),
        resource_categories: vec!["skill.md".into()],
        quick_validation_refs: vec!["validation://live/quick".into()],
        forward_test_refs: vec!["test://live/forward".into()],
        duplicate_candidate_refs: Vec::new(),
        metadata_stale: false,
    }
}

fn metrics(total_tokens: u64, elapsed_ms: u64, tool_calls: u32) -> EvolutionBenchmarkMetrics {
    EvolutionBenchmarkMetrics {
        input_tokens: total_tokens / 2,
        output_tokens: total_tokens / 2,
        total_tokens,
        elapsed_ms,
        tool_calls,
        tool_results: tool_calls,
        retry_count: 1,
        failure_recovery_count: 0,
        quality_score: 91,
        human_intervention_count: 0,
        policy_decision_count: 1,
        activation_count: 1,
        use_count: 1,
        success_count: 1,
    }
}

fn measurement(label: &str, total_tokens: u64) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.live".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(metrics(total_tokens, total_tokens * 10, 4)),
        evidence_refs: vec![format!("eventlog://live/benchmark/{label}")],
        artifact_refs: vec![format!("artifact://live/benchmark/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

fn memento() -> EvolutionRollbackMemento {
    EvolutionRollbackMemento {
        memento_id: "memento-live".into(),
        target_type: EvolutionTargetType::SkillPackage,
        scope: scope(),
        state_ref: "memento://live/state".into(),
        evidence_refs: vec!["eventlog://live/memento".into()],
    }
}

fn safe_policy() -> EvolutionReleasePolicyInput {
    EvolutionReleasePolicyInput {
        capability_diff_refs: vec!["diff://live/capability".into()],
        package_ownership_refs: vec!["ownership://live/package".into()],
        resource_permission_refs: vec!["permission://live/resource".into()],
        benchmark_decision: BenchmarkDecision::Passed,
        benchmark_refs: vec!["benchmark://live/paired".into()],
        rollback_mementos: vec![memento()],
        trust_level: EvolutionTrustLevel::Trusted,
        executable_change: false,
        blast_radius_score: 20,
        canary_failure_reason_codes: Vec::new(),
    }
}

fn tick(idempotency_key: &str) -> EvolutionLiveTickCommand {
    EvolutionLiveTickCommand {
        run_id: "run-live-generic".into(),
        actor_id: "agent.live".into(),
        lease_id: "lease-live-generic".into(),
        idempotency_key: idempotency_key.into(),
        trace: TraceContext::new("trace-live-generic"),
        scope: scope(),
        candidate: candidate(EvolutionTargetType::SkillPackage),
        baseline: measurement("baseline", 1_000),
        candidate_measurement: measurement("candidate", 800),
        release_action: EvolutionReleaseAction::StartCanary,
        release_policy: safe_policy(),
        adapter_dispatch: EvolutionLiveAdapterDispatch {
            adapter_id: "adapter.skill.generic".into(),
            supported: true,
            evidence_refs: vec!["adapter://live/skill".into()],
            rollback_refs: vec!["memento://live/state".into()],
        },
        observer_evidence_refs: vec!["eventlog://live/observer".into()],
        policy_decision_refs: vec!["policy://live/allow".into()],
        audit_refs: vec!["audit://live/start".into()],
        replay_after_sequence: None,
    }
}

#[tokio::test]
async fn live_tick_advances_candidate_through_governed_phases() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service.run_live_tick(tick("tick-success")).await.unwrap();

    assert!(result.accepted);
    assert_eq!(result.status, EvolutionLivePhaseStatus::CanaryRunning);
    assert!(result
        .phases
        .iter()
        .any(|phase| phase.phase == EvolutionLivePhase::Admission && phase.accepted));
    assert!(result
        .phases
        .iter()
        .any(|phase| phase.phase == EvolutionLivePhase::Benchmark && phase.accepted));
    assert!(result.adapter_dispatch_performed);
    assert!(result.source_mutation_performed == false);
}

#[tokio::test]
async fn duplicate_live_tick_returns_checkpoint_without_second_dispatch() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let first = service.run_live_tick(tick("tick-duplicate")).await.unwrap();
    let second = service.run_live_tick(tick("tick-duplicate")).await.unwrap();

    assert_eq!(first.idempotency_key, second.idempotency_key);
    assert!(!second.adapter_dispatch_performed);
    assert!(second
        .reason_codes
        .iter()
        .any(|reason| reason == "duplicate_idempotency_key"));
}

#[tokio::test]
async fn missing_admission_evidence_fails_closed() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut command = tick("tick-missing-admission");
    command.candidate.quick_validation_refs.clear();

    let result = service.run_live_tick(command).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionLivePhaseStatus::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "admission_not_accepted"));
}

#[tokio::test]
async fn unsupported_target_adapter_returns_unavailable() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut command = tick("tick-unsupported-adapter");
    command.adapter_dispatch.supported = false;

    let result = service.run_live_tick(command).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionLivePhaseStatus::Unavailable);
    assert!(!result.adapter_dispatch_performed);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "target_adapter_unavailable"));
}

#[tokio::test]
async fn rollback_required_release_without_memento_is_denied() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut command = tick("tick-missing-memento");
    command.release_action = EvolutionReleaseAction::Rollback;
    command.release_policy.rollback_mementos.clear();

    let result = service.run_live_tick(command).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionLivePhaseStatus::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "release_safety_denied"));
}

#[tokio::test]
async fn audit_replays_live_tick_checkpoint_after_provider_clone() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let restarted = service.clone();

    service.run_live_tick(tick("tick-audit")).await.unwrap();
    let audit = restarted
        .audit_live_run(EvolutionLiveAuditCommand {
            run_id: "run-live-generic".into(),
            trace: TraceContext::new("trace-live-audit"),
            scope: scope(),
            after_sequence: None,
            limit: 10,
        })
        .await
        .unwrap();

    assert_eq!(audit.run_id, "run-live-generic");
    assert!(!audit.checkpoints.is_empty());
    assert!(audit
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.idempotency_key == "tick-audit"));
}
