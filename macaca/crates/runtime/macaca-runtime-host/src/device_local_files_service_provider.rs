//! Runtime-host Strategy for provider-neutral local-file commands.
//!
//! The mock returns opaque references only. It never reads paths, file bytes,
//! directory names, credentials, or host filesystem metadata.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::device_local_files::{
    DEVICE_LOCAL_FILES_COMMANDS, DEVICE_LOCAL_FILES_PACK_ID, DEVICE_LOCAL_FILES_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFilesRuntimeEvent {
    pub command: String,
    /// Stable provider-neutral audit name; payloads and host paths are omitted.
    pub event_name: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Provider-neutral grant states used by picker/handle Strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFileGrantState {
    Requested,
    Granted,
    Active,
    Revoked,
    Expired,
    Failed,
    Unavailable,
}

/// Provider-neutral transfer states used by import/export Strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFileTransferState {
    Requested,
    Active,
    Completed,
    Cancelled,
    Failed,
    Unavailable,
}

/// State-pattern transition for grants; invalid transitions fail closed.
pub fn transition_local_file_grant(
    state: LocalFileGrantState,
    operation: &str,
) -> Option<LocalFileGrantState> {
    match (state, operation) {
        (LocalFileGrantState::Requested, "grant") => Some(LocalFileGrantState::Granted),
        (LocalFileGrantState::Granted, "activate") => Some(LocalFileGrantState::Active),
        (LocalFileGrantState::Active, "revoke") => Some(LocalFileGrantState::Revoked),
        (LocalFileGrantState::Active, "expire") => Some(LocalFileGrantState::Expired),
        (_, "fail") => Some(LocalFileGrantState::Failed),
        (_, "unavailable") => Some(LocalFileGrantState::Unavailable),
        _ => None,
    }
}

/// State-pattern transition for transfers; invalid transitions fail closed.
pub fn transition_local_file_transfer(
    state: LocalFileTransferState,
    operation: &str,
) -> Option<LocalFileTransferState> {
    match (state, operation) {
        (LocalFileTransferState::Requested, "start") => Some(LocalFileTransferState::Active),
        (LocalFileTransferState::Active, "complete") => Some(LocalFileTransferState::Completed),
        (LocalFileTransferState::Active, "cancel") => Some(LocalFileTransferState::Cancelled),
        (_, "fail") => Some(LocalFileTransferState::Failed),
        (_, "unavailable") => Some(LocalFileTransferState::Unavailable),
        _ => None,
    }
}

#[derive(Default)]
struct LocalFilesLedger {
    handles: RwLock<usize>,
    transfers: RwLock<usize>,
}

impl LocalFilesLedger {
    async fn record(&self, operation: &str) {
        match operation {
            "local_files.request_open_handle"
            | "local_files.request_save_handle"
            | "local_files.request_directory_handle" => *self.handles.write().await += 1,
            "local_files.revoke_handle" => *self.handles.write().await = 0,
            "local_files.import_file" | "local_files.export_file" => {
                *self.transfers.write().await += 1
            }
            "local_files.cancel_transfer" => *self.transfers.write().await = 0,
            _ => {}
        }
    }
    async fn clear(&self) {
        *self.handles.write().await = 0;
        *self.transfers.write().await = 0;
    }
    async fn counts(&self) -> (usize, usize) {
        (*self.handles.read().await, *self.transfers.read().await)
    }
}

/// Mock/Null Object provider selected only by runtime-host composition.
pub struct DeviceLocalFilesSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: tokio::sync::broadcast::Sender<LocalFilesRuntimeEvent>,
    ledger: Arc<LocalFilesLedger>,
}

impl DeviceLocalFilesSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            unavailable_reason,
            events,
            ledger: Arc::new(LocalFilesLedger::default()),
        }
    }
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LocalFilesRuntimeEvent> {
        self.events.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let (handles, transfers) = self.ledger.counts().await;
        let snapshot = BTreeMap::from([
            (
                "provider_class".into(),
                if self.unavailable_reason.is_some() {
                    "unavailable"
                } else {
                    "mock"
                }
                .into(),
            ),
            ("active_handle_count".into(), handles.to_string()),
            ("active_transfer_count".into(), transfers.to_string()),
            (
                "command_count".into(),
                DEVICE_LOCAL_FILES_COMMANDS.len().to_string(),
            ),
            (
                "snapshot_schema".into(),
                "device.local_files.replay.v1".into(),
            ),
        ]);
        self.emit(
            "local_files.snapshot_recorded",
            "snapshot:provider",
            "snapshot_recorded",
        );
        snapshot
    }
    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(LocalFilesRuntimeEvent {
            command: command.into(),
            event_name: command.into(),
            trace_id: trace_id.into(),
            replay_ref: format!("replay:local-files:{trace_id}"),
            outcome,
        });
    }
}

#[async_trait]
impl SystemService for DeviceLocalFilesSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(DEVICE_LOCAL_FILES_SERVICE_ID),
            ServiceType::new("device.local_files"),
            TraceSchemaRef::new("device.local_files.replay.v1"),
        );
        descriptor
            .metadata
            .insert("pack_id".into(), DEVICE_LOCAL_FILES_PACK_ID.into());
        descriptor.metadata.insert(
            "provider_class".into(),
            if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
        );
        descriptor
    }
    async fn start(&self) -> ServiceResult<()> {
        self.emit(
            "local_files.pack_declared",
            "lifecycle:local-files",
            "pack_declared",
        );
        info!(
            service_id = DEVICE_LOCAL_FILES_SERVICE_ID,
            "local files provider started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !DEVICE_LOCAL_FILES_COMMANDS.contains(&operation) {
            self.emit("local_files.command_failed", &trace.trace_id, "unsupported");
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit("local_files.unavailable", &trace.trace_id, "unavailable");
            warn!(service_id = DEVICE_LOCAL_FILES_SERVICE_ID, command = operation, reason_code = %reason, "local files provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        for event_name in [
            "local_files.admission_validated",
            "local_files.policy_decision",
            "local_files.entitlement_checked",
            "local_files.resource_reserved",
        ] {
            self.emit(event_name, &trace.trace_id, "validated");
        }
        self.ledger.record(operation).await;
        let success_event = local_files_success_event(operation);
        self.emit(success_event, &trace.trace_id, "started");
        if matches!(
            operation,
            "local_files.request_open_handle"
                | "local_files.request_save_handle"
                | "local_files.request_directory_handle"
        ) {
            self.emit("local_files.handle_granted", &trace.trace_id, "granted");
        }
        if matches!(
            operation,
            "local_files.read"
                | "local_files.write"
                | "local_files.append"
                | "local_files.truncate"
                | "local_files.import_file"
                | "local_files.export_file"
        ) {
            self.emit(
                "local_files.transfer_progressed",
                &trace.trace_id,
                "progressed",
            );
            self.emit(
                "local_files.transfer_completed",
                &trace.trace_id,
                "completed",
            );
        }
        info!(service_id = DEVICE_LOCAL_FILES_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "local files command completed with opaque reference");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"reference_only","operation":operation,"handle_ref":format!("local-file-reference:{}", trace.trace_id)}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.ledger.clear().await;
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.ledger.clear().await;
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self
            .unavailable_reason
            .as_ref()
            .map_or(ServiceHealth::Healthy, |reason| {
                ServiceHealth::Unavailable {
                    reason: reason.clone(),
                }
            }))
    }
}

fn local_files_success_event(operation: &str) -> &'static str {
    match operation {
        "local_files.request_open_handle" => "local_files.picker_requested",
        "local_files.request_save_handle" => "local_files.picker_requested",
        "local_files.request_directory_handle" => "local_files.picker_requested",
        "local_files.inspect_handle" => "local_files.handle_inspected",
        "local_files.list_handles" => "local_files.handles_listed",
        "local_files.revoke_handle" => "local_files.handle_revoked",
        "local_files.read" => "local_files.transfer_started",
        "local_files.write" | "local_files.append" | "local_files.truncate" => {
            "local_files.transfer_started"
        }
        "local_files.list_directory" => "local_files.directory_listed",
        "local_files.import_file" | "local_files.export_file" => "local_files.transfer_started",
        "local_files.cancel_transfer" => "local_files.transfer_cancelled",
        "local_files.inspect_host" => "local_files.host_inspected",
        _ => "local_files.command_completed",
    }
}
