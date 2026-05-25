use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_autonomy_evolution::{
    BenchmarkDecision, EvolutionAdmissionCandidate, EvolutionBenchmarkMeasurement,
    EvolutionBenchmarkMetrics, EvolutionLiveAdapterDispatch, EvolutionLiveTickCommand,
    EvolutionReleaseAction, EvolutionReleasePolicyInput, EvolutionRollbackMemento, EvolutionScope,
    EvolutionTargetType, EvolutionTrustLevel, OsCodeEvolutionProposalCommand,
    OsCodeEvolutionProposalEvidence, OsCodeEvolutionProposalIntent,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::{
    AutonomyEvolutionLiveExecutionCommand, AutonomyEvolutionLiveExecutor,
    AutonomyEvolutionSystemServiceProvider, ServiceProviderFactoryContext, ServiceProviderInstance,
    ServiceRuntime, ServiceRuntimeConfig, SkillSystemServiceProvider, StaticServiceProviderFactory,
};
use macaca_skill::{
    SkillAutonomousMaterializationRunCommand, SkillEvolutionCandidateClassification,
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand, SkillPackageOwnershipClass,
    SkillServicePolicyHints, SkillServiceScope, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
};

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
        candidate_id: format!("candidate-{target_type:?}"),
        target_type,
        package_name: "generic-evolution-package".into(),
        trigger_descriptions: vec![
            "Use when a generic repeated task family benefits from reuse".into(),
        ],
        skill_summary: "A generic candidate produced from bounded observer evidence.".into(),
        resource_categories: vec!["skill.md".into()],
        quick_validation_refs: vec!["validation://quick".into()],
        forward_test_refs: vec!["test://forward".into()],
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
        retry_count: 0,
        failure_recovery_count: 0,
        quality_score: 92,
        human_intervention_count: 0,
        policy_decision_count: 1,
        activation_count: 1,
        use_count: 1,
        success_count: 1,
    }
}

fn measurement(
    label: &str,
    target_type: EvolutionTargetType,
    total_tokens: u64,
) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.self-evolution".into(),
        target_type,
        metrics: Some(metrics(total_tokens, total_tokens * 10, 3)),
        evidence_refs: vec![format!("eventlog://benchmark/{label}")],
        artifact_refs: vec![format!("artifact://benchmark/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

fn rollback(target_type: EvolutionTargetType) -> EvolutionRollbackMemento {
    EvolutionRollbackMemento {
        memento_id: "memento-generic".into(),
        target_type,
        scope: scope(),
        state_ref: "memento://generic/state".into(),
        evidence_refs: vec!["eventlog://memento".into()],
    }
}

fn policy(target_type: EvolutionTargetType) -> EvolutionReleasePolicyInput {
    EvolutionReleasePolicyInput {
        capability_diff_refs: vec!["diff://capability".into()],
        package_ownership_refs: vec!["ownership://package".into()],
        resource_permission_refs: vec!["permission://resource".into()],
        benchmark_decision: BenchmarkDecision::Passed,
        benchmark_refs: vec!["benchmark://paired".into()],
        rollback_mementos: vec![rollback(target_type)],
        trust_level: EvolutionTrustLevel::Trusted,
        executable_change: false,
        blast_radius_score: 20,
        canary_failure_reason_codes: Vec::new(),
    }
}

fn live_tick(target_type: EvolutionTargetType, key: &str) -> EvolutionLiveTickCommand {
    EvolutionLiveTickCommand {
        run_id: format!("run-{key}"),
        actor_id: "agent.self-evolution".into(),
        lease_id: "lease-self-evolution".into(),
        idempotency_key: key.into(),
        trace: TraceContext::new(format!("trace-{key}")),
        scope: scope(),
        candidate: candidate(target_type.clone()),
        baseline: measurement("baseline", target_type.clone(), 1_000),
        candidate_measurement: measurement("candidate", target_type.clone(), 760),
        release_action: EvolutionReleaseAction::StartCanary,
        release_policy: policy(target_type),
        adapter_dispatch: EvolutionLiveAdapterDispatch {
            adapter_id: "adapter.generic".into(),
            supported: true,
            evidence_refs: vec!["adapter://generic".into()],
            rollback_refs: vec!["memento://generic/state".into()],
        },
        observer_evidence_refs: vec!["eventlog://observer".into()],
        policy_decision_refs: vec!["policy://allow".into()],
        audit_refs: vec!["audit://start".into()],
        replay_after_sequence: None,
    }
}

async fn runtime_with_autonomy() -> Arc<ServiceRuntime> {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let autonomy = Arc::new(AutonomyEvolutionSystemServiceProvider::in_memory());
    register(&runtime, autonomy.descriptor(), autonomy).await;
    runtime
}

async fn register(
    runtime: &ServiceRuntime,
    descriptor: macaca_proto::ServiceDescriptor,
    service: Arc<dyn SystemService>,
) {
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .expect("service should register");
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

fn materialization(root: std::path::PathBuf) -> SkillAutonomousMaterializationRunCommand {
    SkillAutonomousMaterializationRunCommand {
        trace: TraceContext::new("trace-skill-materialization-target"),
        scope: SkillServiceScope::default(),
        dry_run: true,
        batch_limit: 1,
        min_ready_score: 40,
        package_collection_root: root,
        ownership: SkillPackageOwnershipClass::AgentPrivate,
        reason: "approved autonomous end-to-end live execution".into(),
        evidence_ids: vec!["evidence://operator/run".into()],
        policy_decision_refs: vec!["policy://operator/approved".into()],
        audit_event_ids: vec!["audit://operator/run".into()],
        policy: approved_policy(),
    }
}

async fn create_skill_proposal(provider: &SkillSystemServiceProvider) {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "evidence_ref.artifact_0".into(),
        "tool:file_write:generic".into(),
    );
    let command = SkillExperienceProposalCommand {
        trace: TraceContext::new("trace-create-skill-proposal"),
        scope: SkillServiceScope::default(),
        candidate: SkillExperienceCandidate {
            task_id: "task-generic-self-evolution".into(),
            session_id: Some("session-generic-self-evolution".into()),
            application_id: None,
            agent_name: Some("agent".into()),
            verified_terminal_success: true,
            evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
            bounded_summary: "Verified terminal completion produced a reusable procedure.".into(),
            trace_digest: Some("trace-digest://generic".into()),
            memory_digest_refs: Vec::new(),
            reusable_procedure: "Inspect evidence, extract reusable steps, and validate outputs."
                .into(),
            classification: SkillEvolutionCandidateClassification::ReusableProcedure,
            destination: SkillExperienceCandidateDestination::NewSkillDraft,
            recommended_action: SkillEvolutionProposalAction::CreateDraft,
            target_skill_id: None,
            target_skill_name: Some("generic-self-evolution-skill".into()),
            evidence_ids: vec!["evidence://operator/run".into()],
            metadata,
        },
    };
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND),
            serde_json::to_value(command).unwrap(),
            TraceContext::new("trace-create-skill-proposal-call"),
        ))
        .await
        .expect("proposal should be accepted");
}

fn os_code_proposal(run_id: &str) -> OsCodeEvolutionProposalCommand {
    OsCodeEvolutionProposalCommand {
        proposal_id: "proposal-os-code-generic".into(),
        run_id: run_id.into(),
        actor_id: "agent.self-evolution".into(),
        trace: TraceContext::new("trace-os-code-proposal"),
        scope: scope(),
        intent: OsCodeEvolutionProposalIntent {
            title: "Generic OS capability proposal".into(),
            summary: "A governed source-code proposal bundle without direct mutation.".into(),
            affected_capability_refs: vec!["capability://autonomy-evolution".into()],
            requested_change_refs: vec!["openspec://generic-change".into()],
            allow_source_mutation: false,
            blast_radius_score: 20,
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
        policy_decision_refs: vec!["policy://allow".into()],
        audit_refs: vec!["audit://os-code".into()],
    }
}

#[tokio::test]
async fn executor_dispatches_skill_target_and_replays_audit() {
    let runtime = runtime_with_autonomy().await;
    let skill = Arc::new(SkillSystemServiceProvider::new());
    create_skill_proposal(&skill).await;
    register(runtime.as_ref(), skill.descriptor(), skill).await;
    let temp = tempfile::tempdir().unwrap();
    let tick = live_tick(EvolutionTargetType::SkillPackage, "skill-end-to-end");

    let result = AutonomyEvolutionLiveExecutor::new(runtime)
        .execute(AutonomyEvolutionLiveExecutionCommand {
            trace: TraceContext::new("trace-skill-end-to-end"),
            scope: scope(),
            live_tick: tick,
            skill_materialization: Some(materialization(temp.path().join("skills"))),
            os_code_proposal: None,
        })
        .await
        .unwrap();

    assert!(result.accepted);
    assert!(result.live_tick.accepted);
    assert!(result.target.accepted);
    assert_eq!(
        result.target.command_name.as_deref(),
        Some(macaca_skill::SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND)
    );
    assert!(!result.audit.checkpoints.is_empty());
}

#[tokio::test]
async fn executor_dispatches_os_code_proposal_without_source_mutation() {
    let runtime = runtime_with_autonomy().await;
    let tick = live_tick(EvolutionTargetType::OsCodeProposal, "os-code-end-to-end");

    let result = AutonomyEvolutionLiveExecutor::new(runtime)
        .execute(AutonomyEvolutionLiveExecutionCommand {
            trace: TraceContext::new("trace-os-code-end-to-end"),
            scope: scope(),
            os_code_proposal: Some(os_code_proposal(&tick.run_id)),
            live_tick: tick,
            skill_materialization: None,
        })
        .await
        .unwrap();

    assert!(result.accepted);
    assert_eq!(
        result.target.command_name.as_deref(),
        Some(macaca_autonomy_evolution::AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND)
    );
    assert_eq!(result.target.status, "ReadyForReview");
    assert!(!result.target.source_mutation_performed);
    assert!(!result.audit.checkpoints.is_empty());
}

#[tokio::test]
async fn executor_fails_closed_for_unsupported_target() {
    let runtime = runtime_with_autonomy().await;
    let tick = live_tick(
        EvolutionTargetType::Unsupported("generic.future.target".into()),
        "unsupported-end-to-end",
    );

    let result = AutonomyEvolutionLiveExecutor::new(runtime)
        .execute(AutonomyEvolutionLiveExecutionCommand {
            trace: TraceContext::new("trace-unsupported-end-to-end"),
            scope: scope(),
            live_tick: tick,
            skill_materialization: None,
            os_code_proposal: None,
        })
        .await
        .unwrap();

    assert!(!result.accepted);
    assert_eq!(result.target.status, "unavailable");
    assert!(result
        .target
        .reason_codes
        .iter()
        .any(|reason| reason == "target_adapter_unavailable"));
    assert!(!result.audit.checkpoints.is_empty());
}
