//! Provider-neutral contracts for `service.review`.
//!
//! The review service models generic review runs, progress, structured
//! findings, and bounded evidence.  It does not encode coding-product workflow
//! semantics; coding, migration, documentation, compliance, and QA
//! applications can all request the same review surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.review";
pub const CAPABILITY_ID: &str = "capability.review";
pub const START_COMMAND: &str = "review.start";
pub const PROGRESS_COMMAND: &str = "review.progress";
pub const RESULT_COMMAND: &str = "review.result";
pub const FINDINGS_LIST_COMMAND: &str = "review.findings.list";

pub const COMMANDS: &[&str] = &[
    START_COMMAND,
    PROGRESS_COMMAND,
    RESULT_COMMAND,
    FINDINGS_LIST_COMMAND,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    pub target_ref: Option<String>,
    pub summary: Option<BoundedSummary>,
}

pub type Command = WorkbenchCommand<Request>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewStartRequest {
    pub subject_ref: String,
    pub changed_paths: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rationale: Option<BoundedSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRefRequest {
    pub review_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLocation {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub finding_ref: String,
    pub severity: ReviewSeverity,
    pub location: Option<ReviewLocation>,
    pub title: String,
    pub rationale: BoundedSummary,
    pub evidence_refs: Vec<String>,
    pub trace_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewRunStatus {
    Started,
    Running,
    Complete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRun {
    pub review_ref: String,
    pub subject_ref: String,
    pub status: ReviewRunStatus,
    pub progress_percent: u8,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewResult {
    pub run: ReviewRun,
    pub findings: Vec<ReviewFinding>,
    pub artifact_ref: Option<super::ArtifactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewServiceResponse {
    Started(ReviewRun),
    Progress(ReviewRun),
    Result(ReviewResult),
    Findings(Vec<ReviewFinding>),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.review.events.v1",
        "service.review.audit.v1",
        "service.review.snapshot.v1",
    )
}
