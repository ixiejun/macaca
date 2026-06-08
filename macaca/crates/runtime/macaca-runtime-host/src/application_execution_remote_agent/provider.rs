//! Remote-agent ApplicationExecutionProvider Strategy.
//!
//! The provider orchestrates registry selection, lease issuance, transport
//! dispatch, and event-store append operations.  Each public method emits trace
//! logs with provider-neutral dimensions so operators can audit remote execution
//! without inspecting application payloads.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use macaca_proto::{
    AppendExecutionEventCommand, ApplicationExecutionCommandStatus,
    ApplicationExecutionControlCommand, ApplicationExecutionControlKind,
    ApplicationExecutionControlResult, ApplicationExecutionError,
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionHeartbeatPolicy, ApplicationExecutionLifecycleState,
    ApplicationExecutionPayload, ApplicationExecutionProviderDescriptor,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionProviderLease, ApplicationExecutionScope, ApplicationExecutionSnapshot,
    ServiceError, StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_provider_registry::ApplicationExecutionProvider;

use super::registration::RemoteAgentRegistration;
use super::registry::RemoteAgentRegistry;
use super::transport::RemoteAgentExecutionTransport;

const REMOTE_PROVIDER_ID: &str = "provider.remote_agent";
const REMOTE_PROTOCOL_VERSION: &str = "application-execution.v1";
const REMOTE_TRANSPORT_KIND: &str = "remote_agent";

/// Remote-agent provider strategy exposed to the application execution service.
pub struct RemoteAgentApplicationExecutionProvider {
    descriptor: ApplicationExecutionProviderDescriptor,
    event_store: ApplicationExecutionEventStore,
    registry: Arc<RemoteAgentRegistry>,
    transport: Arc<dyn RemoteAgentExecutionTransport>,
}

impl RemoteAgentApplicationExecutionProvider {
    /// Build a provider around an agent registry and transport adapter.
    pub fn new(
        event_store: ApplicationExecutionEventStore,
        registry: Arc<RemoteAgentRegistry>,
        transport: Arc<dyn RemoteAgentExecutionTransport>,
    ) -> Self {
        Self {
            descriptor: ApplicationExecutionProviderDescriptor {
                provider_id: REMOTE_PROVIDER_ID.into(),
                provider_kind: ApplicationExecutionProviderKind::RemoteAgent,
                protocol_version: REMOTE_PROTOCOL_VERSION.into(),
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
                    ApplicationExecutionEventType::ProviderSnapshot,
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
                    required: true,
                },
                control_delivery: "remote_agent_control_channel".into(),
                capability_declarations: Vec::new(),
                resource_profile: BTreeMap::new(),
                transport_kind: REMOTE_TRANSPORT_KIND.into(),
                health_state: ApplicationExecutionProviderHealth::Healthy,
            },
            event_store,
            registry,
            transport,
        }
    }

    /// Derive the execution scope after the service layer assigns session/run ids.
    fn scope_from_start(
        command: &StartApplicationExecutionCommand,
    ) -> Result<ApplicationExecutionScope, ServiceError> {
        let Some(session_id) = command.session_id.clone() else {
            return Err(ServiceError::InvalidArgument(
                "remote_agent start requires session_id after service assignment".into(),
            ));
        };
        let Some(run_id) = command.run_id.clone() else {
            return Err(ServiceError::InvalidArgument(
                "remote_agent start requires run_id after service assignment".into(),
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

    /// Issue a scoped lease bounded by the selected agent heartbeat policy.
    fn build_lease(
        &self,
        agent: &RemoteAgentRegistration,
        scope: ApplicationExecutionScope,
    ) -> ApplicationExecutionProviderLease {
        let now = Utc::now();
        let timeout = ChronoDuration::milliseconds(agent.heartbeat_policy.timeout_ms as i64);
        ApplicationExecutionProviderLease {
            lease_id: format!("remote-lease-{}", Uuid::new_v4()),
            provider_id: agent.agent_id.clone(),
            scope,
            expires_at: now + timeout,
            heartbeat_deadline: now + timeout,
            callback_identity_ref: agent.transport.callback_identity_ref.clone(),
            allowed_event_types: agent.supported_events.clone(),
            allowed_controls: agent.supported_controls.clone(),
        }
    }

    /// Append one idempotent execution event and return its cursor ref.
    async fn append_event(
        &self,
        scope: &ApplicationExecutionScope,
        provider_id: &str,
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
                provider_id,
                self.descriptor.provider_kind,
                trace,
                payload,
                idempotency_key,
            ))
            .await?;
        Ok(persisted.seq.map(|seq| format!("event/{seq}")))
    }

    /// Validate and append a remote-agent gateway event against the stored lease.
    pub async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> Result<ApplicationExecutionEventEnvelope, ServiceError> {
        let Some(lease_id) = command.lease_id.clone() else {
            return Err(ServiceError::InvalidArgument(
                "remote agent gateway ingress requires a lease id".into(),
            ));
        };
        let Some(lease) = self.registry.lease_by_id(&lease_id).await else {
            return Err(ServiceError::InvalidArgument(
                "remote agent gateway ingress rejected stale lease".into(),
            ));
        };
        self.event_store
            .clone()
            .with_lease(lease)
            .append_gateway_event(command)
            .await
    }
}

#[async_trait]
impl ApplicationExecutionProvider for RemoteAgentApplicationExecutionProvider {
    fn describe(&self) -> ApplicationExecutionProviderDescriptor {
        self.descriptor.clone()
    }

    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        let scope = Self::scope_from_start(&command)?;
        let agent = self.registry.select(&command).await?;
        let lease = self.build_lease(&agent, scope.clone());
        self.registry.store_lease(lease.clone()).await;
        let status = match self
            .transport
            .start(agent.clone(), lease.clone(), command.clone())
            .await
        {
            Ok(status) => status,
            Err(error) => {
                let reason = error.to_string();
                let cursor = self
                    .append_event(
                        &scope,
                        &agent.agent_id,
                        ApplicationExecutionEventType::ExecutionFailed,
                        command.trace.clone(),
                        ApplicationExecutionPayload::summary("remote agent start dispatch failed"),
                        format!("{}:remote-agent-failed", command.idempotency_key),
                    )
                    .await?;
                warn!(
                    application_id = %scope.application_id,
                    session_id = %scope.session_id,
                    run_id = %scope.run_id,
                    remote_agent_id = %agent.agent_id,
                    trace_id = %command.trace.trace_id,
                    reason = %reason,
                    "remote agent start dispatch failed"
                );
                return Ok(StartApplicationExecutionResult {
                    status: ApplicationExecutionCommandStatus::Unavailable,
                    session_id: Some(scope.session_id),
                    run_id: Some(scope.run_id),
                    provider_id: Some(agent.agent_id),
                    provider_kind: self.descriptor.provider_kind,
                    event_cursor: cursor,
                    control_ref: None,
                    workspace_ref: command.workspace_ref,
                    error: Some(ApplicationExecutionError {
                        code: ApplicationExecutionCommandStatus::Unavailable,
                        layer: "service.application_execution.remote_agent".into(),
                        operation: "start".into(),
                        application_id: Some(scope.application_id),
                        session_id: None,
                        run_id: None,
                        provider_id: None,
                        provider_kind: Some(self.descriptor.provider_kind),
                        trace_id: Some(command.trace.trace_id),
                        reason,
                        retryable: true,
                    }),
                });
            }
        };
        let cursor = self
            .append_event(
                &scope,
                &agent.agent_id,
                ApplicationExecutionEventType::ExecutionAccepted,
                command.trace.clone(),
                ApplicationExecutionPayload {
                    summary: "remote agent execution lease issued".into(),
                    data: Some(serde_json::json!({
                        "lease_id": lease.lease_id,
                        "remote_transport_kind": agent.transport.transport_kind,
                    })),
                    payload_ref: None,
                    truncated: false,
                },
                format!("{}:remote-agent-accepted", command.idempotency_key),
            )
            .await?;
        info!(
            application_id = %scope.application_id,
            session_id = %scope.session_id,
            run_id = %scope.run_id,
            remote_agent_id = %agent.agent_id,
            trace_id = %command.trace.trace_id,
            "remote agent execution lease issued"
        );
        Ok(StartApplicationExecutionResult {
            status,
            session_id: Some(scope.session_id),
            run_id: Some(scope.run_id),
            provider_id: Some(agent.agent_id),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: cursor,
            control_ref: Some(format!("remote-agent://{}", lease.lease_id)),
            workspace_ref: command.workspace_ref,
            error: None,
        })
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        let lease = self
            .registry
            .lease_for_scope(&command.scope)
            .await
            .ok_or_else(|| ServiceError::InvalidArgument("remote agent lease not found".into()))?;
        if !lease
            .allowed_controls
            .iter()
            .any(|allowed| allowed == &command.command)
        {
            return Ok(ApplicationExecutionControlResult {
                status: ApplicationExecutionCommandStatus::Unsupported,
                scope: command.scope,
                provider_id: Some(lease.provider_id),
                provider_kind: self.descriptor.provider_kind,
                event_cursor: None,
                error: None,
            });
        }
        let agent = self.registry.agent_for_lease(&lease).await.ok_or_else(|| {
            ServiceError::InvalidArgument("remote agent registration not found".into())
        })?;
        let status = self
            .transport
            .control(agent.clone(), lease.clone(), command.clone())
            .await?;
        let cursor = self
            .append_event(
                &command.scope,
                &agent.agent_id,
                ApplicationExecutionEventType::ControlDelivered,
                command.trace.clone(),
                ApplicationExecutionPayload::summary("remote agent control delivered"),
                format!("{}:remote-agent-control-delivered", command.idempotency_key),
            )
            .await?;
        Ok(ApplicationExecutionControlResult {
            status,
            scope: command.scope,
            provider_id: Some(agent.agent_id),
            provider_kind: self.descriptor.provider_kind,
            event_cursor: cursor,
            error: None,
        })
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        let lease = self.registry.latest_lease().await;
        Ok(lease.map(|lease| ApplicationExecutionSnapshot {
            scope: lease.scope,
            lifecycle_state: if lease.heartbeat_deadline <= Utc::now() {
                ApplicationExecutionLifecycleState::Failed
            } else {
                ApplicationExecutionLifecycleState::Running
            },
            provider_id: Some(lease.provider_id),
            provider_kind: self.descriptor.provider_kind,
            latest_event_cursor: None,
            latest_checkpoint_ref: None,
            metadata: BTreeMap::from([("lease_id".into(), lease.lease_id)]),
        }))
    }

    async fn health(&self) -> Result<ApplicationExecutionProviderHealth, ServiceError> {
        if !self.registry.has_agents().await {
            Ok(ApplicationExecutionProviderHealth::Unavailable {
                reason: "no remote agents registered".into(),
            })
        } else {
            Ok(ApplicationExecutionProviderHealth::Healthy)
        }
    }
}
