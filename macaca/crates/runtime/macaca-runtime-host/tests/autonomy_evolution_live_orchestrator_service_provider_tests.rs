use macaca_autonomy_evolution::{
    BenchmarkDecision, EvolutionAdmissionCandidate, EvolutionBenchmarkMeasurement,
    EvolutionBenchmarkMetrics, EvolutionLiveAdapterDispatch, EvolutionLivePhaseStatus,
    EvolutionLiveTickCommand, EvolutionReleaseAction, EvolutionReleasePolicyInput,
    EvolutionRollbackMemento, EvolutionScope, EvolutionTargetType, EvolutionTrustLevel,
    AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::AutonomyEvolutionSystemServiceProvider;

fn measurement(label: &str, total_tokens: u64) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.runtime-live".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(EvolutionBenchmarkMetrics {
            input_tokens: total_tokens / 2,
            output_tokens: total_tokens / 2,
            total_tokens,
            elapsed_ms: total_tokens,
            tool_calls: 2,
            tool_results: 2,
            retry_count: 0,
            failure_recovery_count: 0,
            quality_score: 90,
            human_intervention_count: 0,
            policy_decision_count: 1,
            activation_count: 1,
            use_count: 1,
            success_count: 1,
        }),
        evidence_refs: vec![format!("eventlog://runtime/live/{label}")],
        artifact_refs: vec![format!("artifact://runtime/live/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

fn tick(trace: TraceContext) -> EvolutionLiveTickCommand {
    EvolutionLiveTickCommand {
        run_id: "run-runtime-live".into(),
        actor_id: "agent.runtime.live".into(),
        lease_id: "lease-runtime-live".into(),
        idempotency_key: "tick-runtime-live".into(),
        trace,
        scope: EvolutionScope::default(),
        candidate: EvolutionAdmissionCandidate {
            candidate_id: "candidate-runtime-live".into(),
            target_type: EvolutionTargetType::SkillPackage,
            package_name: "generic-runtime-live".into(),
            trigger_descriptions: vec!["Use for generic runtime live orchestrator checks".into()],
            skill_summary: "Generic runtime live orchestrator candidate descriptor.".into(),
            resource_categories: vec!["skill.md".into()],
            quick_validation_refs: vec!["validation://runtime/live".into()],
            forward_test_refs: vec!["test://runtime/live".into()],
            duplicate_candidate_refs: Vec::new(),
            metadata_stale: false,
        },
        baseline: measurement("baseline", 1_000),
        candidate_measurement: measurement("candidate", 800),
        release_action: EvolutionReleaseAction::StartCanary,
        release_policy: EvolutionReleasePolicyInput {
            capability_diff_refs: vec!["diff://runtime/live".into()],
            package_ownership_refs: vec!["ownership://runtime/live".into()],
            resource_permission_refs: vec!["permission://runtime/live".into()],
            benchmark_decision: BenchmarkDecision::Passed,
            benchmark_refs: vec!["benchmark://runtime/live".into()],
            rollback_mementos: vec![EvolutionRollbackMemento {
                memento_id: "memento-runtime-live".into(),
                target_type: EvolutionTargetType::SkillPackage,
                scope: EvolutionScope::default(),
                state_ref: "memento://runtime/live/state".into(),
                evidence_refs: vec!["eventlog://runtime/live/memento".into()],
            }],
            trust_level: EvolutionTrustLevel::Trusted,
            executable_change: false,
            blast_radius_score: 10,
            canary_failure_reason_codes: Vec::new(),
        },
        adapter_dispatch: EvolutionLiveAdapterDispatch {
            adapter_id: "adapter.runtime.live".into(),
            supported: true,
            evidence_refs: vec!["adapter://runtime/live".into()],
            rollback_refs: Vec::new(),
        },
        observer_evidence_refs: vec!["eventlog://runtime/live/observer".into()],
        policy_decision_refs: vec!["policy://runtime/live".into()],
        audit_refs: vec!["audit://runtime/live".into()],
        replay_after_sequence: None,
    }
}

#[tokio::test]
async fn runtime_host_decodes_live_tick_command() {
    let provider = AutonomyEvolutionSystemServiceProvider::in_memory();
    let trace = TraceContext::new("trace-runtime-live");
    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND),
            payload: serde_json::to_value(tick(trace.clone())).unwrap(),
            trace: Some(trace),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    let typed: macaca_autonomy_evolution::EvolutionLiveTickResult =
        serde_json::from_value(result.output).unwrap();

    assert!(typed.accepted);
    assert_eq!(typed.status, EvolutionLivePhaseStatus::CanaryRunning);
    assert!(typed.adapter_dispatch_performed);
}
