//! Provider trait implementation for Macaca-hosted application execution.
//!
//! This submodule keeps the execution state machine separate from the hosted
//! adapter definitions.  The split is architectural rather than cosmetic: the
//! parent module owns construction and Strategy seams, while this file owns the
//! service-facing provider behavior that appends durable events, updates the
//! in-memory Memento cache, and returns structured command results.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlKind, ApplicationExecutionControlResult, ApplicationExecutionError,
    ApplicationExecutionEventType, ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionSnapshot, ServiceError, StartApplicationExecutionCommand,
    StartApplicationExecutionResult,
};
use tracing::{info, warn};

use crate::application_execution_provider_registry::ApplicationExecutionProvider;

use super::{
    HostedApplicationExecutionOutcome, HostedRunState, MacacaHostedApplicationExecutionProvider,
};

#[async_trait]
impl ApplicationExecutionProvider for MacacaHostedApplicationExecutionProvider {
    fn describe(&self) -> macaca_proto::ApplicationExecutionProviderDescriptor {
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
                signals,
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
                cursor = self
                    .append_runtime_signals(
                        &scope,
                        &command.trace,
                        &command.idempotency_key,
                        signals,
                    )
                    .await?
                    .or(cursor);
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
                metadata: std::collections::BTreeMap::from([(
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
                signals,
            } => {
                let signal_cursor = self
                    .append_runtime_signals(
                        &snapshot.scope,
                        &macaca_proto::TraceContext::new("application-execution-hosted-resume"),
                        &format!("{}:{}", snapshot.scope.session_id, snapshot.scope.run_id),
                        signals,
                    )
                    .await?
                    .or(cursor.clone());
                self.runs.write().await.insert(
                    Self::run_key(&snapshot.scope),
                    HostedRunState {
                        scope: snapshot.scope.clone(),
                        lifecycle_state: ApplicationExecutionLifecycleState::Running,
                        latest_checkpoint_ref: checkpoint_ref,
                        pending_approval_ref: None,
                        latest_event_cursor: signal_cursor.clone(),
                    },
                );
                cursor = signal_cursor;
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
