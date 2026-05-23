use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_memory::{
    ActiveRecallRequest, ActiveRecallResult, KnowledgeCompileRequest, KnowledgeCompileResult,
    MemoryDeleteRequest, MemoryGetRequest, MemoryRuntimeFacade, MemoryRuntimeStatus,
    MemorySearchRequest, MemoryWriteRequest,
};
use macaca_proto::{
    ApplicationId, MacacaResult, MemoryEntry, MemoryId, ServiceCommand, ServiceCommandName,
    TraceContext,
};
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolveCommand, SkillAliasResolveResult,
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillAliasUpsertResult, SkillAuthorKind, SkillCurationAction, SkillCurationDryRunCommand,
    SkillCurationDryRunResult, SkillCurationRunCommand, SkillCurationRunRecord,
    SkillCurationRunResult, SkillCurationSnapshotCommand, SkillCurationSnapshotResult,
    SkillCurationStatusCommand, SkillCurationStatusResult, SkillEvolutionCandidateClassification,
    SkillEvolutionProposalAction, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand,
    SkillExperienceProposalResult, SkillExperienceProposalSnapshotCommand,
    SkillExperienceProposalSnapshotResult, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceReadModel, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotRefRecord,
    SkillGovernanceSnapshotResult, SkillProvenanceAction, SkillRollbackRefRecord,
    SkillServiceScope, SkillStatusCommand, SkillStatusResult, SkillUsageEventKind,
    SkillUsageObservation, SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_SNAPSHOT_COMMAND,
    SKILL_ALIAS_UPSERT_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_RUN_COMMAND,
    SKILL_CURATION_SNAPSHOT_COMMAND, SKILL_CURATION_STATUS_COMMAND,
    SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND, SKILL_EVOLUTION_SNAPSHOT_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
};
use tokio::sync::Mutex;

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test command payload must serialize"),
        trace,
    )
}

fn observation(event: SkillUsageEventKind, pinned: Option<bool>) -> SkillUsageObservation {
    SkillUsageObservation {
        skill_id: "skill://agent/example".into(),
        name: "agent-example".into(),
        source: "test".into(),
        source_scope: "workspace".into(),
        event,
        author_kind: SkillAuthorKind::Agent,
        created_by: Some("agent".into()),
        pinned,
        evidence_id: Some("event-1".into()),
        metadata: BTreeMap::new(),
    }
}

fn alias_record() -> SkillAliasRecord {
    let now = chrono::Utc::now();
    SkillAliasRecord {
        source_skill_id: "skill://agent/old".into(),
        source_name: "old-skill".into(),
        target_skill_id: "skill://agent/new".into(),
        target_name: "new-skill".into(),
        kind: SkillAliasKind::AbsorbedInto,
        rationale: "old skill was absorbed into the broader replacement skill".into(),
        created_at: now,
        updated_at: now,
        evidence_ids: vec!["curation-run-1".into()],
    }
}

fn reusable_experience_candidate(evidence_ids: Vec<String>) -> SkillExperienceCandidate {
    SkillExperienceCandidate {
        task_id: "task-verified-1".into(),
        session_id: Some("session-verified-1".into()),
        application_id: None,
        agent_name: Some("agent".into()),
        verified_terminal_success: true,
        evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
        bounded_summary: "A verified task produced a reusable skill maintenance procedure.".into(),
        trace_digest: Some("trace-digest://task/verified-1".into()),
        memory_digest_refs: vec!["memory://digest/task-verified-1".into()],
        reusable_procedure: "Record governance evidence, propose a draft skill, and keep active skill files unchanged until approval.".into(),
        classification: SkillEvolutionCandidateClassification::ReusableProcedure,
        destination: SkillExperienceCandidateDestination::NewSkillDraft,
        recommended_action: SkillEvolutionProposalAction::CreateDraft,
        target_skill_id: None,
        target_skill_name: Some("skill-experience-maintenance".into()),
        evidence_ids,
        metadata: BTreeMap::new(),
    }
}

#[derive(Default)]
struct RecordingMemoryRuntime {
    writes: Mutex<Vec<MemoryWriteRequest>>,
    knowledge_compiles: Mutex<Vec<KnowledgeCompileRequest>>,
}

#[async_trait]
impl MemoryRuntimeFacade for RecordingMemoryRuntime {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
        self.writes.lock().await.push(request);
        Ok(MemoryId::new())
    }

    async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
        Ok(None)
    }

    async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
        Ok(())
    }

    async fn active_recall(
        &self,
        _request: ActiveRecallRequest,
    ) -> MacacaResult<ActiveRecallResult> {
        Ok(ActiveRecallResult {
            provider_id: "recording-memory-runtime".into(),
            candidates: Vec::new(),
            selected: Vec::new(),
            latency_ms: 0,
            diagnostics: Vec::new(),
        })
    }

    async fn compile_knowledge(
        &self,
        request: KnowledgeCompileRequest,
    ) -> MacacaResult<KnowledgeCompileResult> {
        self.knowledge_compiles.lock().await.push(request.clone());
        Ok(KnowledgeCompileResult {
            scope: request.scope,
            claims: Vec::new(),
            conflicts: Vec::new(),
            compiled_at: chrono::Utc::now(),
        })
    }

    async fn status(&self) -> MemoryRuntimeStatus {
        MemoryRuntimeStatus {
            runtime_id: "recording-memory-runtime".into(),
            provider_profile: Some("recording".into()),
            store_available: true,
            search_available: true,
            active_recall_available: true,
            knowledge_available: true,
            diagnostics: Vec::new(),
        }
    }
}

#[tokio::test]
async fn skill_governance_records_usage_and_snapshots_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-governance-record");
    let payload = SkillGovernanceRecordUsageCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        observation: observation(SkillUsageEventKind::Used, None),
    };

    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            payload,
            trace.clone(),
        ))
        .await
        .expect("usage recording should succeed");
    let typed: SkillGovernanceRecordUsageResult =
        serde_json::from_value(result.output).expect("usage result should decode");

    assert_eq!(typed.record.provenance.name, "agent-example");
    assert_eq!(typed.record.telemetry.use_count, 1);
    assert_eq!(typed.record.provenance.author_kind, SkillAuthorKind::Agent);

    let snapshot_payload = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot_payload,
            trace,
        ))
        .await
        .expect("snapshot should succeed");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].telemetry.use_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.record_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.use_count, 1);
}

#[tokio::test]
async fn skill_status_exposes_bounded_telemetry_aggregate() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-status-telemetry");

    for event in [
        SkillUsageEventKind::SuccessfulTask,
        SkillUsageEventKind::FailedTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event, None),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("status aggregate source event should record");
    }

    let status = SkillStatusCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
    };
    let result = provider
        .call(traced_command(SKILL_STATUS_COMMAND, status, trace))
        .await
        .expect("status command should include bounded telemetry");
    let status: SkillStatusResult =
        serde_json::from_value(result.output).expect("status result should decode");

    assert!(status.healthy);
    assert_eq!(status.telemetry_aggregate.record_count, 1);
    assert_eq!(status.telemetry_aggregate.successful_task_count, 1);
    assert_eq!(status.telemetry_aggregate.failed_task_count, 1);
}

#[tokio::test]
async fn skill_governance_dry_run_keeps_pinned_skills_protected() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-dry-run");
    let payload = SkillGovernanceRecordUsageCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        observation: observation(SkillUsageEventKind::Pinned, Some(true)),
    };
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            payload,
            trace.clone(),
        ))
        .await
        .expect("pinned observation should succeed");

    let dry_run = SkillCurationDryRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        stale_after_days: 0,
        narrow_use_threshold: 0,
    };
    let result = provider
        .call(traced_command(
            SKILL_CURATION_DRY_RUN_COMMAND,
            dry_run,
            trace,
        ))
        .await
        .expect("dry-run should succeed");
    let result: SkillCurationDryRunResult =
        serde_json::from_value(result.output).expect("dry-run result should decode");

    assert!(!result.mutated);
    assert_eq!(result.recommendations.len(), 1);
    assert_eq!(
        result.recommendations[0].action,
        SkillCurationAction::Protected
    );
    assert!(result.recommendations[0].protected);
}

#[tokio::test]
async fn skill_curation_status_reports_read_only_local_provider_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-status");
    let result = provider
        .call(traced_command(
            SKILL_CURATION_STATUS_COMMAND,
            SkillCurationStatusCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                interval_ms: 60_000,
                idle_budget_ms: Some(10_000),
            },
            trace,
        ))
        .await
        .expect("curation status should be a read-only provider command");
    let result: SkillCurationStatusResult =
        serde_json::from_value(result.output).expect("curation status result should decode");

    assert!(result.available);
    assert_eq!(result.provider_id, "local-skill-governance");
    assert_eq!(result.interval_ms, 60_000);
    assert_eq!(result.idle_budget_ms, Some(10_000));
    assert!(result.last_run_id.is_none());
    assert!(result.next_eligible_run_at.is_none());
}

#[tokio::test]
async fn skill_curation_run_records_bounded_dry_run_without_file_mutation() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-run-dry");
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::Used, None),
            },
            trace.clone(),
        ))
        .await
        .expect("usage observation should seed curation candidates");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/dry-run".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("dry-run curation command should be accepted");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("run result should decode");

    assert!(result.run.dry_run);
    assert!(!result.mutated);
    assert_eq!(result.run.candidate_count, 1);
    assert_eq!(result.recommendations.len(), 1);
    assert!(result
        .report_ref
        .as_deref()
        .unwrap_or("")
        .contains("REPORT.md"));
    assert!(result.rollback_ref.is_none());

    let status = provider
        .call(traced_command(
            SKILL_CURATION_STATUS_COMMAND,
            SkillCurationStatusCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                interval_ms: 60_000,
                idle_budget_ms: None,
            },
            trace,
        ))
        .await
        .expect("curation status should observe the recorded run");
    let status: SkillCurationStatusResult =
        serde_json::from_value(status.output).expect("status result should decode");
    assert_eq!(status.last_run_id, Some(result.run.run_id));
}

#[tokio::test]
async fn skill_curation_apply_run_requires_approval_and_policy_refs() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-run-apply-denied");

    let err = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: Vec::new(),
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect_err("apply curation run must be approval-gated");

    assert!(
        err.to_string().contains("approval refs"),
        "denial should explain the missing approval gate"
    );
}

#[tokio::test]
async fn skill_curation_snapshot_returns_governance_and_memento_refs_only() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-snapshot");
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::Used, None),
            },
            trace.clone(),
        ))
        .await
        .expect("usage observation should seed snapshot records");
    let run = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: Vec::new(),
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("curation run should create a replayable run ref");
    let run: SkillCurationRunResult =
        serde_json::from_value(run.output).expect("run result should decode");

    let snapshot = provider
        .call(traced_command(
            SKILL_CURATION_SNAPSHOT_COMMAND,
            SkillCurationSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
                include_package_mementos: true,
            },
            trace,
        ))
        .await
        .expect("curation snapshot should be a typed service command");
    let snapshot: SkillCurationSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");

    assert!(!snapshot.mutated);
    assert_eq!(snapshot.snapshot.record_count, 1);
    assert!(snapshot.curation_run_refs.contains(&run.run.run_id));
    assert!(
        !serde_json::to_string(&snapshot)
            .expect("snapshot result should serialize")
            .contains("SKILL.md body"),
        "curation snapshot must not expose full skill instruction bodies"
    );
}

#[tokio::test]
async fn skill_governance_dry_run_uses_success_failure_counters_generically() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-telemetry-rationale");

    for event in [
        SkillUsageEventKind::Activated,
        SkillUsageEventKind::SuccessfulTask,
        SkillUsageEventKind::FailedTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event, None),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("effectiveness telemetry should record without application semantics");
    }

    let dry_run = SkillCurationDryRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        stale_after_days: 30,
        narrow_use_threshold: 1,
    };
    let result = provider
        .call(traced_command(
            SKILL_CURATION_DRY_RUN_COMMAND,
            dry_run,
            trace.clone(),
        ))
        .await
        .expect("dry-run should use generic effectiveness counters");
    let result: SkillCurationDryRunResult =
        serde_json::from_value(result.output).expect("dry-run result should decode");

    assert_eq!(result.recommendations.len(), 1);
    assert_eq!(result.recommendations[0].action, SkillCurationAction::Keep);
    assert!(result.recommendations[0]
        .rationale
        .contains("sufficient usage"));

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("snapshot should expose aggregate counters used by dry-run");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    assert_eq!(snapshot.telemetry_aggregate.activation_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.successful_task_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.failed_task_count, 1);
}

#[tokio::test]
async fn skill_alias_upsert_resolve_and_snapshot_are_service_owned() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-alias-upsert");
    let upsert = SkillAliasUpsertCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record: alias_record(),
    };

    let result = provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            upsert,
            trace.clone(),
        ))
        .await
        .expect("alias upsert should succeed");
    let upsert: SkillAliasUpsertResult =
        serde_json::from_value(result.output).expect("alias upsert result should decode");
    assert_eq!(upsert.record.kind, SkillAliasKind::AbsorbedInto);

    let resolve = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/old".into(),
        name: Some("old-skill".into()),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_RESOLVE_COMMAND,
            resolve,
            trace.clone(),
        ))
        .await
        .expect("alias resolve should succeed");
    let resolved: SkillAliasResolveResult =
        serde_json::from_value(result.output).expect("alias resolve result should decode");
    assert!(resolved.resolved);
    assert_eq!(
        resolved.target_skill_id.as_deref(),
        Some("skill://agent/new")
    );

    let snapshot = SkillAliasSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("alias snapshot should succeed");
    let snapshot: SkillAliasSnapshotResult =
        serde_json::from_value(result.output).expect("alias snapshot result should decode");
    assert_eq!(snapshot.aliases.len(), 1);
    assert_eq!(snapshot.aliases[0].target_name, "new-skill");
}

#[tokio::test]
async fn skill_alias_resolve_without_record_does_not_fake_fallback() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-alias-unresolved");
    let resolve = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/missing".into(),
        name: Some("missing-skill".into()),
    };

    let result = provider
        .call(traced_command(SKILL_ALIAS_RESOLVE_COMMAND, resolve, trace))
        .await
        .expect("alias resolve should succeed");
    let resolved: SkillAliasResolveResult =
        serde_json::from_value(result.output).expect("alias resolve result should decode");

    assert!(!resolved.resolved);
    assert!(resolved.target_skill_id.is_none());
    assert!(resolved.kind.is_none());
}

#[tokio::test]
async fn skill_experience_proposal_creates_draft_without_mutating_governance_records() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-proposal");
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate: reusable_experience_candidate(vec!["artifact-proof-1".into()]),
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("verified reusable task evidence should create a proposal");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert!(!proposal.mutated);
    assert_eq!(
        proposal.proposal.recommended_action,
        SkillEvolutionProposalAction::CreateDraft
    );
    assert_eq!(
        proposal.proposal.classification,
        SkillEvolutionCandidateClassification::ReusableProcedure
    );
    assert_eq!(
        proposal.proposal.destination,
        SkillExperienceCandidateDestination::NewSkillDraft
    );
    assert_eq!(
        proposal.proposal.trace_digest.as_deref(),
        Some("trace-digest://task/verified-1")
    );
    assert_eq!(
        proposal.proposal.memory_digest_refs,
        vec!["memory://digest/task-verified-1"]
    );
    assert_eq!(proposal.proposal.evidence_ids, vec!["artifact-proof-1"]);
    assert!(proposal.proposal.proposal_id.starts_with("skill-exp-"));

    let snapshot = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: true,
        lifecycle_filters: Vec::new(),
    };
    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot,
            trace,
        ))
        .await
        .expect("governance snapshot should remain available");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(result.output).expect("snapshot result should decode");
    assert!(
        snapshot.records.is_empty(),
        "draft proposals must not become active governance records"
    );
}

#[tokio::test]
async fn skill_experience_memory_destination_routes_through_memory_facade() {
    let memory_runtime = Arc::new(RecordingMemoryRuntime::default());
    let provider = SkillSystemServiceProvider::new()
        .with_memory_runtime(Arc::clone(&memory_runtime) as Arc<dyn MemoryRuntimeFacade>);
    let trace = TraceContext::new("trace-skill-experience-memory-route");
    let application_id = ApplicationId::new();
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-memory".into()]);
    candidate.application_id = Some(application_id);
    candidate.destination = SkillExperienceCandidateDestination::MemoryFact;
    candidate.recommended_action = SkillEvolutionProposalAction::Discard;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope {
            application_id: Some(application_id),
            session_id: Some("session-memory-route".into()),
            tenant_id: Some("tenant-route".into()),
            agent_name: Some("agent".into()),
        },
        candidate,
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect("memory destination should route through the injected memory facade");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert!(!proposal.mutated);
    assert_eq!(proposal.destination_route.status.as_str(), "routed");
    assert_eq!(
        proposal.destination_route.destination,
        SkillExperienceCandidateDestination::MemoryFact
    );
    assert!(proposal
        .destination_route
        .target_ref
        .as_deref()
        .is_some_and(|target| target.starts_with("memory://")));
    let writes = memory_runtime.writes.lock().await;
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].scope.identity.application_id, application_id);
    assert_eq!(
        writes[0].scope.identity.session_id.as_deref(),
        Some("session-memory-route")
    );
    assert!(writes[0]
        .content
        .contains("verified task produced a reusable skill maintenance procedure"));
    assert!(!writes[0].content.contains("SKILL.md"));
}

#[tokio::test]
async fn skill_experience_knowledge_destination_routes_through_knowledge_facade() {
    let memory_runtime = Arc::new(RecordingMemoryRuntime::default());
    let provider = SkillSystemServiceProvider::new()
        .with_memory_runtime(Arc::clone(&memory_runtime) as Arc<dyn MemoryRuntimeFacade>);
    let trace = TraceContext::new("trace-skill-experience-knowledge-route");
    let application_id = ApplicationId::new();
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-knowledge".into()]);
    candidate.application_id = Some(application_id);
    candidate.destination = SkillExperienceCandidateDestination::KnowledgeDigest;
    candidate.recommended_action = SkillEvolutionProposalAction::Discard;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope {
            application_id: Some(application_id),
            session_id: Some("session-knowledge-route".into()),
            tenant_id: None,
            agent_name: Some("agent".into()),
        },
        candidate,
    };

    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect("knowledge destination should route through the injected knowledge facade");
    let proposal: SkillExperienceProposalResult =
        serde_json::from_value(result.output).expect("proposal result should decode");

    assert_eq!(proposal.destination_route.status.as_str(), "routed");
    assert_eq!(
        proposal.destination_route.destination,
        SkillExperienceCandidateDestination::KnowledgeDigest
    );
    assert!(proposal
        .destination_route
        .target_ref
        .as_deref()
        .is_some_and(|target| target.starts_with("knowledge://")));
    let compiles = memory_runtime.knowledge_compiles.lock().await;
    assert_eq!(compiles.len(), 1);
    assert_eq!(compiles[0].scope.identity.application_id, application_id);
    assert_eq!(compiles[0].candidates.len(), 1);
    assert_eq!(
        compiles[0].candidates[0].source,
        macaca_memory::CandidateSource::AgentSummary
    );
    assert!(compiles[0].candidates[0]
        .content
        .contains("verified task produced a reusable skill maintenance procedure"));
}

#[tokio::test]
async fn skill_experience_snapshot_lists_draft_proposals_without_mutation() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-snapshot");

    for evidence_id in ["artifact-proof-2", "artifact-proof-1"] {
        let command = SkillExperienceProposalCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            candidate: reusable_experience_candidate(vec![evidence_id.into()]),
        };
        provider
            .call(traced_command(
                SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
                command,
                trace.clone(),
            ))
            .await
            .expect("verified task evidence should create a proposal");
    }

    let snapshot_command = SkillExperienceProposalSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_discarded: false,
    };
    let result = provider
        .call(traced_command(
            SKILL_EVOLUTION_SNAPSHOT_COMMAND,
            snapshot_command,
            trace,
        ))
        .await
        .expect("proposal snapshot should be available");
    let snapshot: SkillExperienceProposalSnapshotResult =
        serde_json::from_value(result.output).expect("proposal snapshot should decode");

    assert_eq!(snapshot.proposals.len(), 2);
    assert!(!snapshot.mutated);
    assert!(snapshot.proposals[0].proposal_id <= snapshot.proposals[1].proposal_id);
    assert_eq!(
        snapshot.proposals[0].trace_id,
        "trace-skill-experience-snapshot"
    );
    assert!(snapshot
        .proposals
        .iter()
        .all(|proposal| proposal.target_skill_name.as_deref()
            == Some("skill-experience-maintenance")));
}

#[tokio::test]
async fn skill_experience_proposal_rejects_missing_evidence() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-missing-evidence");
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate: reusable_experience_candidate(Vec::new()),
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("proposal without evidence must be rejected");

    assert!(
        err.to_string().contains("evidence"),
        "validation error should explain the missing evidence"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_unverified_terminal_status() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-unverified-status");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.verified_terminal_success = false;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("unverified terminal task status must be rejected");

    assert!(
        err.to_string().contains("verified terminal"),
        "validation error should explain terminal success requirement"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_rejected_evidence_gate() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-evidence-rejected");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.evidence_gate = SkillExperienceEvidenceGateStatus::Rejected;
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("rejected evidence gate must be rejected before proposal creation");

    assert!(
        err.to_string().contains("evidence gate"),
        "validation error should explain evidence gate requirement"
    );
}

#[tokio::test]
async fn skill_experience_proposal_rejects_oversize_summary() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-experience-oversize-summary");
    let mut candidate = reusable_experience_candidate(vec!["artifact-proof-1".into()]);
    candidate.bounded_summary = "x".repeat(2_049);
    let command = SkillExperienceProposalCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        candidate,
    };

    let err = provider
        .call(traced_command(
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command,
            trace,
        ))
        .await
        .expect_err("oversize bounded summary must be rejected");

    assert!(
        err.to_string().contains("too large"),
        "validation error should explain bounded summary limit"
    );
}

#[test]
fn skill_governance_event_replay_restores_read_model_without_skill_bodies() {
    let trace = TraceContext::new("trace-skill-governance-replay");
    let created_at = chrono::Utc::now();
    let proposal = macaca_skill::SkillExperienceProposalRecord::from_candidate(
        &trace,
        reusable_experience_candidate(vec!["artifact-proof-1".into()]),
        created_at,
    );
    let run = SkillCurationRunRecord {
        run_id: "curation-run-1".into(),
        trace_id: trace.trace_id.clone(),
        provider_id: "builtin-local-governance-store".into(),
        dry_run: true,
        candidate_count: 1,
        actions: vec!["skill://agent/example:WouldArchive".into()],
        snapshot_refs: vec!["store://skill-curation/run-1/snapshot".into()],
        started_at: created_at,
        finished_at: Some(created_at),
        report_ref: Some("store://skill-curation/run-1/report".into()),
        rollback_ref: None,
        policy_decision_ids: vec!["policy-decision-1".into()],
        audit_event_ids: vec!["audit-event-1".into()],
    };
    let rollback = SkillRollbackRefRecord {
        rollback_ref: "store://skill-curation/run-1/rollback".into(),
        run_id: run.run_id.clone(),
        trace_id: trace.trace_id.clone(),
        before_snapshot_ref: "store://skill-curation/run-1/before".into(),
        after_snapshot_ref: Some("store://skill-curation/run-1/after".into()),
        report_ref: run.report_ref.clone(),
        captured_at: created_at,
    };
    let events = vec![
        SkillGovernanceEventRecord::new(
            "event-discovery-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Created,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-usage-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Used,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-patch-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::UsageRecorded(observation(
                SkillUsageEventKind::Patched,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-lifecycle-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::LifecycleApplied(observation(
                SkillUsageEventKind::Archived,
                None,
            )),
        ),
        SkillGovernanceEventRecord::new(
            "event-alias-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::AliasUpserted(alias_record()),
        ),
        SkillGovernanceEventRecord::new(
            "event-proposal-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::ProposalCreated(proposal),
        ),
        SkillGovernanceEventRecord::new(
            "event-run-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::CurationRunRecorded(run),
        ),
        SkillGovernanceEventRecord::new(
            "event-snapshot-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::SnapshotRefRecorded(SkillGovernanceSnapshotRefRecord {
                snapshot_ref: "store://skill-curation/run-1/snapshot".into(),
                trace_id: trace.trace_id.clone(),
                record_count: 1,
                captured_at: created_at,
                report_ref: Some("store://skill-curation/run-1/report".into()),
            }),
        ),
        SkillGovernanceEventRecord::new(
            "event-rollback-1",
            &trace,
            SkillServiceScope::default(),
            created_at,
            SkillGovernanceEventPayload::RollbackRefRecorded(rollback),
        ),
    ];

    let read_model = SkillGovernanceReadModel::from_events(events);

    assert_eq!(read_model.records.len(), 1);
    assert_eq!(read_model.records[0].telemetry.use_count, 1);
    assert_eq!(read_model.records[0].telemetry.patch_count, 1);
    assert_eq!(
        read_model.records[0].lifecycle,
        macaca_skill::SkillLifecycleState::Archived
    );
    assert_eq!(read_model.aliases.len(), 1);
    assert_eq!(read_model.proposals.len(), 1);
    assert_eq!(read_model.curation_runs.len(), 1);
    assert_eq!(read_model.snapshot_refs.len(), 1);
    assert_eq!(read_model.rollback_refs.len(), 1);
    assert_eq!(read_model.replayed_events, 9);
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::Discovered)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::Patched)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::CurationRecorded)));
    assert!(read_model
        .provenance_events
        .iter()
        .any(|event| matches!(event.action, SkillProvenanceAction::RollbackRecorded)));
    assert!(serde_json::to_string(&read_model)
        .expect("read model should serialize")
        .contains("store://skill-curation/run-1/report"));
    assert!(
        !serde_json::to_string(&read_model)
            .expect("read model should serialize")
            .contains("SKILL.md body"),
        "governance replay must not materialize full skill instruction bodies"
    );
}
