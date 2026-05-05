//! Tests for profile policy ordering, loader containment, and provider wiring.
//!
//! `MEMORY.md` coverage proves the provider only surfaces text — there is no `macaca-memory`
//! dependency in `ProfileFileContextProvider`, so vector ingestion cannot occur in this module.

use std::path::Path;

use macaca_proto::config::AgentProfileContextConfig;
use tempfile::tempdir;
use tokio::fs;

use crate::composer::{ContextComposeContext, ContextProvider};
use crate::engine::ContextAssembleInput;
use crate::profile::kinds::AgentProfileFileKind;
use crate::profile::loader::{canonical_root, load_profile_file, ProfileSkipReason};
use crate::profile::policy::default_policy_for;
use crate::profile::provider::ProfileFileContextProvider;
use macaca_proto::LlmOptions;

#[test]
fn default_priority_order_matches_spec() {
    let cfg = AgentProfileContextConfig::default();
    let agents = default_policy_for(AgentProfileFileKind::Agents, &cfg).priority;
    let soul = default_policy_for(AgentProfileFileKind::Soul, &cfg).priority;
    let tools = default_policy_for(AgentProfileFileKind::Tools, &cfg).priority;
    assert!(agents > soul && soul > tools);
}

#[tokio::test]
async fn missing_files_yield_none_without_error() {
    let dir = tempdir().unwrap();
    let cfg = AgentProfileContextConfig {
        enabled: true,
        max_file_bytes: 1024,
        ..Default::default()
    };
    let r = load_profile_file(dir.path(), AgentProfileFileKind::Identity, &cfg, false)
        .await
        .unwrap();
    assert!(r.is_none());
}

#[tokio::test]
async fn oversized_read_is_truncated() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("SOUL.md");
    let body = vec![b'a'; 16];
    fs::write(&path, &body).await.unwrap();

    let cfg = AgentProfileContextConfig {
        enabled: true,
        max_file_bytes: 8,
        ..Default::default()
    };
    let out = load_profile_file(dir.path(), AgentProfileFileKind::Soul, &cfg, false)
        .await
        .unwrap()
        .expect("soul exists");
    assert!(out.truncated_by_cap);
    assert_eq!(out.text.len(), 8);
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let evil = dir.path().join("IDENTITY.md");
    symlink("/etc/hosts", &evil).expect("symlink");

    let cfg = AgentProfileContextConfig {
        enabled: true,
        max_file_bytes: 64,
        ..Default::default()
    };

    let err = load_profile_file(dir.path(), AgentProfileFileKind::Identity, &cfg, false)
        .await
        .expect_err("escape");
    match err {
        ProfileSkipReason::PathNotConfinedToRoot => {}
        other => panic!("unexpected err: {other}"),
    }
}

#[tokio::test]
async fn memory_file_is_candidate_with_audit_diagnostics() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("MEMORY.md"),
        "seed: prefer concise summaries",
    )
    .await
    .unwrap();

    let cfg = AgentProfileContextConfig {
        enabled: true,
        include_memory_seed: true,
        max_file_bytes: 1024,
        ..Default::default()
    };

    let assemble = ContextAssembleInput {
        app_id: None,
        session_id: None,
        agent_name: "t".into(),
        model: "m".into(),
        base_messages: Vec::new(),
        options: LlmOptions::default(),
        budget: crate::budget::ContextBudget::default(),
    };
    let ctx = ContextComposeContext {
        assemble_input: &assemble,
    };

    let prov = ProfileFileContextProvider::new(dir.path().to_path_buf(), cfg);
    let out = prov.contribute(&ctx).await.unwrap();
    let mem = out
        .candidates
        .iter()
        .find(|c| c.source_id.contains("MEMORY"))
        .expect("memory candidate");
    assert!(
        mem.diagnostics
            .iter()
            .any(|d| d.contains("memory_seed_audit_only")),
        "{:?}",
        mem.diagnostics
    );
}

#[test]
fn canonical_root_errors_when_missing() {
    let absent = Path::new("/nonexistent/macaca_profile_root_test");
    assert!(canonical_root(absent).is_err());
}
