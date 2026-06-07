//! Contract and unit tests for CLI Skill operations adapters.

use super::contract_source::skill_operations_module_sources;
use super::support::{live_operator_payload, normalize_api_base};
use super::types::{SkillCliEvidenceRefs, SkillCliLifecycleAction, SkillCliRuntimeTarget};
use super::execute_skill_operations_snapshot;

#[tokio::test]
async fn cli_skill_snapshot_uses_sdk_null_object() {
    execute_skill_operations_snapshot(SkillCliRuntimeTarget::default())
        .await
        .unwrap();
}

#[test]
fn cli_skill_live_target_builds_public_web_api_url() {
    let target = SkillCliRuntimeTarget {
        app_id: Some("2c96f3f2-b78c-5edd-beb4-740c8c004910".into()),
        api_base: Some("http://127.0.0.1:3001/".into()),
    };
    let client = target.live_client().unwrap().unwrap();
    assert_eq!(
        client.url("/curation/run"),
        "http://127.0.0.1:3001/api/apps/2c96f3f2-b78c-5edd-beb4-740c8c004910/skills/operations/curation/run"
    );
}

#[test]
fn cli_skill_live_payload_keeps_operator_refs_bounded() {
    let payload = live_operator_payload(SkillCliEvidenceRefs {
        reason: Some("approved curation".into()),
        evidence_ref: Some("evidence://run/1".into()),
        policy_ref: Some("policy://decision/1".into()),
        approval_ref: Some("approval://operator/1".into()),
    })
    .with_curation_thresholds(14, 2);
    let json = serde_json::to_value(payload).unwrap();
    assert_eq!(json["reason"], "approved curation");
    assert_eq!(json["evidence_ids"][0], "evidence://run/1");
    assert_eq!(json["policy_decision_refs"][0], "policy://decision/1");
    assert_eq!(json["approval_refs"][0], "approval://operator/1");
    assert_eq!(json["stale_after_days"], 14);
    assert_eq!(json["narrow_use_threshold"], 2);
}

#[test]
fn cli_skill_live_target_rejects_non_http_api_base() {
    assert!(normalize_api_base("file:///tmp/socket").is_err());
    assert_eq!(
        normalize_api_base("http://127.0.0.1:3001/").unwrap(),
        "http://127.0.0.1:3001"
    );
}

#[test]
fn cli_skill_lifecycle_uses_public_route_segments() {
    assert_eq!(SkillCliLifecycleAction::Pin.route_segment(), "pin");
    assert_eq!(
        SkillCliLifecycleAction::ReleaseQuarantine.route_segment(),
        "release-quarantine"
    );
}

#[test]
fn cli_skill_operations_do_not_import_runtime_or_web() {
    let source = skill_operations_module_sources();
    let runtime_host_import = ["macaca", "_runtime_host::"].concat();
    let web_import = ["macaca", "_web::"].concat();
    let provider_state_symbol = ["SkillProvider", "GovernanceState"].concat();
    assert!(source.contains("UnavailableSystemSkillClient"));
    assert!(!source.contains(&runtime_host_import));
    assert!(!source.contains(&web_import));
    assert!(!source.contains(&provider_state_symbol));
}
