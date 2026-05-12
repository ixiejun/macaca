//! Certification Facade and rules.
//!
//! `ApplicationCertificationKit` uses Visitor-style traversal over Manifest v1
//! and Specification-style rule methods.  The implementation never executes
//! application code and only emits redacted identifiers and reason codes.

use macaca_proto::{
    AbilityImplementationKind, ApplicationAbiDeclaration, ApplicationAbilityDescriptor,
    ApplicationManifestV1, ApplicationPluginDependency, PackageRuntimeKind,
};

use super::types::{
    ApplicationCertificationContext, ApplicationCertificationFixture,
    ApplicationCertificationReport,
};
use super::wasm_admission::{WasmPackageAdmissionSpec, WasmPackageAdmissionStatus};

/// Facade for Application Platform certification.
#[derive(Debug, Clone, Default)]
pub struct ApplicationCertificationKit;

impl ApplicationCertificationKit {
    /// Certify one data-only fixture against the supplied availability context.
    pub fn certify(
        &self,
        fixture: &ApplicationCertificationFixture,
        context: &ApplicationCertificationContext,
    ) -> ApplicationCertificationReport {
        let mut report =
            ApplicationCertificationReport::new(&fixture.fixture_id, &fixture.manifest);
        report.trace_id = context.trace_id.clone();
        self.visit_manifest(&fixture.manifest, &mut report, context);
        if let Some(abi) = &fixture.abi {
            self.visit_abi(abi, &mut report);
        } else if fixture.manifest.runtime.kind == PackageRuntimeKind::WasmComponent {
            report.fail(
                "missing_abi_declaration",
                fixture.manifest.package_id.as_str(),
                "WASM component fixtures must include ABI metadata before execution",
            );
        }
        if fixture.manifest.runtime.kind == PackageRuntimeKind::WasmComponent {
            let wasm_report = WasmPackageAdmissionSpec::default().evaluate(fixture, context);
            match wasm_report.status {
                WasmPackageAdmissionStatus::Accepted => {}
                WasmPackageAdmissionStatus::Unavailable => {
                    for code in &wasm_report.reason_codes {
                        report.unavailable(
                            code.clone(),
                            fixture.manifest.package_id.as_str(),
                            "WASM package admission reported unavailable runtime capability",
                        );
                    }
                }
                WasmPackageAdmissionStatus::Rejected => {
                    for code in &wasm_report.reason_codes {
                        report.fail(
                            code.clone(),
                            fixture.manifest.package_id.as_str(),
                            "WASM package admission rejected the fixture",
                        );
                    }
                }
            }
            report.wasm_admission = Some(wasm_report);
        }
        tracing::info!(
            fixture_id = %report.fixture_id,
            app_id = %report.app_id,
            operation = %report.operation,
            status = ?report.status,
            diagnostic_count = report.diagnostics.len(),
            trace_id = report.trace_id.as_deref().unwrap_or(""),
            "application platform certification completed"
        );
        report
    }

    fn visit_manifest(
        &self,
        manifest: &ApplicationManifestV1,
        report: &mut ApplicationCertificationReport,
        context: &ApplicationCertificationContext,
    ) {
        self.require_trace(report);
        self.check_runtime(manifest, report, context);
        self.check_manifest_shape(manifest, report);
        self.check_redacted_metadata("manifest.metadata", manifest.metadata.keys(), report);
        for permission in &manifest.permissions {
            if permission.reason.trim().is_empty() {
                report.fail(
                    "missing_permission_reason",
                    &permission.name,
                    "Permission reason is required",
                );
            }
        }
        for ability in &manifest.abilities {
            self.visit_ability(ability, report, context);
        }
        for dependency in &manifest.plugin_dependencies {
            self.visit_plugin_dependency(dependency, report, context);
        }
        if let Some(ui) = &manifest.ui {
            self.check_ui(manifest, ui, report);
        }
        if let Some(commerce) = &manifest.commerce {
            if commerce.store_required && commerce.license.trim().is_empty() {
                report.fail(
                    "missing_commerce_license",
                    manifest.package_id.as_str(),
                    "Store-gated apps require a license",
                );
            }
        }
    }

    fn check_ui(
        &self,
        manifest: &ApplicationManifestV1,
        ui: &macaca_proto::ApplicationUiDeclaration,
        report: &mut ApplicationCertificationReport,
    ) {
        if ui.kind.trim().is_empty() {
            report.fail(
                "missing_ui_kind",
                manifest.package_id.as_str(),
                "UI declarations require a kind",
            );
        }
        if ui.entry.as_deref().unwrap_or("").trim().is_empty() {
            report.fail(
                "missing_ui_entry",
                manifest.package_id.as_str(),
                "UI declarations require an entry",
            );
        }
    }

    fn visit_ability(
        &self,
        ability: &ApplicationAbilityDescriptor,
        report: &mut ApplicationCertificationReport,
        context: &ApplicationCertificationContext,
    ) {
        report.ability_ids.push(ability.id.clone());
        if ability.id.trim().is_empty() {
            report.fail(
                "missing_ability_id",
                report.app_id.clone(),
                "Ability descriptors require stable ids",
            );
        }
        self.check_ability_permissions(ability, report);
        self.check_ability_services(ability, report, context);
        self.check_ability_capabilities(ability, report);
        if ability.implementation == AbilityImplementationKind::WasmComponent
            && !context
                .available_runtimes
                .contains(&PackageRuntimeKind::WasmComponent)
        {
            report.unavailable(
                "wasm_runtime_unavailable",
                &ability.id,
                "WASM runtime is not available",
            );
        }
    }

    fn check_ability_permissions(
        &self,
        ability: &ApplicationAbilityDescriptor,
        report: &mut ApplicationCertificationReport,
    ) {
        for permission in &ability.permissions {
            if permission.reason.trim().is_empty() {
                report.fail(
                    "missing_permission_reason",
                    &ability.id,
                    "Ability permission reason is required",
                );
            }
        }
    }

    fn check_ability_services(
        &self,
        ability: &ApplicationAbilityDescriptor,
        report: &mut ApplicationCertificationReport,
        context: &ApplicationCertificationContext,
    ) {
        for service in &ability.services {
            report.service_ids.push(service.service.to_string());
            if service.reason.trim().is_empty() {
                report.fail(
                    "missing_service_reason",
                    service.service.as_str(),
                    "Service reason is required",
                );
            }
            if !context.available_services.contains(&service.service) {
                if service.optional {
                    report.unavailable(
                        "optional_service_unavailable",
                        service.service.as_str(),
                        "Optional service is unavailable",
                    );
                } else {
                    report.fail(
                        "required_service_unavailable",
                        service.service.as_str(),
                        "Required service is unavailable",
                    );
                }
            }
        }
    }

    fn check_ability_capabilities(
        &self,
        ability: &ApplicationAbilityDescriptor,
        report: &mut ApplicationCertificationReport,
    ) {
        for capability in &ability.capabilities {
            report.capability_ids.push(capability.id.to_string());
            if capability.description.trim().is_empty() {
                report.fail(
                    "missing_capability_description",
                    capability.id.as_str(),
                    "Capability description is required",
                );
            }
        }
    }

    fn visit_plugin_dependency(
        &self,
        dependency: &ApplicationPluginDependency,
        report: &mut ApplicationCertificationReport,
        context: &ApplicationCertificationContext,
    ) {
        if dependency.reason.trim().is_empty() {
            report.fail(
                "missing_plugin_reason",
                &dependency.plugin_id,
                "Plugin dependency reason is required",
            );
        }
        if !context.available_plugins.contains(&dependency.plugin_id) {
            if dependency.optional {
                report.unavailable(
                    "optional_plugin_unavailable",
                    &dependency.plugin_id,
                    "Optional plugin is unavailable",
                );
            } else {
                report.fail(
                    "required_plugin_unavailable",
                    &dependency.plugin_id,
                    "Required plugin is unavailable",
                );
            }
        }
    }

    fn visit_abi(
        &self,
        abi: &ApplicationAbiDeclaration,
        report: &mut ApplicationCertificationReport,
    ) {
        if let Err(error) = abi.validate_required_exports() {
            report.fail(
                "missing_required_export",
                &abi.application_id,
                error.to_string(),
            );
        }
        if abi.imports.is_empty() {
            report.fail(
                "missing_abi_imports",
                &abi.application_id,
                "ABI declarations must list imports",
            );
        }
    }

    fn require_trace(&self, report: &mut ApplicationCertificationReport) {
        if report.trace_id.as_deref().unwrap_or("").trim().is_empty() {
            report.fail(
                "missing_trace",
                report.app_id.clone(),
                "Certification requires trace context",
            );
        }
    }

    fn check_runtime(
        &self,
        manifest: &ApplicationManifestV1,
        report: &mut ApplicationCertificationReport,
        context: &ApplicationCertificationContext,
    ) {
        if !context.available_runtimes.contains(&manifest.runtime.kind) {
            report.unavailable(
                "runtime_unavailable",
                manifest.package_id.as_str(),
                "Declared runtime is unavailable",
            );
        }
    }

    fn check_manifest_shape(
        &self,
        manifest: &ApplicationManifestV1,
        report: &mut ApplicationCertificationReport,
    ) {
        if manifest.abilities.is_empty() {
            report.fail(
                "missing_ability",
                manifest.package_id.as_str(),
                "Application manifests require at least one ability",
            );
        }
        if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
            report.fail(
                "missing_manifest_identity",
                manifest.package_id.as_str(),
                "Application identity fields are required",
            );
        }
    }

    fn check_redacted_metadata<'a>(
        &self,
        subject: &str,
        keys: impl Iterator<Item = &'a String>,
        report: &mut ApplicationCertificationReport,
    ) {
        for key in keys {
            let lower = key.to_ascii_lowercase();
            if lower.contains("secret")
                || lower.contains("api_key")
                || lower.contains("private_key")
                || lower.contains("raw")
                || lower.contains("payload")
                || lower.contains("prompt")
                || lower.contains("env")
            {
                report.fail(
                    "unsafe_metadata_key",
                    subject,
                    "Unsafe metadata category is not allowed in certification reports",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certification::ApplicationCertificationFixture;
    use macaca_proto::{
        AbilityImplementationKind, ApplicationAbilityDescriptor, ApplicationAbilityKind,
        ApplicationRuntimeProfile, DeveloperId, PackageId,
    };

    fn fixture() -> ApplicationCertificationFixture {
        let manifest = ApplicationManifestV1::new(
            PackageId::new("application.fixture.certification"),
            DeveloperId::new("developer.fixture"),
            "Certification Fixture",
            "1.0.0",
            ApplicationRuntimeProfile::new(PackageRuntimeKind::Yaml, "1"),
            macaca_proto::ApplicationCompatibilityDeclaration::new("0.1.0"),
        )
        .ability(ApplicationAbilityDescriptor::new(
            "ability.fixture.agent",
            ApplicationAbilityKind::Agent,
            AbilityImplementationKind::Declarative,
        ));
        ApplicationCertificationFixture::new("fixture.certification.agent", manifest)
    }

    #[test]
    fn certification_accepts_traceable_manifest() {
        let context = ApplicationCertificationContext::new()
            .trace_id("trace-certification-test")
            .runtime(PackageRuntimeKind::Yaml);
        let report = ApplicationCertificationKit.certify(&fixture(), &context);
        assert!(report.is_success(), "{report:?}");
    }

    #[test]
    fn certification_rejects_missing_trace() {
        let context = ApplicationCertificationContext::new().runtime(PackageRuntimeKind::Yaml);
        let report = ApplicationCertificationKit.certify(&fixture(), &context);
        assert!(!report.is_success());
        assert!(report
            .reason_codes
            .iter()
            .any(|code| code == "missing_trace"));
    }
}
