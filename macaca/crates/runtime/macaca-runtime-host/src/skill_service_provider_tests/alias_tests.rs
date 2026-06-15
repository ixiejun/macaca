//! Contract tests for Skill alias upsert, resolve, snapshot, and policy statuses.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillAliasKind, SkillAliasRecord, SkillAliasResolutionPolicy, SkillAliasResolutionStatus,
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillServiceScope,
    SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::alias_helpers::{resolve_alias_for_policy_test, upsert_alias_for_policy_test};
use super::fixtures::{alias_record, traced_command};

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
