use macaca_autonomy_evolution::{
    BenchmarkDecision, EvolutionReleaseAction, EvolutionReleaseCommand,
    EvolutionReleasePolicyInput, EvolutionReleaseStatus, EvolutionRollbackMemento, EvolutionScope,
    EvolutionTargetType, EvolutionTrustLevel, AUTONOMY_EVOLUTION_RELEASE_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::AutonomyEvolutionSystemServiceProvider;

fn scope() -> EvolutionScope {
    EvolutionScope {
        application_id: Some("app.generic".into()),
        tenant_id: Some("tenant.generic".into()),
        session_id: Some("session.generic".into()),
        task_id: Some("task.generic".into()),
    }
}

#[tokio::test]
async fn runtime_host_autonomy_evolution_provider_decodes_release_command() {
    let provider = AutonomyEvolutionSystemServiceProvider::in_memory();
    let trace = TraceContext::new("trace-runtime-release");
    let command = EvolutionReleaseCommand {
        release_id: "release-runtime".into(),
        run_id: "run-runtime-release".into(),
        actor_id: "agent.release".into(),
        target_type: EvolutionTargetType::SkillPackage,
        action: EvolutionReleaseAction::StartCanary,
        trace: trace.clone(),
        scope: scope(),
        policy_decision_refs: vec!["policy://runtime/release".into()],
        audit_refs: vec!["audit://runtime/release".into()],
        evidence_refs: vec!["eventlog://runtime/release".into()],
        policy: EvolutionReleasePolicyInput {
            capability_diff_refs: vec!["diff://runtime/release".into()],
            package_ownership_refs: vec!["ownership://runtime/release".into()],
            resource_permission_refs: vec!["permission://runtime/release".into()],
            benchmark_decision: BenchmarkDecision::Passed,
            benchmark_refs: vec!["benchmark://runtime/release".into()],
            rollback_mementos: vec![EvolutionRollbackMemento {
                memento_id: "memento-runtime-release".into(),
                target_type: EvolutionTargetType::SkillPackage,
                scope: scope(),
                state_ref: "memento://runtime/release/state".into(),
                evidence_refs: vec!["eventlog://runtime/release/memento".into()],
            }],
            trust_level: EvolutionTrustLevel::Trusted,
            executable_change: false,
            blast_radius_score: 10,
            canary_failure_reason_codes: Vec::new(),
        },
        dry_run: false,
    };

    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(AUTONOMY_EVOLUTION_RELEASE_COMMAND),
            payload: serde_json::to_value(command).unwrap(),
            trace: Some(trace),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    let typed: macaca_autonomy_evolution::EvolutionReleaseResult =
        serde_json::from_value(result.output).unwrap();

    assert!(typed.accepted);
    assert_eq!(typed.status, EvolutionReleaseStatus::CanaryRunning);
    assert_eq!(typed.release_id, "release-runtime");
}
