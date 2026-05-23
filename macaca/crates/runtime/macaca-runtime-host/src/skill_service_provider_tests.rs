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
    SkillAliasKind, SkillAliasRecord, SkillAliasResolutionPolicy, SkillAliasResolutionStatus,
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillAuthorKind,
    SkillCurationAction, SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillCurationPhase,
    SkillCurationRollbackCommand, SkillCurationRollbackResult, SkillCurationRunCommand,
    SkillCurationRunRecord, SkillCurationRunResult, SkillCurationSnapshotCommand,
    SkillCurationSnapshotResult, SkillCurationStatusCommand, SkillCurationStatusResult,
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceEvidenceGateStatus,
    SkillExperienceProposalCommand, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotCommand, SkillExperienceProposalSnapshotResult,
    SkillGovernanceEventPayload, SkillGovernanceEventRecord, SkillGovernanceReadModel,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotRefRecord,
    SkillGovernanceSnapshotResult, SkillProvenanceAction, SkillRollbackRefRecord,
    SkillSemanticReviewStatus, SkillServiceScope, SkillStatusCommand, SkillStatusResult,
    SkillUsageEventKind, SkillUsageObservation, SKILL_ALIAS_RESOLVE_COMMAND,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_CURATION_STATUS_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
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
        resolution_policy: SkillAliasResolutionPolicy::Redirect,
        valid_from: now,
        valid_until: None,
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
    assert!(result
        .run_json_ref
        .as_deref()
        .unwrap_or("")
        .contains("run.json"));
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
async fn skill_curation_dry_run_does_not_mutate_active_governance_or_alias_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-dry-run-immutability");
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
        .expect("usage observation should seed immutable dry-run records");
    provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            SkillAliasUpsertCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                record: alias_record(),
            },
            trace.clone(),
        ))
        .await
        .expect("alias state should exist before dry-run");

    let before_governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace.clone(),
        ))
        .await
        .expect("pre dry-run governance snapshot should decode");
    let before_governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(before_governance.output).expect("snapshot should decode");
    let before_aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("pre dry-run alias snapshot should decode");
    let before_aliases: SkillAliasSnapshotResult =
        serde_json::from_value(before_aliases.output).expect("alias snapshot should decode");

    let dry_run = provider
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
                audit_event_ids: vec!["audit://skill-curation/dry-run-immutability".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("dry-run command should be admitted without approval refs");
    let dry_run: SkillCurationRunResult =
        serde_json::from_value(dry_run.output).expect("dry-run result should decode");
    assert!(!dry_run.mutated);
    assert!(dry_run.rollback_ref.is_none());

    let after_governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace.clone(),
        ))
        .await
        .expect("post dry-run governance snapshot should decode");
    let after_governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(after_governance.output).expect("snapshot should decode");
    let after_aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("post dry-run alias snapshot should decode");
    let after_aliases: SkillAliasSnapshotResult =
        serde_json::from_value(after_aliases.output).expect("alias snapshot should decode");
    assert_eq!(after_governance.records, before_governance.records);
    assert_eq!(after_aliases.aliases, before_aliases.aliases);

    let curation_snapshot = provider
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
        .expect("curation snapshot should expose refs only after dry-run");
    let curation_snapshot: SkillCurationSnapshotResult =
        serde_json::from_value(curation_snapshot.output).expect("curation snapshot should decode");
    assert!(curation_snapshot.rollback_refs.is_empty());
    assert!(curation_snapshot.package_memento_refs.is_empty());
}

#[tokio::test]
async fn skill_curation_report_keeps_protected_ownership_and_absent_provider_sanitized() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-report-sanitized");
    let mut protected_system_skill = observation(SkillUsageEventKind::Created, None);
    protected_system_skill.skill_id = "skill://system/protected".into();
    protected_system_skill.name = "system-protected".into();
    protected_system_skill.author_kind = SkillAuthorKind::System;
    protected_system_skill.evidence_id = Some("evidence://system/protected".into());
    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: protected_system_skill,
            },
            trace.clone(),
        ))
        .await
        .expect("protected ownership observation should seed curation report");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 0,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/report-sanitized".into()],
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect("dry-run report command should be accepted without semantic provider");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("curation run result should decode");

    assert!(result.semantic_analysis_status.contains("unavailable"));
    let recommendation = result
        .recommendations
        .iter()
        .find(|candidate| candidate.skill_id == "skill://system/protected")
        .expect("protected ownership recommendation should exist");
    assert_eq!(recommendation.action, SkillCurationAction::Protected);
    assert!(recommendation.protected);
    assert!(recommendation
        .phases
        .contains(&SkillCurationPhase::Protected));

    let report_ref = result.report_ref.as_deref().unwrap_or("");
    let run_json_ref = result.run_json_ref.as_deref().unwrap_or("");
    assert!(report_ref.starts_with("store://skill-curation/"));
    assert!(report_ref.ends_with("/REPORT.md"));
    assert!(run_json_ref.starts_with("store://skill-curation/"));
    assert!(run_json_ref.ends_with("/run.json"));
    assert!(!report_ref.contains("SKILL.md"));
    assert!(!report_ref.contains("prompt"));
    assert!(!run_json_ref.contains("provider_payload"));
    assert!(result.rollback_ref.is_none());
    assert!(!result.mutated);
}

#[tokio::test]
async fn skill_curation_run_records_structured_absent_semantic_provider() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-semantic-provider-absent");

    let result = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: true,
                stale_after_days: 30,
                narrow_use_threshold: 0,
                approval_refs: Vec::new(),
                policy_decision_refs: Vec::new(),
                audit_event_ids: vec!["audit://skill-curation/semantic-absent".into()],
                policy: Default::default(),
            },
            trace,
        ))
        .await
        .expect("dry-run should preserve deterministic curation when semantic provider is absent");
    let result: SkillCurationRunResult =
        serde_json::from_value(result.output).expect("curation run result should decode");

    assert_eq!(
        result.semantic_review.status,
        SkillSemanticReviewStatus::Unavailable
    );
    assert!(result.semantic_review.proposals.is_empty());
    assert!(!result.semantic_review.mutated);
    assert!(result
        .semantic_review
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.contains("semantic review provider is unavailable") }));
    let sanitized_debug = format!("{:?}", result.semantic_review);
    assert!(!sanitized_debug.contains("raw_provider_payload"));
    assert!(!sanitized_debug.contains("prompt"));
    assert!(!sanitized_debug.contains("secret"));
    assert!(result.semantic_analysis_status.contains("unavailable"));
    assert!(!result.mutated);
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
async fn skill_curation_rollback_restores_governance_alias_and_refs_from_memento() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-curation-rollback");
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
        .expect("usage observation should seed rollback memento records");

    let run = provider
        .call(traced_command(
            SKILL_CURATION_RUN_COMMAND,
            SkillCurationRunCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                dry_run: false,
                stale_after_days: 30,
                narrow_use_threshold: 1,
                approval_refs: vec!["approval://skill-curation/apply".into()],
                policy_decision_refs: vec!["policy://skill-curation/apply".into()],
                audit_event_ids: vec!["audit://skill-curation/apply".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("approved apply run should create rollback memento refs");
    let run: SkillCurationRunResult =
        serde_json::from_value(run.output).expect("apply run result should decode");
    let rollback_ref = run
        .rollback_ref
        .clone()
        .expect("apply run should return a rollback ref");

    provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            SkillGovernanceRecordUsageCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                observation: observation(SkillUsageEventKind::FailedTask, None),
            },
            trace.clone(),
        ))
        .await
        .expect("post-memento telemetry mutation should be observable");
    provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            SkillAliasUpsertCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                record: alias_record(),
            },
            trace.clone(),
        ))
        .await
        .expect("post-memento alias mutation should be observable");

    let rollback = provider
        .call(traced_command(
            SKILL_CURATION_ROLLBACK_COMMAND,
            SkillCurationRollbackCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                rollback_ref: rollback_ref.clone(),
                approval_refs: vec!["approval://skill-curation/rollback".into()],
                policy_decision_refs: vec!["policy://skill-curation/rollback".into()],
                audit_event_ids: vec!["audit://skill-curation/rollback".into()],
                policy: Default::default(),
            },
            trace.clone(),
        ))
        .await
        .expect("rollback command should restore the pre-apply memento");
    let rollback: SkillCurationRollbackResult =
        serde_json::from_value(rollback.output).expect("rollback result should decode");
    assert_eq!(rollback.rollback_ref, rollback_ref);
    assert!(rollback.mutated);
    assert_eq!(rollback.restored_record_count, 1);
    assert_eq!(rollback.restored_alias_count, 0);
    assert!(rollback
        .restored_report_refs
        .iter()
        .any(|report| report.contains("REPORT.md")));
    assert!(rollback
        .restored_report_refs
        .iter()
        .any(|report| report.contains("run.json")));
    assert!(rollback.package_memento_refs.contains(&rollback_ref));

    let governance = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            SkillGovernanceSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
                include_archived: true,
                lifecycle_filters: Vec::new(),
            },
            trace.clone(),
        ))
        .await
        .expect("governance snapshot should show restored telemetry");
    let governance: SkillGovernanceSnapshotResult =
        serde_json::from_value(governance.output).expect("governance snapshot should decode");
    assert_eq!(governance.records.len(), 1);
    assert_eq!(governance.records[0].telemetry.use_count, 1);
    assert_eq!(governance.records[0].telemetry.failed_task_count, 0);

    let aliases = provider
        .call(traced_command(
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            SkillAliasSnapshotCommand {
                trace: trace.clone(),
                scope: SkillServiceScope::default(),
            },
            trace,
        ))
        .await
        .expect("alias snapshot should show restored alias map");
    let aliases: SkillAliasSnapshotResult =
        serde_json::from_value(aliases.output).expect("alias snapshot should decode");
    assert!(aliases.aliases.is_empty());
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
async fn skill_alias_policy_statuses_cover_warn_deny_expired_and_loop() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-alias-policy-status");
    let now = chrono::Utc::now();

    upsert_alias_for_policy_test(
        &provider,
        &trace,
        SkillAliasRecord {
            source_skill_id: "skill://agent/warn-source".into(),
            source_name: "warn-source".into(),
            target_skill_id: "skill://agent/warn-target".into(),
            target_name: "warn-target".into(),
            kind: SkillAliasKind::Redirect,
            resolution_policy: SkillAliasResolutionPolicy::WarnAndRedirect,
            valid_from: now,
            valid_until: None,
            rationale: "warn consumers before following this redirect".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/warn".into()],
        },
    )
    .await;
    let warned =
        resolve_alias_for_policy_test(&provider, &trace, "skill://agent/warn-source").await;
    assert!(warned.resolved);
    assert_eq!(warned.status, SkillAliasResolutionStatus::WarnAndRedirected);
    assert_eq!(
        warned.resolution_policy,
        Some(SkillAliasResolutionPolicy::WarnAndRedirect)
    );

    upsert_alias_for_policy_test(
        &provider,
        &trace,
        SkillAliasRecord {
            source_skill_id: "skill://agent/deny-source".into(),
            source_name: "deny-source".into(),
            target_skill_id: "skill://agent/deny-target".into(),
            target_name: "deny-target".into(),
            kind: SkillAliasKind::Redirect,
            resolution_policy: SkillAliasResolutionPolicy::Deny,
            valid_from: now,
            valid_until: None,
            rationale: "policy denies this historical reference".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/deny".into()],
        },
    )
    .await;
    let denied =
        resolve_alias_for_policy_test(&provider, &trace, "skill://agent/deny-source").await;
    assert!(!denied.resolved);
    assert_eq!(denied.status, SkillAliasResolutionStatus::Denied);
    assert!(denied.target_skill_id.is_none());

    upsert_alias_for_policy_test(
        &provider,
        &trace,
        SkillAliasRecord {
            source_skill_id: "skill://agent/expired-source".into(),
            source_name: "expired-source".into(),
            target_skill_id: "skill://agent/expired-target".into(),
            target_name: "expired-target".into(),
            kind: SkillAliasKind::SupersededBy,
            resolution_policy: SkillAliasResolutionPolicy::Redirect,
            valid_from: now - chrono::Duration::days(2),
            valid_until: Some(now - chrono::Duration::days(1)),
            rationale: "expired redirect must not affect new activations".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/expired".into()],
        },
    )
    .await;
    let expired =
        resolve_alias_for_policy_test(&provider, &trace, "skill://agent/expired-source").await;
    assert!(!expired.resolved);
    assert_eq!(expired.status, SkillAliasResolutionStatus::Expired);

    upsert_alias_for_policy_test(
        &provider,
        &trace,
        SkillAliasRecord {
            source_skill_id: "skill://agent/loop-a".into(),
            source_name: "loop-a".into(),
            target_skill_id: "skill://agent/loop-b".into(),
            target_name: "loop-b".into(),
            kind: SkillAliasKind::Redirect,
            resolution_policy: SkillAliasResolutionPolicy::Redirect,
            valid_from: now,
            valid_until: None,
            rationale: "first half of an invalid alias loop".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/loop-a".into()],
        },
    )
    .await;
    upsert_alias_for_policy_test(
        &provider,
        &trace,
        SkillAliasRecord {
            source_skill_id: "skill://agent/loop-b".into(),
            source_name: "loop-b".into(),
            target_skill_id: "skill://agent/loop-a".into(),
            target_name: "loop-a".into(),
            kind: SkillAliasKind::Redirect,
            resolution_policy: SkillAliasResolutionPolicy::Redirect,
            valid_from: now,
            valid_until: None,
            rationale: "second half of an invalid alias loop".into(),
            created_at: now,
            updated_at: now,
            evidence_ids: vec!["evidence://alias/loop-b".into()],
        },
    )
    .await;
    let looped = resolve_alias_for_policy_test(&provider, &trace, "skill://agent/loop-a").await;
    assert!(!looped.resolved);
    assert_eq!(looped.status, SkillAliasResolutionStatus::LoopPrevented);
}

async fn upsert_alias_for_policy_test(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    record: SkillAliasRecord,
) {
    let command = SkillAliasUpsertCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record,
    };
    provider
        .call(traced_command(
            SKILL_ALIAS_UPSERT_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("alias upsert should succeed for policy-status test");
}

async fn resolve_alias_for_policy_test(
    provider: &SkillSystemServiceProvider,
    trace: &TraceContext,
    skill_id: &str,
) -> SkillAliasResolveResult {
    let name = skill_id
        .rsplit('/')
        .next()
        .expect("test skill id should contain a final path segment")
        .to_string();
    let command = SkillAliasResolveCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: skill_id.into(),
        name: Some(name),
    };
    let result = provider
        .call(traced_command(
            SKILL_ALIAS_RESOLVE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("alias resolve should succeed for policy-status test");
    serde_json::from_value(result.output).expect("alias policy-status result should decode")
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
        run_json_ref: Some("store://skill-curation/run-1/run.json".into()),
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
