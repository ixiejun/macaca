//! Macaca-hosted application execution provider strategy.
//!
//! This module is the runtime-host Adapter for the `MacacaHosted` provider
//! kind.  It does not contain application-specific behavior.  The provider
//! owns the generic execution envelope, durable event emission, and control
//! routing, while concrete application logic enters through an injected
//! [`HostedApplicationExecutionAdapter`].  The default adapter is a Bridge to
//! the existing Application ABI host seam, so production composition roots can
//! connect real application runtimes without teaching the service layer about
//! WASM bytes, YAML workflows, web state, or product domains.

mod provider_impl;

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionError, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionLifecycleState,
    ApplicationExecutionPayload, ApplicationExecutionProviderDescriptor,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionScope, ApplicationExecutionSnapshot, ApplicationHostCommand,
    ApplicationHostCommandStatus, ApplicationImport, CapabilityId, ServiceError,
    StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_hosts::ApplicationHostRuntime;

const HOSTED_PROVIDER_ID: &str = "provider.macaca_hosted";
const HOSTED_PROTOCOL_VERSION: &str = "application-execution.v1";
const HOSTED_TRANSPORT_KIND: &str = "macaca_hosted";

/// Outcome returned by a hosted application adapter after start dispatch.
///
/// The enum models generic lifecycle outcomes only.  It deliberately avoids
/// domain labels, workflow names, model names, and provider-specific payloads
/// so the hosted provider can remain reusable across YAML, WASM, GenUI, and
/// headless applications.
#[derive(Debug, Clone, PartialEq)]
pub enum HostedApplicationExecutionOutcome {
    /// The application accepted the run and will continue asynchronously.
    Running {
        checkpoint_ref: Option<String>,
        summary: String,
        signals: Vec<HostedApplicationExecutionSignal>,
    },
    /// The application reached a generic approval wait point.
    WaitingForApproval {
        approval_ref: String,
        checkpoint_ref: Option<String>,
        summary: String,
    },
    /// The application completed synchronously through the hosted adapter.
    Completed { summary: String },
}

/// Provider-neutral signal produced by a hosted runtime adapter.
///
/// A signal is not a raw WASM or application payload.  It is the sanitized,
/// protocol-shaped fact that the `MacacaHosted` provider may append to the
/// durable application-execution EventLog.  Keeping this small translation
/// object inside runtime-host preserves the Adapter pattern: concrete WASM host
/// import details stay behind the ABI bridge, while the provider emits only
/// generic execution events such as tool dispatch, tool completion, and terminal
/// completion/failure evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct HostedApplicationExecutionSignal {
    pub event_type: ApplicationExecutionEventType,
    pub payload: ApplicationExecutionPayload,
    pub idempotency_suffix: String,
}

impl HostedApplicationExecutionSignal {
    /// Build one signal with a bounded payload summary and small JSON metadata.
    pub fn new(
        event_type: ApplicationExecutionEventType,
        summary: impl Into<String>,
        data: Option<serde_json::Value>,
        idempotency_suffix: impl Into<String>,
    ) -> Self {
        let mut payload = ApplicationExecutionPayload::summary(summary);
        payload.data = data;
        Self {
            event_type,
            payload,
            idempotency_suffix: idempotency_suffix.into(),
        }
    }
}

/// Adapter implemented by concrete hosted application runtimes.
///
/// Runtime-host owns this trait because it is a Strategy seam, not a protocol
/// DTO.  Implementations must route privileged work through declared
/// capabilities and `ServiceRuntime` rather than directly constructing LLM,
/// file, process, sandbox, driver, skill, MCP, or tool providers.
#[async_trait]
pub trait HostedApplicationExecutionAdapter: Send + Sync {
    /// Start one generic application execution envelope.
    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError>;

    /// Deliver a generic control command to the hosted runtime.
    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError>;

    /// Resume a hosted runtime from a checkpoint or snapshot memento.
    async fn resume(
        &self,
        snapshot: ApplicationExecutionSnapshot,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError>;
}

/// Application ABI bridge for hosted execution.
///
/// This Adapter converts `StartApplicationExecutionCommand` into a bounded
/// `ApplicationHostCommand` using the existing Application ABI host runtime.
/// Start dispatch uses the generic WASM export-invoke command so app-specific
/// execution behavior remains in the component artifact and declared host
/// imports rather than in Macaca OS service code.
pub struct ApplicationAbiHostedExecutionAdapter {
    host: Arc<dyn ApplicationHostRuntime>,
}

impl ApplicationAbiHostedExecutionAdapter {
    /// Build an ABI-backed hosted adapter.
    pub fn new(host: Arc<dyn ApplicationHostRuntime>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl HostedApplicationExecutionAdapter for ApplicationAbiHostedExecutionAdapter {
    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError> {
        // Child WASM host imports need the user-visible application-execution
        // scope, not the provider-private runtime session.  The HTTP route only
        // requires a trace id, so this adapter enriches the trace before it
        // crosses into Application Service.  That keeps scope propagation in
        // the generic hosted execution boundary and lets any WASM/YAML/GenUI
        // application reuse the same replay and audit correlation path.
        let outer_session_id = command
            .session_id
            .clone()
            .unwrap_or_else(|| command.idempotency_key.clone());
        let outer_run_id = command
            .run_id
            .clone()
            .unwrap_or_else(|| command.idempotency_key.clone());
        let mut hosted_trace = command.trace.clone();
        hosted_trace.session_id = Some(outer_session_id.clone());
        hosted_trace.task_id = Some(outer_run_id.clone());
        // Hosted execution starts are intentionally represented as a generic
        // WASM export invocation instead of a provider/application-specific
        // host import. The Application Service resolves the app-scoped WASM
        // session, invokes `app:start`, and lets the component runtime route
        // declared host commands through the existing host-import bridge. This
        // keeps application execution extensible while preserving policy,
        // trace, service routing, and audit at the infrastructure boundary.
        let mut host_command = ApplicationHostCommand::with_trace(
            ApplicationImport::Custom("macaca:wasm/invoke".into()),
            serde_json::json!({
                "application_id": command.application_id,
                "session_id": command.session_id,
                "run_id": command.run_id,
                "workspace_ref": command.workspace_ref,
                "requested_capabilities": command.requested_capabilities,
                "task_input_summary": command.task_input.summary,
                "task_input_has_payload_ref": command.task_input.payload_ref.is_some(),
                "chat": {
                    "input": command.task_input.summary,
                    "payload_ref": command.task_input.payload_ref,
                    "workspace_ref": command.workspace_ref,
                    "session_id": outer_session_id,
                    "run_id": outer_run_id,
                    "requested_capabilities": command.requested_capabilities,
                },
                "idempotency_key": command.idempotency_key,
            }),
            hosted_trace,
        );
        host_command
            .metadata
            .insert("wasm.export".into(), "app:start".into());
        host_command
            .metadata
            .insert("execution.operation".into(), "start".into());
        info!(
            application_id = host_command
                .payload
                .get("application_id")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            session_id = host_command
                .payload
                .get("session_id")
                .and_then(|value| value.as_str())
                .unwrap_or("none"),
            run_id = host_command
                .payload
                .get("run_id")
                .and_then(|value| value.as_str())
                .unwrap_or("none"),
            wasm_export = "app:start",
            "Hosted application execution start mapped to WASM export invoke"
        );
        let result = self
            .host
            .dispatch(host_command)
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        match result.status {
            ApplicationHostCommandStatus::Ok => {
                let checkpoint_ref = Some(format!(
                    "application-execution://checkpoint/{}/{}/hosted-start-accepted",
                    command
                        .session_id
                        .as_deref()
                        .unwrap_or("session-unassigned"),
                    command.run_id.as_deref().unwrap_or("run-unassigned")
                ));
                info!(
                    application_id = %command.application_id,
                    session_id = command.session_id.as_deref().unwrap_or("none"),
                    run_id = command.run_id.as_deref().unwrap_or("none"),
                    trace_id = %command.trace.trace_id,
                    wasm_export = "app:start",
                    checkpoint_ref = checkpoint_ref.as_deref().unwrap_or("none"),
                    "Hosted application execution start acknowledged by application runtime"
                );
                // A successful `app:start` invocation only proves that the
                // application boundary accepted the execution envelope. It is
                // not a terminal task result: real LLM, file, process, tool,
                // approval, and completion events must be emitted later by the
                // hosted application runtime or an authorized gateway through
                // the provider-neutral application execution protocol. Treating
                // the ACK as `Completed` would make replay/audit lie about work
                // that has not happened yet, so the adapter returns `Running`
                // with a generic checkpoint memento that the EventLog can
                // persist and replay after browser refresh or subscriber loss.
                let signals = hosted_signals_from_host_result(&result);
                Ok(HostedApplicationExecutionOutcome::Running {
                    checkpoint_ref,
                    summary: "hosted application runtime accepted start dispatch".into(),
                    signals,
                })
            }
            ApplicationHostCommandStatus::RuntimeUnavailable { reason }
            | ApplicationHostCommandStatus::Unavailable { reason } => {
                Err(ServiceError::ServiceUnavailable(reason))
            }
            ApplicationHostCommandStatus::Unsupported { reason }
            | ApplicationHostCommandStatus::Rejected { reason }
            | ApplicationHostCommandStatus::DisabledByPolicy { reason } => {
                Err(ServiceError::InvalidArgument(reason))
            }
        }
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError> {
        let host_command = ApplicationHostCommand::with_trace(
            ApplicationImport::Custom("macaca:application_execution/control".into()),
            serde_json::to_value(&command).map_err(adapter_error)?,
            command.trace,
        );
        let result = self
            .host
            .dispatch(host_command)
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        match result.status {
            ApplicationHostCommandStatus::Ok => Ok(ApplicationExecutionCommandStatus::Completed),
            ApplicationHostCommandStatus::RuntimeUnavailable { reason }
            | ApplicationHostCommandStatus::Unavailable { reason } => {
                Err(ServiceError::ServiceUnavailable(reason))
            }
            ApplicationHostCommandStatus::Unsupported { reason } => {
                Err(ServiceError::UnsupportedCommand(reason))
            }
            ApplicationHostCommandStatus::Rejected { reason }
            | ApplicationHostCommandStatus::DisabledByPolicy { reason } => {
                Err(ServiceError::InvalidArgument(reason))
            }
        }
    }

    async fn resume(
        &self,
        snapshot: ApplicationExecutionSnapshot,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError> {
        let host_command = ApplicationHostCommand::with_trace(
            ApplicationImport::Custom("macaca:application_execution/resume".into()),
            serde_json::to_value(&snapshot).map_err(adapter_error)?,
            macaca_proto::TraceContext::new("application-execution-hosted-resume"),
        );
        let result = self
            .host
            .dispatch(host_command)
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        match result.status {
            ApplicationHostCommandStatus::Ok => Ok(HostedApplicationExecutionOutcome::Running {
                checkpoint_ref: snapshot.latest_checkpoint_ref,
                summary: "hosted application runtime resumed from snapshot".into(),
                signals: hosted_signals_from_host_result(&result),
            }),
            ApplicationHostCommandStatus::RuntimeUnavailable { reason }
            | ApplicationHostCommandStatus::Unavailable { reason } => {
                Err(ServiceError::ServiceUnavailable(reason))
            }
            ApplicationHostCommandStatus::Unsupported { reason } => {
                Err(ServiceError::UnsupportedCommand(reason))
            }
            ApplicationHostCommandStatus::Rejected { reason }
            | ApplicationHostCommandStatus::DisabledByPolicy { reason } => {
                Err(ServiceError::InvalidArgument(reason))
            }
        }
    }
}

/// Runtime state for one hosted execution run.
///
/// The state is a small Memento cache for control routing and diagnostics. The
/// authoritative history remains the durable EventLog, so losing this cache can
/// be recovered by replay and provider-specific checkpoint support.
#[derive(Debug, Clone)]
struct HostedRunState {
    scope: ApplicationExecutionScope,
    lifecycle_state: ApplicationExecutionLifecycleState,
    latest_checkpoint_ref: Option<String>,
    pending_approval_ref: Option<String>,
    latest_event_cursor: Option<String>,
}

/// Macaca-hosted provider strategy.
pub struct MacacaHostedApplicationExecutionProvider {
    descriptor: ApplicationExecutionProviderDescriptor,
    event_store: ApplicationExecutionEventStore,
    adapter: Arc<dyn HostedApplicationExecutionAdapter>,
    runs: RwLock<BTreeMap<String, HostedRunState>>,
}

impl MacacaHostedApplicationExecutionProvider {
    /// Build a hosted provider with an injected runtime adapter.
    pub fn new(
        event_store: ApplicationExecutionEventStore,
        adapter: Arc<dyn HostedApplicationExecutionAdapter>,
        capability_declarations: Vec<CapabilityId>,
    ) -> Self {
        Self {
            descriptor: ApplicationExecutionProviderDescriptor {
                provider_id: HOSTED_PROVIDER_ID.into(),
                provider_kind: ApplicationExecutionProviderKind::MacacaHosted,
                protocol_version: HOSTED_PROTOCOL_VERSION.into(),
                supported_commands: vec![
                    ApplicationExecutionControlKind::Cancel,
                    ApplicationExecutionControlKind::Approve,
                    ApplicationExecutionControlKind::Reject,
                    ApplicationExecutionControlKind::Pause,
                    ApplicationExecutionControlKind::Resume,
                    ApplicationExecutionControlKind::Retry,
                    ApplicationExecutionControlKind::InjectInput,
                ],
                supported_events: vec![
                    ApplicationExecutionEventType::ExecutionAccepted,
                    ApplicationExecutionEventType::ProviderHeartbeat,
                    ApplicationExecutionEventType::ApprovalRequested,
                    ApplicationExecutionEventType::ApprovalResolved,
                    ApplicationExecutionEventType::CheckpointCreated,
                    ApplicationExecutionEventType::ControlDelivered,
                    ApplicationExecutionEventType::ControlCompleted,
                    ApplicationExecutionEventType::ExecutionCompleted,
                    ApplicationExecutionEventType::ExecutionFailed,
                    ApplicationExecutionEventType::ExecutionCancelled,
                ],
                checkpoint_support: true,
                heartbeat_policy: ApplicationExecutionHeartbeatPolicy {
                    interval_ms: 30_000,
                    timeout_ms: 120_000,
                    required: false,
                },
                control_delivery: "hosted_runtime_wait_handle".into(),
                capability_declarations,
                resource_profile: BTreeMap::new(),
                transport_kind: HOSTED_TRANSPORT_KIND.into(),
                health_state: ApplicationExecutionProviderHealth::Healthy,
            },
            event_store,
            adapter,
            runs: RwLock::new(BTreeMap::new()),
        }
    }

    fn run_key(scope: &ApplicationExecutionScope) -> String {
        format!(
            "{}:{}:{}",
            scope.application_id, scope.session_id, scope.run_id
        )
    }

    fn scope_from_start(
        command: &StartApplicationExecutionCommand,
    ) -> Result<ApplicationExecutionScope, ServiceError> {
        let Some(session_id) = command.session_id.clone() else {
            return Err(ServiceError::InvalidArgument(
                "macaca_hosted start requires session_id after service assignment".into(),
            ));
        };
        let Some(run_id) = command.run_id.clone() else {
            return Err(ServiceError::InvalidArgument(
                "macaca_hosted start requires run_id after service assignment".into(),
            ));
        };
        Ok(ApplicationExecutionScope {
            application_id: command.application_id,
            session_id,
            run_id,
            tenant_id: command.tenant_id.clone(),
            actor: command.actor.clone(),
        })
    }

    async fn append_event(
        &self,
        scope: &ApplicationExecutionScope,
        event_type: ApplicationExecutionEventType,
        trace: macaca_proto::TraceContext,
        payload: ApplicationExecutionPayload,
        idempotency_key: String,
    ) -> Result<Option<String>, ServiceError> {
        let persisted = self
            .event_store
            .append_idempotent(build_event(
                scope,
                event_type,
                &self.descriptor.provider_id,
                self.descriptor.provider_kind,
                trace,
                payload,
                idempotency_key,
            ))
            .await?;
        Ok(persisted.seq.map(|seq| format!("event/{seq}")))
    }

    /// Persist runtime-adapter signals as application-execution EventLog rows.
    ///
    /// Hosted adapters can observe generic ABI/host-import facts, but the
    /// provider remains the only owner that appends authoritative execution
    /// events for a run.  This method is the Observer bridge between those two
    /// layers: it accepts pre-sanitized protocol-shaped signals, stamps them
    /// with the run scope and provider identity, and appends them idempotently
    /// under the original start/resume idempotency namespace.
    async fn append_runtime_signals(
        &self,
        scope: &ApplicationExecutionScope,
        trace: &macaca_proto::TraceContext,
        idempotency_prefix: &str,
        signals: Vec<HostedApplicationExecutionSignal>,
    ) -> Result<Option<String>, ServiceError> {
        let mut cursor = None;
        for signal in signals {
            cursor = self
                .append_event(
                    scope,
                    signal.event_type,
                    trace.clone(),
                    signal.payload,
                    format!("{idempotency_prefix}:{}", signal.idempotency_suffix),
                )
                .await?
                .or(cursor);
        }
        Ok(cursor)
    }

    fn start_result(
        &self,
        scope: &ApplicationExecutionScope,
        status: ApplicationExecutionCommandStatus,
        event_cursor: Option<String>,
        workspace_ref: Option<String>,
        error: Option<ApplicationExecutionError>,
    ) -> StartApplicationExecutionResult {
        StartApplicationExecutionResult {
            status,
            session_id: Some(scope.session_id.clone()),
            run_id: Some(scope.run_id.clone()),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: self.descriptor.provider_kind,
            event_cursor,
            control_ref: Some(format!(
                "application-execution://{}/{}/{}",
                scope.application_id, scope.session_id, scope.run_id
            )),
            workspace_ref,
            error,
        }
    }
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

/// Translate ABI host-command results into durable execution-protocol signals.
///
/// Component-model WASM artifacts can declare a sequence of host commands.
/// Those commands may route to ServiceRuntime, Application Service orchestration,
/// or future provider-neutral imports.  The application-execution provider must
/// not persist raw host output, but it should record that generic tool/service
/// work was dispatched and whether the declared work completed, failed, or was
/// queued for asynchronous continuation.  This helper performs that bounded
/// translation without looking at application names, workflow names, provider
/// names, model names, or business-domain fields.
fn hosted_signals_from_host_result(
    result: &macaca_proto::ApplicationHostCommandResult,
) -> Vec<HostedApplicationExecutionSignal> {
    let Some(results) = result
        .output
        .get("host_command_results")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut signals = Vec::new();
    let mut completed = 0usize;
    let mut failed = 0usize;
    let mut queued = 0usize;
    for (index, row) in results.iter().enumerate() {
        let status = host_command_status_label(row);
        let metadata = row
            .get("metadata")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();
        let service_id = metadata
            .get("service_id")
            .or_else(|| metadata.get("service.id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let reason_code = metadata
            .get("reason_code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let task_id = row
            .pointer("/output/output/task_id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                row.pointer("/output/task_id")
                    .and_then(serde_json::Value::as_str)
            });
        let delegated_status = row
            .pointer("/output/output/status")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                row.pointer("/output/status")
                    .and_then(serde_json::Value::as_str)
            });

        let effective_status = delegated_status.unwrap_or(status);
        match effective_status.to_ascii_lowercase().as_str() {
            "queued" => queued += 1,
            "completed" | "ok" => completed += 1,
            _ => failed += 1,
        }
        signals.push(HostedApplicationExecutionSignal::new(
            ApplicationExecutionEventType::ToolCallCompleted,
            "hosted application declared host command completed",
            Some(serde_json::json!({
                "index": index,
                "status": effective_status,
                "service_id": service_id,
                "reason_code": reason_code,
                "task_id": task_id,
            })),
            format!("host-command-{index}-completed"),
        ));
    }
    if !results.is_empty() {
        let terminal = if failed > 0 {
            Some((
                ApplicationExecutionEventType::ExecutionFailed,
                "hosted application declared host commands failed",
                "host-commands-failed",
            ))
        } else if queued == 0 && completed == results.len() {
            Some((
                ApplicationExecutionEventType::ExecutionCompleted,
                "hosted application declared host commands completed",
                "host-commands-completed",
            ))
        } else {
            None
        };
        if let Some((event_type, summary, suffix)) = terminal {
            signals.push(HostedApplicationExecutionSignal::new(
                event_type,
                summary,
                Some(serde_json::json!({
                    "host_command_count": results.len(),
                    "completed": completed,
                    "failed": failed,
                    "queued": queued,
                })),
                suffix,
            ));
        }
    }
    signals
}

fn host_command_status_label(row: &serde_json::Value) -> &str {
    row.get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}
