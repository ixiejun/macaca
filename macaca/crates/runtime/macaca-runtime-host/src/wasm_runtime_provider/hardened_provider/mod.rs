//! Hardened out-of-process WASM runtime provider (facade).
//!
//! The provider is a Strategy implementation for deployments that require a
//! process boundary around guest execution.  Runtime-host still owns policy,
//! trace validation, diagnostics, lifecycle projection, and provider selection;
//! the daemon transport only receives sanitized command envelopes and returns
//! provider-neutral responses.

mod response_mapper;
mod session;

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, PackageRuntimeKind, TraceContext, WasmEngineCapabilities,
    WasmExecutionProfile, WasmRuntimeAvailability, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmRuntimeUnavailableReason,
};
use tracing::{info, warn};

use super::hardened_transport::WasmHardenedHealth;
use super::hardened_transport::WasmHardenedTransport;
use super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

use session::HardenedOutOfProcessWasmExecutionSession;

/// Runtime-host provider that dispatches execution to a hardened daemon.
#[derive(Debug, Clone)]
pub struct HardenedOutOfProcessWasmRuntimeProvider {
    pub(super) transport: Arc<dyn WasmHardenedTransport>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
}

impl HardenedOutOfProcessWasmRuntimeProvider {
    /// Create a provider around a concrete daemon transport adapter.
    ///
    /// The constructor accepts a trait object so tests, local daemons, socket
    /// transports, or remote hardened executors can be swapped without changing
    /// the public `WasmApplicationRuntimeProvider` contract.
    #[allow(dead_code)]
    pub(crate) fn new(transport: Arc<dyn WasmHardenedTransport>) -> Self {
        Self {
            transport,
            telemetry: None,
        }
    }

    /// Return a provider clone that emits sanitized daemon/runtime telemetry.
    #[allow(dead_code)]
    pub(crate) fn with_telemetry_sink(mut self, sink: WasmTelemetrySinkRef) -> Self {
        self.telemetry = Some(sink);
        self
    }

    pub(super) fn capabilities(&self) -> WasmEngineCapabilities {
        WasmEngineCapabilities {
            can_compile: true,
            can_instantiate: true,
            can_execute: true,
            supports_component_model: true,
            supports_host_import_bridge: true,
            supports_wasi: false,
            engine_features: vec![
                "hardened-out-of-process-v0".into(),
                "daemon-transport-bridge-v0".into(),
            ],
            metadata: Default::default(),
        }
    }
}

#[async_trait]
impl WasmApplicationRuntimeProvider for HardenedOutOfProcessWasmRuntimeProvider {
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        let availability = WasmRuntimeAvailability {
            state: "available".into(),
            reason: None,
            capabilities: self.capabilities(),
            diagnostics: None,
            metadata: Default::default(),
        };
        WasmRuntimeProviderDescriptor {
            runtime_kind: PackageRuntimeKind::WasmComponent,
            provider_class: "hardened_out_of_process".into(),
            capabilities: self.capabilities(),
            default_profile: WasmExecutionProfile::default_wasm_component(),
            availability,
            diagnostics: None,
            metadata: Default::default(),
        }
    }

    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability {
        let Some(trace) = trace else {
            warn!(
                provider_class = "hardened_out_of_process",
                reason_code = "missing_trace",
                "WASM hardened provider availability checked without trace"
            );
            return WasmRuntimeAvailability::unavailable(
                PackageRuntimeKind::WasmComponent,
                WasmRuntimeUnavailableReason::provider_missing(
                    "hardened provider availability requires trace context",
                ),
                None,
            );
        };
        match self.transport.health(trace.clone()).await {
            WasmHardenedHealth::Healthy => {
                info!(
                    trace_id = %trace.trace_id,
                    provider_class = "hardened_out_of_process",
                    "WASM hardened provider reported healthy availability"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Daemon,
                        "healthy",
                        "hardened_out_of_process",
                    )
                    .trace_id(trace.trace_id.clone()),
                );
                WasmRuntimeAvailability {
                    state: "available".into(),
                    reason: None,
                    capabilities: self.capabilities(),
                    diagnostics: None,
                    metadata: Default::default(),
                }
            }
            health => {
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Daemon,
                        "unavailable",
                        "hardened_out_of_process",
                    )
                    .trace_id(trace.trace_id.clone())
                    .reason_code(health.reason_code()),
                );
                WasmRuntimeAvailability::unavailable(
                    PackageRuntimeKind::WasmComponent,
                    WasmRuntimeUnavailableReason::provider_missing(health.safe_reason()),
                    Some(trace),
                )
            }
        }
    }

    async fn create_session(
        &self,
        request: WasmRuntimeSessionRequest,
    ) -> Result<Box<dyn WasmExecutionSession>, ApplicationAbiError> {
        request.validate()?;
        let trace = request
            .trace
            .clone()
            .ok_or(ApplicationAbiError::MissingTraceContext)?;
        let session_id = format!(
            "hardened-session-{}",
            super::hardened_transport::sanitize_label(trace.trace_id.clone())
        );
        match self.transport.health(trace.clone()).await {
            WasmHardenedHealth::Healthy => {
                info!(
                    session_id = %session_id,
                    trace_id = %trace.trace_id,
                    application_id = %request.application_id,
                    ability_id = %request.ability_id,
                    provider_class = "hardened_out_of_process",
                    "WASM hardened provider created execution session"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Session,
                        "created",
                        "hardened_out_of_process",
                    )
                    .trace_id(trace.trace_id.clone())
                    .session_id(session_id.clone()),
                );
                Ok(Box::new(HardenedOutOfProcessWasmExecutionSession {
                    session_id,
                    request,
                    transport: Arc::clone(&self.transport),
                    telemetry: self.telemetry.clone(),
                }))
            }
            health => {
                warn!(
                    trace_id = %trace.trace_id,
                    provider_class = "hardened_out_of_process",
                    reason_code = %health.reason_code(),
                    "WASM hardened provider refused session creation"
                );
                emit_wasm_telemetry(
                    self.telemetry.as_ref(),
                    WasmTelemetryEvent::new(
                        WasmTelemetryStage::Daemon,
                        "rejected",
                        "hardened_out_of_process",
                    )
                    .trace_id(trace.trace_id.clone())
                    .reason_code(health.reason_code()),
                );
                Err(ApplicationAbiError::RuntimeUnavailable(
                    health.safe_reason(),
                ))
            }
        }
    }
}
