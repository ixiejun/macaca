use macaca_proto::TraceContext;
use macaca_proto::{
    BenchmarkDecision, EvolutionAdmissionCandidate, EvolutionBenchmarkMeasurement,
    EvolutionBenchmarkMetrics, EvolutionLiveAdapterDispatch, EvolutionLiveAuditCommand,
    EvolutionLivePhaseStatus, EvolutionLiveTickCommand, EvolutionReleaseAction,
    EvolutionReleasePolicyInput, EvolutionScope, EvolutionTargetType, EvolutionTrustLevel,
    OsCodeEvolutionProposalCommand, OsCodeEvolutionProposalDecision,
    OsCodeEvolutionProposalEvidence, OsCodeEvolutionProposalIntent,
};
use macaca_sdk::{SystemAutonomyEvolutionClient, UnavailableSystemAutonomyEvolutionClient};

fn scope() -> EvolutionScope {
    EvolutionScope::default()
}

fn measurement(label: &str) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.sdk-live".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(EvolutionBenchmarkMetrics {
            input_tokens: 100,
            output_tokens: 100,
            total_tokens: 200,
            elapsed_ms: 1_000,
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
        evidence_refs: vec![format!("eventlog://sdk/live/{label}")],
        artifact_refs: vec![format!("artifact://sdk/live/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

fn release_policy() -> EvolutionReleasePolicyInput {
    EvolutionReleasePolicyInput {
        capability_diff_refs: vec!["diff://sdk/live".into()],
        package_ownership_refs: vec!["ownership://sdk/live".into()],
        resource_permission_refs: vec!["permission://sdk/live".into()],
        benchmark_decision: BenchmarkDecision::Passed,
        benchmark_refs: vec!["benchmark://sdk/live".into()],
        rollback_mementos: Vec::new(),
        trust_level: EvolutionTrustLevel::Trusted,
        executable_change: false,
        blast_radius_score: 10,
        canary_failure_reason_codes: Vec::new(),
    }
}

fn tick() -> EvolutionLiveTickCommand {
    EvolutionLiveTickCommand {
        run_id: "run-sdk-live".into(),
        actor_id: "agent.sdk.live".into(),
        lease_id: "lease-sdk-live".into(),
        idempotency_key: "tick-sdk-live".into(),
        trace: TraceContext::new("trace-sdk-live"),
        scope: scope(),
        candidate: EvolutionAdmissionCandidate {
            candidate_id: "candidate-sdk-live".into(),
            target_type: EvolutionTargetType::SkillPackage,
            package_name: "generic-sdk-live".into(),
            trigger_descriptions: vec!["Use for generic SDK live orchestrator checks".into()],
            skill_summary: "Generic SDK live orchestrator candidate descriptor.".into(),
            resource_categories: vec!["skill.md".into()],
            quick_validation_refs: vec!["validation://sdk/live".into()],
            forward_test_refs: vec!["test://sdk/live".into()],
            duplicate_candidate_refs: Vec::new(),
            metadata_stale: false,
        },
        baseline: measurement("baseline"),
        candidate_measurement: measurement("candidate"),
        release_action: EvolutionReleaseAction::StartCanary,
        release_policy: release_policy(),
        adapter_dispatch: EvolutionLiveAdapterDispatch {
            adapter_id: "adapter.sdk.live".into(),
            supported: true,
            evidence_refs: vec!["adapter://sdk/live".into()],
            rollback_refs: Vec::new(),
        },
        observer_evidence_refs: vec!["eventlog://sdk/live/observer".into()],
        policy_decision_refs: vec!["policy://sdk/live".into()],
        audit_refs: vec!["audit://sdk/live".into()],
        replay_after_sequence: None,
    }
}

fn os_code_proposal() -> OsCodeEvolutionProposalCommand {
    OsCodeEvolutionProposalCommand {
        proposal_id: "proposal-sdk-os-code".into(),
        run_id: "run-sdk-live".into(),
        actor_id: "agent.sdk.live".into(),
        trace: TraceContext::new("trace-sdk-os-code"),
        scope: scope(),
        intent: OsCodeEvolutionProposalIntent {
            title: "Generic SDK OS proposal".into(),
            summary: "A generic non-mutating OS-code proposal.".into(),
            affected_capability_refs: vec!["capability://sdk".into()],
            requested_change_refs: vec!["openspec://sdk".into()],
            allow_source_mutation: false,
            blast_radius_score: 10,
        },
        evidence: OsCodeEvolutionProposalEvidence {
            openspec_proposal_refs: vec!["openspec://proposal".into()],
            openspec_design_refs: vec!["openspec://design".into()],
            openspec_tasks_refs: vec!["openspec://tasks".into()],
            superpowers_design_refs: vec!["superpowers://design".into()],
            superpowers_plan_refs: vec!["superpowers://plan".into()],
            gitnexus_impact_refs: vec!["gitnexus://impact".into()],
            expected_test_refs: vec!["cargo://test".into()],
            release_gate_refs: vec!["release://gate".into()],
            rollback_refs: vec!["rollback://memento".into()],
        },
        policy_decision_refs: vec!["policy://sdk/live".into()],
        audit_refs: vec!["audit://sdk/live".into()],
    }
}

#[tokio::test]
async fn unavailable_client_returns_unavailable_live_tick() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client.run_live_tick(tick()).await.unwrap();

    assert!(!result.accepted);
    assert_eq!(result.status, EvolutionLivePhaseStatus::Unavailable);
    assert!(!result.adapter_dispatch_performed);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason.contains("unavailable")));
}

#[tokio::test]
async fn unavailable_client_returns_empty_live_audit() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .audit_live_run(EvolutionLiveAuditCommand {
            run_id: "run-sdk-live".into(),
            trace: TraceContext::new("trace-sdk-live-audit"),
            scope: scope(),
            after_sequence: None,
            limit: 10,
        })
        .await
        .unwrap();

    assert!(result.checkpoints.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|reason| reason.contains("unavailable")));
}

#[tokio::test]
async fn unavailable_client_denies_os_code_proposal() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .evaluate_os_code_proposal(os_code_proposal())
        .await
        .unwrap();

    assert_eq!(result.decision, OsCodeEvolutionProposalDecision::Denied);
    assert!(!result.source_mutation_performed);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason.contains("unavailable")));
}
