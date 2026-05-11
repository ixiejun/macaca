//! Builder helpers for public plugin contracts.
//!
//! The builders apply the Builder pattern around protocol DTOs. They keep SDK
//! examples concise while ensuring every produced value remains serializable,
//! provider-neutral, and independent from runtime-host/kernel internals.

use macaca_proto::{
    CapabilityId, DeveloperId, KernelServiceId, PluginCapabilityDescriptor, PluginCapabilityKind,
    PluginCapabilityPermissionHint, PluginConfigRequirement, PluginHookDescriptor,
    PluginHookFailurePolicy, PluginHookKind, PluginHookName, PluginHookTimeoutPolicy, PluginId,
    PluginManifest, PluginPermission, PluginProvidedService, PluginResource,
    PluginRuntimeDeclaration, PluginRuntimeKind, PluginSecretRequirement, PluginServiceCategory,
    PluginSignature, PluginVersion, ServiceDescriptor, ServiceScope, ServiceType, TraceSchemaRef,
};

/// Manifest builder for descriptor, built-in, WASM, process, and remote plugins.
#[derive(Debug, Clone, Default)]
pub struct PluginManifestBuilder {
    plugin_id: Option<PluginId>,
    version: Option<PluginVersion>,
    developer_id: Option<DeveloperId>,
    runtime_kind: Option<PluginRuntimeKind>,
    permissions: Vec<PluginPermission>,
    resources: Vec<PluginResource>,
    services: Vec<PluginProvidedService>,
    signature: Option<PluginSignature>,
}

impl PluginManifestBuilder {
    pub fn plugin_id(mut self, plugin_id: PluginId) -> Self {
        self.plugin_id = Some(plugin_id);
        self
    }

    pub fn version(mut self, version: PluginVersion) -> Self {
        self.version = Some(version);
        self
    }

    pub fn developer_id(mut self, developer_id: DeveloperId) -> Self {
        self.developer_id = Some(developer_id);
        self
    }

    pub fn runtime(mut self, runtime_kind: PluginRuntimeKind) -> Self {
        self.runtime_kind = Some(runtime_kind);
        self
    }

    pub fn permission(mut self, id: impl Into<String>, reason: impl Into<String>) -> Self {
        self.permissions.push(PluginPermission {
            id: id.into().trim().to_string(),
            reason: reason.into().trim().to_string(),
            optional: false,
            metadata: Default::default(),
        });
        self
    }

    pub fn resource(
        mut self,
        id: impl Into<String>,
        kind: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.resources.push(PluginResource {
            id: id.into().trim().to_string(),
            kind: kind.into().trim().to_string(),
            reason: reason.into().trim().to_string(),
            optional: false,
            metadata: Default::default(),
        });
        self
    }

    pub fn service(
        mut self,
        service_id: KernelServiceId,
        category: PluginServiceCategory,
        required_permission: impl Into<String>,
    ) -> Self {
        let required_permission = required_permission.into().trim().to_string();
        let mut service = ServiceDescriptor::new(
            service_id,
            ServiceType::new(category.to_string()),
            TraceSchemaRef::new(format!("trace.plugin.{category}")),
        );
        service
            .required_permissions
            .push(required_permission.clone());
        let mut provided = PluginProvidedService::new(service, category);
        provided.required_permissions.push(required_permission);
        self.services.push(provided);
        self
    }

    pub fn signature_fixture(mut self) -> Self {
        self.signature = Some(PluginSignature {
            algorithm: "fixture".into(),
            key_id: "fixture-key".into(),
            digest: "fixture-digest".into(),
            verification_status: "verification_pending".into(),
        });
        self
    }

    pub fn build(self) -> Result<PluginManifest, String> {
        let plugin_id = self.plugin_id.ok_or("missing plugin id")?;
        let version = self.version.ok_or("missing plugin version")?;
        let developer_id = self.developer_id.ok_or("missing developer id")?;
        let runtime_kind = self
            .runtime_kind
            .unwrap_or(PluginRuntimeKind::DescriptorOnly);
        let mut manifest = PluginManifest::new(
            plugin_id,
            version,
            developer_id,
            PluginRuntimeDeclaration::new(runtime_kind, "plugin_abi_v1"),
        );
        manifest.permissions = self.permissions;
        manifest.resources = self.resources;
        manifest.provides_services = self.services;
        manifest.signature = self.signature;
        Ok(manifest)
    }
}

/// Builder for capability descriptors registered by Plugin Capability Registry.
#[derive(Debug, Clone)]
pub struct PluginCapabilityBuilder {
    descriptor: PluginCapabilityDescriptor,
}

impl PluginCapabilityBuilder {
    pub fn new(
        capability_id: CapabilityId,
        plugin_id: PluginId,
        kind: PluginCapabilityKind,
        name: impl Into<String>,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            descriptor: PluginCapabilityDescriptor::new(
                capability_id,
                plugin_id,
                kind,
                name,
                trace_schema,
            ),
        }
    }

    pub fn permission_hint(
        mut self,
        permission: impl Into<String>,
        required: bool,
        reason: impl Into<String>,
    ) -> Self {
        self.descriptor
            .permission_hints
            .push(PluginCapabilityPermissionHint {
                permission: permission.into().trim().to_string(),
                required,
                reason: reason.into().trim().to_string(),
                metadata: Default::default(),
            });
        self
    }

    pub fn scope(mut self, scope: ServiceScope) -> Self {
        self.descriptor.scopes.push(scope);
        self
    }

    pub fn build(self) -> PluginCapabilityDescriptor {
        self.descriptor
    }
}

/// Builder for typed hook descriptors.
#[derive(Debug, Clone)]
pub struct PluginHookBuilder {
    descriptor: PluginHookDescriptor,
}

impl PluginHookBuilder {
    pub fn observer(
        plugin_id: PluginId,
        hook_name: PluginHookName,
        priority: i32,
        trace_schema: TraceSchemaRef,
    ) -> Self {
        Self {
            descriptor: PluginHookDescriptor::observer(
                plugin_id,
                hook_name,
                priority,
                trace_schema,
            ),
        }
    }

    pub fn kind(mut self, kind: PluginHookKind) -> Self {
        self.descriptor.kind = kind;
        self
    }

    pub fn failure_policy(mut self, policy: PluginHookFailurePolicy) -> Self {
        self.descriptor.failure_policy = policy;
        self
    }

    pub fn timeout_policy(mut self, policy: PluginHookTimeoutPolicy) -> Self {
        self.descriptor.timeout_policy = policy;
        self
    }

    pub fn build(self) -> PluginHookDescriptor {
        self.descriptor
    }
}

/// Builder for sanitized config requirement descriptors.
#[derive(Debug, Clone)]
pub struct PluginConfigBuilder {
    requirement: PluginConfigRequirement,
}

impl PluginConfigBuilder {
    pub fn required(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            requirement: PluginConfigRequirement {
                key: key.into().trim().to_string(),
                required: true,
                present: false,
                description: description.into().trim().to_string(),
            },
        }
    }

    pub fn present(mut self, present: bool) -> Self {
        self.requirement.present = present;
        self
    }

    pub fn build(self) -> PluginConfigRequirement {
        self.requirement
    }
}

/// Builder for sanitized secret requirement descriptors.
#[derive(Debug, Clone)]
pub struct PluginSecretRequirementBuilder {
    requirement: PluginSecretRequirement,
}

impl PluginSecretRequirementBuilder {
    pub fn required(name: impl Into<String>) -> Self {
        Self {
            requirement: PluginSecretRequirement {
                name: name.into().trim().to_string(),
                required: true,
                present: false,
                source_hint: None,
            },
        }
    }

    pub fn present(mut self, present: bool) -> Self {
        self.requirement.present = present;
        self
    }

    pub fn build(self) -> PluginSecretRequirement {
        self.requirement
    }
}

/// Composite SDK registration produced by plugin packages.
#[derive(Debug, Clone)]
pub struct PluginRegistration {
    pub manifest: PluginManifest,
    pub capabilities: Vec<PluginCapabilityDescriptor>,
    pub hooks: Vec<PluginHookDescriptor>,
    pub config_requirements: Vec<PluginConfigRequirement>,
    pub secret_requirements: Vec<PluginSecretRequirement>,
}

/// Composite Builder that groups manifest, capabilities, hooks, and requirements.
#[derive(Debug, Clone, Default)]
pub struct PluginRegistrationBuilder {
    manifest: Option<PluginManifest>,
    capabilities: Vec<PluginCapabilityDescriptor>,
    hooks: Vec<PluginHookDescriptor>,
    config_requirements: Vec<PluginConfigRequirement>,
    secret_requirements: Vec<PluginSecretRequirement>,
}

impl PluginRegistrationBuilder {
    pub fn manifest(mut self, manifest: PluginManifest) -> Self {
        self.manifest = Some(manifest);
        self
    }

    pub fn capability(mut self, capability: PluginCapabilityDescriptor) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn hook(mut self, hook: PluginHookDescriptor) -> Self {
        self.hooks.push(hook);
        self
    }

    pub fn config_requirement(mut self, requirement: PluginConfigRequirement) -> Self {
        self.config_requirements.push(requirement);
        self
    }

    pub fn secret_requirement(mut self, requirement: PluginSecretRequirement) -> Self {
        self.secret_requirements.push(requirement);
        self
    }

    pub fn build(self) -> Result<PluginRegistration, String> {
        Ok(PluginRegistration {
            manifest: self.manifest.ok_or("missing plugin manifest")?,
            capabilities: self.capabilities,
            hooks: self.hooks,
            config_requirements: self.config_requirements,
            secret_requirements: self.secret_requirements,
        })
    }
}
