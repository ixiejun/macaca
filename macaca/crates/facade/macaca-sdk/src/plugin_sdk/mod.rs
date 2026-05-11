//! Public Plugin SDK facade.
//!
//! This module is the stable developer-facing surface for Macaca plugins. It
//! intentionally exposes protocol DTOs, builders, and contract checks only.
//! Plugin authors should not import kernel, runtime-host, web, CLI, or service
//! implementation crates to describe a plugin package.

mod builders;
mod context;
mod testkit;

pub use builders::{
    PluginCapabilityBuilder, PluginConfigBuilder, PluginHookBuilder, PluginManifestBuilder,
    PluginRegistration, PluginRegistrationBuilder, PluginSecretRequirementBuilder,
};
pub use context::PluginContext;
pub use testkit::{PluginContractDiagnostic, PluginContractReport, PluginContractTestKit};

/// Small Facade used by plugin authors and built-in adapter authors.
///
/// `PluginSdk` does not hold runtime state. It is a namespace for builders and
/// validators, which keeps plugin package construction deterministic and easy
/// to test without starting Macaca services.
#[derive(Debug, Clone, Default)]
pub struct PluginSdk;

impl PluginSdk {
    /// Create a new stateless SDK facade.
    pub fn new() -> Self {
        Self
    }

    /// Start a provider-neutral manifest builder.
    pub fn manifest(&self) -> PluginManifestBuilder {
        PluginManifestBuilder::default()
    }

    /// Start a provider-neutral registration builder.
    pub fn registration(&self) -> PluginRegistrationBuilder {
        PluginRegistrationBuilder::default()
    }

    /// Create a contract test kit for validating SDK output.
    pub fn contract_tests(&self) -> PluginContractTestKit {
        PluginContractTestKit::default()
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        CapabilityId, DeveloperId, PluginHostOperation, PluginHostResult, PluginHostStatus,
        PluginId, PluginRuntimeKind, PluginServiceCategory, PluginVersion, TraceContext,
        TraceSchemaRef,
    };

    use super::*;

    #[test]
    fn sdk_builds_descriptor_registration_without_runtime_host_types() {
        let sdk = PluginSdk::new();
        let registration = sdk
            .registration()
            .manifest(
                sdk.manifest()
                    .plugin_id(PluginId::new("plugin.sdk.fixture"))
                    .version(PluginVersion::new("1.0.0"))
                    .developer_id(DeveloperId::new("developer.fixture"))
                    .runtime(PluginRuntimeKind::DescriptorOnly)
                    .permission("permission.fixture", "Fixture permission")
                    .resource("resource.fixture", "workspace", "Fixture resource")
                    .signature_fixture()
                    .build()
                    .unwrap(),
            )
            .capability(
                PluginCapabilityBuilder::new(
                    CapabilityId::new("capability.fixture"),
                    PluginId::new("plugin.sdk.fixture"),
                    macaca_proto::PluginCapabilityKind::Tool,
                    "fixture",
                    TraceSchemaRef::new("trace.fixture"),
                )
                .permission_hint("permission.fixture", true, "Fixture permission")
                .build(),
            )
            .build()
            .unwrap();

        let report = sdk.contract_tests().validate_registration(&registration);
        assert!(report.is_success(), "{report:?}");
    }

    #[test]
    fn contract_test_kit_catches_missing_privileged_permission() {
        let manifest = PluginSdk::new()
            .manifest()
            .plugin_id(PluginId::new("plugin.sdk.invalid"))
            .version(PluginVersion::new("1.0.0"))
            .developer_id(DeveloperId::new("developer.fixture"))
            .runtime(PluginRuntimeKind::DescriptorOnly)
            .service(
                macaca_proto::KernelServiceId::new("service.fixture"),
                PluginServiceCategory::Skill,
                "permission.missing",
            )
            .resource("resource.fixture", "workspace", "Fixture resource")
            .signature_fixture()
            .build()
            .unwrap();

        let report = PluginContractTestKit::default().validate_manifest(&manifest);
        assert!(!report.is_success());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing_permission_declaration"));
    }

    #[test]
    fn unavailable_safe_contract_accepts_structured_result() {
        let descriptor = PluginContext::new(
            PluginId::new("plugin.sdk.fixture"),
            PluginVersion::new("1.0.0"),
            PluginRuntimeKind::Process,
        )
        .host_descriptor();
        let command = macaca_proto::PluginHostCommand::new(
            descriptor,
            PluginHostOperation::Start,
            serde_json::json!({}),
            TraceContext::new("sdk-host-trace"),
        );
        let result = PluginHostResult::unavailable(&command, "process disabled");
        let report = PluginContractTestKit::default().validate_unavailable_result(&result);

        assert!(report.is_success());
        assert_eq!(result.status, PluginHostStatus::Unavailable);
    }
}
