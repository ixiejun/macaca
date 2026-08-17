//! Contract tests for deterministic and unavailable filesystem providers.

use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};

use crate::{FilesystemService, MockFilesystemProvider, UnavailableFilesystemProvider};

fn command(name: &str, payload: serde_json::Value) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        payload,
        TraceContext::new(format!("trace-{name}")),
    )
}

#[tokio::test]
async fn mock_provider_replays_all_declared_commands_without_raw_path_or_content() {
    let provider = MockFilesystemProvider::default();
    for operation in macaca_proto::FOUNDATION_FILESYSTEM_COMMANDS {
        let payload = match *operation {
            "filesystem.open_handle"
            | "filesystem.read_file"
            | "filesystem.stat_path"
            | "filesystem.list_directory"
            | "filesystem.create_directory"
            | "filesystem.delete_path"
            | "filesystem.watch_path" => {
                serde_json::json!({"path":{"relative_path":"settings.json"}})
            }
            "filesystem.write_file" | "filesystem.append_file" => serde_json::json!({
                "path":{"relative_path":"settings.json"},
                "content":{"content_ref":"artifact:filesystem-test"}
            }),
            _ => serde_json::json!({}),
        };
        let reply = provider.call(command(operation, payload)).await.unwrap();
        assert_eq!(
            reply.metadata.get("replay.filesystem_command"),
            Some(&operation.to_string())
        );
        assert!(!serde_json::to_string(&reply.metadata)
            .unwrap()
            .contains("settings.json"));
    }
    assert!(provider.provider_capabilities().supports_watch);
    assert_eq!(provider.snapshot().provider_class, "mock");
}

#[tokio::test]
async fn mock_provider_watch_lifecycle_and_shutdown_are_bounded() {
    let provider = MockFilesystemProvider::default();
    let watch = provider
        .call(command(
            "filesystem.watch_path",
            serde_json::json!({"path":{"relative_path":"settings.json"}}),
        ))
        .await
        .unwrap();
    let checkpoint = watch.output["watch_checkpoint"].as_str().unwrap();
    assert_eq!(provider.snapshot().active_watch_count, 1);
    provider.cancel_watch(checkpoint).await.unwrap();
    assert_eq!(provider.snapshot().active_watch_count, 0);
    provider.shutdown().await.unwrap();
}

#[tokio::test]
async fn unavailable_provider_returns_structured_traceable_diagnostics() {
    let provider = UnavailableFilesystemProvider::default();
    let reply = provider
        .call(command("filesystem.read_file", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(reply.status, "unavailable");
    assert_eq!(
        reply.metadata.get("filesystem.audit_event"),
        Some(&"filesystem_pack_unavailable".into())
    );
    assert_eq!(provider.snapshot().provider_class, "unavailable");
}
