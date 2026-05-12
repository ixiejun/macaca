//! Contract TestKit for Macaca Application Platform SDK fixtures.
//!
//! The TestKit applies Specification-style checks to provider-neutral
//! manifests.  It never executes application code and never constructs runtime
//! objects.  This keeps developer validation fast, deterministic, and safe for
//! Store/certification tooling.

use macaca_proto::{ApplicationHostCommand, ApplicationManifestV1};

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

    /// Validate one host command that must carry trace context.
    ///
    /// Application host commands can be assembled by package tests before any
    /// runtime exists.  This check implements the Specification pattern for the
    /// Route C trace invariant: every host operation that crosses the
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ApplicationHostCommandBuilder;
    use crate::application_kit::ApplicationKit;
    use macaca_proto::ApplicationImport;

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
}
