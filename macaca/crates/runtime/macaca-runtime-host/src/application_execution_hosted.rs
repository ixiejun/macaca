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

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionControlResult, ApplicationExecutionError,
    ApplicationExecutionEventType, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationExecutionSnapshot,
    ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, CapabilityId,
    ServiceError, StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_provider_registry::ApplicationExecutionProvider;
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
/// It intentionally dispatches a custom import label instead of calling a
/// concrete runtime API, keeping runtime choice and package loading outside the
/// provider strategy.
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
        let host_command = ApplicationHostCommand::with_trace(
            ApplicationImport::Custom("macaca:application_execution/start".into()),
            serde_json::json!({
                "application_id": command.application_id,
                "session_id": command.session_id,
                "run_id": command.run_id,
                "workspace_ref": command.workspace_ref,
                "requested_capabilities": command.requested_capabilities,
                "task_input_summary": command.task_input.summary,
                "task_input_has_payload_ref": command.task_input.payload_ref.is_some(),
                "idempotency_key": command.idempotency_key,
            }),
            command.trace,
        );
        let result = self
            .host
            .dispatch(host_command)
            .await
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
        match result.status {
            ApplicationHostCommandStatus::Ok => Ok(HostedApplicationExecutionOutcome::Completed {
                summary: "hosted application runtime completed start dispatch".into(),
            }),
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
/// The state is a small Memento cache for control routing and diagnostics.  The
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

#[async_trait]
impl ApplicationExecutionProvider for MacacaHostedApplicationExecutionProvider {
    fn describe(&self) -> ApplicationExecutionProviderDescriptor {
        self.descriptor.clone()
    }

    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        let scope = Self::scope_from_start(&command)?;
        info!(
            application_id = %scope.application_id,
            session_id = %scope.session_id,
            run_id = %scope.run_id,
            provider_id = %self.descriptor.provider_id,
            trace_id = %command.trace.trace_id,
            "macaca_hosted provider accepted start command"
        );
        let mut cursor = self
            .append_event(
                &scope,
                ApplicationExecutionEventType::ExecutionAccepted,
                command.trace.clone(),
                ApplicationExecutionPayload::summary("macaca_hosted execution accepted"),
                format!("{}:hosted-accepted", command.idempotency_key),
            )
            .await?;
        cursor = self
            .append_event(
                &scope,
                ApplicationExecutionEventType::ProviderHeartbeat,
                command.trace.clone(),
                ApplicationExecutionPayload::summary("macaca_hosted provider heartbeat"),
                format!("{}:hosted-heartbeat", command.idempotency_key),
            )
            .await?
            .or(cursor);

        let outcome = match self.adapter.start(command.clone()).await {
            Ok(outcome) => outcome,
            Err(error) => {
                let reason = error.to_string();
                let failure_cursor = self
                    .append_event(
                        &scope,
                        ApplicationExecutionEventType::ExecutionFailed,
                        command.trace.clone(),
                        ApplicationExecutionPayload::summary(
                            "macaca_hosted runtime unavailable or failed",
                        ),
                        format!("{}:hosted-failed", command.idempotency_key),
                    )
                    .await?
                    .or(cursor);
                warn!(
                    application_id = %scope.application_id,
                    session_id = %scope.session_id,
                    run_id = %scope.run_id,
                    provider_id = %self.descriptor.provider_id,
                    trace_id = %command.trace.trace_id,
                    reason = %reason,
                    "macaca_hosted provider failed start dispatch"
                );
                return Ok(self.start_result(
                    &scope,
                    ApplicationExecutionCommandStatus::Unavailable,
                    failure_cursor,
                    command.workspace_ref,
                    Some(ApplicationExecutionError {
                        code: ApplicationExecutionCommandStatus::Unavailable,
                        layer: "service.application_execution.macaca_hosted".into(),
                        operation: "start".into(),
                        application_id: Some(scope.application_id),
                        session_id: Some(scope.session_id.clone()),
                        run_id: Some(scope.run_id.clone()),
                        provider_id: Some(self.descriptor.provider_id.clone()),
                        provider_kind: Some(self.descriptor.provider_kind),
                        trace_id: Some(command.trace.trace_id.clone()),
                        reason,
                        retryable: false,
                    }),
                ));
            }
        };

        let mut state = HostedRunState {
            scope: scope.clone(),
            lifecycle_state: ApplicationExecutionLifecycleState::Running,
            latest_checkpoint_ref: None,
            pending_approval_ref: None,
            latest_event_cursor: cursor.clone(),
        };
        let status = match outcome {
            HostedApplicationExecutionOutcome::Running {
                checkpoint_ref,
                summary,
            } => {
                if let Some(checkpoint_ref) = checkpoint_ref.clone() {
                    state.latest_checkpoint_ref = Some(checkpoint_ref);
                    cursor = self
                        .append_event(
                            &scope,
                            ApplicationExecutionEventType::CheckpointCreated,
                            command.trace.clone(),
                            ApplicationExecutionPayload::summary(summary),
                            format!("{}:hosted-checkpoint", command.idempotency_key),
                        )
                        .await?
                        .or(cursor);
                }
                ApplicationExecutionCommandStatus::Accepted
            }
            HostedApplicationExecutionOutcome::WaitingForApproval {
                approval_ref,
                checkpoint_ref,
                summary,
            } => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::WaitingForApproval;
                state.pending_approval_ref = Some(approval_ref);
                state.latest_checkpoint_ref = checkpoint_ref;
                cursor = self
                    .append_event(
                        &scope,
                        ApplicationExecutionEventType::ApprovalRequested,
                        command.trace.clone(),
                        ApplicationExecutionPayload::summary(summary),
                        format!("{}:hosted-approval-requested", command.idempotency_key),
                    )
                    .await?
                    .or(cursor);
                ApplicationExecutionCommandStatus::Accepted
            }
            HostedApplicationExecutionOutcome::Completed { summary } => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Completed;
                cursor = self
                    .append_event(
                        &scope,
                        ApplicationExecutionEventType::ExecutionCompleted,
                        command.trace.clone(),
                        ApplicationExecutionPayload::summary(summary),
                        format!("{}:hosted-completed", command.idempotency_key),
                    )
                    .await?
                    .or(cursor);
                ApplicationExecutionCommandStatus::Accepted
            }
        };
        state.latest_event_cursor = cursor.clone();
        self.runs.write().await.insert(Self::run_key(&scope), state);
        Ok(self.start_result(&scope, status, cursor, command.workspace_ref, None))
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        let status = self.adapter.control(command.clone()).await?;
        let mut runs = self.runs.write().await;
        let run_key = Self::run_key(&command.scope);
        let state = runs.entry(run_key).or_insert_with(|| HostedRunState {
            scope: command.scope.clone(),
            lifecycle_state: ApplicationExecutionLifecycleState::Running,
            latest_checkpoint_ref: None,
            pending_approval_ref: None,
            latest_event_cursor: None,
        });
        let event_type = match command.command {
            ApplicationExecutionControlKind::Cancel => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Cancelled;
                ApplicationExecutionEventType::ExecutionCancelled
            }
            ApplicationExecutionControlKind::Approve | ApplicationExecutionControlKind::Reject => {
                state.pending_approval_ref = None;
                state.lifecycle_state =
                    if matches!(command.command, ApplicationExecutionControlKind::Reject) {
                        ApplicationExecutionLifecycleState::Cancelled
                    } else {
                        ApplicationExecutionLifecycleState::Completed
                    };
                ApplicationExecutionEventType::ApprovalResolved
            }
            ApplicationExecutionControlKind::Pause => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Paused;
                ApplicationExecutionEventType::ControlDelivered
            }
            ApplicationExecutionControlKind::Resume | ApplicationExecutionControlKind::Retry => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Running;
                ApplicationExecutionEventType::ControlDelivered
            }
            ApplicationExecutionControlKind::InjectInput => {
                ApplicationExecutionEventType::ControlDelivered
            }
        };
        let mut cursor = self
            .append_event(
                &command.scope,
                event_type,
                command.trace.clone(),
                ApplicationExecutionPayload::summary("macaca_hosted control delivered"),
                format!("{}:hosted-control-delivered", command.idempotency_key),
            )
            .await?;
        if matches!(command.command, ApplicationExecutionControlKind::Approve) {
            cursor = self
                .append_event(
                    &command.scope,
                    ApplicationExecutionEventType::ExecutionCompleted,
                    command.trace.clone(),
                    ApplicationExecutionPayload::summary(
                        "macaca_hosted execution completed after approval",
                    ),
                    format!("{}:hosted-control-completed", command.idempotency_key),
                )
                .await?
                .or(cursor);
        }
        state.latest_event_cursor = cursor.clone();
        info!(
            application_id = %command.scope.application_id,
            session_id = %command.scope.session_id,
            run_id = %command.scope.run_id,
            provider_id = %self.descriptor.provider_id,
            trace_id = %command.trace.trace_id,
            control_kind = ?command.command,
            status = ?status,
            "macaca_hosted provider routed control command"
        );
        Ok(ApplicationExecutionControlResult {
            status,
            scope: command.scope,
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: cursor,
            error: None,
        })
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        let runs = self.runs.read().await;
        Ok(runs
            .values()
            .last()
            .map(|state| ApplicationExecutionSnapshot {
                scope: state.scope.clone(),
                lifecycle_state: state.lifecycle_state,
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: self.descriptor.provider_kind,
                latest_event_cursor: state.latest_event_cursor.clone(),
                latest_checkpoint_ref: state.latest_checkpoint_ref.clone(),
                metadata: BTreeMap::from([(
                    "pending_approval".into(),
                    state.pending_approval_ref.is_some().to_string(),
                )]),
            }))
    }

    async fn resume(
        &self,
        snapshot: ApplicationExecutionSnapshot,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        let outcome = self.adapter.resume(snapshot.clone()).await?;
        let trace = macaca_proto::TraceContext::new("application-execution-hosted-resume");
        let mut cursor = self
            .append_event(
                &snapshot.scope,
                ApplicationExecutionEventType::CheckpointCreated,
                trace,
                ApplicationExecutionPayload::summary("macaca_hosted resume checkpoint accepted"),
                format!(
                    "{}:{}:hosted-resume",
                    snapshot.scope.session_id, snapshot.scope.run_id
                ),
            )
            .await?;
        let lifecycle_state = match outcome {
            HostedApplicationExecutionOutcome::Completed { summary } => {
                cursor = self
                    .append_event(
                        &snapshot.scope,
                        ApplicationExecutionEventType::ExecutionCompleted,
                        macaca_proto::TraceContext::new("application-execution-hosted-resume"),
                        ApplicationExecutionPayload::summary(summary),
                        format!(
                            "{}:{}:hosted-resume-completed",
                            snapshot.scope.session_id, snapshot.scope.run_id
                        ),
                    )
                    .await?
                    .or(cursor);
                ApplicationExecutionLifecycleState::Completed
            }
            HostedApplicationExecutionOutcome::WaitingForApproval {
                approval_ref,
                checkpoint_ref,
                summary: _,
            } => {
                self.runs.write().await.insert(
                    Self::run_key(&snapshot.scope),
                    HostedRunState {
                        scope: snapshot.scope.clone(),
                        lifecycle_state: ApplicationExecutionLifecycleState::WaitingForApproval,
                        latest_checkpoint_ref: checkpoint_ref,
                        pending_approval_ref: Some(approval_ref),
                        latest_event_cursor: cursor.clone(),
                    },
                );
                ApplicationExecutionLifecycleState::WaitingForApproval
            }
            HostedApplicationExecutionOutcome::Running {
                checkpoint_ref,
                summary: _,
            } => {
                self.runs.write().await.insert(
                    Self::run_key(&snapshot.scope),
                    HostedRunState {
                        scope: snapshot.scope.clone(),
                        lifecycle_state: ApplicationExecutionLifecycleState::Running,
                        latest_checkpoint_ref: checkpoint_ref,
                        pending_approval_ref: None,
                        latest_event_cursor: cursor.clone(),
                    },
                );
                ApplicationExecutionLifecycleState::Running
            }
        };
        Ok(StartApplicationExecutionResult {
            status: ApplicationExecutionCommandStatus::Accepted,
            session_id: Some(snapshot.scope.session_id.clone()),
            run_id: Some(snapshot.scope.run_id.clone()),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: cursor,
            control_ref: Some(format!(
                "application-execution://{}/{}/{}",
                snapshot.scope.application_id, snapshot.scope.session_id, snapshot.scope.run_id
            )),
            workspace_ref: None,
            error: if lifecycle_state.is_terminal() {
                None
            } else {
                None
            },
        })
    }

    async fn shutdown(&self) -> Result<(), ServiceError> {
        let active_runs = self.runs.read().await.len();
        info!(
            provider_id = %self.descriptor.provider_id,
            active_runs,
            "macaca_hosted provider shutdown requested"
        );
        Ok(())
    }
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
