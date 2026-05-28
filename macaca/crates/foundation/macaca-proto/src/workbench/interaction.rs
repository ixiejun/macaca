//! Typed contracts for `service.interaction`.
//!
//! The interaction service owns generic Thread, Turn, and Item ledger state.
//! It does not know whether an application is a coding tool, a document
//! workflow, a support assistant, or any other product.  Every mutation travels
//! through a trace-scoped command envelope so Runtime Host can apply policy,
//! append sanitized EventLog/audit evidence, and rebuild state from a replayable
//! store instead of presentation-shell memory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactRef, BoundedSummary, MacacaResult, ServiceCommand, ServiceCommandName,
    WorkbenchCommand, WorkbenchCommandResult, WorkbenchCommandScope, WorkbenchServiceDescriptor,
};

pub const SERVICE_ID: &str = "service.interaction";
pub const CAPABILITY_ID: &str = "capability.interaction_ledger";

pub const THREAD_START_COMMAND: &str = "interaction.thread.start";
pub const THREAD_RESUME_COMMAND: &str = "interaction.thread.resume";
pub const THREAD_FORK_COMMAND: &str = "interaction.thread.fork";
pub const THREAD_ARCHIVE_COMMAND: &str = "interaction.thread.archive";
pub const THREAD_UNARCHIVE_COMMAND: &str = "interaction.thread.unarchive";
pub const THREAD_ROLLBACK_COMMAND: &str = "interaction.thread.rollback";
pub const THREAD_LIST_COMMAND: &str = "interaction.thread.list";
pub const THREAD_READ_COMMAND: &str = "interaction.thread.read";
pub const THREAD_LOADED_LIST_COMMAND: &str = "interaction.thread.loaded.list";
pub const TURN_START_COMMAND: &str = "interaction.turn.start";
pub const TURN_INTERRUPT_COMMAND: &str = "interaction.turn.interrupt";
pub const TURN_STEER_COMMAND: &str = "interaction.turn.steer";
pub const TURN_COMPLETE_COMMAND: &str = "interaction.turn.complete";
pub const TURN_FAIL_COMMAND: &str = "interaction.turn.fail";
pub const TURN_LIST_COMMAND: &str = "interaction.turn.list";
pub const ITEM_APPEND_COMMAND: &str = "interaction.item.append";
pub const ITEM_COMPLETE_COMMAND: &str = "interaction.item.complete";
pub const ITEM_FAIL_COMMAND: &str = "interaction.item.fail";
pub const ITEM_LIST_COMMAND: &str = "interaction.item.list";
pub const ITEM_WATCH_COMMAND: &str = "interaction.item.watch";
pub const ITEM_SUBSCRIBE_COMMAND: &str = "interaction.item.subscribe";
pub const SNAPSHOT_COMMAND: &str = "interaction.snapshot";

pub const COMMANDS: &[&str] = &[
    THREAD_START_COMMAND,
    THREAD_RESUME_COMMAND,
    THREAD_FORK_COMMAND,
    THREAD_ARCHIVE_COMMAND,
    THREAD_UNARCHIVE_COMMAND,
    THREAD_ROLLBACK_COMMAND,
    THREAD_LIST_COMMAND,
    THREAD_READ_COMMAND,
    THREAD_LOADED_LIST_COMMAND,
    TURN_START_COMMAND,
    TURN_INTERRUPT_COMMAND,
    TURN_STEER_COMMAND,
    TURN_COMPLETE_COMMAND,
    TURN_FAIL_COMMAND,
    TURN_LIST_COMMAND,
    ITEM_APPEND_COMMAND,
    ITEM_COMPLETE_COMMAND,
    ITEM_FAIL_COMMAND,
    ITEM_LIST_COMMAND,
    ITEM_WATCH_COMMAND,
    ITEM_SUBSCRIBE_COMMAND,
    SNAPSHOT_COMMAND,
];

pub type InteractionCommand<T> = WorkbenchCommand<T>;
pub type InteractionCommandResult = WorkbenchCommandResult<InteractionResponse>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionThreadStatus {
    Active,
    Archived,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionTurnStatus {
    Active,
    Interrupted,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionItemStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionItemKind {
    UserInput,
    AgentOutput,
    ReasoningSummary,
    ToolCall,
    ToolResult,
    ShellOutput,
    FileEdit,
    Approval,
    Review,
    Diagnostics,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionThreadRecord {
    pub thread_id: String,
    pub scope: WorkbenchCommandScope,
    pub status: InteractionThreadStatus,
    pub title: Option<String>,
    pub source_thread_id: Option<String>,
    pub rollback_boundary_item_id: Option<String>,
    pub event_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionTurnRecord {
    pub turn_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub status: InteractionTurnStatus,
    pub reason: Option<String>,
    pub event_refs: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionItemRecord {
    pub item_id: String,
    pub session_id: String,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub kind: InteractionItemKind,
    pub status: InteractionItemStatus,
    pub summary: BoundedSummary,
    pub artifact_ref: Option<ArtifactRef>,
    pub event_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadStartRequest {
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRefRequest {
    pub thread_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadForkRequest {
    pub source_thread_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRollbackRequest {
    pub thread_id: String,
    pub boundary_item_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadListRequest {
    pub include_archived: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStartRequest {
    pub thread_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRefRequest {
    pub thread_id: String,
    pub turn_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnListRequest {
    pub thread_id: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemAppendRequest {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub kind: InteractionItemKind,
    pub summary: BoundedSummary,
    pub sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRefRequest {
    pub thread_id: String,
    pub item_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemListRequest {
    pub thread_id: String,
    pub since_index: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionSnapshotRequest {
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionResponse {
    Thread(InteractionThreadRecord),
    Threads(Vec<InteractionThreadRecord>),
    Turn(InteractionTurnRecord),
    Turns(Vec<InteractionTurnRecord>),
    Item(InteractionItemRecord),
    Items(Vec<InteractionItemRecord>),
    Watch {
        watch_id: String,
        latest_index: usize,
    },
    Snapshot(InteractionServiceSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionServiceSnapshot {
    pub threads: usize,
    pub turns: usize,
    pub items: usize,
    pub loaded_threads: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.interaction.events.v1",
        "service.interaction.audit.v1",
        "service.interaction.snapshot.v1",
    )
}

pub fn service_command<T: Serialize>(
    command_name: &str,
    command: InteractionCommand<T>,
) -> MacacaResult<ServiceCommand> {
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(command_name),
        serde_json::to_value(&command)?,
        command.trace,
    ))
}
