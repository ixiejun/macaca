//! Contract TestKit for Macaca Application Platform SDK fixtures.
//!
//! The TestKit applies Specification-style checks to provider-neutral
//! manifests.  It never executes application code and never constructs runtime
//! objects.  This keeps developer validation fast, deterministic, and safe for
//! Store/certification tooling.

use macaca_proto::{
    ApplicationAbiDeclaration, ApplicationHostCommand, ApplicationManifestV1, PackageRuntimeKind,
    WasmAbiNegotiationResult, WasmComponentArtifactDescriptor, WasmEngineCapabilities,
};

/// Safe diagnostic returned by SDK-side contract tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationContractDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl ApplicationContractDiagnostic {
    fn new(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }
}

/// Serializable-friendly report shape for application contract checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationContractReport {
    pub diagnostics: Vec<ApplicationContractDiagnostic>,
}

impl ApplicationContractReport {
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }

    fn push(&mut self, diagnostic: ApplicationContractDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

/// Facade for SDK contract validation.
#[derive(Debug, Default, Clone, Copy)]
pub struct ApplicationContractTestKit;

impl ApplicationContractTestKit {
    /// Validate manifest and ability declarations without runtime execution.
    pub fn validate_manifest(&self, manifest: &ApplicationManifestV1) -> ApplicationContractReport {
        let mut report = ApplicationContractReport::default();
        if manifest.abilities.is_empty() {
            report.push(ApplicationContractDiagnostic::new(
                "missing_ability",
                manifest.package_id.as_str(),
                "Application manifests must declare at least one ability",
            ));
        }
        for ability in &manifest.abilities {
            if ability.id.trim().is_empty() {
                report.push(ApplicationContractDiagnostic::new(
                    "missing_ability_id",
                    manifest.package_id.as_str(),
                    "Ability descriptors must have stable ids",
                ));
            }
            for permission in &ability.permissions {
                if permission.reason.trim().is_empty() {
                    report.push(ApplicationContractDiagnostic::new(
                        "missing_permission_reason",
                        &permission.name,
                        "Ability permissions must explain why they are required",
                    ));
                }
            }
            for service in &ability.services {
                if service.reason.trim().is_empty() {
                    report.push(ApplicationContractDiagnostic::new(
                        "missing_service_reason",
                        service.service.as_str(),
                        "Ability service requirements must explain why they are required",
                    ));
                }
            }
        }
        tracing::info!(
            package_id = %manifest.package_id,
            ability_count = manifest.abilities.len(),
            diagnostic_count = report.diagnostics.len(),
            "application contract test kit validated manifest"
        );
        report
    }

    /// Validate ABI declarations without loading or executing application code.
    ///
    /// The check is intentionally conservative.  It verifies the required ABI
    /// exports and import trace model while reporting only safe identifiers and
    /// reason codes.  It does not inspect raw manifest bodies, component bytes,
    /// prompt bodies, secrets, environment values, or host payloads.
    pub fn validate_abi_declaration(
        &self,
        declaration: &ApplicationAbiDeclaration,
    ) -> ApplicationContractReport {
        let mut report = ApplicationContractReport::default();
        if declaration.application_id.trim().is_empty() {
            report.push(ApplicationContractDiagnostic::new(
                "missing_application_id",
                "application_abi",
                "Application ABI declarations must carry a stable application id",
            ));
        }
        if let Err(error) = declaration.validate_required_exports() {
            report.push(ApplicationContractDiagnostic::new(
                "missing_required_export",
                &declaration.application_id,
                error.to_string(),
            ));
        }
        if declaration.imports.is_empty() {
            report.push(ApplicationContractDiagnostic::new(
                "missing_imports",
                &declaration.application_id,
                "Application ABI declarations must list host imports explicitly",
            ));
        }
        tracing::info!(
            application_id = %declaration.application_id,
            import_count = declaration.imports.len(),
            export_count = declaration.exports.len(),
            diagnostic_count = report.diagnostics.len(),
            "application contract test kit validated ABI declaration"
        );
        report
    }

    /// Validate that a WASM Manifest v1 fixture is metadata-only and safe.
    ///
    /// The SDK does not execute WASM.  It only confirms the runtime kind,
    /// ability declarations, and ABI metadata required by certification tests.
    pub fn validate_wasm_component_fixture(
        &self,
        manifest: &ApplicationManifestV1,
        declaration: &ApplicationAbiDeclaration,
    ) -> ApplicationContractReport {
        let mut report = self.validate_manifest(manifest);
        if manifest.runtime.kind != PackageRuntimeKind::WasmComponent {
            report.push(ApplicationContractDiagnostic::new(
                "invalid_runtime_kind",
                manifest.package_id.as_str(),
                "WASM component fixtures must declare PackageRuntimeKind::WasmComponent",
            ));
        }
        let abi_report = self.validate_abi_declaration(declaration);
        for diagnostic in abi_report.diagnostics {
            report.push(diagnostic);
        }
        tracing::info!(
            package_id = %manifest.package_id,
            diagnostic_count = report.diagnostics.len(),
            "application contract test kit validated WASM component fixture"
        );
        report
    }

    /// Validate WASM artifact metadata and ABI negotiation inputs.
    ///
    /// This SDK-side Specification mirrors the Application Framework
    /// admission contract without depending on `macaca-app` or a runtime-host
    /// provider.  It checks only metadata DTOs: artifact id/reference, digest,
    /// ABI version match, required import permissions, and runtime
    /// capability flags.  Diagnostics are intentionally bounded to reason
    /// codes and identifiers so SDK tooling never exposes raw WASM bytes, raw
    /// manifests, host payloads, secrets, environment values, API keys, prompts,
    /// private keys, or provider output.
    pub fn validate_wasm_artifact_admission(
        &self,
        manifest: &ApplicationManifestV1,
        artifact: &WasmComponentArtifactDescriptor,
        supported_versions: Vec<String>,
        capabilities: WasmEngineCapabilities,
    ) -> ApplicationContractReport {
        let mut report = ApplicationContractReport::default();
        if artifact.artifact_id.trim().is_empty() || artifact.artifact_ref.is_empty() {
            report.push(ApplicationContractDiagnostic::new(
                "artifact_reference_missing",
                manifest.package_id.as_str(),
                "WASM artifacts must declare a stable id and metadata-only reference",
            ));
        }
        if !artifact.digest.is_present() {
            report.push(ApplicationContractDiagnostic::new(
                "artifact_digest_missing",
                artifact.artifact_id.as_str(),
                "WASM artifacts must declare digest metadata before admission",
            ));
        }

        let negotiation = WasmAbiNegotiationResult::negotiate(
            artifact.abi.clone(),
            supported_versions,
            capabilities,
        );
        for code in negotiation.reason_codes {
            report.push(ApplicationContractDiagnostic::new(
                code,
                artifact.artifact_id.as_str(),
                "WASM ABI negotiation failed closed",
            ));
        }

        let declared_permissions: std::collections::BTreeSet<String> = manifest
            .permissions
            .iter()
            .map(|permission| permission.name.clone())
            .chain(
                manifest
                    .abilities
                    .iter()
                    .flat_map(|ability| ability.permissions.iter())
                    .map(|permission| permission.name.clone()),
            )
            .collect();
        for import in &artifact.required_imports {
            if import.optional {
                continue;
            }
            if let Some(permission) = &import.permission {
                if !declared_permissions.contains(permission) {
                    report.push(ApplicationContractDiagnostic::new(
                        "import_permission_missing",
                        import.import.to_string(),
                        "Required WASM host import permission is not declared",
                    ));
                }
            }
        }

        for key in artifact.metadata.keys() {
            let lower = key.to_ascii_lowercase();
            if SDK_FORBIDDEN_WASM_METADATA_KEYS
                .iter()
                .any(|marker| lower.contains(marker))
            {
                report.push(ApplicationContractDiagnostic::new(
                    "unsafe_wasm_metadata",
                    artifact.artifact_id.as_str(),
                    "Unsafe WASM artifact metadata category is not allowed in SDK diagnostics",
                ));
            }
        }
        tracing::info!(
            package_id = %manifest.package_id,
            artifact_id = %artifact.artifact_id,
            diagnostic_count = report.diagnostics.len(),
            "application contract test kit validated WASM artifact admission"
        );
        report
    }

    /// Validate one host command that must carry trace context.
    ///
    /// Application host commands can be assembled by package tests before any
    /// runtime exists.  This check implements the Specification pattern for the
    /// Protocol trace invariant: every host operation that crosses the
    /// application boundary must either include a trace context or fail closed
    /// before execution.  The diagnostic intentionally reports only the import
    /// label and safe reason code; it never includes raw payloads, prompt
    /// bodies, environment values, or other unbounded application data.
    pub fn validate_trace_required_command(
        &self,
        fixture_id: impl AsRef<str>,
        command: &ApplicationHostCommand,
    ) -> ApplicationContractReport {
        let mut report = ApplicationContractReport::default();
        let fixture_id = fixture_id.as_ref();
        if command
            .trace
            .as_ref()
            .map(|trace| trace.trace_id.trim().is_empty())
            .unwrap_or(true)
        {
            report.push(ApplicationContractDiagnostic::new(
                "missing_trace",
                fixture_id,
                "Trace-required application host commands must include a non-empty trace id",
            ));
        }
        tracing::info!(
            fixture_id = %fixture_id,
            import = ?command.import,
            diagnostic_count = report.diagnostics.len(),
            "application contract test kit validated trace-required command"
        );
        report
    }
}

const SDK_FORBIDDEN_WASM_METADATA_KEYS: &[&str] = &[
    "raw wasm",
    "raw_manifest",
    "raw manifest",
    "raw payload",
    "secret",
    "api_key",
    "apikey",
    "private key",
    "env",
    "prompt",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ApplicationHostCommandBuilder;
    use crate::application_kit::ApplicationKit;
    use macaca_proto::{
        ApplicationImport, WasmAbiRequirement, WasmArtifactDigest, WasmComponentArtifactDescriptor,
        WasmEngineCapabilities, WasmImportRequirement, WasmRuntimeArtifactRef,
    };

    #[test]
    fn testkit_rejects_missing_ability() {
        let manifest = ApplicationKit::manifest(
            "application.invalid",
            "developer.invalid",
            "Invalid",
            "1.0.0",
        )
        .build();

        let report = ApplicationContractTestKit.validate_manifest(&manifest);
        assert!(!report.is_success());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing_ability"));
    }

    #[test]
    fn testkit_rejects_missing_trace_required_command() {
        let command = ApplicationHostCommandBuilder::new(ApplicationImport::ServiceCall).build();

        let report = ApplicationContractTestKit
            .validate_trace_required_command("fixture.trace.required", &command);

        assert!(!report.is_success());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "missing_trace"
                && diagnostic.subject == "fixture.trace.required"));
    }

    #[test]
    fn testkit_accepts_wasm_component_fixture_contract() {
        let fixture = crate::application_kit::WasmComponentApplicationScaffold::new(
            "fixture.application.wasm",
            "developer.fixture",
            "WASM Fixture",
            "1.0.0",
        )
        .build();

        let report = ApplicationContractTestKit
            .validate_wasm_component_fixture(&fixture.manifest, &fixture.abi);

        assert!(report.is_success());
    }

    #[test]
    fn testkit_validates_wasm_artifact_and_abi_negotiation() {
        let fixture = crate::application_kit::WasmComponentApplicationScaffold::new(
            "fixture.application.wasm",
            "developer.fixture",
            "WASM Fixture",
            "1.0.0",
        )
        .build();
        let mut capabilities = WasmEngineCapabilities::unavailable();
        capabilities.can_execute = true;
        capabilities.supports_component_model = true;
        capabilities.supports_host_import_bridge = true;
        let artifact = WasmComponentArtifactDescriptor::new(
            "artifact.fixture",
            WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
            WasmArtifactDigest::sha256("abc123"),
        )
        .abi(WasmAbiRequirement::new("0"))
        .required_import(WasmImportRequirement::permissioned(
            ApplicationImport::ServiceCall,
            "service.call",
        ));

        let report = ApplicationContractTestKit.validate_wasm_artifact_admission(
            &fixture.manifest,
            &artifact,
            vec!["0".into()],
            capabilities,
        );

        assert!(report.is_success(), "{report:?}");
    }
}
