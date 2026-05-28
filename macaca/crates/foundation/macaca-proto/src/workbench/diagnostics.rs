//! Provider-neutral contracts for `service.diagnostics`.
//!
//! Diagnostics is a generic operator-observability service.  It aggregates
//! bounded source snapshots, feedback receipts, trace bundles, and health
//! summaries without owning the semantics of interaction, files, processes,
//! tools, plugins, models, or any application workflow.  Sensitive material is
//! represented through redacted summaries and artifact references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ServiceHealth;

use super::{ArtifactRef, BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.diagnostics";
pub const CAPABILITY_ID: &str = "capability.diagnostics";
pub const SNAPSHOT_COMMAND: &str = "diagnostics.snapshot";
pub const FEEDBACK_UPLOAD_COMMAND: &str = "diagnostics.feedback.upload";
pub const TRACE_BUNDLE_COMMAND: &str = "diagnostics.trace.bundle";
pub const HEALTH_SUMMARY_COMMAND: &str = "diagnostics.health.summary";

pub const COMMANDS: &[&str] = &[
    SNAPSHOT_COMMAND,
    FEEDBACK_UPLOAD_COMMAND,
    TRACE_BUNDLE_COMMAND,
    HEALTH_SUMMARY_COMMAND,
];

pub type DiagnosticsCommand<T> = WorkbenchCommand<T>;

/// Source families that can contribute sanitized diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DiagnosticsSourceKind {
    Interaction,
    File,
    Process,
    Sandbox,
    Approval,
    Hook,
    Config,
    Plugin,
    Mcp,
    Skill,
    Tool,
    Llm,
    Git,
    Review,
    AppProtocol,
}

/// Redaction policy applied before any diagnostic material becomes visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsRedactionProfile {
    pub profile_id: String,
    pub max_inline_bytes: usize,
    pub secret_markers: Vec<String>,
}

impl Default for DiagnosticsRedactionProfile {
    fn default() -> Self {
        Self {
            profile_id: "diagnostics.default-redaction.v1".into(),
            max_inline_bytes: 512,
            secret_markers: vec![
                "secret".into(),
                "token".into(),
                "password".into(),
                "credential".into(),
                "private_key".into(),
            ],
        }
    }
}

/// One sanitized source snapshot included in a diagnostic report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsSourceSnapshot {
    pub source: DiagnosticsSourceKind,
    pub service_id: String,
    pub health: ServiceHealth,
    pub summary: Option<BoundedSummary>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsSnapshotRequest {
    pub include_sources: Vec<DiagnosticsSourceKind>,
    pub redaction_profile: DiagnosticsRedactionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsFeedbackUploadRequest {
    pub category: String,
    pub operator_ref: Option<String>,
    pub summary: BoundedSummary,
    pub attachment_refs: Vec<ArtifactRef>,
    pub allow_remote_upload: bool,
    pub redaction_profile: DiagnosticsRedactionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsTraceBundleRequest {
    pub trace_refs: Vec<String>,
    pub include_audit_refs: bool,
    pub redaction_profile: DiagnosticsRedactionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsHealthSummaryRequest {
    pub service_ids: Vec<String>,
    pub redaction_profile: DiagnosticsRedactionProfile,
}

/// Replayable diagnostic memento. Inline summaries are already redacted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsBundle {
    pub bundle_ref: String,
    pub redaction_profile_id: String,
    pub sources: Vec<DiagnosticsSourceSnapshot>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub bounded: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsFeedbackReceipt {
    pub feedback_ref: String,
    pub uploaded: bool,
    pub summary: BoundedSummary,
    pub attachment_refs: Vec<ArtifactRef>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsHealthSummary {
    pub services: Vec<DiagnosticsSourceSnapshot>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticsServiceResponse {
    Snapshot(DiagnosticsBundle),
    FeedbackUploaded(DiagnosticsFeedbackReceipt),
    TraceBundle(DiagnosticsBundle),
    HealthSummary(DiagnosticsHealthSummary),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.diagnostics.events.v1",
        "service.diagnostics.audit.v1",
        "service.diagnostics.snapshot.v1",
    )
}
