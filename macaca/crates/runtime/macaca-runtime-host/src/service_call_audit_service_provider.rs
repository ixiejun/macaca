//! Generic system-service provider for `service.call` audit replay.
//!
//! This adapter exposes a provider-neutral command surface to query structured
//! service-call audit events by trace id or session id.  It intentionally does
//! not encode any application name, workflow name, or business semantics.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceCallResult, ServiceCommand, ServiceCommandName,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType,
    TraceContext, TraceSchemaRef,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{ServiceCallAuditEvent, ServiceCallAuditSink};

/// Stable service id for generic audit replay queries.
pub const SERVICE_CALL_AUDIT_SERVICE_ID: &str = "system.service_audit";
/// Command name to replay one evidence chain by trace id.
pub const SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND: &str = "service.audit.replay.trace";
/// Command name to replay one evidence chain by session id.
pub const SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND: &str = "service.audit.replay.session";

/// Query payload for trace-id replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceCallAuditReplayTraceCommand {
    pub trace: TraceContext,
    pub trace_id: String,
}

/// Query payload for session-id replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceCallAuditReplaySessionCommand {
    pub trace: TraceContext,
    pub session_id: String,
}

/// Structured replay output so callers can extend response metadata safely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceCallAuditEventView {
    pub stage: String,
    pub trace_id: String,
    pub session_id: Option<String>,
    pub app_id: Option<String>,
    pub service_id: String,
    pub provider_id: Option<String>,
    pub decision: Option<String>,
    pub retry_count: Option<u32>,
    pub latency_ms: Option<u64>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub emitted_at_rfc3339: String,
}

impl From<ServiceCallAuditEvent> for ServiceCallAuditEventView {
    fn from(value: ServiceCallAuditEvent) -> Self {
        Self {
            stage: value.stage,
            trace_id: value.trace_id,
            session_id: value.session_id,
            app_id: value.app_id,
            service_id: value.service_id,
            provider_id: value.provider_id,
            decision: value.decision,
            retry_count: value.retry_count,
            latency_ms: value.latency_ms,
            input_hash: value.input_hash,
            output_hash: value.output_hash,
            emitted_at_rfc3339: value.emitted_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceCallAuditReplayResult {
    pub events: Vec<ServiceCallAuditEventView>,
}

/// Runtime-host provider exposing generic service-call audit replay commands.
pub struct ServiceCallAuditSystemServiceProvider {
    descriptor: ServiceDescriptor,
    sink: Arc<dyn ServiceCallAuditSink>,
}

impl ServiceCallAuditSystemServiceProvider {
    /// Build a provider over a shared audit sink.
    pub fn new(sink: Arc<dyn ServiceCallAuditSink>) -> Self {
        Self {
            descriptor: service_call_audit_service_descriptor(),
            sink,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result<T: Serialize>(value: T, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(value)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for ServiceCallAuditSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "service-call-audit provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "service-call-audit command accepted"
        );
        match command.name.as_str() {
            SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND => {
                let typed: ServiceCallAuditReplayTraceCommand = decode(command.payload)?;
                let events = self
                    .sink
                    .replay_by_trace_id(&typed.trace_id)
                    .map_err(|error| {
                        tracing::warn!(
                            service_id = %self.descriptor.id,
                            command = SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND,
                            trace_id = %typed.trace.trace_id,
                            query_trace_id = %typed.trace_id,
                            error = %error,
                            "service-call-audit trace replay failed"
                        );
                        ServiceError::AdapterFailure(error.to_string())
                    })?
                    .into_iter()
                    .map(ServiceCallAuditEventView::from)
                    .collect();
                Self::result(ServiceCallAuditReplayResult { events }, typed.trace)
            }
            SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND => {
                let typed: ServiceCallAuditReplaySessionCommand = decode(command.payload)?;
                let events = self
                    .sink
                    .replay_by_session_id(&typed.session_id)
                    .map_err(|error| {
                        tracing::warn!(
                            service_id = %self.descriptor.id,
                            command = SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND,
                            trace_id = %typed.trace.trace_id,
                            query_session_id = %typed.session_id,
                            error = %error,
                            "service-call-audit session replay failed"
                        );
                        ServiceError::AdapterFailure(error.to_string())
                    })?
                    .into_iter()
                    .map(ServiceCallAuditEventView::from)
                    .collect();
                Self::result(ServiceCallAuditReplayResult { events }, typed.trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported service-call-audit command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "service-call-audit provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "service-call-audit provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Build provider-neutral descriptor for service-call audit replay.
pub fn service_call_audit_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(SERVICE_CALL_AUDIT_SERVICE_ID),
        ServiceType::new("service_audit"),
        TraceSchemaRef::new("trace.system_service.service_audit.v1"),
    );
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor
        .metadata
        .insert("phase".into(), "audit_replay_v1".into());
    descriptor
}

/// Build a typed replay-by-trace command.
pub fn service_call_audit_replay_trace_command(
    payload: ServiceCallAuditReplayTraceCommand,
) -> ServiceResult<ServiceCommand> {
    let trace = payload.trace.clone();
    service_call_audit_command(SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND, payload, trace)
}

/// Build a typed replay-by-session command.
pub fn service_call_audit_replay_session_command(
    payload: ServiceCallAuditReplaySessionCommand,
) -> ServiceResult<ServiceCommand> {
    let trace = payload.trace.clone();
    service_call_audit_command(SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND, payload, trace)
}

fn service_call_audit_command<T: Serialize>(
    command_name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceResult<ServiceCommand> {
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(command_name),
        serde_json::to_value(payload)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        trace,
    ))
}

fn decode<T>(value: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use macaca_proto::{ServiceBusSource, ServiceCommand, ServiceCommandName, TraceContext};

    use crate::wasm_runtime_provider::{WasmHostImportBridge, WasmHostImportBridgeConfig};
    use crate::{
        InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
        ServiceRouteRequest, ServiceRouter, ServiceRuntime, ServiceRuntimeConfig,
    };

    use super::*;

    #[tokio::test]
    async fn service_call_audit_provider_replays_shared_bridge_events() {
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let sink: Arc<dyn ServiceCallAuditSink> = Arc::new(InMemoryServiceCallAuditSink::new());
        let bridge = WasmHostImportBridge::new_with_audit_sink(
            Arc::clone(&runtime),
            WasmHostImportBridgeConfig::default(),
            sink.clone(),
        );

        // Emit one deterministic event through the router path so the provider
        // validates real replay integration instead of only sink internals.
        let router = ServiceRouter::new(
            Arc::clone(&runtime),
            ServiceBusSource::new("test.audit"),
            Arc::new(InMemoryServiceContractRegistry::new()),
            Arc::new(InMemoryServicePolicyEngine::new()),
        )
        .with_audit_sink(bridge.service_call_audit_sink());
        let _ = router
            .route(ServiceRouteRequest {
                app_id: Some("app.audit.provider".into()),
                tenant_id: None,
                session_id: Some("session-audit-provider".into()),
                service_id: macaca_proto::KernelServiceId::new("service.missing"),
                operation: macaca_proto::ServiceCommandName::new("call"),
                payload: serde_json::json!({}),
                metadata: BTreeMap::from([
                    ("trace.id".into(), "trace-audit-provider".into()),
                    ("session.id".into(), "session-audit-provider".into()),
                ]),
                trace: TraceContext::new("trace-audit-provider"),
            })
            .await;

        let provider = ServiceCallAuditSystemServiceProvider::new(sink);
        let trace_result = provider
            .call(
                service_call_audit_replay_trace_command(ServiceCallAuditReplayTraceCommand {
                    trace: TraceContext::new("trace-query"),
                    trace_id: "trace-audit-provider".into(),
                })
                .expect("trace replay command should build"),
            )
            .await
            .expect("trace replay should succeed");
        let trace_view: ServiceCallAuditReplayResult =
            serde_json::from_value(trace_result.output).expect("trace replay should decode");
        assert!(
            !trace_view.events.is_empty(),
            "shared sink should replay trace events"
        );

        let session_result = provider
            .call(
                service_call_audit_replay_session_command(ServiceCallAuditReplaySessionCommand {
                    trace: TraceContext::new("session-query"),
                    session_id: "session-audit-provider".into(),
                })
                .expect("session replay command should build"),
            )
            .await
            .expect("session replay should succeed");
        let session_view: ServiceCallAuditReplayResult =
            serde_json::from_value(session_result.output).expect("session replay should decode");
        assert!(
            !session_view.events.is_empty(),
            "shared sink should replay session events"
        );
    }

    #[tokio::test]
    async fn service_call_audit_provider_returns_empty_replay_for_unknown_trace_and_session() {
        // This provider is expected to return an empty evidence chain for
        // unknown identifiers, instead of throwing an opaque transport-level
        // failure. The contract keeps replay deterministic for callers.
        let sink: Arc<dyn ServiceCallAuditSink> = Arc::new(InMemoryServiceCallAuditSink::new());
        let provider = ServiceCallAuditSystemServiceProvider::new(sink);

        let trace_result = provider
            .call(
                service_call_audit_replay_trace_command(ServiceCallAuditReplayTraceCommand {
                    trace: TraceContext::new("trace-query-empty"),
                    trace_id: "trace-not-exist".into(),
                })
                .expect("trace replay command should build"),
            )
            .await
            .expect("trace replay should succeed");
        let trace_view: ServiceCallAuditReplayResult =
            serde_json::from_value(trace_result.output).expect("trace replay output should decode");
        assert!(
            trace_view.events.is_empty(),
            "unknown trace should yield an empty replay result"
        );

        let session_result = provider
            .call(
                service_call_audit_replay_session_command(ServiceCallAuditReplaySessionCommand {
                    trace: TraceContext::new("session-query-empty"),
                    session_id: "session-not-exist".into(),
                })
                .expect("session replay command should build"),
            )
            .await
            .expect("session replay should succeed");
        let session_view: ServiceCallAuditReplayResult =
            serde_json::from_value(session_result.output)
                .expect("session replay output should decode");
        assert!(
            session_view.events.is_empty(),
            "unknown session should yield an empty replay result"
        );
    }

    #[tokio::test]
    async fn service_call_audit_provider_rejects_invalid_replay_payload_with_structured_error() {
        // Malformed payloads must fail deterministically with a typed error so
        // callers can distinguish contract violations from runtime faults.
        let sink: Arc<dyn ServiceCallAuditSink> = Arc::new(InMemoryServiceCallAuditSink::new());
        let provider = ServiceCallAuditSystemServiceProvider::new(sink);

        let invalid_command = ServiceCommand::with_trace(
            ServiceCommandName::new(SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND),
            serde_json::json!({"trace_id": 12345}),
            TraceContext::new("trace-invalid-payload"),
        );

        let error = provider
            .call(invalid_command)
            .await
            .expect_err("invalid payload should be rejected");
        assert!(
            matches!(error, ServiceError::UnsupportedCommand(_)),
            "invalid payload should map to UnsupportedCommand"
        );
    }
}
