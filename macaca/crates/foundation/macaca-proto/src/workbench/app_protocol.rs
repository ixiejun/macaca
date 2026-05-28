//! Typed contracts for `service.app_protocol`.
//!
//! The app protocol gateway is a shell/gateway Adapter over focused service
//! clients.  It owns transport connection state, subscription state,
//! backpressure diagnostics, and protocol framing.  It deliberately does not
//! own Thread/Turn/Item, filesystem, process, approval, tool, plugin, MCP,
//! skill, Git, review, or diagnostics semantics; protocol requests are either
//! initialization/subscription operations or explicitly forwarded to the owning
//! service boundary with trace and bounded payload metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BoundedSummary, MacacaResult, ServiceCommand, ServiceCommandName, WorkbenchCommand,
    WorkbenchCommandResult, WorkbenchCommandScope, WorkbenchServiceDescriptor,
};

pub const SERVICE_ID: &str = "service.app_protocol";
pub const CAPABILITY_ID: &str = "capability.app_protocol";

pub const INITIALIZE_COMMAND: &str = "app_protocol.connection.initialize";
pub const SUBSCRIPTION_CREATE_COMMAND: &str = "app_protocol.subscription.create";
pub const SUBSCRIPTION_CLOSE_COMMAND: &str = "app_protocol.subscription.close";
pub const NOTIFICATION_EMIT_COMMAND: &str = "app_protocol.notification.emit";
pub const HEALTH_READ_COMMAND: &str = "app_protocol.health.read";
pub const JSON_RPC_RECEIVE_COMMAND: &str = "app_protocol.json_rpc.receive";
pub const SNAPSHOT_COMMAND: &str = "app_protocol.snapshot";

pub const COMMANDS: &[&str] = &[
    INITIALIZE_COMMAND,
    SUBSCRIPTION_CREATE_COMMAND,
    SUBSCRIPTION_CLOSE_COMMAND,
    NOTIFICATION_EMIT_COMMAND,
    HEALTH_READ_COMMAND,
    JSON_RPC_RECEIVE_COMMAND,
    SNAPSHOT_COMMAND,
];

pub type AppProtocolCommand<T> = WorkbenchCommand<T>;
pub type AppProtocolCommandResult = WorkbenchCommandResult<AppProtocolResponse>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppProtocolTransportKind {
    WebSocket,
    Stdio,
    UnixSocket,
    HttpSse,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppProtocolConnectionStatus {
    Initializing,
    Initialized,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppProtocolSubscriptionStatus {
    Active,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppProtocolRetryableReason {
    IngressQueueSaturated,
    OutboundQueueSaturated,
    ConnectionNotInitialized,
    DuplicateInitialization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolClientMetadata {
    pub client_name: String,
    pub client_version: Option<String>,
    pub transport: AppProtocolTransportKind,
    pub wants_notifications: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolInitializeRequest {
    pub connection_id: Option<String>,
    pub protocol_version: String,
    pub client: AppProtocolClientMetadata,
    pub requested_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolConnectionRecord {
    pub connection_id: String,
    pub scope: WorkbenchCommandScope,
    pub protocol_version: String,
    pub client: AppProtocolClientMetadata,
    pub negotiated_capabilities: Vec<String>,
    pub status: AppProtocolConnectionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolSubscriptionCreateRequest {
    pub connection_id: String,
    pub target: AppProtocolSubscriptionTarget,
    pub event_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolSubscriptionCloseRequest {
    pub connection_id: String,
    pub subscription_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppProtocolSubscriptionTarget {
    Application { application_id: String },
    Thread { thread_id: String },
    Service { service_id: String },
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolSubscriptionRecord {
    pub subscription_id: String,
    pub connection_id: String,
    pub target: AppProtocolSubscriptionTarget,
    pub event_kinds: Vec<String>,
    pub status: AppProtocolSubscriptionStatus,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolEventEnvelope {
    pub event_kind: String,
    pub source_service_id: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub item_id: Option<String>,
    pub trace_id: String,
    pub audit_ref: Option<String>,
    pub summary: Option<BoundedSummary>,
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolNotification {
    pub notification_id: String,
    pub subscription_id: String,
    pub connection_id: String,
    pub event: AppProtocolEventEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolNotificationEmitRequest {
    pub event: AppProtocolEventEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolHealthReadRequest {
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolJsonRpcReceiveRequest {
    pub connection_id: String,
    pub frame: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppProtocolJsonRpcFrame {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppProtocolJsonRpcResponseFrame {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<AppProtocolJsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolJsonRpcError {
    pub code: i64,
    pub message: String,
    pub retryable: bool,
    pub reason: Option<AppProtocolRetryableReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolSnapshotRequest {
    pub include_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProtocolServiceSnapshot {
    pub active_connections: usize,
    pub active_subscriptions: usize,
    pub closed_connections: usize,
    pub closed_subscriptions: usize,
    pub ingress_queue_limit: usize,
    pub outbound_queue_limit: usize,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppProtocolResponse {
    Initialized(AppProtocolConnectionRecord),
    Subscription(AppProtocolSubscriptionRecord),
    SubscriptionClosed(AppProtocolSubscriptionRecord),
    Notifications(Vec<AppProtocolNotification>),
    JsonRpc(AppProtocolJsonRpcResponseFrame),
    TransportUnavailable {
        transport: AppProtocolTransportKind,
        reason: String,
    },
    Forwarded {
        service_id: String,
        command_name: String,
        payload: Value,
    },
    Health(AppProtocolServiceSnapshot),
    Snapshot(AppProtocolServiceSnapshot),
    Overloaded {
        reason: AppProtocolRetryableReason,
        retry_after_ms: u64,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.app_protocol.events.v1",
        "service.app_protocol.audit.v1",
        "service.app_protocol.snapshot.v1",
    )
}

pub fn service_command<T: Serialize>(
    command_name: &str,
    command: AppProtocolCommand<T>,
) -> MacacaResult<ServiceCommand> {
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(command_name),
        serde_json::to_value(&command)?,
        command.trace,
    ))
}
