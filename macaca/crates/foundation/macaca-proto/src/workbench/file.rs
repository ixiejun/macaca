//! Provider-neutral contracts for `service.file`.
//!
//! The filesystem service owns workspace-scoped file capabilities for many
//! application classes, not only coding workbenches.  Public commands are
//! trace-scoped and carry explicit workspace roots so runtime-host providers
//! can apply path policy, write policy, mementos, bounded payload handling, and
//! audit before touching the host filesystem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.file";
pub const CAPABILITY_ID: &str = "capability.file";
pub const READ_COMMAND: &str = "file.read";
pub const WRITE_COMMAND: &str = "file.write";
pub const PATCH_COMMAND: &str = "file.patch";
pub const COPY_COMMAND: &str = "file.copy";
pub const REMOVE_COMMAND: &str = "file.remove";
pub const METADATA_COMMAND: &str = "file.metadata";
pub const DIRECTORY_LIST_COMMAND: &str = "file.directory.list";
pub const WATCH_COMMAND: &str = "file.watch";
pub const UNWATCH_COMMAND: &str = "file.unwatch";
pub const DIFF_COMMAND: &str = "file.diff";
pub const TOOL_INVOKE_COMMAND: &str = "file.tool.invoke";

pub const COMMANDS: &[&str] = &[
    READ_COMMAND,
    WRITE_COMMAND,
    PATCH_COMMAND,
    COPY_COMMAND,
    REMOVE_COMMAND,
    METADATA_COMMAND,
    DIRECTORY_LIST_COMMAND,
    WATCH_COMMAND,
    UNWATCH_COMMAND,
    DIFF_COMMAND,
    TOOL_INVOKE_COMMAND,
];

/// Compatibility request used by early generic SDK tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    pub target_ref: Option<String>,
    pub summary: Option<BoundedSummary>,
}

pub type Command = WorkbenchCommand<Request>;

/// Workspace-local path request shared by read, metadata, list, and remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePathRequest {
    pub workspace_root: String,
    pub path: String,
    pub follow_symlinks: bool,
}

/// Read options that bound inline file contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReadRequest {
    pub target: FilePathRequest,
    pub inline_budget_bytes: u64,
}

/// Write request with explicit approval state and pre-write snapshot intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWriteRequest {
    pub target: FilePathRequest,
    pub content: String,
    pub create_parent_directories: bool,
    pub require_approval: bool,
    pub approval_ref: Option<String>,
}

/// Minimal structured patch operation for provider-owned patch application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePatchRequest {
    pub target: FilePathRequest,
    pub search: String,
    pub replace: String,
    pub require_approval: bool,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCopyRequest {
    pub source: FilePathRequest,
    pub destination_path: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWatchRequest {
    pub target: FilePathRequest,
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUnwatchRequest {
    pub watch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiffRequest {
    pub target: FilePathRequest,
    pub proposed_content: String,
}

/// Metadata excludes raw file content and host-only handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub byte_len: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub readonly: bool,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub metadata: FileMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMementoRef {
    pub id: String,
    pub path: String,
    pub content_hash: Option<String>,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWatchRecord {
    pub watch_id: String,
    pub path: String,
    pub recursive: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChangeNotification {
    pub watch_id: String,
    pub path: String,
    pub change_kind: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiffSummary {
    pub changed: bool,
    pub before_hash: Option<String>,
    pub after_hash: String,
    pub line_delta: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileServiceResponse {
    Read {
        path: String,
        content: Option<String>,
        binary: bool,
        metadata: FileMetadata,
    },
    Written {
        path: String,
        metadata: FileMetadata,
        memento: Option<FileMementoRef>,
    },
    Patched {
        path: String,
        metadata: FileMetadata,
        memento: FileMementoRef,
        replaced_count: u64,
    },
    Copied {
        source_path: String,
        destination_path: String,
        metadata: FileMetadata,
    },
    Removed {
        path: String,
        memento: Option<FileMementoRef>,
    },
    Metadata(FileMetadata),
    DirectoryList {
        path: String,
        entries: Vec<FileEntry>,
        truncated: bool,
    },
    Watch(FileWatchRecord),
    Unwatch {
        watch_id: String,
        removed: bool,
    },
    Diff(FileDiffSummary),
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.file.events.v1",
        "service.file.audit.v1",
        "service.file.snapshot.v1",
    )
}
