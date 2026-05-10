//! Ecosystem package compatibility checker facade.
//!
//! Phase 13 uses this additive facade for developer certification.  The facade
//! does not execute packages and does not grant permissions.  It coordinates a
//! set of Specification-style rules, lets those rules visit package metadata,
//! and returns a traceable/auditable report for SDK tooling, Web UI, CLI, Store
//! submission checks, and integration tests.

#[path = "compatibility_checker/context.rs"]
mod context;
#[path = "compatibility_checker/report.rs"]
mod report;
#[path = "compatibility_checker/rules.rs"]
mod rules;

use macaca_proto::PackageDescriptor;
use tracing::{info, warn};

pub use context::CompatibilityHostContext;
pub use report::{
    CompatibilityDiagnostic, CompatibilityReport, CompatibilitySeverity, CompatibilityStatus,
    CompatibilityTraceEvent,
};

use rules::{
    AbiRule, CommerceRule, CompatibilityRule, ManifestVersionRule, OptionalModuleRule,
    PackageTypeRule, PermissionRule, RuntimeRule, ServiceRule, TraceRule, UpgradeRule,
};

/// Facade used by SDK tooling, certification tests, Web UI, CLI, and Store tools.
#[derive(Default)]
pub struct PackageCompatibilityChecker {
    rules: Vec<Box<dyn CompatibilityRule>>,
}

impl PackageCompatibilityChecker {
    /// Create the default checker rule chain.
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(ManifestVersionRule),
                Box::new(RuntimeRule),
                Box::new(AbiRule),
                Box::new(PackageTypeRule),
                Box::new(PermissionRule),
                Box::new(ServiceRule),
                Box::new(OptionalModuleRule),
                Box::new(CommerceRule),
                Box::new(TraceRule),
                Box::new(UpgradeRule),
            ],
        }
    }

    /// Evaluate one package descriptor without executing or authorizing it.
    pub fn check(
        &self,
        descriptor: &PackageDescriptor,
        context: &CompatibilityHostContext,
    ) -> CompatibilityReport {
        let mut visitor = CompatibilityVisitor::new(descriptor, context);
        visitor.trace("checker", "started", "package compatibility check started");
        info!(
            package_id = %descriptor.manifest.id,
            package_type = %descriptor.manifest.package_type,
            "package compatibility check started"
        );

        for rule in &self.rules {
            visitor.trace(rule.name(), "started", "rule evaluation started");
            rule.evaluate(&mut visitor);
        }

        visitor.finish()
    }
}

/// Visitor over package descriptor sections.
///
/// A visitor keeps traversal state and reporting separate from individual
/// rules.  New package fields can add new visitor helpers without turning the
/// checker facade into a monolithic branch-heavy function.
pub(crate) struct CompatibilityVisitor<'a> {
    pub(crate) descriptor: &'a PackageDescriptor,
    pub(crate) context: &'a CompatibilityHostContext,
    diagnostics: Vec<CompatibilityDiagnostic>,
    trace_events: Vec<CompatibilityTraceEvent>,
    upgrade_notes: Vec<String>,
}

impl<'a> CompatibilityVisitor<'a> {
    fn new(
        descriptor: &'a PackageDescriptor,
        context: &'a CompatibilityHostContext,
    ) -> CompatibilityVisitor<'a> {
        Self {
            descriptor,
            context,
            diagnostics: Vec::new(),
            trace_events: Vec::new(),
            upgrade_notes: Vec::new(),
        }
    }

    /// Add a checker trace event without binding to any presentation surface.
    pub(crate) fn trace(
        &mut self,
        rule: impl Into<String>,
        outcome: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.trace_events.push(CompatibilityTraceEvent {
            package_id: self.descriptor.manifest.id.to_string(),
            rule: rule.into(),
            outcome: outcome.into(),
            message: message.into(),
        });
    }

    /// Record a structured diagnostic and mirror it into trace/log channels.
    pub(crate) fn diagnostic(&mut self, diagnostic: CompatibilityDiagnostic) {
        match diagnostic.severity {
            CompatibilitySeverity::Error => warn!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package compatibility rule failed"
            ),
            CompatibilitySeverity::Warning => warn!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package compatibility rule warned"
            ),
            CompatibilitySeverity::Info => info!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package compatibility rule noted"
            ),
        }
        self.trace(
            diagnostic.code,
            match diagnostic.severity {
                CompatibilitySeverity::Info => "info",
                CompatibilitySeverity::Warning => "warning",
                CompatibilitySeverity::Error => "error",
            },
            diagnostic.message.clone(),
        );
        self.diagnostics.push(diagnostic);
    }

    /// Attach one upgrade note to the final report.
    pub(crate) fn upgrade_note(&mut self, note: impl Into<String>) {
        self.upgrade_notes.push(note.into());
    }

    fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == CompatibilitySeverity::Error)
    }

    fn has_warning(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == CompatibilitySeverity::Warning)
    }

    fn finish(mut self) -> CompatibilityReport {
        let status = if self.has_error() {
            CompatibilityStatus::Incompatible
        } else if self.has_warning() {
            CompatibilityStatus::CompatibleWithWarnings
        } else {
            CompatibilityStatus::Compatible
        };
        self.trace("checker", "finished", format!("final status: {status:?}"));
        info!(
            package_id = %self.descriptor.manifest.id,
            status = ?status,
            "package compatibility check finished"
        );
        CompatibilityReport {
            package_id: self.descriptor.manifest.id.to_string(),
            package_type: self.descriptor.manifest.package_type.clone(),
            runtime_kind: self.descriptor.manifest.runtime.kind.clone(),
            status,
            diagnostics: self.diagnostics,
            trace_events: self.trace_events,
            upgrade_notes: self.upgrade_notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use macaca_proto::{EntitlementState, PackageRuntimeKind, PackageType};
    use macaca_sdk::{
        free_skill_fixture, invalid_missing_required_service_fixture,
        invalid_missing_runtime_fixture, paid_skill_fixture, web3_optional_fixture,
        yaml_app_fixture, EcosystemPackageFixtureBuilder,
    };

    use super::*;

    fn context() -> CompatibilityHostContext {
        CompatibilityHostContext::default()
            .with_service("service.agent.runtime")
            .with_service("service.application.abi")
            .with_service("service.ui.runtime")
            .with_service("service.gateway.registry")
            .with_service("service.driver.registry")
    }

    #[test]
    fn compatibility_checker_accepts_valid_package() {
        let report = PackageCompatibilityChecker::new().check(&yaml_app_fixture(), &context());
        assert_eq!(report.status, CompatibilityStatus::Compatible);
        assert!(report
            .trace_events
            .iter()
            .any(|event| event.rule == "checker"));
    }

    #[test]
    fn compatibility_checker_warns_for_optional_module_unavailable() {
        let report = PackageCompatibilityChecker::new().check(&web3_optional_fixture(), &context());
        assert_eq!(report.status, CompatibilityStatus::CompatibleWithWarnings);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "optional_module.unavailable"));
    }

    #[test]
    fn compatibility_checker_rejects_invalid_metadata() {
        let missing_runtime = PackageCompatibilityChecker::new()
            .check(&invalid_missing_runtime_fixture(), &context());
        let missing_service = PackageCompatibilityChecker::new()
            .check(&invalid_missing_required_service_fixture(), &context());
        assert_eq!(missing_runtime.status, CompatibilityStatus::Incompatible);
        assert_eq!(missing_service.status, CompatibilityStatus::Incompatible);
    }

    #[test]
    fn compatibility_checker_distinguishes_paid_entitlement_states() {
        let denied = PackageCompatibilityChecker::new().check(&paid_skill_fixture(), &context());
        assert_eq!(denied.status, CompatibilityStatus::CompatibleWithWarnings);

        let allowed_context = context().with_entitlement_state(EntitlementState::valid());
        let allowed =
            PackageCompatibilityChecker::new().check(&paid_skill_fixture(), &allowed_context);
        assert_eq!(allowed.status, CompatibilityStatus::Compatible);

        let free = PackageCompatibilityChecker::new().check(&free_skill_fixture(), &context());
        assert_eq!(free.status, CompatibilityStatus::Compatible);
    }

    #[test]
    fn compatibility_checker_reports_version_and_future_kind_diagnostics() {
        let mut descriptor = yaml_app_fixture();
        descriptor.manifest.runtime.abi_version = "0".into();
        let rejected = PackageCompatibilityChecker::new().check(&descriptor, &context());
        assert_eq!(rejected.status, CompatibilityStatus::Incompatible);

        let future = EcosystemPackageFixtureBuilder::new(
            "fixture.future",
            PackageType::Custom("future.package".into()),
            PackageRuntimeKind::Custom("future.runtime".into()),
        )
        .metadata("package.manifest.version", "2")
        .build();
        let warned = PackageCompatibilityChecker::new().check(&future, &context());
        assert_eq!(warned.status, CompatibilityStatus::CompatibleWithWarnings);
    }
}
