use std::collections::BTreeMap;

use chrono::{Duration, Utc};
use macaca_proto::TraceContext;

use crate::{
    SkillAuthorKind, SkillCurationAction, SkillCurationDryRunCommand, SkillCurationDryRunResult,
    SkillCurationPhase, SkillGovernanceRecord, SkillLifecycleState, SkillServiceScope,
    SkillUsageEventKind, SkillUsageObservation,
};

#[test]
fn deterministic_curation_reports_all_service_owned_phases() {
    let command = SkillCurationDryRunCommand {
        trace: TraceContext::new("trace-deterministic-curation-phases"),
        scope: SkillServiceScope::default(),
        stale_after_days: 1,
        narrow_use_threshold: 0,
    };
    let result = SkillCurationDryRunResult::from_records(
        vec![
            inactive_record("skill://agent/stale", SkillLifecycleState::Active),
            inactive_record("skill://agent/archive", SkillLifecycleState::Stale),
            diagnostic_record(
                "skill://agent/quarantine",
                [("skill.quarantine_required", "true")],
            ),
            diagnostic_record("skill://agent/size", [("skill.size_bytes", "70000")]),
            diagnostic_record(
                "skill://agent/metadata",
                [("skill.metadata_valid", "false")],
            ),
            diagnostic_record(
                "skill://agent/dependency",
                [("skill.missing_dependencies", "tool://missing")],
            ),
            protected_record("skill://system/protected"),
        ],
        &command,
        Utc::now(),
    );

    assert_recommendation(
        &result,
        "skill://agent/stale",
        SkillCurationAction::WouldMarkStale,
        SkillCurationPhase::Stale,
    );
    assert_recommendation(
        &result,
        "skill://agent/archive",
        SkillCurationAction::WouldArchive,
        SkillCurationPhase::Archive,
    );
    assert_recommendation(
        &result,
        "skill://agent/quarantine",
        SkillCurationAction::WouldQuarantine,
        SkillCurationPhase::Quarantine,
    );
    assert_recommendation(
        &result,
        "skill://agent/size",
        SkillCurationAction::WouldReduceSize,
        SkillCurationPhase::Size,
    );
    assert_recommendation(
        &result,
        "skill://agent/metadata",
        SkillCurationAction::WouldFixMetadata,
        SkillCurationPhase::InvalidMetadata,
    );
    assert_recommendation(
        &result,
        "skill://agent/dependency",
        SkillCurationAction::WouldResolveMissingDependency,
        SkillCurationPhase::MissingDependency,
    );
    assert_recommendation(
        &result,
        "skill://system/protected",
        SkillCurationAction::Protected,
        SkillCurationPhase::Protected,
    );
    assert!(!result.mutated);
}

#[test]
fn deterministic_curation_protects_package_ownership_classes_from_auto_archive() {
    let command = SkillCurationDryRunCommand {
        trace: TraceContext::new("trace-deterministic-curation-ownership"),
        scope: SkillServiceScope::default(),
        stale_after_days: 1,
        narrow_use_threshold: 0,
    };
    let records = [
        ("skill://ownership/bundled", "bundled"),
        ("skill://ownership/marketplace", "marketplace"),
        ("skill://ownership/application", "application_owned"),
        ("skill://ownership/paid", "paid"),
        ("skill://ownership/encrypted", "encrypted"),
        ("skill://ownership/plugin", "plugin_provided"),
    ]
    .into_iter()
    .map(|(skill_id, ownership)| {
        let mut record = base_record(
            skill_id,
            SkillAuthorKind::Agent,
            metadata_map([("skill.package_ownership", ownership)]),
        );
        record.lifecycle = SkillLifecycleState::Stale;
        record.provenance.created_at = Utc::now() - Duration::days(10);
        record
    });

    let result = SkillCurationDryRunResult::from_records(records, &command, Utc::now());

    for recommendation in result.recommendations {
        assert_eq!(recommendation.action, SkillCurationAction::Protected);
        assert!(recommendation.protected);
        assert!(recommendation
            .phases
            .contains(&SkillCurationPhase::Protected));
    }
}

fn inactive_record(skill_id: &str, lifecycle: SkillLifecycleState) -> SkillGovernanceRecord {
    let mut record = base_record(skill_id, SkillAuthorKind::Agent, BTreeMap::new());
    record.lifecycle = lifecycle;
    record.provenance.created_at = Utc::now() - Duration::days(10);
    record
}

fn diagnostic_record<const N: usize>(
    skill_id: &str,
    metadata: [(&str, &str); N],
) -> SkillGovernanceRecord {
    base_record(skill_id, SkillAuthorKind::Agent, metadata_map(metadata))
}

fn protected_record(skill_id: &str) -> SkillGovernanceRecord {
    base_record(skill_id, SkillAuthorKind::System, BTreeMap::new())
}

fn base_record(
    skill_id: &str,
    author_kind: SkillAuthorKind,
    metadata: BTreeMap<String, String>,
) -> SkillGovernanceRecord {
    let observed_at = Utc::now();
    let observation = SkillUsageObservation {
        skill_id: skill_id.into(),
        name: skill_id.replace("skill://", ""),
        source: "test".into(),
        source_scope: "workspace".into(),
        event: SkillUsageEventKind::Created,
        author_kind,
        created_by: Some("agent".into()),
        pinned: None,
        evidence_id: Some(format!("{skill_id}#evidence")),
        metadata,
    };
    SkillGovernanceRecord::from_observation(&observation, observed_at)
}

fn metadata_map<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn assert_recommendation(
    result: &SkillCurationDryRunResult,
    skill_id: &str,
    action: SkillCurationAction,
    phase: SkillCurationPhase,
) {
    let recommendation = result
        .recommendations
        .iter()
        .find(|candidate| candidate.skill_id == skill_id)
        .expect("deterministic phase recommendation should exist");
    assert_eq!(recommendation.action, action);
    assert!(recommendation.phases.contains(&phase));
}
