use macaca_kernel::SystemService;
use macaca_proto::device_local_files::DEVICE_LOCAL_FILES_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::device_local_files_service_provider::{
    transition_local_file_grant, transition_local_file_transfer,
    DeviceLocalFilesSystemServiceProvider, LocalFileGrantState, LocalFileTransferState,
};

#[tokio::test]
async fn local_file_commands_are_reference_only_and_redacted() {
    let provider = DeviceLocalFilesSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in DEVICE_LOCAL_FILES_COMMANDS {
        let marker = "raw-path-and-content-marker";
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"path":marker,"contents":marker,"credentials":marker}),
                TraceContext::new(format!("local-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.output["status"], "reference_only");
        assert!(!result.output.to_string().contains(marker));
        assert!(!events.recv().await.unwrap().replay_ref.contains(marker));
    }
}

#[tokio::test]
async fn unavailable_local_files_provider_fails_closed_and_cleanup_releases_counts() {
    let unavailable = DeviceLocalFilesSystemServiceProvider::unavailable("module_absent");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("local_files.read"),
                serde_json::json!({"path":"must-not-read"}),
                TraceContext::new("unavailable")
            ))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));

    let provider = DeviceLocalFilesSystemServiceProvider::mock();
    for command in ["local_files.request_open_handle", "local_files.import_file"] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    assert_eq!(provider.snapshot().await["active_handle_count"], "1");
    assert_eq!(provider.snapshot().await["active_transfer_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_handle_count"], "0");
}

#[tokio::test]
async fn local_files_replay_reference_is_stable_after_provider_restart() {
    let trace_id = "local-files-restart-trace";
    let first = DeviceLocalFilesSystemServiceProvider::mock();
    first
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("local_files.inspect_host"),
            serde_json::json!({"path":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    first.cleanup().await.unwrap();
    let restarted = DeviceLocalFilesSystemServiceProvider::mock();
    let mut events = restarted.subscribe();
    restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("local_files.inspect_host"),
            serde_json::json!({"contents":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.trace_id, trace_id);
    assert_eq!(event.replay_ref, format!("replay:local-files:{trace_id}"));
}

#[test]
fn local_files_grant_and_transfer_state_machines_fail_closed() {
    assert_eq!(
        transition_local_file_grant(LocalFileGrantState::Requested, "grant"),
        Some(LocalFileGrantState::Granted)
    );
    assert_eq!(
        transition_local_file_grant(LocalFileGrantState::Granted, "activate"),
        Some(LocalFileGrantState::Active)
    );
    assert_eq!(
        transition_local_file_grant(LocalFileGrantState::Active, "revoke"),
        Some(LocalFileGrantState::Revoked)
    );
    assert_eq!(
        transition_local_file_grant(LocalFileGrantState::Revoked, "activate"),
        None
    );
    assert_eq!(
        transition_local_file_transfer(LocalFileTransferState::Requested, "start"),
        Some(LocalFileTransferState::Active)
    );
    assert_eq!(
        transition_local_file_transfer(LocalFileTransferState::Active, "complete"),
        Some(LocalFileTransferState::Completed)
    );
    assert_eq!(
        transition_local_file_transfer(LocalFileTransferState::Completed, "cancel"),
        None
    );
}

#[tokio::test]
async fn local_files_emits_stable_audit_event_taxonomy() {
    let provider = DeviceLocalFilesSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    for command in [
        "local_files.request_open_handle",
        "local_files.read",
        "local_files.cancel_transfer",
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({"path":"redacted"}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    let mut names = Vec::new();
    while let Ok(event) = events.try_recv() {
        names.push(event.event_name);
    }
    for expected in [
        "local_files.pack_declared",
        "local_files.admission_validated",
        "local_files.policy_decision",
        "local_files.entitlement_checked",
        "local_files.resource_reserved",
        "local_files.picker_requested",
        "local_files.handle_granted",
        "local_files.transfer_started",
        "local_files.transfer_progressed",
        "local_files.transfer_completed",
        "local_files.transfer_cancelled",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}
