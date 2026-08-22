use macaca_kernel::SystemService;
use macaca_proto::device_local_files::DEVICE_LOCAL_FILES_COMMANDS;
use macaca_proto::{ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext};

use super::device_local_files_service_provider::DeviceLocalFilesSystemServiceProvider;

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
