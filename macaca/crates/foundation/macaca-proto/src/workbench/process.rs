//! Provider-neutral contracts for `service.process`.
//!
//! The process service owns command execution, background process handles,
//! PTY-like interactive sessions, output deltas, and cleanup records for many
//! application classes.  Public DTOs avoid raw OS handles and raw environment
//! dumps.  Runtime providers must resolve sandbox and permission profiles
//! before spawn, then emit bounded output and sanitized audit references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ArtifactRef, BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.process";
pub const CAPABILITY_ID: &str = "capability.process";
pub const EXEC_COMMAND: &str = "process.exec";
pub const SPAWN_COMMAND: &str = "process.spawn";
pub const STDIN_WRITE_COMMAND: &str = "process.stdin.write";
pub const PTY_RESIZE_COMMAND: &str = "process.pty.resize";
pub const TERMINATE_COMMAND: &str = "process.terminate";
pub const OUTPUT_SUBSCRIBE_COMMAND: &str = "process.output.subscribe";
pub const STATUS_COMMAND: &str = "process.status";
pub const CLEANUP_COMMAND: &str = "process.cleanup";
pub const TOOL_INVOKE_COMMAND: &str = "process.tool.invoke";

pub const COMMANDS: &[&str] = &[
    EXEC_COMMAND,
    SPAWN_COMMAND,
    STDIN_WRITE_COMMAND,
    PTY_RESIZE_COMMAND,
    TERMINATE_COMMAND,
    OUTPUT_SUBSCRIBE_COMMAND,
    STATUS_COMMAND,
    CLEANUP_COMMAND,
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

/// Workspace and policy information required before a process can start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSandboxRequest {
    pub workspace_root: String,
    pub cwd: Option<String>,
    pub sandbox_profile_ref: String,
    pub permission_profile_ref: String,
    pub approval_ref: Option<String>,
    pub resource_lease_refs: Vec<String>,
}

/// Command request shared by bounded exec and long-running spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessCommandRequest {
    pub executable: String,
    pub args: Vec<String>,
    pub env: Vec<ProcessEnvVar>,
    pub sandbox: ProcessSandboxRequest,
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub thread_ref: Option<String>,
    pub inline_output_budget_bytes: u64,
    pub timeout_ms: Option<u64>,
    pub allocate_pty: bool,
}

/// Environment entries are explicit data so providers can redact by key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEnvVar {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessHandleRef {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStdinWriteRequest {
    pub handle: ProcessHandleRef,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessPtyResizeRequest {
    pub handle: ProcessHandleRef,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessTerminateRequest {
    pub handle: ProcessHandleRef,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessOutputSubscribeRequest {
    pub handle: ProcessHandleRef,
    pub since_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessStatusRequest {
    pub handle: ProcessHandleRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessCleanupRequest {
    pub application_id: Option<String>,
    pub session_id: Option<String>,
    pub thread_ref: Option<String>,
    pub reason: String,
}

/// Bounded process output delta.  Binary bytes are base64 encoded by providers
/// and may be replaced with an artifact reference when the inline budget is
/// exceeded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessOutputDelta {
    pub sequence: u64,
    pub stream: ProcessOutputStream,
    pub data: Option<String>,
    pub base64: bool,
    pub byte_len: u64,
    pub truncated: bool,
    pub artifact_ref: Option<ArtifactRef>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessOutputStream {
    Stdout,
    Stderr,
    PseudoPty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessLifecycleState {
    Spawned,
    Running,
    Exited,
    Terminated,
    Failed,
}

/// Replayable process record.  It stores hashes and summaries, never raw OS
/// handles, unbounded output, secrets, or shell-owned state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub handle: ProcessHandleRef,
    pub command_hash: String,
    pub executable_summary: String,
    pub cwd_scope: String,
    pub sandbox_ref: String,
    pub permission_profile_ref: String,
    pub resource_lease_refs: Vec<String>,
    pub state: ProcessLifecycleState,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub duration_ms: Option<u64>,
    pub stdout_byte_count: u64,
    pub stderr_byte_count: u64,
    pub pty_rows: Option<u16>,
    pub pty_cols: Option<u16>,
    pub audit_refs: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessServiceResponse {
    Exec {
        record: ProcessRecord,
        output: Vec<ProcessOutputDelta>,
    },
    Spawned(ProcessRecord),
    StdinWritten {
        handle: ProcessHandleRef,
        byte_len: u64,
    },
    PtyResized {
        handle: ProcessHandleRef,
        rows: u16,
        cols: u16,
    },
    Terminated(ProcessRecord),
    OutputSubscribed {
        handle: ProcessHandleRef,
        deltas: Vec<ProcessOutputDelta>,
    },
    Status(ProcessRecord),
    Cleanup {
        terminated_handles: Vec<ProcessHandleRef>,
        retained_handles: Vec<ProcessHandleRef>,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.process.events.v1",
        "service.process.audit.v1",
        "service.process.snapshot.v1",
    )
}
