//! Developer-facing ecosystem package fixtures.
//!
//! These fixtures are intentionally generic and provider-neutral.  They give
//! documentation, examples, and certification tests one shared source of truth
//! for package shapes without forcing third-party developers to copy internal
//! Macaca implementation details.  The module uses a small Builder-style
//! helper so every fixture can share safe defaults while still declaring the
//! package-specific capabilities, services, and commerce metadata that Phase
//! 13 needs to certify.

use macaca_proto::{
    CapabilityId, DeveloperId, EntitlementId, KernelServiceId, LicenseType, PackageCapability,
    PackageDescriptor, PackageEntry, PackageId, PackageManifest, PackagePermission, PackageRuntime,
    PackageRuntimeKind, PackageServiceRequirement, PackageType,
};

/// Builder for generic ecosystem package fixtures.
///
/// The builder exists only for SDK examples and certification tests.  It keeps
/// fixture construction concise while avoiding any application-specific,
/// provider-specific, driver-specific, gateway-specific, or chain-specific
/// branching in tests.
#[derive(Debug, Clone)]
pub struct EcosystemPackageFixtureBuilder {
    manifest: PackageManifest,
}

impl EcosystemPackageFixtureBuilder {
    /// Start a fixture from the canonical Package Manifest v0 fields.
    pub fn new(
        id: impl Into<String>,
        package_type: PackageType,
        runtime_kind: PackageRuntimeKind,
    ) -> Self {
        let mut manifest = PackageManifest::new(
            PackageId::new(id),
            package_type,
            "1.0.0",
            DeveloperId::new("developer.fixture"),
            PackageRuntime::new(runtime_kind, "1"),
        );
        manifest.compatibility.min_os_version = Some("1".into());
        manifest.compatibility.supported_abi_versions = vec!["1".into()];
        manifest
            .metadata
            .insert("package.manifest.version".into(), "1".into());
        manifest
            .metadata
            .insert("trace.required".into(), "true".into());
        Self { manifest }
    }

    /// Add an entry declaration.
    pub fn entry(mut self, kind: impl Into<String>, value: impl Into<String>) -> Self {
        self.manifest.entry = Some(PackageEntry {
            kind: kind.into(),
            value: value.into(),
        });
        self
    }

    /// Add a required service declaration.
    pub fn required_service(mut self, service: impl Into<String>) -> Self {
        self.manifest
            .required_services
            .push(PackageServiceRequirement {
                service: KernelServiceId::new(service),
                capability: None,
                reason: "Fixture requires this system service for certification".into(),
            });
        self
    }

    /// Add an optional service declaration.
    pub fn optional_service(mut self, service: impl Into<String>) -> Self {
        self.manifest
            .optional_services
            .push(PackageServiceRequirement {
                service: KernelServiceId::new(service),
                capability: None,
                reason: "Fixture can degrade when this optional service is unavailable".into(),
            });
        self
    }

    /// Add a package capability.
    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        let capability = capability.into();
        self.manifest.provides.push(PackageCapability {
            id: CapabilityId::new(&capability),
            description: format!("Generic fixture capability {capability}"),
        });
        self
    }

    /// Add a permission declaration.
    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.manifest.permissions.push(PackagePermission {
            name: permission.into(),
            reason: "Generic fixture permission used by certification".into(),
        });
        self
    }

    /// Add one metadata key/value pair.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.manifest.metadata.insert(key.into(), value.into());
        self
    }

    /// Mark the fixture as commercial and Store-gated.
    pub fn paid(mut self) -> Self {
        self.manifest.commerce.license_type = LicenseType::subscription();
        self.manifest.commerce.store_required = true;
        self.manifest.commerce.entitlement_id = Some(EntitlementId::new("entitlement.fixture"));
        self
    }

    /// Finish the package descriptor.
    pub fn build(self) -> PackageDescriptor {
        PackageDescriptor::new(self.manifest)
    }
}

/// Return a generic YAML application package descriptor.
pub fn yaml_app_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.application.yaml",
        PackageType::Application,
        PackageRuntimeKind::Yaml,
    )
    .entry("agent", "entry")
    .required_service("service.agent.runtime")
    .capability("capability.application.execute")
    .permission("workspace.read")
    .metadata("application.abi.version", "1")
    .build()
}

/// Return a WASM metadata fixture that is intentionally not executable yet.
pub fn wasm_stub_app_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.application.wasm",
        PackageType::Application,
        PackageRuntimeKind::WasmComponent,
    )
    .entry("component", "component.wasm")
    .required_service("service.application.abi")
    .capability("capability.application.metadata")
    .metadata("application.abi.version", "1")
    .metadata("execution.status", "runtime_unavailable")
    .build()
}

/// Return a GenUI application package descriptor.
pub fn genui_app_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.application.genui",
        PackageType::Application,
        PackageRuntimeKind::Yaml,
    )
    .entry("ui", "surface")
    .required_service("service.ui.runtime")
    .capability("capability.ui.render")
    .permission("ui.render")
    .metadata("genui.schema.version", "1")
    .metadata("trace.ui.required", "true")
    .build()
}

/// Return a generic gateway plugin descriptor.
pub fn gateway_plugin_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.plugin.gateway",
        PackageType::Plugin,
        PackageRuntimeKind::NativeAdapter,
    )
    .entry("plugin", "gateway")
    .required_service("service.gateway.registry")
    .capability("capability.gateway.ingress")
    .permission("network.listen")
    .metadata("plugin.lifecycle", "install,activate,disable")
    .build()
}

/// Return a generic driver plugin descriptor.
pub fn driver_plugin_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.plugin.driver",
        PackageType::Driver,
        PackageRuntimeKind::NativeAdapter,
    )
    .entry("plugin", "driver")
    .required_service("service.driver.registry")
    .capability("capability.driver.execute")
    .permission("resource.lock")
    .metadata("plugin.lifecycle", "install,activate,disable")
    .build()
}

/// Return a free skill descriptor.
pub fn free_skill_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.skill.free",
        PackageType::Skill,
        PackageRuntimeKind::EncryptedTextBundle,
    )
    .entry("skill", "skill.md")
    .capability("capability.skill.invoke")
    .metadata("skill.metadata.encrypted", "false")
    .build()
}

/// Return a paid skill descriptor.
pub fn paid_skill_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.skill.paid",
        PackageType::Skill,
        PackageRuntimeKind::EncryptedTextBundle,
    )
    .entry("skill", "skill.enc")
    .capability("capability.skill.invoke")
    .metadata("skill.metadata.encrypted", "true")
    .paid()
    .build()
}

/// Return an optional Web3 application descriptor.
pub fn web3_optional_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.application.web3",
        PackageType::Application,
        PackageRuntimeKind::Yaml,
    )
    .entry("agent", "entry")
    .optional_service("service.web3.node")
    .capability("capability.web3.optional")
    .metadata("optional.module.web3", "true")
    .build()
}

/// Return an optional EVM/DApp descriptor.
pub fn evm_optional_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.application.evm",
        PackageType::Application,
        PackageRuntimeKind::Yaml,
    )
    .entry("dapp", "entry")
    .optional_service("service.evm.runtime")
    .capability("capability.evm.optional")
    .metadata("optional.module.evm", "true")
    .build()
}

/// Return an invalid descriptor with no runtime kind.
pub fn invalid_missing_runtime_fixture() -> PackageDescriptor {
    let mut descriptor = yaml_app_fixture();
    descriptor.manifest.runtime.kind = None;
    descriptor
}

/// Return an invalid descriptor with a required service that certification will not provide.
pub fn invalid_missing_required_service_fixture() -> PackageDescriptor {
    EcosystemPackageFixtureBuilder::new(
        "fixture.invalid.required_service",
        PackageType::Application,
        PackageRuntimeKind::Yaml,
    )
    .required_service("service.unavailable.required")
    .metadata("application.abi.version", "1")
    .build()
}
