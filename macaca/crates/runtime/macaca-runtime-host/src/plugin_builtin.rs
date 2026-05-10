//! Built-in service descriptor adapters for Plugin Runtime v0.
//!
//! These adapters expose already-compiled OS services as plugin descriptors.
//! They are intentionally data-only: no gateway, driver, memory, skill, MCP,
//! model, provider, app, workflow, process, or business logic is executed here.

use macaca_proto::{
    CapabilityId, KernelServiceId, PluginManifest, PluginProvidedCapability, PluginProvidedService,
    PluginResource, PluginRuntimeKind, PluginServiceCategory, ServiceCapability, ServiceDescriptor,
    ServiceHealth, ServiceType, TraceSchemaRef,
};
use tracing::info;

/// Declarative descriptor for a built-in OS service exposed through Plugin Runtime.
///
/// This adapter does not call the underlying service and does not encode a
/// provider name. It converts an already-known OS service capability into the
/// same manifest shape third-party plugins use, which keeps discovery and
/// lifecycle audit uniform without coupling execution paths to Plugin Runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltInPluginDescriptor {
    pub service_id: KernelServiceId,
    pub category: PluginServiceCategory,
    pub capability_id: CapabilityId,
    pub description: String,
    pub required_permissions: Vec<String>,
    pub resource_kind: String,
    pub trace_schema: TraceSchemaRef,
}

impl BuiltInPluginDescriptor {
    /// Create a provider-neutral descriptor for a built-in service adapter.
    ///
    /// Callers provide only protocol-level ids and categories. The constructor
    /// intentionally avoids concrete provider identities so equivalent service
    /// implementations can be swapped without changing runtime-host code.
    pub fn new(
        service_id: KernelServiceId,
        category: PluginServiceCategory,
        capability_id: CapabilityId,
        description: impl Into<String>,
        resource_kind: impl Into<String>,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            service_id,
            category,
            capability_id,
            description: description.into(),
            required_permissions: Vec::new(),
            resource_kind: resource_kind.into(),
            trace_schema,
        }
    }

    /// Add one permission required by the underlying built-in service contract.
    pub fn with_required_permission(mut self, permission: impl Into<String>) -> Self {
        self.required_permissions.push(permission.into());
        self
    }
}

/// Composite builder that maps built-in descriptors into plugin manifest data.
///
/// The builder is a small Adapter plus Composite: each built-in descriptor is
/// adapted into `PluginProvidedService`, `PluginProvidedCapability`, and
/// resource declarations, while the manifest owns a composite set of services.
#[derive(Debug, Clone)]
pub struct BuiltInPluginDescriptorAdapter {
    manifest: PluginManifest,
}

impl BuiltInPluginDescriptorAdapter {
    /// Start a built-in adapter manifest from an existing plugin manifest shell.
    ///
    /// The shell must carry plugin id, developer id, version, signature policy,
    /// and runtime metadata. This keeps identity decisions outside the adapter
    /// and prevents hardcoded service or vendor identities in runtime-host.
    pub fn new(mut manifest: PluginManifest) -> Self {
        manifest.runtime.kind = PluginRuntimeKind::BuiltInAdapter;
        Self { manifest }
    }

    /// Attach an optional built-in service descriptor.
    ///
    /// `None` means the service is unavailable in the current deployment. The
    /// adapter logs absence and leaves the manifest unchanged instead of
    /// fabricating descriptors or failing activation globally.
    pub fn maybe_add_descriptor(mut self, descriptor: Option<BuiltInPluginDescriptor>) -> Self {
        let Some(descriptor) = descriptor else {
            info!("built-in plugin descriptor absent; adapter remains optional");
            return self;
        };

        let service_type = ServiceType::new(descriptor.category.to_string());
        let mut service = ServiceDescriptor::new(
            descriptor.service_id.clone(),
            service_type,
            descriptor.trace_schema.clone(),
        );
        service.health = ServiceHealth::Unknown;
        service.required_permissions = descriptor.required_permissions.clone();
        service.capabilities.push(ServiceCapability {
            id: descriptor.capability_id.clone(),
            description: descriptor.description.clone(),
            required_permissions: descriptor.required_permissions.clone(),
        });

        let mut provided = PluginProvidedService::new(service, descriptor.category.clone());
        provided.required_permissions = descriptor.required_permissions.clone();
        provided
            .metadata
            .insert("adapter_kind".into(), "built_in".into());

        self.manifest.provides_services.push(provided);
        self.manifest
            .provides_capabilities
            .push(PluginProvidedCapability {
                id: descriptor.capability_id,
                service: Some(descriptor.service_id.clone()),
                category: descriptor.category,
                description: descriptor.description,
                required_permissions: descriptor.required_permissions,
                metadata: [("adapter_kind".into(), "built_in".into())].into(),
            });
        self.manifest.resources.push(PluginResource {
            id: format!("resource.{}", descriptor.service_id),
            kind: descriptor.resource_kind,
            reason: "Built-in adapter descriptor requires the declared OS resource scope".into(),
            optional: false,
            metadata: [("adapter_kind".into(), "built_in".into())].into(),
        });
        self
    }

    /// Return the provider-neutral plugin manifest built from descriptors.
    pub fn build(self) -> PluginManifest {
        self.manifest
    }
}
