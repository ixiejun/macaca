//! Source-level logging contract tests for the local Skill governance store.
//!
//! These tests intentionally inspect the provider source for stable structured
//! log fields.  Runtime log capture would require a global subscriber and would
//! make otherwise deterministic service tests order-dependent.  The contract
//! still protects the audit surface: append, replay, snapshot, and unavailable
//! decisions must remain visible through bounded identifiers and counters.

use std::collections::BTreeMap;

use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasUpsertCommand, SkillAuthorKind,
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceDestinationRouteResult,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand,
    SkillGovernanceRecordUsageCommand, SkillGovernanceStoreStrategy, SkillServicePolicyHints,
    SkillServiceScope, SkillUsageEventKind, SkillUsageObservation,
};

use crate::skill_service_provider_state::SkillProviderGovernanceState;

#[test]
fn local_governance_store_logs_key_audit_nodes_with_structured_fields() {
    let state_source = include_str!("skill_service_provider_state.rs");
    let unavailable_source = include_str!("../../../services/macaca-skill/src/governance_store.rs");

    assert!(state_source
        .contains("skill governance event appended through local compatibility adapter"));
    assert!(state_source.contains("event_kind = %governance_event_kind(&event.payload)"));
    assert!(state_source.contains("provenance_action = ?event.provenance.action"));
    assert!(state_source.contains("application_id = ?event.scope.application_id"));
    assert!(state_source
        .contains("skill governance read model replayed through local compatibility adapter"));
    assert!(state_source.contains("records = read_model.records.len()"));
    assert!(state_source.contains("provenance_events = read_model.provenance_events.len()"));
    assert!(state_source.contains("aliases = read_model.aliases.len()"));
    assert!(state_source
        .contains("skill governance snapshot built through local compatibility adapter"));
    assert!(state_source.contains("include_archived = include_archived"));
    assert!(state_source.contains("filtered_records = filtered_out"));

    assert!(unavailable_source.contains("skill governance store append unavailable"));
    assert!(unavailable_source.contains("skill governance store replay unavailable"));
    assert!(unavailable_source.contains("reason = %self.reason"));
}

#[tokio::test]
async fn local_governance_state_replays_through_store_strategy_adapter() {
    let state = SkillProviderGovernanceState::default();
    let trace = TraceContext::new("trace-local-governance-store-adapter");
    let scope = SkillServiceScope {
        application_id: None,
        session_id: Some("session-local-1".into()),
        tenant_id: Some("tenant-local-1".into()),
        agent_name: Some("agent".into()),
    };

    let mut usage_metadata = BTreeMap::new();
    usage_metadata.insert("task_id".into(), "task-local-1".into());
    state
        .record_usage(SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            observation: SkillUsageObservation {
                skill_id: "skill://agent/local".into(),
                name: "local-skill".into(),
                source: "test".into(),
                source_scope: "workspace".into(),
                event: SkillUsageEventKind::Used,
                author_kind: SkillAuthorKind::Agent,
                created_by: Some("agent".into()),
                pinned: None,
                evidence_id: Some("usage-evidence-1".into()),
                metadata: usage_metadata,
            },
        })
        .await;
    state
        .apply_lifecycle(
            SkillCurationLifecycleCommand {
                trace: trace.clone(),
                scope: scope.clone(),
                skill_id: "skill://agent/local".into(),
                name: "local-skill".into(),
                source: "test".into(),
                source_scope: "workspace".into(),
                author_kind: SkillAuthorKind::Agent,
                reason: "verified stale lifecycle metadata".into(),
                evidence_ids: vec!["lifecycle-evidence-1".into()],
                task_id: Some("task-local-1".into()),
                policy_decision_refs: vec!["policy://decision/local-lifecycle".into()],
                policy: SkillServicePolicyHints::default(),
            },
            SkillCurationLifecycleAction::Archive,
        )
        .await
        .expect("local lifecycle adapter should accept evidence");
    let now = chrono::Utc::now();
    state
        .upsert_alias(SkillAliasUpsertCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            record: SkillAliasRecord {
                source_skill_id: "skill://agent/old-local".into(),
                source_name: "old-local".into(),
                target_skill_id: "skill://agent/local".into(),
                target_name: "local-skill".into(),
                kind: SkillAliasKind::SupersededBy,
                rationale: "local test alias".into(),
                created_at: now,
                updated_at: now,
                evidence_ids: vec!["alias-evidence-1".into()],
            },
        })
        .await;
    state
        .propose_experience(
            SkillExperienceProposalCommand {
                trace: trace.clone(),
                scope,
                candidate: SkillExperienceCandidate {
                    task_id: "task-local-1".into(),
                    session_id: Some("session-local-1".into()),
                    application_id: None,
                    agent_name: Some("agent".into()),
                    verified_terminal_success: true,
                    evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
                    bounded_summary: "Verified local governance replay evidence.".into(),
                    trace_digest: Some("trace-digest://task/local-1".into()),
                    memory_digest_refs: vec!["memory://digest/local-1".into()],
                    reusable_procedure:
                        "Replay append-only local governance events through the store strategy adapter."
                            .into(),
                    classification: SkillEvolutionCandidateClassification::ReusableProcedure,
                    destination: SkillExperienceCandidateDestination::NewSkillDraft,
                    recommended_action: SkillEvolutionProposalAction::CreateDraft,
                    target_skill_id: None,
                    target_skill_name: Some("local-governance-replay".into()),
                    evidence_ids: vec!["proposal-evidence-1".into()],
                    metadata: BTreeMap::new(),
                },
            },
            SkillExperienceDestinationRouteResult::default(),
        )
        .await;

    let read_model = state
        .read_model()
        .await
        .expect("local in-memory state should replay through the store strategy");

    assert_eq!(read_model.records.len(), 1);
    assert_eq!(read_model.provenance_events.len(), 4);
    assert_eq!(read_model.records[0].telemetry.activation_count, 1);
    assert_eq!(
        read_model.provenance_events[0]
            .provenance
            .task_id
            .as_deref(),
        Some("task-local-1")
    );
    assert!(read_model.provenance_events.iter().all(|event| event
        .provenance
        .session_id
        .as_deref()
        == Some("session-local-1")));
    assert!(read_model
        .provenance_events
        .iter()
        .all(|event| event.provenance.tenant_id.as_deref() == Some("tenant-local-1")));
    let lifecycle_event = read_model
        .provenance_events
        .iter()
        .find(|event| matches!(event.action, macaca_skill::SkillProvenanceAction::Archived))
        .expect("lifecycle mutation should emit archived provenance");
    assert_eq!(
        lifecycle_event.provenance.task_id.as_deref(),
        Some("task-local-1")
    );
    assert_eq!(read_model.aliases.len(), 1);
    assert_eq!(read_model.proposals.len(), 1);
    assert_eq!(read_model.replayed_events, 4);
}
