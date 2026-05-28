use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::workbench::plugin_marketplace::*;
use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PluginId, PluginInstallRequest,
    PluginInstallSourceKind, PluginPackageLocation, PluginProvidedCapability,
    PluginProvidedService, PluginResource, PluginRuntimeDeclaration, PluginRuntimeKind,
    PluginServiceCategory, PluginSignature, PluginTargetCommand, PluginVersion, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceType, TraceContext, TraceSchemaRef,
    WorkbenchCommandResult, WorkbenchCommandStatus,
};

use crate::PluginMarketplaceSystemServiceProvider;

/// Build a generic manifest fixture that exercises every marketplace summary
/// category without naming or special-casing any concrete application.
fn manifest(plugin_id: &str) -> macaca_proto::PluginManifest {
    let mut manifest = macaca_proto::PluginManifest::new(
        PluginId::new(plugin_id),
        PluginVersion::new("1.0.0"),
        DeveloperId::new("developer.fixture"),
        PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "plugin_abi_v1"),
    );
    let service = ServiceDescriptor::new(
        KernelServiceId::new("service.fixture.plugin"),
        ServiceType::new("fixture"),
        TraceSchemaRef::new("trace.fixture.plugin.v1"),
    );
    manifest.provides_services.push(PluginProvidedService::new(
        service,
        PluginServiceCategory::Custom("generic".into()),
    ));
    manifest
        .provides_capabilities
        .push(PluginProvidedCapability {
            id: CapabilityId::new("capability.fixture.plugin"),
            service: Some(KernelServiceId::new("service.fixture.plugin")),
            category: PluginServiceCategory::Custom("generic".into()),
            description: "Generic fixture capability".into(),
            required_permissions: Vec::new(),
            metadata: BTreeMap::new(),
        });
    manifest.resources.push(PluginResource {
        id: "resource.fixture.plugin".into(),
        kind: "descriptor".into(),
        reason: "Fixture resource required by descriptor-safe plugin tests".into(),
        optional: false,
        metadata: BTreeMap::new(),
    });
    for key in ["tool", "skill", "mcp", "hook", "app", "app_ui"] {
        manifest
            .metadata
            .insert(format!("bundle.{key}.count"), "1".into());
    }
    manifest.signature = Some(PluginSignature {
        algorithm: "test-ed25519".into(),
        key_id: "test-key".into(),
        digest: "sha256:test".into(),
        verification_status: "verified".into(),
    });
    manifest
}

/// Create an install request with all generic marketplace gates allowed.
fn install_request(plugin_id: &str) -> PluginInstallRequest {
    PluginInstallRequest {
        location: PluginPackageLocation::new(PluginInstallSourceKind::UserLocal, "/tmp/plugin"),
        manifest: Some(manifest(plugin_id)),
        enable_after_install: true,
        trace: TraceContext::new(format!("trace-install-{plugin_id}")),
        metadata: [
            ("entitlement.status".into(), "allowed".into()),
            ("license.status".into(), "allowed".into()),
            ("metering.status".into(), "allowed".into()),
        ]
        .into(),
    }
}

async fn call<T: serde::Serialize>(
    service: &PluginMarketplaceSystemServiceProvider,
    name: &str,
    payload: T,
    trace_id: &str,
) -> WorkbenchCommandResult<PluginMarketplaceResponse> {
    let result = service
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new(name),
            serde_json::to_value(payload).unwrap(),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    serde_json::from_value(result.output).unwrap()
}

#[tokio::test]
async fn unavailable_provider_reports_workbench_unavailable() {
    let service = PluginMarketplaceSystemServiceProvider::unavailable("disabled by host policy");
    let result = call(
        &service,
        SNAPSHOT_COMMAND,
        PluginMarketplaceSnapshotCommand {
            include_audit_tail: true,
        },
        "trace-marketplace-unavailable",
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}

#[tokio::test]
async fn install_without_manifest_is_denied_and_audited() {
    let service = PluginMarketplaceSystemServiceProvider::mock();
    let mut request = install_request("plugin.marketplace.missing_manifest");
    request.manifest = None;
    let result = call(
        &service,
        PLUGIN_INSTALL_COMMAND,
        PluginMarketplaceInstallCommand {
            marketplace_id: "local".into(),
            request,
        },
        "trace-marketplace-missing-manifest",
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Denied { .. }
    ));
    assert!(result.audit_ref.is_some());
}

#[tokio::test]
async fn entitlement_denial_blocks_install_before_control_mutation() {
    let service = PluginMarketplaceSystemServiceProvider::mock();
    let mut request = install_request("plugin.marketplace.denied");
    request
        .metadata
        .insert("entitlement.status".into(), "denied".into());
    let result = call(
        &service,
        PLUGIN_INSTALL_COMMAND,
        PluginMarketplaceInstallCommand {
            marketplace_id: "local".into(),
            request,
        },
        "trace-marketplace-entitlement-denied",
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Denied { .. }
    ));
    let snapshot = snapshot(&service).await;
    assert!(snapshot.installed_plugins.is_empty());
}

#[tokio::test]
async fn disabled_marketplace_policy_denies_install() {
    let service = PluginMarketplaceSystemServiceProvider::mock();
    let descriptor = MarketplaceDescriptor {
        marketplace_id: "disabled".into(),
        kind: MarketplaceSourceKind::Local,
        enabled: true,
        source_policy: MarketplaceSourcePolicy {
            disabled_by_admin: true,
            ..MarketplaceSourcePolicy::local_default()
        },
        metadata: BTreeMap::new(),
    };
    call(
        &service,
        MARKETPLACE_ADD_COMMAND,
        MarketplaceCommand { descriptor },
        "trace-marketplace-add-disabled",
    )
    .await;
    let result = call(
        &service,
        PLUGIN_INSTALL_COMMAND,
        PluginMarketplaceInstallCommand {
            marketplace_id: "disabled".into(),
            request: install_request("plugin.marketplace.disabled"),
        },
        "trace-marketplace-disabled-policy",
    )
    .await;

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Denied { .. }
    ));
}

#[tokio::test]
async fn install_reports_bundled_capabilities_and_snapshot_replays_audit() {
    let service = PluginMarketplaceSystemServiceProvider::mock();
    let result = call(
        &service,
        PLUGIN_INSTALL_COMMAND,
        PluginMarketplaceInstallCommand {
            marketplace_id: "local".into(),
            request: install_request("plugin.marketplace.summary"),
        },
        "trace-marketplace-install-summary",
    )
    .await;

    let Some(PluginMarketplaceResponse::Installed {
        capability_summary,
        rollback_ref,
        ..
    }) = result.output
    else {
        panic!("expected installed response");
    };
    assert!(!rollback_ref.is_empty());
    assert!(capability_summary
        .iter()
        .any(|item| item.kind == PluginBundleCapabilityKind::Service));
    assert!(capability_summary
        .iter()
        .any(|item| item.kind == PluginBundleCapabilityKind::AppUi));

    let snapshot = snapshot(&service).await;
    assert_eq!(snapshot.installed_plugins.len(), 1);
    assert_eq!(snapshot.rollback_records.len(), 1);
    assert!(snapshot
        .audit_tail
        .iter()
        .any(|record| record.operation == "plugin.install"));
}

#[tokio::test]
async fn uninstall_records_cleanup_rollback_memento() {
    let service = PluginMarketplaceSystemServiceProvider::mock();
    call(
        &service,
        PLUGIN_INSTALL_COMMAND,
        PluginMarketplaceInstallCommand {
            marketplace_id: "local".into(),
            request: install_request("plugin.marketplace.uninstall"),
        },
        "trace-marketplace-install-before-uninstall",
    )
    .await;
    let result = call(
        &service,
        PLUGIN_UNINSTALL_COMMAND,
        PluginTargetCommand::new(
            PluginId::new("plugin.marketplace.uninstall"),
            TraceContext::new("trace-marketplace-uninstall-inner"),
        ),
        "trace-marketplace-uninstall",
    )
    .await;

    assert!(matches!(result.status, WorkbenchCommandStatus::Completed));
    let snapshot = snapshot(&service).await;
    assert_eq!(snapshot.rollback_records.len(), 2);
    assert!(snapshot
        .audit_tail
        .iter()
        .any(|record| record.operation == "plugin.uninstall"));
}

async fn snapshot(service: &PluginMarketplaceSystemServiceProvider) -> PluginMarketplaceSnapshot {
    let result = call(
        service,
        SNAPSHOT_COMMAND,
        PluginMarketplaceSnapshotCommand {
            include_audit_tail: true,
        },
        "trace-marketplace-snapshot",
    )
    .await;
    let Some(PluginMarketplaceResponse::Snapshot(snapshot)) = result.output else {
        panic!("expected snapshot response");
    };
    snapshot
}
