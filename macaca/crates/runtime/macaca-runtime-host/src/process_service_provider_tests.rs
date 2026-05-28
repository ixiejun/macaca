//! Focused tests for `service.process`.
//!
//! The tests exercise policy-before-spawn, bounded exec output, background
//! lifecycle, stdin, PTY resize metadata, timeout/denial, output subscription,
//! cleanup, and unavailable-safe behavior without any application-specific
//! workflow.

use macaca_kernel::SystemService;
use macaca_proto::workbench::process::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, ServiceError, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::ProcessSystemServiceProvider;

fn sandbox(root: &tempfile::TempDir) -> ProcessSandboxRequest {
    ProcessSandboxRequest {
        workspace_root: root.path().to_string_lossy().to_string(),
        cwd: Some(".".into()),
        sandbox_profile_ref: "sandbox.local.workspace".into(),
        permission_profile_ref: "permission.workspace.process".into(),
        approval_ref: None,
        resource_lease_refs: vec!["lease.process.test".into()],
    }
}

fn request(root: &tempfile::TempDir, executable: &str, args: &[&str]) -> ProcessCommandRequest {
    ProcessCommandRequest {
        executable: executable.into(),
        args: args.iter().map(|arg| (*arg).into()).collect(),
        env: Vec::new(),
        sandbox: sandbox(root),
        application_id: Some("app.process.test".into()),
        session_id: Some("session.process.test".into()),
        thread_ref: Some("thread.process.test".into()),
        inline_output_budget_bytes: 4096,
        timeout_ms: Some(3_000),
        allocate_pty: false,
    }
}

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<ProcessServiceResponse> {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn exec_runs_bounded_command_with_audit_record() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();

    let result = decode(
        provider
            .call(command(
                EXEC_COMMAND,
                request(&root, "printf", &["process-ok"]),
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(result.status, WorkbenchCommandStatus::Completed));
    let Some(ProcessServiceResponse::Exec { record, output }) = result.output else {
        panic!("expected exec response");
    };
    assert_eq!(record.state, ProcessLifecycleState::Exited);
    assert_eq!(record.exit_code, Some(0));
    assert!(record.command_hash.len() >= 8);
    assert!(record
        .audit_refs
        .iter()
        .any(|entry| entry == "process:exec"));
    assert_eq!(output[0].data.as_deref(), Some("process-ok"));
}

#[tokio::test]
async fn denied_permission_profile_blocks_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();
    let mut denied = request(&root, "printf", &["blocked"]);
    denied.sandbox.permission_profile_ref = "deny-process".into();

    let err = provider
        .call(command(EXEC_COMMAND, denied))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));
}

#[tokio::test]
async fn privileged_permission_profile_requires_approval_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();
    let mut privileged = request(&root, "printf", &["blocked"]);
    privileged.sandbox.permission_profile_ref = "permission.full_access".into();

    let err = provider
        .call(command(EXEC_COMMAND, privileged.clone()))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::DisabledByPolicy(_)));

    privileged.sandbox.approval_ref = Some("approval://process/full-access".into());
    let result = decode(
        provider
            .call(command(EXEC_COMMAND, privileged))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(result.status, WorkbenchCommandStatus::Completed));
}

#[tokio::test]
async fn exec_timeout_returns_adapter_failure_without_fake_success() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();
    let mut slow = request(&root, "sh", &["-c", "sleep 1"]);
    slow.timeout_ms = Some(10);

    let err = provider
        .call(command(EXEC_COMMAND, slow))
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceError::AdapterFailure(_)));
}

#[tokio::test]
async fn oversized_output_uses_artifact_delta() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();
    let mut req = request(&root, "printf", &["abcdef"]);
    req.inline_output_budget_bytes = 2;

    let result = decode(
        provider
            .call(command(EXEC_COMMAND, req))
            .await
            .unwrap()
            .output,
    );

    let Some(ProcessServiceResponse::Exec { output, .. }) = result.output else {
        panic!("expected exec response");
    };
    assert!(output[0].truncated);
    assert!(output[0].artifact_ref.is_some());
}

#[tokio::test]
async fn background_process_accepts_stdin_resize_status_output_and_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let provider = ProcessSystemServiceProvider::local();
    let mut req = request(&root, "cat", &[]);
    req.allocate_pty = true;
    req.timeout_ms = None;

    let spawned = decode(
        provider
            .call(command(SPAWN_COMMAND, req))
            .await
            .unwrap()
            .output,
    );
    let Some(ProcessServiceResponse::Spawned(record)) = spawned.output else {
        panic!("expected spawned response");
    };

    let handle = record.handle.clone();
    let written = decode(
        provider
            .call(command(
                STDIN_WRITE_COMMAND,
                ProcessStdinWriteRequest {
                    handle: handle.clone(),
                    data: "hello\n".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        written.output,
        Some(ProcessServiceResponse::StdinWritten { byte_len: 6, .. })
    ));

    let resized = decode(
        provider
            .call(command(
                PTY_RESIZE_COMMAND,
                ProcessPtyResizeRequest {
                    handle: handle.clone(),
                    rows: 40,
                    cols: 120,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        resized.output,
        Some(ProcessServiceResponse::PtyResized {
            rows: 40,
            cols: 120,
            ..
        })
    ));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let subscribed = decode(
        provider
            .call(command(
                OUTPUT_SUBSCRIBE_COMMAND,
                ProcessOutputSubscribeRequest {
                    handle: handle.clone(),
                    since_sequence: 1,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let Some(ProcessServiceResponse::OutputSubscribed { deltas, .. }) = subscribed.output else {
        panic!("expected output subscription response");
    };
    assert!(deltas
        .iter()
        .any(|delta| delta.data.as_deref() == Some("hello\n")));

    let cleaned = decode(
        provider
            .call(command(
                CLEANUP_COMMAND,
                ProcessCleanupRequest {
                    application_id: Some("app.process.test".into()),
                    session_id: Some("session.process.test".into()),
                    thread_ref: Some("thread.process.test".into()),
                    reason: "test cleanup".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        cleaned.output,
        Some(ProcessServiceResponse::Cleanup {
            terminated_handles,
            ..
        }) if !terminated_handles.is_empty()
    ));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_unavailable() {
    let provider = ProcessSystemServiceProvider::unavailable("process disabled");

    let result = decode(
        provider
            .call(command(
                STATUS_COMMAND,
                ProcessStatusRequest {
                    handle: ProcessHandleRef {
                        id: "missing".into(),
                    },
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
