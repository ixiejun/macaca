//! Contract tests for Skill governance store DTOs and fail-closed unavailable Strategy.
//!
//! Extracted from `governance_store.rs` so durable event/read-model contracts stay
//! under the OS 500-line constitution while privacy sanitization remains guarded.

use super::*;
use crate::governance::{SkillAuthorKind, SkillLifecycleState};

#[test]
fn durable_governance_dtos_capture_store_backed_skill_state_without_bodies() {
    let now = Utc::now();
    let provenance = SkillProvenance {
        skill_id: "skill://agent/refactor".into(),
        version: Some("1.0.0".into()),
        author_kind: SkillAuthorKind::Agent,
        author_agent_id: Some("agent-maintainer".into()),
        application_id: Some("app-alpha".into()),
        session_id: Some("session-1".into()),
        tenant_id: Some("tenant-alpha".into()),
        task_id: Some("task-1".into()),
        trace_id: "trace-1".into(),
        evidence_refs: vec!["store://evidence/task-1".into()],
        source_scope: "workspace".into(),
        trust_level: Some("verified".into()),
        created_at: now,
        updated_at: now,
    };
    let telemetry = SkillUsageTelemetry {
        view_count: 1,
        activation_count: 2,
        resource_read_count: 3,
        use_count: 2,
        patch_count: 4,
        successful_task_count: 5,
        failed_task_count: 6,
        last_viewed_at: Some(now),
        last_resource_read_at: Some(now),
        last_used_at: Some(now),
        last_patched_at: Some(now),
        last_successful_task_at: Some(now),
        last_failed_task_at: Some(now),
        last_lifecycle_event_at: Some(now),
        last_observed_at: Some(now),
    };
    let record = DurableSkillGovernanceRecord {
        skill_id: provenance.skill_id.clone(),
        lifecycle: SkillLifecycleState::Active,
        pinned: false,
        source_scope: provenance.source_scope.clone(),
        ownership: "agent-private".into(),
        trust_level: provenance.trust_level.clone(),
        evidence_refs: provenance.evidence_refs.clone(),
        policy_decision_refs: vec!["policy://decision/1".into()],
        audit_event_ids: vec!["audit://event/1".into()],
        provenance,
        telemetry,
        created_at: now,
        updated_at: now,
    };
    let alias = SkillAliasMap {
        source_skill_id: "skill://agent/old".into(),
        target_skill_id: "skill://agent/refactor".into(),
        reason: "superseded by a broader maintained skill".into(),
        run_id: Some("curation-run-1".into()),
        valid_from: now,
        valid_until: Some(now),
        resolution_policy: SkillAliasResolutionPolicy::WarnAndRedirect,
    };

    let serialized =
        serde_json::to_string(&(record, alias)).expect("durable governance DTOs should serialize");

    assert!(serialized.contains("activation_count"));
    assert!(serialized.contains("WarnAndRedirect"));
    assert!(!serialized.contains("SKILL.md body"));
}

#[tokio::test]
async fn unavailable_governance_store_strategy_returns_structured_unavailable() {
    let store = UnavailableSkillGovernanceStore::new("Store/EventLog provider is absent");
    let trace = TraceContext::new("trace-governance-store-unavailable");
    let event = SkillGovernanceEventRecord::new(
        "event-unavailable-1",
        &trace,
        SkillServiceScope::default(),
        Utc::now(),
        SkillGovernanceEventPayload::UsageRecorded(SkillUsageObservation {
            skill_id: "skill://agent/missing-store".into(),
            name: "missing-store".into(),
            source: "test".into(),
            source_scope: "workspace".into(),
            event: crate::governance::SkillUsageEventKind::Viewed,
            author_kind: SkillAuthorKind::Agent,
            created_by: None,
            pinned: None,
            evidence_id: Some("evidence-1".into()),
            metadata: BTreeMap::new(),
        }),
    );

    let append_err = store
        .append_event(event)
        .await
        .expect_err("unavailable store must not fake append success");
    let replay_err = store
        .read_model()
        .await
        .expect_err("unavailable store must not fake replay success");

    assert_eq!(
        append_err.kind,
        SkillGovernanceStoreUnavailableKind::Unavailable
    );
    assert_eq!(
        replay_err.kind,
        SkillGovernanceStoreUnavailableKind::Unavailable
    );
    assert!(!append_err.retryable_without_reconfiguration);
    assert!(append_err
        .reason
        .contains("Store/EventLog provider is absent"));
}

#[test]
fn governance_event_record_strips_raw_observation_metadata() {
    let mut metadata = BTreeMap::new();
    metadata.insert("task_id".into(), "task-safe".into());
    metadata.insert("evidence_ref.1".into(), "evidence://task/extra".into());
    metadata.insert("raw_prompt".into(), "sensitive prompt".into());
    metadata.insert("provider_payload_ref".into(), "provider://raw".into());
    metadata.insert("long_ref".into(), "x".repeat(300));

    let event = SkillGovernanceEventRecord::new(
        "event-sanitize-1",
        &TraceContext::new("trace-event-sanitize"),
        SkillServiceScope::default(),
        Utc::now(),
        SkillGovernanceEventPayload::UsageRecorded(SkillUsageObservation {
            skill_id: "skill://agent/safe".into(),
            name: "safe".into(),
            source: "test".into(),
            source_scope: "workspace".into(),
            event: crate::governance::SkillUsageEventKind::SuccessfulTask,
            author_kind: SkillAuthorKind::Agent,
            created_by: Some("agent".into()),
            pinned: None,
            evidence_id: Some("evidence://task/primary".into()),
            metadata,
        }),
    );

    let serialized =
        serde_json::to_string(&event).expect("governance event should serialize safely");
    assert!(serialized.contains("task-safe"));
    assert!(serialized.contains("evidence://task/extra"));
    assert_eq!(
        match &event.payload {
            SkillGovernanceEventPayload::UsageRecorded(observation) => observation
                .metadata
                .get("long_ref")
                .expect("safe refs are retained")
                .chars()
                .count(),
            _ => 0,
        },
        256
    );
    assert!(!serialized.contains("sensitive prompt"));
    assert!(!serialized.contains("provider://raw"));
}
