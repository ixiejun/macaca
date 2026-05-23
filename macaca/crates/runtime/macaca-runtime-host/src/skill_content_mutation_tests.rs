use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_skill::{
    SkillContentMutationCommand, SkillContentMutationKind, SkillContentMutationResult,
    SkillContentMutationStatus, SkillPackageOwnershipClass, SkillServicePolicyHints,
    SkillServiceScope, SKILL_CONTENT_MUTATE_COMMAND,
};

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

fn approved_policy() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: vec!["skill.mutation".into()],
        entitlement_ready: Some(true),
        package_ready: Some(true),
        metadata: BTreeMap::new(),
    }
}

fn mutation_command(
    root: std::path::PathBuf,
    relative_path: &str,
    content: Option<Vec<u8>>,
    ownership: SkillPackageOwnershipClass,
) -> SkillContentMutationCommand {
    SkillContentMutationCommand {
        trace: TraceContext::new("trace-skill-content-mutation"),
        scope: SkillServiceScope::default(),
        skill_id: "skill://agent/mutable".into(),
        package_root: root,
        relative_path: relative_path.into(),
        kind: SkillContentMutationKind::WriteSupportFile,
        ownership,
        content,
        reason: "approved support-file materialization".into(),
        evidence_ids: vec!["evidence://mutation/1".into()],
        policy_decision_refs: vec!["policy://decision/mutation".into()],
        allow_executable_script_mutation: false,
        policy: approved_policy(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn skill_content_mutation_writes_support_file_with_rollback_memento() {
    let temp = tempfile::tempdir().expect("temp skill package root should exist");
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-content-mutation-write");
    let command = mutation_command(
        temp.path().to_path_buf(),
        "references/verified-task.md",
        Some(b"# Verified Task\n\nReusable bounded procedure.\n".to_vec()),
        SkillPackageOwnershipClass::AgentPrivate,
    );

    let result = provider
        .call(traced_command(
            SKILL_CONTENT_MUTATE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("approved support-file mutation should be applied");
    let result: SkillContentMutationResult =
        serde_json::from_value(result.output).expect("mutation result should decode");

    assert_eq!(result.status, SkillContentMutationStatus::Applied);
    assert!(result.mutated);
    assert!(result.rollback_memento_ref.is_some());
    assert_eq!(result.bytes_written, 45);
    let written = tokio::fs::read_to_string(temp.path().join("references/verified-task.md"))
        .await
        .expect("support file should be written");
    assert!(written.contains("Reusable bounded procedure."));
    let memento_dir = temp.path().join(".macaca/skill-mutation-mementos");
    assert!(memento_dir.exists());
}

#[tokio::test]
async fn skill_content_mutation_denies_executable_script_without_policy() {
    let temp = tempfile::tempdir().expect("temp skill package root should exist");
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-content-mutation-script-denied");
    let command = mutation_command(
        temp.path().to_path_buf(),
        "scripts/run.sh",
        Some(b"#!/bin/sh\necho safe\n".to_vec()),
        SkillPackageOwnershipClass::AgentPrivate,
    );

    let result = provider
        .call(traced_command(
            SKILL_CONTENT_MUTATE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("validation denial should be returned as structured result");
    let result: SkillContentMutationResult =
        serde_json::from_value(result.output).expect("mutation denial should decode");

    assert_eq!(result.status, SkillContentMutationStatus::Denied);
    assert!(!result.mutated);
    assert!(result
        .denied_reason
        .as_deref()
        .unwrap_or_default()
        .contains("executable script"));
    assert!(!temp.path().join("scripts/run.sh").exists());
}

#[tokio::test]
async fn skill_content_mutation_denies_protected_package_ownership() {
    let temp = tempfile::tempdir().expect("temp skill package root should exist");
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-content-mutation-protected-denied");
    let command = mutation_command(
        temp.path().to_path_buf(),
        "references/marketplace.md",
        Some(b"marketplace overlay should stay draft-only\n".to_vec()),
        SkillPackageOwnershipClass::Marketplace,
    );

    let result = provider
        .call(traced_command(
            SKILL_CONTENT_MUTATE_COMMAND,
            command,
            trace.clone(),
        ))
        .await
        .expect("protected package denial should be structured");
    let result: SkillContentMutationResult =
        serde_json::from_value(result.output).expect("mutation denial should decode");

    assert_eq!(result.status, SkillContentMutationStatus::Denied);
    assert!(result
        .denied_reason
        .as_deref()
        .unwrap_or_default()
        .contains("protected skill package"));
    assert!(!temp.path().join("references/marketplace.md").exists());
}
