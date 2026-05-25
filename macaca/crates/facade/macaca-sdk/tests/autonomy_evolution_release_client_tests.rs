use macaca_autonomy_evolution::{
    BenchmarkDecision, EvolutionReleaseAction, EvolutionReleaseCommand,
    EvolutionReleasePolicyInput, EvolutionReleaseStatus, EvolutionScope, EvolutionTargetType,
    EvolutionTrustLevel,
};
use macaca_proto::TraceContext;
use macaca_sdk::{SystemAutonomyEvolutionClient, UnavailableSystemAutonomyEvolutionClient};

#[tokio::test]
async fn unavailable_autonomy_evolution_client_denies_release_action() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .evaluate_release(EvolutionReleaseCommand {
            release_id: "release-sdk-unavailable".into(),
            run_id: "run-sdk-unavailable".into(),
            actor_id: "agent.release".into(),
            target_type: EvolutionTargetType::SkillPackage,
            action: EvolutionReleaseAction::Promote,
            trace: TraceContext::new("trace-sdk-release-unavailable"),
            scope: EvolutionScope::default(),
            policy_decision_refs: vec!["policy://sdk/release".into()],
            audit_refs: vec!["audit://sdk/release".into()],
            evidence_refs: vec!["eventlog://sdk/release".into()],
            policy: EvolutionReleasePolicyInput {
                capability_diff_refs: vec!["diff://sdk/release".into()],
                package_ownership_refs: vec!["ownership://sdk/release".into()],
                resource_permission_refs: vec!["permission://sdk/release".into()],
                benchmark_decision: BenchmarkDecision::Passed,
                benchmark_refs: vec!["benchmark://sdk/release".into()],
                rollback_mementos: Vec::new(),
                trust_level: EvolutionTrustLevel::Trusted,
                executable_change: false,
                blast_radius_score: 10,
                canary_failure_reason_codes: Vec::new(),
            },
            dry_run: false,
        })
        .await
        .unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionReleaseStatus::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason.contains("unavailable")));
}
