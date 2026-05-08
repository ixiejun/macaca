//! Compatibility checker report types.
//!
//! Reports are presentation-neutral.  They are safe to render in Web UI, CLI,
//! Store submission tooling, or logs without coupling those surfaces to checker
//! internals.

use macaca_proto::{PackageRuntimeKind, PackageType};

/// Final ecosystem compatibility status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Compatible,
    CompatibleWithWarnings,
    Incompatible,
}

/// Diagnostic severity emitted by checker rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilitySeverity {
    Info,
    Warning,
    Error,
}

/// One stable, actionable compatibility diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityDiagnostic {
    pub code: &'static str,
    pub severity: CompatibilitySeverity,
    pub field: String,
    pub message: String,
}

impl CompatibilityDiagnostic {
    /// Build a diagnostic with a stable code and manifest field path.
    pub fn new(
        code: &'static str,
        severity: CompatibilitySeverity,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Presentation-neutral trace event emitted by the compatibility checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityTraceEvent {
    pub package_id: String,
    pub rule: String,
    pub outcome: String,
    pub message: String,
}

/// Full compatibility report returned by the checker facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub package_id: String,
    pub package_type: PackageType,
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub status: CompatibilityStatus,
    pub diagnostics: Vec<CompatibilityDiagnostic>,
    pub trace_events: Vec<CompatibilityTraceEvent>,
    pub upgrade_notes: Vec<String>,
}
