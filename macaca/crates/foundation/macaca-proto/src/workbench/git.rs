//! Provider-neutral contracts for `service.git`.
//!
//! The service owns repository status, diff, patch application, rollback
//! markers, path policy, and patch provenance.  DTOs carry hashes and bounded
//! summaries rather than raw repository internals so callers can audit changes
//! without receiving unrestricted filesystem contents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.git";
pub const CAPABILITY_ID: &str = "capability.git";
pub const STATUS_COMMAND: &str = "git.status";
pub const DIFF_COMMAND: &str = "git.diff";
pub const APPLY_PATCH_COMMAND: &str = "git.apply_patch";
pub const ROLLBACK_MARKER_CREATE_COMMAND: &str = "git.rollback_marker.create";
pub const ROLLBACK_MARKER_REPLAY_COMMAND: &str = "git.rollback_marker.replay";

pub const COMMANDS: &[&str] = &[
    STATUS_COMMAND,
    DIFF_COMMAND,
    APPLY_PATCH_COMMAND,
    ROLLBACK_MARKER_CREATE_COMMAND,
    ROLLBACK_MARKER_REPLAY_COMMAND,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    pub target_ref: Option<String>,
    pub summary: Option<BoundedSummary>,
}

pub type Command = WorkbenchCommand<Request>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepositoryRequest {
    pub workspace_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiffRequest {
    pub workspace_root: String,
    pub pathspecs: Vec<String>,
    pub inline_budget_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitApplyPatchRequest {
    pub workspace_root: String,
    pub patch_text: String,
    pub require_approval: bool,
    pub approval_ref: Option<String>,
    pub actor_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRollbackMarkerReplayRequest {
    pub workspace_root: String,
    pub marker_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRollbackMarker {
    pub marker_ref: String,
    pub workspace_hash: String,
    pub inverse_patch: BoundedSummary,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitPatchProvenance {
    pub provenance_ref: String,
    pub actor_ref: String,
    pub affected_paths: Vec<String>,
    pub pre_change_marker: GitRollbackMarker,
    pub post_change_hash: String,
    pub applied_at: DateTime<Utc>,
    pub audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitServiceResponse {
    Status {
        branch: Option<String>,
        head: Option<String>,
        files: Vec<GitFileStatus>,
        clean: bool,
    },
    Diff {
        diff: BoundedSummary,
        pathspecs: Vec<String>,
    },
    PatchApplied(GitPatchProvenance),
    RollbackMarker(GitRollbackMarker),
    RollbackReplay {
        marker_ref: String,
        applied: bool,
        audit_ref: String,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.git.events.v1",
        "service.git.audit.v1",
        "service.git.snapshot.v1",
    )
}
