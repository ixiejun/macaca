use macaca_proto::{
    DeveloperId, PluginId, PluginInstallRequest, PluginInstallSourceKind, PluginLifecycleState,
    PluginPackageLocation, PluginRuntimeDeclaration, PluginRuntimeKind, PluginVersion,
    TraceContext,
};

use super::*;

fn manifest(id: &str) -> macaca_proto::PluginManifest {
    let mut manifest = macaca_proto::PluginManifest::new(
        PluginId::new(id),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.fixture"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
    );
    manifest.signature = Some(macaca_proto::PluginSignature {
        algorithm: "fixture".into(),
        key_id: "fixture-key".into(),
        digest: "fixture-digest".into(),
        verification_status: "verification_pending".into(),
    });
    manifest
}

fn install_request(id: &str, enable_after_install: bool) -> PluginInstallRequest {
    PluginInstallRequest {
        location: PluginPackageLocation::new(PluginInstallSourceKind::UserLocal, "/tmp/plugin"),
        manifest: Some(manifest(id)),
        enable_after_install,
        trace: TraceContext::new("trace-plugin-control-test"),
        metadata: Default::default(),
    }
}

#[tokio::test]
async fn install_without_enable_records_disabled_plugin() {
    let service = PluginControlService::in_memory();
    let result = service
        .install(install_request("plugin.control.disabled", false))
        .await
        .unwrap();

    assert!(!result.record.enabled);
    assert_eq!(
        result.record.activation_state,
        macaca_proto::PluginActivationState::Disabled
    );
    let listed = service
        .list(macaca_proto::PluginListCommand {
            repository_id: None,
            include_disabled: true,
            trace: TraceContext::new("trace-list"),
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn enable_registers_descriptor_safe_plugin() {
    let service = PluginControlService::in_memory();
    service
        .install(install_request("plugin.control.enabled", false))
        .await
        .unwrap();

    let record = service
        .enable(macaca_proto::PluginTargetCommand::new(
            PluginId::new("plugin.control.enabled"),
            TraceContext::new("trace-enable"),
        ))
        .await
        .unwrap();

    assert!(record.enabled);
    assert_eq!(record.lifecycle_state, PluginLifecycleState::Registered);
}

#[tokio::test]
async fn project_local_install_fails_closed_by_default() {
    let service = PluginControlService::in_memory();
    let mut request = install_request("plugin.control.project", false);
    request.location.source_kind = PluginInstallSourceKind::ProjectLocal;

    let error = service.install(request).await.unwrap_err();
    assert!(matches!(error, macaca_proto::PluginError::Unavailable(_)));
}

#[tokio::test]
async fn diagnostics_never_echo_secret_metadata_values() {
    let service = PluginControlService::in_memory();
    let mut request = install_request("plugin.control.secret", false);
    request
        .metadata
        .insert("required_secrets".into(), "PLUGIN_TOKEN".into());
    request
        .metadata
        .insert("secret_value".into(), "do-not-echo".into());
    service.install(request).await.unwrap();

    let diagnostics = service
        .diagnostics(macaca_proto::PluginTargetCommand::new(
            PluginId::new("plugin.control.secret"),
            TraceContext::new("trace-diagnostics"),
        ))
        .await
        .unwrap();
    let encoded = serde_json::to_string(&diagnostics).unwrap();
    assert!(encoded.contains("PLUGIN_TOKEN"));
    assert!(!encoded.contains("do-not-echo"));
}
