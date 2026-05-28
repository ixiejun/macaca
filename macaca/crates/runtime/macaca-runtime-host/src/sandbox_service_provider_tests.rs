//! Focused tests for `service.sandbox`.
//!
//! The tests exercise permission-profile resolution, optional provider
//! unavailability, policy explanations, cleanup after cancellation, and
//! resource lease release without encoding any application-specific workflow.

use macaca_kernel::SystemService;
use macaca_proto::workbench::sandbox::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::SandboxSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<SandboxServiceResponse> {
    serde_json::from_value(value).unwrap()
}

fn prepare(root: &tempfile::TempDir) -> SandboxEnvironmentPrepareRequest {
    SandboxEnvironmentPrepareRequest {
        requested_profile_ref: WORKSPACE_WRITE_PROFILE.into(),
        runtime_kind: SandboxRuntimeKind::LocalSandbox,
        scope: SandboxEnvironmentScope::Session,
        workspace_root: Some(root.path().to_string_lossy().to_string()),
        cwd: Some(".".into()),
        application_id: Some("app.sandbox.test".into()),
        session_id: Some("session.sandbox.test".into()),
        thread_ref: Some("thread.sandbox.test".into()),
        resource_request_refs: vec!["process".into(), "artifact_root".into()],
    }
}

#[tokio::test]
async fn profile_catalog_resolves_workspace_write_profile() {
    let provider = SandboxSystemServiceProvider::local();

    let listed = decode(
        provider
            .call(command(
                PROFILE_LIST_COMMAND,
                SandboxProfileListRequest {
                    application_id: None,
                    workspace_root: None,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(SandboxServiceResponse::ProfileList { profiles }) = listed.output else {
        panic!("expected profile list");
    };
    assert!(profiles
        .iter()
        .any(|profile| profile.profile_ref == WORKSPACE_WRITE_PROFILE));

    let resolved = decode(
        provider
            .call(command(
                PROFILE_RESOLVE_COMMAND,
                SandboxProfileResolveRequest {
                    requested_profile_ref: WORKSPACE_WRITE_PROFILE.into(),
                    application_id: None,
                    workspace_root: None,
                    runtime_kind: SandboxRuntimeKind::LocalSandbox,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        resolved.output,
        Some(SandboxServiceResponse::ProfileResolved { profile })
            if profile.can_write_workspace
    ));
}

#[tokio::test]
async fn optional_docker_provider_returns_structured_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let provider = SandboxSystemServiceProvider::local();
    let mut request = prepare(&root);
    request.runtime_kind = SandboxRuntimeKind::Docker;

    let result = decode(
        provider
            .call(command(ENVIRONMENT_PREPARE_COMMAND, request))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
    assert!(matches!(
        result.output,
        Some(SandboxServiceResponse::EnvironmentPrepared { record })
            if record.state == SandboxEnvironmentState::Unavailable
    ));
}

#[tokio::test]
async fn cleanup_after_cancellation_releases_resource_leases() {
    let root = tempfile::tempdir().unwrap();
    let provider = SandboxSystemServiceProvider::local();

    let prepared = decode(
        provider
            .call(command(ENVIRONMENT_PREPARE_COMMAND, prepare(&root)))
            .await
            .unwrap()
            .output,
    );
    let Some(SandboxServiceResponse::EnvironmentPrepared { record }) = prepared.output else {
        panic!("expected prepared record");
    };

    let cleaned = decode(
        provider
            .call(command(
                ENVIRONMENT_CLEANUP_COMMAND,
                SandboxEnvironmentCleanupRequest {
                    environment_id: record.environment_id,
                    reason_code: "cancelled".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        cleaned.output,
        Some(SandboxServiceResponse::EnvironmentCleaned {
            record,
            released_lease_refs
        }) if record.resource_leases.iter().all(|lease| lease.released)
            && released_lease_refs.len() == 2
    ));
}

#[tokio::test]
async fn policy_explanation_reports_network_and_write_denials() {
    let root = tempfile::tempdir().unwrap();
    let provider = SandboxSystemServiceProvider::local();

    let explained = decode(
        provider
            .call(command(
                POLICY_EXPLAIN_COMMAND,
                SandboxPolicyExplainRequest {
                    requested_profile_ref: READ_ONLY_PROFILE.into(),
                    runtime_kind: SandboxRuntimeKind::LocalSandbox,
                    workspace_root: Some(root.path().to_string_lossy().to_string()),
                    path: Some("target.txt".into()),
                    network_host_ref: Some("host.example".into()),
                    env_var_key: Some("API_SECRET".into()),
                    write_requested: true,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(SandboxServiceResponse::PolicyExplanation { decisions }) = explained.output else {
        panic!("expected policy explanation");
    };
    assert!(decisions
        .iter()
        .any(|decision| decision.rule == "network" && !decision.allowed));
    assert!(decisions
        .iter()
        .any(|decision| decision.rule == "write_scope" && !decision.allowed));
    assert!(decisions
        .iter()
        .any(|decision| decision.rule == "environment_variable" && !decision.allowed));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_unavailable() {
    let provider = SandboxSystemServiceProvider::unavailable("sandbox disabled");

    let result = decode(
        provider
            .call(command(
                ENVIRONMENT_HEALTH_COMMAND,
                SandboxEnvironmentHealthRequest {
                    environment_id: None,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}
