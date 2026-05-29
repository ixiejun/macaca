//! Runtime-host provider for `service.interaction`.
//!
//! This provider is the host-owned Adapter between generic `ServiceCommand`
//! envelopes and the typed Thread/Turn/Item ledger contract.  It applies the
//! State pattern for lifecycle transitions, Memento through persisted records,
//! Observer through EventLog/broadcast notifications, and Strategy for the
//! replayable store.  It deliberately contains no application-specific
//! workflow logic.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::workbench::interaction::*;
use macaca_proto::{
    ArtifactRef, BoundedSummary, CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, TraceContext,
    WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

use crate::interaction_ledger_store::InteractionLedgerStore;

const INLINE_ITEM_BUDGET_BYTES: u64 = 8 * 1024;

/// Lifecycle notification emitted after persisted state has changed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractionLedgerEvent {
    pub event_type: String,
    pub session_id: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub item_id: Option<String>,
    pub trace_id: String,
}

/// ServiceRuntime provider for durable interaction-ledger commands.
pub struct InteractionSystemServiceProvider {
    descriptor: ServiceDescriptor,
    store: Option<Arc<dyn InteractionLedgerStore>>,
    event_log: Option<Arc<EventLog>>,
    notify_tx: broadcast::Sender<InteractionLedgerEvent>,
}

impl InteractionSystemServiceProvider {
    /// Create a provider over a replayable store Strategy.
    pub fn new(store: Arc<dyn InteractionLedgerStore>) -> Self {
        Self::with_event_log(store, None)
    }

    /// Create a provider that also mirrors sanitized lifecycle evidence into
    /// the existing session EventLog compatibility path.
    pub fn with_event_log(
        store: Arc<dyn InteractionLedgerStore>,
        event_log: Option<Arc<EventLog>>,
    ) -> Self {
        let (notify_tx, _) = broadcast::channel(1024);
        Self {
            descriptor: descriptor().base,
            store: Some(store),
            event_log,
            notify_tx,
        }
    }

    /// Create a Null Object provider for hosts without ledger persistence.
    pub fn unavailable() -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            descriptor: descriptor().base,
            store: None,
            event_log: None,
            notify_tx,
        }
    }

    /// Subscribe to persisted interaction lifecycle notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<InteractionLedgerEvent> {
        self.notify_tx.subscribe()
    }

    pub(crate) fn store(&self) -> ServiceResult<Arc<dyn InteractionLedgerStore>> {
        self.store.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("interaction ledger store is not configured".into())
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn result(
        output: InteractionResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        audit_ref: Option<String>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(output),
            summary: None,
            audit_ref,
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    pub(crate) async fn emit(
        &self,
        event_type: &str,
        trace: &TraceContext,
        session_id: &str,
        thread_id: Option<&str>,
        turn_id: Option<&str>,
        item_id: Option<&str>,
    ) -> String {
        let event = InteractionLedgerEvent {
            event_type: event_type.into(),
            session_id: session_id.into(),
            thread_id: thread_id.map(str::to_owned),
            turn_id: turn_id.map(str::to_owned),
            item_id: item_id.map(str::to_owned),
            trace_id: trace.trace_id.clone(),
        };
        let payload = serde_json::to_value(&event).unwrap_or_else(|_| serde_json::json!({}));
        let event_ref = if let Some(event_log) = &self.event_log {
            let seq = event_log
                .append_command(AppendEventCommand::new(
                    session_id, event_type, SERVICE_ID, payload,
                ))
                .await;
            format!("eventlog:{session_id}:{seq}")
        } else {
            format!("interaction:{event_type}:{}", trace.trace_id)
        };
        let _ = self.notify_tx.send(event);
        event_ref
    }

    pub(crate) fn artifact_backed_summary(
        mut summary: BoundedSummary,
        sensitive: bool,
    ) -> BoundedSummary {
        let too_large = summary.original_byte_len.unwrap_or(0) > INLINE_ITEM_BUDGET_BYTES;
        if (sensitive || too_large) && summary.artifact_ref.is_none() {
            summary.artifact_ref = Some(ArtifactRef {
                id: format!("interaction-artifact-{}", Uuid::new_v4()),
                media_type: "application/octet-stream".into(),
                sha256: None,
                byte_len: summary.original_byte_len.unwrap_or(0),
                redaction_profile: "interaction-default".into(),
            });
        }
        if sensitive || too_large {
            summary.text = "[artifact-backed interaction item]".into();
            summary.truncated = true;
        }
        summary
    }

    pub(crate) async fn ensure_thread(
        store: &Arc<dyn InteractionLedgerStore>,
        session_id: &str,
        thread_id: &str,
    ) -> ServiceResult<InteractionThreadRecord> {
        store
            .get_thread(session_id, thread_id)
            .await
            .map_err(persist_error)?
            .ok_or_else(|| ServiceError::InvalidArgument(format!("unknown thread '{thread_id}'")))
    }
}

#[async_trait]
impl SystemService for InteractionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.store.is_some(),
            "interaction service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "interaction service command accepted"
        );
        match command.name.as_str() {
            THREAD_START_COMMAND => self.thread_start(command.payload).await,
            THREAD_RESUME_COMMAND | THREAD_READ_COMMAND => self.thread_read(command.payload).await,
            THREAD_FORK_COMMAND => self.thread_fork(command.payload).await,
            THREAD_ARCHIVE_COMMAND => self.thread_status(command.payload, true).await,
            THREAD_UNARCHIVE_COMMAND => self.thread_status(command.payload, false).await,
            THREAD_ROLLBACK_COMMAND => self.thread_rollback(command.payload).await,
            THREAD_LIST_COMMAND => self.thread_list(command.payload).await,
            THREAD_LOADED_LIST_COMMAND => self.loaded_threads(command.payload).await,
            TURN_START_COMMAND => self.turn_start(command.payload).await,
            TURN_INTERRUPT_COMMAND => {
                self.turn_status(command.payload, InteractionTurnStatus::Interrupted)
                    .await
            }
            TURN_COMPLETE_COMMAND => {
                self.turn_status(command.payload, InteractionTurnStatus::Completed)
                    .await
            }
            TURN_FAIL_COMMAND => {
                self.turn_status(command.payload, InteractionTurnStatus::Failed)
                    .await
            }
            TURN_STEER_COMMAND => self.turn_steer(command.payload).await,
            TURN_LIST_COMMAND => self.turn_list(command.payload).await,
            ITEM_APPEND_COMMAND => self.item_append(command.payload).await,
            ITEM_COMPLETE_COMMAND => {
                self.item_status(command.payload, InteractionItemStatus::Completed)
                    .await
            }
            ITEM_FAIL_COMMAND => {
                self.item_status(command.payload, InteractionItemStatus::Failed)
                    .await
            }
            ITEM_LIST_COMMAND => self.item_list(command.payload).await,
            ITEM_WATCH_COMMAND | ITEM_SUBSCRIBE_COMMAND => self.item_watch(command.payload).await,
            SNAPSHOT_COMMAND => self.snapshot(command.payload).await,
            // The shell workbench performs a generic `service.snapshot` sweep
            // across every focused workbench client.  The typed
            // `interaction.snapshot` command remains the ledger contract, and
            // this alias adapts the generic workbench envelope into a bounded
            // ledger memento without changing interaction semantics.
            "service.snapshot" => self.snapshot_default(command.payload).await,
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "interaction service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "interaction service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.store.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "interaction ledger store is not configured".into(),
            })
        }
    }
}

pub(crate) fn decode<T>(value: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

pub(crate) fn persist_error(error: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

/// Build a trace-scoped interaction service command for tests and adapters.
pub fn interaction_service_command<T: serde::Serialize>(
    command_name: &str,
    command: InteractionCommand<T>,
) -> ServiceResult<ServiceCommand> {
    service_command(command_name, command)
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}
