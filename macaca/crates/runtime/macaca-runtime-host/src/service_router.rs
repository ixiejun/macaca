//! Unified service router for host-side `service.call`.
//!
//! The router is the Policy Enforcement Point (PEP). It coordinates contract
//! resolution, policy decision, runtime dispatch, and audit-grade logs.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use macaca_proto::{
    KernelServiceId, ServiceBusSource, ServiceCommand, ServiceCommandName, TraceContext,
};
use serde_json::Value;
use tokio::time::{timeout, Duration};

use crate::service_call_audit::{ServiceCallAuditEvent, ServiceCallAuditSink};
use crate::service_contract_registry::{ServiceContractDescriptor, ServiceContractRegistry};
use crate::service_policy_engine::{ServicePolicyEngine, ServicePolicyInput};
use crate::service_provider_selector::{ProviderSelectionStrategy, ProviderSelector};
use crate::{ServiceRuntime, ServiceRuntimeError};

/// Canonical route request received from host import bridges or other callers.
#[derive(Debug, Clone)]
pub struct ServiceRouteRequest {
    pub app_id: Option<String>,
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub service_id: KernelServiceId,
    pub operation: ServiceCommandName,
    pub payload: Value,
    pub metadata: BTreeMap<String, String>,
    pub trace: TraceContext,
}

/// Service route response returned to host import callers.
#[derive(Debug, Clone)]
pub struct ServiceRouteResponse {
    pub output: Value,
    pub status: String,
    pub metadata: BTreeMap<String, String>,
}

/// Router implementation that dispatches through `ServiceRuntime`.
pub struct ServiceRouter {
    runtime: Arc<ServiceRuntime>,
    source: ServiceBusSource,
    contracts: Arc<dyn ServiceContractRegistry>,
    policy_engine: Arc<dyn ServicePolicyEngine>,
    audit_sink: Option<Arc<dyn ServiceCallAuditSink>>,
}

impl ServiceRouter {
    /// Build a router with explicit dependencies.
    pub fn new(
        runtime: Arc<ServiceRuntime>,
        source: ServiceBusSource,
        contracts: Arc<dyn ServiceContractRegistry>,
        policy_engine: Arc<dyn ServicePolicyEngine>,
    ) -> Self {
        Self {
            runtime,
            source,
            contracts,
            policy_engine,
            audit_sink: None,
        }
    }

    /// Attach a structured audit sink so callers can replay service-call chains.
    pub fn with_audit_sink(mut self, sink: Arc<dyn ServiceCallAuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Route one service call through contract + policy + runtime dispatch.
    pub async fn route(
        &self,
        request: ServiceRouteRequest,
    ) -> Result<ServiceRouteResponse, ServiceRuntimeError> {
        let start = Instant::now();
        tracing::info!(
            app_id = request.app_id.as_deref().unwrap_or("none"),
            session_id = request.session_id.as_deref().unwrap_or("none"),
            service_id = %request.service_id,
            operation = %request.operation,
            trace_id = %request.trace.trace_id,
            "service_call_requested"
        );
        self.emit_audit_event(
            ServiceCallAuditEvent::new(
                "service_call_requested",
                request.trace.trace_id.clone(),
                request.service_id.to_string(),
            )
            .app_id(request.app_id.clone())
            .session_id(request.session_id.clone())
            .operation(Some(request.operation.to_string()))
            .input_hash(hash_json(&request.payload)),
        );
        let strategy = ProviderSelectionStrategy::from_metadata(
            request
                .metadata
                .get("service.provider_strategy")
                .map(String::as_str),
        );
        let provider_candidates = ProviderSelector::snapshots_from_metadata(&request.metadata);
        let selected_provider = ProviderSelector::select(
            strategy,
            request
                .metadata
                .get("service.previous_provider")
                .map(String::as_str),
            &provider_candidates,
        )
        .unwrap_or_else(|| "runtime.default".to_string());
        tracing::info!(
            app_id = request.app_id.as_deref().unwrap_or("none"),
            service_id = %request.service_id,
            provider_id = %selected_provider,
            provider_strategy = ?strategy,
            trace_id = %request.trace.trace_id,
            "service_call_provider_selected"
        );
        self.emit_audit_event(
            ServiceCallAuditEvent::new(
                "service_call_dispatched",
                request.trace.trace_id.clone(),
                request.service_id.to_string(),
            )
            .app_id(request.app_id.clone())
            .session_id(request.session_id.clone())
            .operation(Some(request.operation.to_string()))
            .provider_id(Some(selected_provider.clone())),
        );
        let contract = self
            .contracts
            .resolve(&request.service_id, None)
            .unwrap_or_else(|| ServiceContractDescriptor::v1(request.service_id.clone()));
        tracing::info!(
            service_id = %request.service_id,
            contract_version = %contract.version,
            trace_id = %request.trace.trace_id,
            "service_call_contract_resolved"
        );
        let decision = self.policy_engine.evaluate(&ServicePolicyInput {
            app_id: request.app_id.clone(),
            tenant_id: request.tenant_id.clone(),
            service_id: request.service_id.clone(),
        });
        if !decision.allow {
            tracing::warn!(
                app_id = request.app_id.as_deref().unwrap_or("none"),
                service_id = %request.service_id,
                reason_code = %decision.reason_code,
                trace_id = %request.trace.trace_id,
                "service_call_denied"
            );
            self.emit_audit_event(
                ServiceCallAuditEvent::new(
                    "service_call_denied",
                    request.trace.trace_id.clone(),
                    request.service_id.to_string(),
                )
                .app_id(request.app_id.clone())
                .session_id(request.session_id.clone())
                .operation(Some(request.operation.to_string()))
                .decision(Some(decision.reason_code.clone())),
            );
            return Err(ServiceRuntimeError::PolicyDenied(decision.reason_code));
        }
        tracing::info!(
            app_id = request.app_id.as_deref().unwrap_or("none"),
            service_id = %request.service_id,
            reason_code = %decision.reason_code,
            timeout_ms = decision.timeout_ms,
            max_retries = decision.max_retries,
            trace_id = %request.trace.trace_id,
            "service_call_authorized"
        );
        let max_attempts = decision.max_retries.saturating_add(1);
        let mut attempt: u32 = 0;
        loop {
            attempt = attempt.saturating_add(1);
            let mut command = ServiceCommand::with_trace(
                request.operation.clone(),
                request.payload.clone(),
                request.trace.clone(),
            );
            command.metadata = request.metadata.clone();
            let timeout_budget = Duration::from_millis(decision.timeout_ms);
            let call_result = timeout(
                timeout_budget,
                self.runtime
                    .call(&request.service_id, self.source.clone(), command),
            )
            .await;
            let reply = match call_result {
                Ok(Ok(reply)) => reply,
                Ok(Err(error)) => {
                    tracing::warn!(
                        app_id = request.app_id.as_deref().unwrap_or("none"),
                        service_id = %request.service_id,
                        provider_id = %selected_provider,
                        attempt,
                        max_attempts,
                        trace_id = %request.trace.trace_id,
                        "service_call_failed_attempt"
                    );
                    self.emit_audit_event(
                        ServiceCallAuditEvent::new(
                            "service_call_failed",
                            request.trace.trace_id.clone(),
                            request.service_id.to_string(),
                        )
                        .app_id(request.app_id.clone())
                        .session_id(request.session_id.clone())
                        .operation(Some(request.operation.to_string()))
                        .provider_id(Some(selected_provider.clone()))
                        .retry_count(Some(attempt.saturating_sub(1))),
                    );
                    if attempt < max_attempts {
                        continue;
                    }
                    return Err(error);
                }
                Err(_) => {
                    let timeout_error = ServiceRuntimeError::PolicyDenied(
                        "policy_timeout_budget_exceeded".to_string(),
                    );
                    tracing::warn!(
                        app_id = request.app_id.as_deref().unwrap_or("none"),
                        service_id = %request.service_id,
                        provider_id = %selected_provider,
                        timeout_ms = decision.timeout_ms,
                        attempt,
                        max_attempts,
                        trace_id = %request.trace.trace_id,
                        "service_call_timeout"
                    );
                    self.emit_audit_event(
                        ServiceCallAuditEvent::new(
                            "service_call_failed",
                            request.trace.trace_id.clone(),
                            request.service_id.to_string(),
                        )
                        .app_id(request.app_id.clone())
                        .session_id(request.session_id.clone())
                        .operation(Some(request.operation.to_string()))
                        .provider_id(Some(selected_provider.clone()))
                        .decision(Some("policy_timeout_budget_exceeded".into()))
                        .retry_count(Some(attempt.saturating_sub(1))),
                    );
                    if attempt < max_attempts {
                        continue;
                    }
                    return Err(timeout_error);
                }
            };
            tracing::info!(
                service_id = %request.service_id,
                provider_id = %selected_provider,
                status = %reply.status,
                retry_count = attempt.saturating_sub(1),
                latency_ms = start.elapsed().as_millis() as u64,
                trace_id = reply.trace.as_ref().map(|value| value.trace_id.as_str()).unwrap_or("none"),
                "service_call_succeeded"
            );
            let latency_ms = start.elapsed().as_millis() as u64;
            let replay_metadata = sanitized_replay_metadata(&reply.metadata);
            self.emit_audit_event(
                ServiceCallAuditEvent::new(
                    "service_call_succeeded",
                    request.trace.trace_id.clone(),
                    request.service_id.to_string(),
                )
                .app_id(request.app_id.clone())
                .session_id(request.session_id.clone())
                .operation(Some(request.operation.to_string()))
                .provider_id(Some(selected_provider.clone()))
                .decision(Some(decision.reason_code.clone()))
                .retry_count(Some(attempt.saturating_sub(1)))
                .latency_ms(Some(latency_ms))
                .replay_metadata(replay_metadata)
                .output_hash(hash_json(&reply.output.clone().unwrap_or(Value::Null))),
            );
            let mut metadata = reply.metadata;
            metadata.insert("provider_id".into(), selected_provider.clone());
            metadata.insert("retry_count".into(), attempt.saturating_sub(1).to_string());
            metadata.insert("latency_ms".into(), latency_ms.to_string());
            metadata.insert("policy_decision".into(), decision.reason_code.clone());
            return Ok(ServiceRouteResponse {
                output: reply.output.unwrap_or(Value::Null),
                status: reply.status,
                metadata,
            });
        }
    }

    /// Emit one audit event and keep routing fail-closed behavior stable even if
    /// the sink temporarily fails.
    fn emit_audit_event(&self, event: ServiceCallAuditEvent) {
        if let Some(sink) = &self.audit_sink {
            if let Err(error) = sink.emit(event) {
                tracing::warn!(error = %error, "service_call_audit_emit_failed");
            }
        }
    }

    /// Query one audit evidence chain by trace id.
    pub fn replay_audit_by_trace_id(
        &self,
        trace_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        let sink = self.audit_sink.as_ref().ok_or_else(|| {
            ServiceRuntimeError::State("service call audit sink not configured".into())
        })?;
        sink.replay_by_trace_id(trace_id)
    }

    /// Query one audit evidence chain by session id.
    pub fn replay_audit_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<ServiceCallAuditEvent>, ServiceRuntimeError> {
        let sink = self.audit_sink.as_ref().ok_or_else(|| {
            ServiceRuntimeError::State("service call audit sink not configured".into())
        })?;
        sink.replay_by_session_id(session_id)
    }
}

impl ServiceCallAuditEvent {
    fn app_id(mut self, app_id: Option<String>) -> Self {
        self.app_id = app_id;
        self
    }

    fn session_id(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    fn provider_id(mut self, provider_id: Option<String>) -> Self {
        self.provider_id = provider_id;
        self
    }

    fn operation(mut self, operation: Option<String>) -> Self {
        self.operation = operation;
        self
    }

    fn decision(mut self, decision: Option<String>) -> Self {
        self.decision = decision;
        self
    }

    fn retry_count(mut self, retry_count: Option<u32>) -> Self {
        self.retry_count = retry_count;
        self
    }

    fn latency_ms(mut self, latency_ms: Option<u64>) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    fn input_hash(mut self, input_hash: Option<String>) -> Self {
        self.input_hash = input_hash;
        self
    }

    fn output_hash(mut self, output_hash: Option<String>) -> Self {
        self.output_hash = output_hash;
        self
    }

    fn replay_metadata(mut self, replay_metadata: BTreeMap<String, String>) -> Self {
        self.replay_metadata = replay_metadata;
        self
    }
}

/// Copy only service-declared facts that are useful to replay and bounded by
/// their key/value limits. The router never reflects arbitrary metadata into
/// the audit chain because metadata may have originated from a provider.
fn sanitized_replay_metadata(metadata: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    const ALLOWED_KEYS: &[&str] = &[
        "replay.clock_source",
        "replay.provider_class",
        "replay.timezone_data_version",
        "replay.monotonic_unit",
        "replay.random_command",
        "replay.random_length",
        "replay.config_command",
        "replay.filesystem_command",
    ];
    metadata
        .iter()
        .filter(|(key, value)| {
            ALLOWED_KEYS.contains(&key.as_str()) && value.len() <= 96 && value.is_ascii()
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn hash_json(value: &Value) -> Option<String> {
    use std::hash::{Hash, Hasher};
    let encoded = serde_json::to_vec(value).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    encoded.hash(&mut hasher);
    Some(format!("{:016x}", hasher.finish()))
}
