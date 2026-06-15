//! WASM lifecycle contract tests for the runtime-host provider boundary.
//!
//! These tests exercise the public session trait instead of private helper
//! functions.  That keeps the contract focused on observable behavior:
//! provider-neutral transition results, sanitized checkpoint mementos,
//! ABI mismatch failures, and fail-closed unavailable behavior.

use macaca_proto::{
    ApplicationAbiVersion, TraceContext, WasmExecutionProfile, WasmLifecycleCommand,
    WasmLifecycleOperation, WasmLifecycleOperationStatus, WasmLifecycleReasonCode,
    WasmLifecycleState, WasmRestoreRequest, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
};

use super::{
    DefaultInProcessWasmRuntimeProvider, UnavailableWasmRuntimeProvider,
    WasmApplicationRuntimeProvider,
};

#[tokio::test]
async fn wasm_lifecycle_default_provider_runs_init_start_and_shutdown_transitions() {
    let session = lifecycle_session("wasm-lifecycle-valid").await;

    assert_eq!(session.lifecycle_state(), WasmLifecycleState::Instantiated);

    let init = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Init,
            TraceContext::new("trace-wasm-lifecycle-init"),
        ))
        .await
        .unwrap();
    assert_eq!(init.status, WasmLifecycleOperationStatus::Completed);
    assert_eq!(init.to, WasmLifecycleState::Initialized);

    let start = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Start,
            TraceContext::new("trace-wasm-lifecycle-start"),
        ))
        .await
        .unwrap();
    assert_eq!(start.status, WasmLifecycleOperationStatus::Completed);
    assert_eq!(start.to, WasmLifecycleState::Started);

    let shutdown = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Shutdown,
            TraceContext::new("trace-wasm-lifecycle-shutdown"),
        ))
        .await
        .unwrap();
    assert_eq!(shutdown.status, WasmLifecycleOperationStatus::Completed);
    assert_eq!(shutdown.to, WasmLifecycleState::ShuttingDown);
}

#[tokio::test]
async fn wasm_lifecycle_default_provider_rejects_invalid_transition_without_state_change() {
    let session = lifecycle_session("wasm-lifecycle-invalid").await;

    let start = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Start,
            TraceContext::new("trace-wasm-lifecycle-invalid-start"),
        ))
        .await
        .unwrap();

    assert_eq!(start.status, WasmLifecycleOperationStatus::Rejected);
    assert_eq!(
        start.reason_code,
        WasmLifecycleReasonCode::InvalidTransition.as_code()
    );
    assert_eq!(session.lifecycle_state(), WasmLifecycleState::Instantiated);
}

#[tokio::test]
async fn wasm_lifecycle_default_provider_reports_pause_and_drain_as_unsupported() {
    let session = started_lifecycle_session("wasm-lifecycle-unsupported").await;

    let pause = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Pause,
            TraceContext::new("trace-wasm-lifecycle-pause"),
        ))
        .await
        .unwrap();
    assert_eq!(pause.status, WasmLifecycleOperationStatus::Unsupported);
    assert_eq!(
        pause.reason_code,
        WasmLifecycleReasonCode::Unsupported.as_code()
    );
    assert_eq!(session.lifecycle_state(), WasmLifecycleState::Started);

    let drain = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Drain,
            TraceContext::new("trace-wasm-lifecycle-drain"),
        ))
        .await
        .unwrap();
    assert_eq!(drain.status, WasmLifecycleOperationStatus::Unsupported);
    assert_eq!(session.lifecycle_state(), WasmLifecycleState::Started);
}

#[tokio::test]
async fn wasm_lifecycle_checkpoint_is_sanitized_and_restore_rejects_abi_mismatch() {
    let session = started_lifecycle_session("wasm-lifecycle-checkpoint").await;
    let mut command = WasmLifecycleCommand::traced(
        WasmLifecycleOperation::Checkpoint,
        TraceContext::new("trace-wasm-lifecycle-checkpoint"),
    );
    command
        .metadata
        .insert("safe.label".into(), "checkpoint".into());
    command
        .metadata
        .insert("raw_memory_dump".into(), "secret bytes".into());
    command
        .metadata
        .insert("prompt".into(), "secret prompt".into());

    let mut checkpoint = session.checkpoint(command).await.unwrap();

    assert_eq!(checkpoint.lifecycle_state, WasmLifecycleState::Started);
    assert_eq!(
        checkpoint.metadata.get("safe.label").map(String::as_str),
        Some("checkpoint")
    );
    assert!(!checkpoint.metadata.contains_key("raw_memory_dump"));
    assert!(!checkpoint.metadata.contains_key("prompt"));
    assert!(!checkpoint.artifact_digest_prefix.is_empty());

    checkpoint.abi_version = ApplicationAbiVersion::new("999");
    let report = session
        .restore(WasmRestoreRequest {
            checkpoint,
            trace: Some(TraceContext::new("trace-wasm-lifecycle-restore")),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(report.status, WasmLifecycleOperationStatus::Rejected);
    assert_eq!(
        report.reason_code,
        WasmLifecycleReasonCode::AbiMismatch.as_code()
    );
    assert_eq!(session.lifecycle_state(), WasmLifecycleState::Started);
}

#[tokio::test]
async fn wasm_lifecycle_unavailable_provider_fails_closed_for_transition() {
    let provider = UnavailableWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-wasm-lifecycle-unavailable-provider"),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();
    let session = provider.create_session(request).await.unwrap();

    let result = session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Init,
            TraceContext::new("trace-wasm-lifecycle-unavailable-command"),
        ))
        .await
        .unwrap();

    assert_eq!(result.status, WasmLifecycleOperationStatus::Unavailable);
    assert_eq!(
        result.reason_code,
        WasmLifecycleReasonCode::Unavailable.as_code()
    );
}

async fn started_lifecycle_session(name: &str) -> Box<dyn super::WasmExecutionSession> {
    let session = lifecycle_session(name).await;
    session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Init,
            TraceContext::new(format!("trace-{name}-init")),
        ))
        .await
        .unwrap();
    session
        .transition_lifecycle(WasmLifecycleCommand::traced(
            WasmLifecycleOperation::Start,
            TraceContext::new(format!("trace-{name}-start")),
        ))
        .await
        .unwrap();
    session
}

/// Create a default in-process WASM session backed by a tiny multi-export module.
///
/// The fixture module exports the Application ABI lifecycle functions used by
/// this slice.  Each exported function is a nullary no-op so tests exercise the
/// lifecycle orchestration layer rather than guest business logic.
async fn lifecycle_session(name: &str) -> Box<dyn super::WasmExecutionSession> {
    let artifact_path = write_fixture_wasm(name, lifecycle_exports_module());
    let provider = DefaultInProcessWasmRuntimeProvider::default();
    let request = WasmRuntimeSessionRequest::new(
        TraceContext::new(format!("trace-{name}-provider")),
        "fixture.application",
        "main",
        WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
        WasmExecutionProfile::default_wasm_component(),
    )
    .unwrap();

    provider.create_session(request).await.unwrap()
}

/// Write a temporary WASM artifact and return a path suitable for file URI use.
fn write_fixture_wasm(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let directory = tempfile::Builder::new()
        .prefix("macaca-wasm-lifecycle-")
        .tempdir()
        .unwrap()
        .keep();
    let path = directory.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Return a minimal core-WASM binary with no-op lifecycle exports.
///
/// The byte sequence is kept inline to avoid adding a WAT compiler dependency.
/// It contains five functions and exports them as `app:init`, `app:start`,
/// `app:handle_event`, `app:render`, and `app:shutdown`.
fn lifecycle_exports_module() -> &'static [u8] {
    &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x06, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x47, 0x05, 0x08, b'a', b'p', b'p', b':',
        b'i', b'n', b'i', b't', 0x00, 0x00, 0x09, b'a', b'p', b'p', b':', b's', b't', b'a', b'r',
        b't', 0x00, 0x01, 0x10, b'a', b'p', b'p', b':', b'h', b'a', b'n', b'd', b'l', b'e', b'_',
        b'e', b'v', b'e', b'n', b't', 0x00, 0x02, 0x0a, b'a', b'p', b'p', b':', b'r', b'e', b'n',
        b'd', b'e', b'r', 0x00, 0x03, 0x0c, b'a', b'p', b'p', b':', b's', b'h', b'u', b't', b'd',
        b'o', b'w', b'n', 0x00, 0x04, 0x0a, 0x10, 0x05, 0x02, 0x00, 0x0b, 0x02, 0x00, 0x0b, 0x02,
        0x00, 0x0b, 0x02, 0x00, 0x0b, 0x02, 0x00, 0x0b,
    ]
}
