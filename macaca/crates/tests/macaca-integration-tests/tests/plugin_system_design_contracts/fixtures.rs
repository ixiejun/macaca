//! Shared test plugin fixtures for plugin system design contracts.
//!
//! These fixtures are intentionally data-only. They build manifests,
//! capabilities, hooks, and host descriptors through `macaca-sdk` and
//! `macaca-proto` so integration tests can validate Plugin Fabric contracts
//! without executing third-party code.

use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PluginCapabilityKind, PluginConfigRequirement,
    PluginHookFailurePolicy, PluginHookKind, PluginHookName, PluginId, PluginInstallRequest,
    PluginInstallSourceKind, PluginPackageLocation, PluginPermission, PluginProvidedCapability,
    PluginResource, PluginRuntimeKind, PluginSecretRequirement, PluginServiceCategory,
    PluginVersion, ServiceScope, TraceContext, TraceSchemaRef,
};
use macaca_sdk::{
    PluginCapabilityBuilder, PluginConfigBuilder, PluginContext, PluginContractReport,
    PluginHookBuilder, PluginRegistration, PluginSdk, PluginSecretRequirementBuilder,
};

/// Common contract implemented by local test plugin fixtures.
pub trait TestPluginFixture {
    fn plugin_id(&self) -> PluginId;
    fn runtime_kind(&self) -> PluginRuntimeKind;
    fn registration(&self) -> PluginRegistration;

    /// Run SDK contract tests against this fixture's registration output.
    fn contract_report(&self) -> PluginContractReport {
        PluginSdk::new()
            .contract_tests()
            .validate_registration(&self.registration())
    }

    /// Build the host context used by runtime-host skeleton tests.
    fn host_context(&self) -> PluginContext {
        PluginContext::new(
            self.plugin_id(),
            PluginVersion::new("1.0.0"),
            self.runtime_kind(),
        )
    }
}

/// Descriptor fixture that provides one generic tool capability and one hook.
pub struct DescriptorToolPlugin;

impl TestPluginFixture for DescriptorToolPlugin {
    fn plugin_id(&self) -> PluginId {
        PluginId::new("plugin.test.descriptor_tool")
    }

    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::DescriptorOnly
    }

    fn registration(&self) -> PluginRegistration {
        let sdk = PluginSdk::new();
        let manifest = base_manifest(&sdk, self.plugin_id(), self.runtime_kind())
            .permission(
                "permission.test.tool",
                "Tool capability contract permission",
            )
            .resource(
                "resource.test.tool",
                "workspace",
                "Tool capability resource",
            )
            .signature_fixture()
            .build()
            .unwrap();
        let capability = tool_capability(self.plugin_id(), "capability.test.tool")
            .permission_hint(
                "permission.test.tool",
                true,
                "Tool invocation requires explicit permission declaration",
            )
            .build();
        let hook = PluginHookBuilder::observer(
            self.plugin_id(),
            PluginHookName::BeforeToolCall,
            10,
            TraceSchemaRef::new("trace.test.tool_hook"),
        )
        .build();

        sdk.registration()
            .manifest(manifest)
            .capability(capability)
            .hook(hook)
            .config_requirement(config_requirement("test.tool.enabled"))
            .secret_requirement(secret_requirement("TEST_TOOL_TOKEN"))
            .build()
            .unwrap()
    }
}

/// Built-in adapter fixture verifies compiled OS adapters use the same host seam.
pub struct BuiltInAdapterPlugin;

impl TestPluginFixture for BuiltInAdapterPlugin {
    fn plugin_id(&self) -> PluginId {
        PluginId::new("plugin.test.builtin_adapter")
    }

    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::BuiltInAdapter
    }

    fn registration(&self) -> PluginRegistration {
        let sdk = PluginSdk::new();
        let manifest = base_manifest(&sdk, self.plugin_id(), self.runtime_kind())
            .permission("permission.test.adapter", "Built-in adapter permission")
            .resource(
                "resource.test.adapter",
                "os-service",
                "Built-in adapter resource",
            )
            .signature_fixture()
            .build()
            .unwrap();
        sdk.registration().manifest(manifest).build().unwrap()
    }
}

/// Process fixture verifies unavailable-safe execution skeleton behavior.
pub struct ProcessSkeletonPlugin;

impl TestPluginFixture for ProcessSkeletonPlugin {
    fn plugin_id(&self) -> PluginId {
        PluginId::new("plugin.test.process_skeleton")
    }

    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Process
    }

    fn registration(&self) -> PluginRegistration {
        skeleton_registration(self.plugin_id(), self.runtime_kind())
    }
}

/// Remote proxy fixture verifies remote execution remains a proxy boundary.
pub struct RemoteProxySkeletonPlugin;

impl TestPluginFixture for RemoteProxySkeletonPlugin {
    fn plugin_id(&self) -> PluginId {
        PluginId::new("plugin.test.remote_proxy_skeleton")
    }

    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::RemoteProxy
    }

    fn registration(&self) -> PluginRegistration {
        skeleton_registration(self.plugin_id(), self.runtime_kind())
    }
}

/// WASM fixture verifies ABI metadata can exist before real WASM execution.
pub struct WasmSkeletonPlugin;

impl TestPluginFixture for WasmSkeletonPlugin {
    fn plugin_id(&self) -> PluginId {
        PluginId::new("plugin.test.wasm_skeleton")
    }

    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Wasm
    }

    fn registration(&self) -> PluginRegistration {
        skeleton_registration(self.plugin_id(), self.runtime_kind())
    }
}

pub fn base_manifest(
    sdk: &PluginSdk,
    plugin_id: PluginId,
    runtime_kind: PluginRuntimeKind,
) -> macaca_sdk::PluginManifestBuilder {
    sdk.manifest()
        .plugin_id(plugin_id)
        .version(PluginVersion::new("1.0.0"))
        .developer_id(DeveloperId::new("developer.test"))
        .runtime(runtime_kind)
}

pub fn tool_capability(plugin_id: PluginId, capability_id: &str) -> PluginCapabilityBuilder {
    PluginCapabilityBuilder::new(
        CapabilityId::new(capability_id),
        plugin_id,
        PluginCapabilityKind::Tool,
        "test-tool",
        TraceSchemaRef::new("trace.test.tool"),
    )
    .scope(ServiceScope::Global)
}

pub fn control_plane_manifest() -> macaca_proto::PluginManifest {
    let sdk = PluginSdk::new();
    let mut manifest = base_manifest(
        &sdk,
        PluginId::new("plugin.test.controlled_descriptor"),
        PluginRuntimeKind::DescriptorOnly,
    )
    .permission(
        "permission.test.controlled",
        "Controlled descriptor test permission",
    )
    .resource(
        "resource.test.controlled",
        "workspace",
        "Controlled descriptor test resource",
    )
    .signature_fixture()
    .build()
    .unwrap();
    manifest.permissions.push(PluginPermission {
        id: "permission.test.controlled.capability".into(),
        reason: "Capability permission declared for manifest discovery".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest.resources.push(PluginResource {
        id: "resource.test.controlled.capability".into(),
        kind: "workspace".into(),
        reason: "Capability resource declared for manifest discovery".into(),
        optional: false,
        metadata: Default::default(),
    });
    manifest
        .provides_capabilities
        .push(PluginProvidedCapability {
            id: CapabilityId::new("capability.test.controlled"),
            service: Some(KernelServiceId::new("service.test.controlled")),
            category: PluginServiceCategory::Custom("tool".into()),
            description: "Controlled descriptor capability".into(),
            required_permissions: vec!["permission.test.controlled.capability".into()],
            metadata: [("trace_schema".into(), "trace.test.controlled".into())].into(),
        });
    manifest
}

pub fn install_request(
    manifest: macaca_proto::PluginManifest,
    enable_after_install: bool,
) -> PluginInstallRequest {
    PluginInstallRequest {
        location: PluginPackageLocation::new(
            PluginInstallSourceKind::UserLocal,
            "/tmp/test-plugin",
        ),
        manifest: Some(manifest),
        enable_after_install,
        trace: TraceContext::new("trace-plugin-test-install"),
        metadata: Default::default(),
    }
}

pub fn observer_hook(plugin_id: &str, priority: i32) -> macaca_proto::PluginHookDescriptor {
    PluginHookBuilder::observer(
        PluginId::new(plugin_id),
        PluginHookName::BeforeToolCall,
        priority,
        TraceSchemaRef::new("trace.test.hook.observer"),
    )
    .build()
}

pub fn blocking_hook(plugin_id: &str) -> macaca_proto::PluginHookDescriptor {
    PluginHookBuilder::observer(
        PluginId::new(plugin_id),
        PluginHookName::BeforeToolCall,
        0,
        TraceSchemaRef::new("trace.test.hook.blocking"),
    )
    .kind(PluginHookKind::Blocking)
    .failure_policy(PluginHookFailurePolicy::FailClosed)
    .build()
}

fn skeleton_registration(
    plugin_id: PluginId,
    runtime_kind: PluginRuntimeKind,
) -> PluginRegistration {
    let sdk = PluginSdk::new();
    let manifest = base_manifest(&sdk, plugin_id, runtime_kind)
        .permission("permission.test.skeleton", "Skeleton host permission")
        .resource("resource.test.skeleton", "host", "Skeleton host resource")
        .signature_fixture()
        .build()
        .unwrap();
    sdk.registration().manifest(manifest).build().unwrap()
}

fn config_requirement(key: &str) -> PluginConfigRequirement {
    PluginConfigBuilder::required(key, "Test plugin config key")
        .present(true)
        .build()
}

fn secret_requirement(name: &str) -> PluginSecretRequirement {
    PluginSecretRequirementBuilder::required(name)
        .present(false)
        .build()
}
