//! Sanitization gates for self-evolving Skill operations.
//!
//! The self-evolving Skill OS may receive raw content at privileged command
//! boundaries, but public replay models, reports, route results, and logs must
//! contain only bounded metadata, ids, refs, and structured states.

use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillContentMutationCommand, SkillContentMutationKind, SkillContentMutationResult,
    SkillContentMutationStatus, SkillGovernanceEventPayload, SkillGovernanceEventRecord,
    SkillGovernanceReadModel, SkillSemanticReviewResult, SkillServicePolicyHints,
    SkillServiceScope, SkillUsageEventKind, SkillUsageObservation, SKILL_CONTENT_MUTATE_COMMAND,
};

use crate::SkillSystemServiceProvider;

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test payload should serialize"),
        trace,
    )
}

fn approved_policy() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.mutation".into()],
        entitlement_ready: Some(true),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

#[test]
fn governance_replay_drops_raw_metadata_from_snapshots() {
    let trace = TraceContext::new("trace-skill-sanitized-replay");
    let mut metadata = BTreeMap::new();
    metadata.insert("task_id".into(), "task-sanitized".into());
    metadata.insert("evidence_ref".into(), "evidence://safe".into());
    metadata.insert("raw_prompt".into(), "RAW_PROMPT_SHOULD_NOT_SURFACE".into());
    metadata.insert(
        "provider_payload".into(),
        "RAW_PROVIDER_PAYLOAD_SHOULD_NOT_SURFACE".into(),
    );
    metadata.insert("manifest_body".into(), "MANIFEST_SHOULD_NOT_SURFACE".into());
    metadata.insert(
        "package_bytes".into(),
        "PACKAGE_BYTES_SHOULD_NOT_SURFACE".into(),
    );
    metadata.insert(
        "skill_body".into(),
        "FULL_SKILL_BODY_SHOULD_NOT_SURFACE".into(),
    );
    metadata.insert("credential".into(), "CREDENTIAL_SHOULD_NOT_SURFACE".into());
    metadata.insert(
        "raw_signature".into(),
        "SIGNATURE_SHOULD_NOT_SURFACE".into(),
    );

    let read_model = SkillGovernanceReadModel::from_events([SkillGovernanceEventRecord::new(
        "event-sanitized-replay",
        &trace,
        SkillServiceScope::default(),
        chrono::Utc::now(),
        SkillGovernanceEventPayload::UsageRecorded(SkillUsageObservation {
            skill_id: "skill://agent/sanitized".into(),
            name: "sanitized-skill".into(),
            source: "test".into(),
            source_scope: "workspace".into(),
            event: SkillUsageEventKind::Viewed,
            author_kind: macaca_skill::SkillAuthorKind::Agent,
            created_by: Some("agent".into()),
            pinned: None,
            evidence_id: Some("evidence://safe-primary".into()),
            metadata,
        }),
    )]);
    let encoded = serde_json::to_string(&read_model).expect("read model should serialize");

    assert!(encoded.contains("evidence://safe"));
    for forbidden in [
        "RAW_PROMPT_SHOULD_NOT_SURFACE",
        "RAW_PROVIDER_PAYLOAD_SHOULD_NOT_SURFACE",
        "MANIFEST_SHOULD_NOT_SURFACE",
        "PACKAGE_BYTES_SHOULD_NOT_SURFACE",
        "FULL_SKILL_BODY_SHOULD_NOT_SURFACE",
        "CREDENTIAL_SHOULD_NOT_SURFACE",
        "SIGNATURE_SHOULD_NOT_SURFACE",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "{forbidden} leaked into replay"
        );
    }
}

#[test]
fn semantic_report_status_sanitizes_provider_failure_payloads() {
    let result = SkillSemanticReviewResult::provider_error(
        "test-semantic-review",
        "raw_provider_payload prompt secret credential should be redacted",
        chrono::Utc::now(),
    );
    let encoded = serde_json::to_string(&result).expect("semantic report should serialize");
    let status = result.status_message();

    for public_text in [encoded, status] {
        assert!(!public_text.contains("raw_provider_payload"));
        assert!(!public_text.contains("provider_payload"));
        assert!(!public_text.contains("prompt"));
        assert!(!public_text.contains("secret"));
    }
}

#[tokio::test]
async fn mutation_route_result_does_not_echo_raw_content_or_full_skill_body() {
    let temp = tempfile::tempdir().expect("temp skill package root should exist");
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-sanitized-mutation-result");
    let raw_content =
        b"raw prompt text and provider_payload-like full skill body that may be written".to_vec();
    let command = SkillContentMutationCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/sanitized-mutation".into(),
        package_root: temp.path().to_path_buf(),
        relative_path: "references/sanitized.md".into(),
        kind: SkillContentMutationKind::WriteSupportFile,
        ownership: macaca_skill::SkillPackageOwnershipClass::AgentPrivate,
        content: Some(raw_content),
        reason: "approved sanitized result boundary test".into(),
        evidence_ids: vec!["evidence://sanitized-mutation".into()],
        policy_decision_refs: vec!["policy://decision/sanitized-mutation".into()],
        allow_executable_script_mutation: false,
        policy: approved_policy(),
        metadata: BTreeMap::new(),
    };

    let result = provider
        .call(traced_command(SKILL_CONTENT_MUTATE_COMMAND, command, trace))
        .await
        .expect("approved mutation should return a structured route result");
    let encoded_output =
        serde_json::to_string(&result.output).expect("route result should serialize");
    let result: SkillContentMutationResult =
        serde_json::from_value(result.output).expect("mutation result should decode");

    assert_eq!(result.status, SkillContentMutationStatus::Applied);
    assert!(result.mutated);
    assert!(!encoded_output.contains("raw prompt text"));
    assert!(!encoded_output.contains("provider_payload-like"));
    assert!(!encoded_output.contains("full skill body"));
}

#[test]
fn skill_operation_logs_are_source_guarded_against_raw_payload_fields() {
    let mutation_source = include_str!("skill_service_content_mutation.rs");
    let curation_source = concat!(
        include_str!("skill_service_provider_curation/mod.rs"),
        include_str!("skill_service_provider_curation/store.rs"),
    );
    let provider_source = concat!(
        include_str!("skill_service_provider/system_service.rs"),
        include_str!("skill_service_provider/command_dispatch.rs"),
        include_str!("skill_service_provider/runtime_facade_commands.rs"),
        include_str!("skill_service_provider/governance_alias_commands.rs"),
        include_str!("skill_service_provider/evolution_commands.rs"),
        include_str!("skill_service_provider/evaluation_content_commands.rs"),
    );

    assert!(mutation_source.contains("skill content mutation applied through local strategy"));
    assert!(curation_source.contains("skill curation run completed"));
    assert!(provider_source.contains("skill service command accepted"));
    for source in [mutation_source, curation_source, provider_source] {
        assert_no_tracing_field(source, "content");
        assert_no_tracing_field(source, "payload");
        assert_no_tracing_field(source, "provider_payload");
        assert_no_tracing_field(source, "skill_body");
        assert_no_tracing_field(source, "package_bytes");
    }
}

fn assert_no_tracing_field(source: &str, field_name: &str) {
    let field_pattern = format!("{field_name} =");
    let mut in_tracing_macro = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("tracing::") {
            in_tracing_macro = true;
        }
        if in_tracing_macro {
            assert!(
                !trimmed.contains(&field_pattern),
                "unsafe tracing field `{field_name}` found in `{trimmed}`"
            );
        }
        if in_tracing_macro && trimmed.ends_with(");") {
            in_tracing_macro = false;
        }
    }
}
