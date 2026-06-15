//! Ecosystem package conformance checker facade.
//!
//! Phase 13 uses this additive facade for developer certification.  The facade
//! does not execute packages and does not grant permissions.  It coordinates a
//! set of Specification-style rules, lets those rules visit package metadata,
//! and returns a traceable/auditable report for SDK tooling, Web UI, CLI, Store
//! submission checks, and integration tests.

#[path = "conformance_checker/context.rs"]
mod context;
#[path = "conformance_checker/report.rs"]
mod report;
#[path = "conformance_checker/rules.rs"]
mod rules;

use macaca_proto::PackageDescriptor;
use tracing::{info, warn};

pub use context::ConformanceHostContext;
pub use report::{
    ConformanceDiagnostic, ConformanceReport, ConformanceSeverity, ConformanceStatus,
    ConformanceTraceEvent,
};

use rules::{
    AbiRule, CommerceRule, ConformanceRule, ManifestVersionRule, OptionalModuleRule,
    PackageTypeRule, PermissionRule, RuntimeRule, ServiceRule, TraceRule, UpgradeRule,
};

/// Facade used by SDK tooling, certification tests, Web UI, CLI, and Store tools.
#[derive(Default)]
pub struct PackageConformanceChecker {
    rules: Vec<Box<dyn ConformanceRule>>,
}

impl PackageConformanceChecker {
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
        context: &ConformanceHostContext,
    ) -> ConformanceReport {
        let mut visitor = ConformanceVisitor::new(descriptor, context);
        visitor.trace("checker", "started", "package conformance check started");
        info!(
            package_id = %descriptor.manifest.id,
            package_type = %descriptor.manifest.package_type,
            "package conformance check started"
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
pub(crate) struct ConformanceVisitor<'a> {
    pub(crate) descriptor: &'a PackageDescriptor,
    pub(crate) context: &'a ConformanceHostContext,
    diagnostics: Vec<ConformanceDiagnostic>,
    trace_events: Vec<ConformanceTraceEvent>,
    upgrade_notes: Vec<String>,
}

impl<'a> ConformanceVisitor<'a> {
    fn new(
        descriptor: &'a PackageDescriptor,
        context: &'a ConformanceHostContext,
    ) -> ConformanceVisitor<'a> {
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
        self.trace_events.push(ConformanceTraceEvent {
            package_id: self.descriptor.manifest.id.to_string(),
            rule: rule.into(),
            outcome: outcome.into(),
            message: message.into(),
        });
    }

    /// Record a structured diagnostic and mirror it into trace/log channels.
    pub(crate) fn diagnostic(&mut self, diagnostic: ConformanceDiagnostic) {
        match diagnostic.severity {
            ConformanceSeverity::Error => warn!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package conformance rule failed"
            ),
            ConformanceSeverity::Warning => warn!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package conformance rule warned"
            ),
            ConformanceSeverity::Info => info!(
                package_id = %self.descriptor.manifest.id,
                code = diagnostic.code,
                field = %diagnostic.field,
                "package conformance rule noted"
            ),
        }
        self.trace(
            diagnostic.code,
            match diagnostic.severity {
                ConformanceSeverity::Info => "info",
                ConformanceSeverity::Warning => "warning",
                ConformanceSeverity::Error => "error",
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
            .any(|diagnostic| diagnostic.severity == ConformanceSeverity::Error)
    }

    fn has_warning(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ConformanceSeverity::Warning)
    }

    fn finish(mut self) -> ConformanceReport {
        let status = if self.has_error() {
            ConformanceStatus::NonConformant
        } else if self.has_warning() {
            ConformanceStatus::ConformantWithWarnings
        } else {
            ConformanceStatus::Conformant
        };
        self.trace("checker", "finished", format!("final status: {status:?}"));
        info!(
            package_id = %self.descriptor.manifest.id,
            status = ?status,
            "package conformance check finished"
        );
        ConformanceReport {
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

    fn context() -> ConformanceHostContext {
        ConformanceHostContext::default()
            .with_service("service.agent.runtime")
            .with_service("service.application.abi")
            .with_service("service.ui.runtime")
            .with_service("service.gateway.registry")
            .with_service("service.driver.registry")
    }

    #[test]
    fn conformance_checker_accepts_valid_package() {
        let report = PackageConformanceChecker::new().check(&yaml_app_fixture(), &context());
        assert_eq!(report.status, ConformanceStatus::Conformant);
        assert!(report
            .trace_events
            .iter()
            .any(|event| event.rule == "checker"));
    }

    #[test]
    fn conformance_checker_warns_for_optional_module_unavailable() {
        let report = PackageConformanceChecker::new().check(&web3_optional_fixture(), &context());
        assert_eq!(report.status, ConformanceStatus::ConformantWithWarnings);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "optional_module.unavailable"));
    }

    #[test]
    fn conformance_checker_rejects_invalid_metadata() {
        let missing_runtime =
            PackageConformanceChecker::new().check(&invalid_missing_runtime_fixture(), &context());
        let missing_service = PackageConformanceChecker::new()
            .check(&invalid_missing_required_service_fixture(), &context());
        assert_eq!(missing_runtime.status, ConformanceStatus::NonConformant);
        assert_eq!(missing_service.status, ConformanceStatus::NonConformant);
    }

    #[test]
    fn conformance_checker_distinguishes_paid_entitlement_states() {
        let denied = PackageConformanceChecker::new().check(&paid_skill_fixture(), &context());
        assert_eq!(denied.status, ConformanceStatus::ConformantWithWarnings);

        let allowed_context = context().with_entitlement_state(EntitlementState::valid());
        let allowed =
            PackageConformanceChecker::new().check(&paid_skill_fixture(), &allowed_context);
        assert_eq!(allowed.status, ConformanceStatus::Conformant);

        let free = PackageConformanceChecker::new().check(&free_skill_fixture(), &context());
        assert_eq!(free.status, ConformanceStatus::Conformant);
    }

    #[test]
    fn conformance_checker_reports_version_and_future_kind_diagnostics() {
        let mut descriptor = yaml_app_fixture();
        descriptor.manifest.runtime.abi_version = "0".into();
        let rejected = PackageConformanceChecker::new().check(&descriptor, &context());
        assert_eq!(rejected.status, ConformanceStatus::NonConformant);

        let future = EcosystemPackageFixtureBuilder::new(
            "fixture.future",
            PackageType::Custom("future.package".into()),
            PackageRuntimeKind::Custom("future.runtime".into()),
        )
        .metadata("package.manifest.version", "2")
        .build();
        let warned = PackageConformanceChecker::new().check(&future, &context());
        assert_eq!(warned.status, ConformanceStatus::ConformantWithWarnings);
    }
}
