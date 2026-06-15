//! Conformance checker report types.
//!
//! Reports are presentation-neutral.  They are safe to render in Web UI, CLI,
//! Store submission tooling, or logs without coupling those surfaces to checker
//! internals.

use macaca_proto::{PackageRuntimeKind, PackageType};

/// Final ecosystem conformance status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceStatus {
    Conformant,
    ConformantWithWarnings,
    NonConformant,
}

/// Diagnostic severity emitted by checker rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceSeverity {
    Info,
    Warning,
    Error,
}

/// One stable, actionable conformance diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceDiagnostic {
    pub code: &'static str,
    pub severity: ConformanceSeverity,
    pub field: String,
    pub message: String,
}

impl ConformanceDiagnostic {
    /// Build a diagnostic with a stable code and manifest field path.
    pub fn new(
        code: &'static str,
        severity: ConformanceSeverity,
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

/// Presentation-neutral trace event emitted by the conformance checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceTraceEvent {
    pub package_id: String,
    pub rule: String,
    pub outcome: String,
    pub message: String,
}

/// Full conformance report returned by the checker facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    pub package_id: String,
    pub package_type: PackageType,
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub status: ConformanceStatus,
    pub diagnostics: Vec<ConformanceDiagnostic>,
    pub trace_events: Vec<ConformanceTraceEvent>,
    pub upgrade_notes: Vec<String>,
}
