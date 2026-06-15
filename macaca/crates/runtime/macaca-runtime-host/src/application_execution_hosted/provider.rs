//! Macaca-hosted provider construction and durable EventLog helpers.
//!
//! The provider is a Strategy that owns generic execution envelopes, idempotent event
//! emission, and an in-memory Memento cache. Concrete application behavior is injected
//! through `HostedApplicationExecutionAdapter` implementations such as the ABI bridge.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlKind, ApplicationExecutionError,
    ApplicationExecutionEventType, ApplicationExecutionHeartbeatPolicy,
    ApplicationExecutionPayload, ApplicationExecutionProviderDescriptor,
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderKind,
    ApplicationExecutionScope, CapabilityId, ServiceError, StartApplicationExecutionCommand,
    StartApplicationExecutionResult,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;

use super::types::{
    HostedApplicationExecutionAdapter, HostedApplicationExecutionSignal, HostedRunState,
    HOSTED_PROTOCOL_VERSION, HOSTED_PROVIDER_ID, HOSTED_TRANSPORT_KIND,
};

/// Macaca-hosted provider strategy.
pub struct MacacaHostedApplicationExecutionProvider {
    pub(super) descriptor: ApplicationExecutionProviderDescriptor,
    pub(super) event_store: ApplicationExecutionEventStore,
    pub(super) adapter: Arc<dyn HostedApplicationExecutionAdapter>,
    pub(super) runs: RwLock<BTreeMap<String, HostedRunState>>,
}

impl MacacaHostedApplicationExecutionProvider {
    /// Build a hosted provider with an injected runtime adapter and capability declarations.
    pub fn new(
        event_store: ApplicationExecutionEventStore,
        adapter: Arc<dyn HostedApplicationExecutionAdapter>,
        capability_declarations: Vec<CapabilityId>,
    ) -> Self {
        info!(
            provider_id = HOSTED_PROVIDER_ID,
            transport_kind = HOSTED_TRANSPORT_KIND,
            capability_count = capability_declarations.len(),
            "macaca hosted application execution provider constructed"
        );
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

    /// Derive the in-memory run key from one execution scope.
    pub(super) fn run_key(scope: &ApplicationExecutionScope) -> String {
        format!(
            "{}:{}:{}",
            scope.application_id, scope.session_id, scope.run_id
        )
    }

    /// Build execution scope after the service layer assigns session and run identifiers.
    pub(super) fn scope_from_start(
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

    /// Append one idempotent event row to the durable application-execution EventLog.
    pub(super) async fn append_event(
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

    /// Persist runtime-adapter signals as application-execution EventLog rows (Observer bridge).
    pub(super) async fn append_runtime_signals(
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

    /// Build the structured start result returned to the application-execution service.
    pub(super) fn start_result(
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
