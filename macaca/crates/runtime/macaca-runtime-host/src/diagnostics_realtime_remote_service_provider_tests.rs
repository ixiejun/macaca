//! Focused tests for diagnostics, realtime, and remote environment services.
//!
//! The tests prove privacy redaction, optional-module absence, and generic
//! remote environment state without encoding application-specific workflow.

use macaca_kernel::SystemService;
use macaca_proto::workbench::diagnostics as diag;
use macaca_proto::workbench::realtime as rt;
use macaca_proto::workbench::remote_environment as renv;
use macaca_proto::{
    BoundedSummary, ServiceCommand, ServiceCommandName, ServiceHealth, TraceContext,
    WorkbenchCommandResult, WorkbenchCommandStatus,
};

use crate::{
    DiagnosticsSystemServiceProvider, RealtimeSystemServiceProvider,
    RemoteEnvironmentSystemServiceProvider,
};

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> WorkbenchCommandResult<T> {
    serde_json::from_value(value).unwrap()
}

fn summary(text: &str) -> BoundedSummary {
    BoundedSummary {
        text: text.into(),
        truncated: false,
        original_byte_len: Some(text.len() as u64),
        artifact_ref: None,
    }
}

#[tokio::test]
async fn diagnostics_snapshot_redacts_sensitive_source_summaries() {
    let provider = DiagnosticsSystemServiceProvider::mock();
    provider
        .upsert_source(diag::DiagnosticsSourceSnapshot {
            source: diag::DiagnosticsSourceKind::Process,
            service_id: "service.process".into(),
            health: ServiceHealth::Healthy,
            summary: Some(summary("process token should never leak")),
            artifact_refs: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
        .await;

    let result = decode::<diag::DiagnosticsServiceResponse>(
        provider
            .call(command(
                diag::SNAPSHOT_COMMAND,
                diag::DiagnosticsSnapshotRequest {
                    include_sources: vec![diag::DiagnosticsSourceKind::Process],
                    redaction_profile: diag::DiagnosticsRedactionProfile::default(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(diag::DiagnosticsServiceResponse::Snapshot(bundle)) = result.output else {
        panic!("expected diagnostics snapshot");
    };
    assert_eq!(bundle.sources.len(), 1);
    assert_eq!(
        bundle.sources[0].summary.as_ref().unwrap().text,
        "<redacted>"
    );
    assert!(matches!(result.status, WorkbenchCommandStatus::Completed));
    assert!(bundle.bundle_ref.starts_with("diagnostics-bundle-"));
}

#[tokio::test]
async fn diagnostics_feedback_upload_returns_bounded_receipt() {
    let provider = DiagnosticsSystemServiceProvider::mock();
    let result = decode::<diag::DiagnosticsServiceResponse>(
        provider
            .call(command(
                diag::FEEDBACK_UPLOAD_COMMAND,
                diag::DiagnosticsFeedbackUploadRequest {
                    category: "operator-report".into(),
                    operator_ref: Some("operator://local".into()),
                    summary: summary("credential appeared in local report"),
                    attachment_refs: Vec::new(),
                    allow_remote_upload: false,
                    redaction_profile: diag::DiagnosticsRedactionProfile::default(),
                },
            ))
            .await
            .unwrap()
            .output,
    );

    let Some(diag::DiagnosticsServiceResponse::FeedbackUploaded(receipt)) = result.output else {
        panic!("expected feedback receipt");
    };
    assert!(!receipt.uploaded);
    assert_eq!(receipt.summary.text, "<redacted>");
    assert!(receipt.feedback_ref.starts_with("diagnostics-feedback-"));
}

#[tokio::test]
async fn realtime_unavailable_provider_returns_structured_unavailable() {
    let provider = RealtimeSystemServiceProvider::unavailable("realtime module absent");
    let result = decode::<rt::RealtimeServiceResponse>(
        provider
            .call(command(
                rt::START_COMMAND,
                rt::RealtimeStartRequest {
                    modality: rt::RealtimeModality::TextAndAudio,
                    transport: rt::RealtimeTransportKind::WebRtc,
                    provider_ref: None,
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
    let Some(rt::RealtimeServiceResponse::Health { configured, reason }) = result.output else {
        panic!("expected unavailable realtime health");
    };
    assert!(!configured);
    assert_eq!(reason.as_deref(), Some("realtime module absent"));
}

#[tokio::test]
async fn remote_environment_registers_health_and_workspace_roots() {
    let provider = RemoteEnvironmentSystemServiceProvider::mock();
    let add = decode::<renv::RemoteEnvironmentServiceResponse>(
        provider
            .call(command(
                renv::ADD_COMMAND,
                renv::RemoteEnvironmentAddRequest {
                    environment_ref: Some("remote-env-test".into()),
                    provider_kind: renv::RemoteEnvironmentProviderKind::RemoteExecServer,
                    endpoint_ref: "endpoint://redacted".into(),
                    workspace_roots: vec!["/workspace".into()],
                    connection_summary: summary("remote exec-server connected"),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(renv::RemoteEnvironmentServiceResponse::Environment(record)) = add.output else {
        panic!("expected remote environment record");
    };
    assert_eq!(record.environment_ref, "remote-env-test");

    let roots = decode::<renv::RemoteEnvironmentServiceResponse>(
        provider
            .call(command(
                renv::WORKSPACE_ROOTS_COMMAND,
                renv::RemoteEnvironmentRefRequest {
                    environment_ref: "remote-env-test".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(renv::RemoteEnvironmentServiceResponse::WorkspaceRoots {
        workspace_roots, ..
    }) = roots.output
    else {
        panic!("expected workspace roots");
    };
    assert_eq!(workspace_roots, vec!["/workspace"]);
}

#[tokio::test]
async fn remote_environment_unavailable_provider_is_non_fatal() {
    let provider = RemoteEnvironmentSystemServiceProvider::unavailable("remote provider absent");
    let result = decode::<renv::RemoteEnvironmentServiceResponse>(
        provider
            .call(command(
                renv::HEALTH_COMMAND,
                renv::RemoteEnvironmentRefRequest {
                    environment_ref: "missing".into(),
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
    let Some(renv::RemoteEnvironmentServiceResponse::Health(record)) = result.output else {
        panic!("expected unavailable health record");
    };
    assert!(matches!(record.health, ServiceHealth::Unavailable { .. }));
}

#[test]
fn optional_services_do_not_add_base_transport_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("webrtc"));
    assert!(!manifest.contains("ssh2"));
    assert!(!manifest.contains("libssh"));
}
