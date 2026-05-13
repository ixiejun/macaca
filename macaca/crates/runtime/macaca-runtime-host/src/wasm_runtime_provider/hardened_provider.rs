//! Hardened out-of-process WASM runtime provider.
//!
//! The provider is a Strategy implementation for deployments that require a
//! process boundary around guest execution.  Runtime-host still owns policy,
//! trace validation, diagnostics, lifecycle projection, and provider selection;
//! the daemon transport only receives sanitized command envelopes and returns
//! provider-neutral responses.  That split keeps hardened execution flexible
//! without turning daemon details into public ABI or kernel-owned lifecycle.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    sanitize_wasm_lifecycle_metadata, ApplicationAbiError, ApplicationHostCommand,
    ApplicationHostCommandResult, ApplicationHostCommandStatus, PackageRuntimeKind, TraceContext,
    WasmCheckpointMemento, WasmEngineCapabilities, WasmExecutionProfile, WasmLifecycleCommand,
    WasmLifecycleOperationStatus, WasmLifecycleReasonCode, WasmLifecycleState,
    WasmLifecycleTransitionResult, WasmRestoreReport, WasmRestoreRequest, WasmRollbackReport,
    WasmRollbackRequest, WasmRuntimeAvailability, WasmRuntimeDiagnostics,
    WasmRuntimeProviderDescriptor, WasmRuntimeSessionRequest, WasmRuntimeUnavailableReason,
    WasmUpgradeReport, WasmUpgradeRequest,
};
use tracing::{info, warn};

use super::diagnostics::non_empty_trace;
use super::hardened_transport::{
    sanitize_hardened_text, sanitize_label, WasmHardenedHealth, WasmHardenedTransport,
};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};
use super::{WasmHardenedProviderEnvelope, WasmHardenedProviderResponse};

/// Runtime-host provider that dispatches execution to a hardened daemon.
#[derive(Debug, Clone)]
pub struct HardenedOutOfProcessWasmRuntimeProvider {
    transport: Arc<dyn WasmHardenedTransport>,
}

impl HardenedOutOfProcessWasmRuntimeProvider {
    /// Create a provider around a concrete daemon transport adapter.
    ///
    /// The constructor accepts a trait object so tests, local daemons, socket
    /// transports, or remote hardened executors can be swapped without changing
    /// the public `WasmApplicationRuntimeProvider` contract.
    #[allow(dead_code)]
    pub(crate) fn new(transport: Arc<dyn WasmHardenedTransport>) -> Self {
        Self { transport }
    }

    fn capabilities(&self) -> WasmEngineCapabilities {
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
                WasmRuntimeAvailability {
                    state: "available".into(),
                    reason: None,
                    capabilities: self.capabilities(),
                    diagnostics: None,
                    metadata: Default::default(),
                }
            }
            health => WasmRuntimeAvailability::unavailable(
                PackageRuntimeKind::WasmComponent,
                WasmRuntimeUnavailableReason::provider_missing(health.safe_reason()),
                Some(trace),
            ),
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
        let session_id = format!("hardened-session-{}", sanitize_label(trace.trace_id.clone()));
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
                Ok(Box::new(HardenedOutOfProcessWasmExecutionSession {
                    session_id,
                    request,
                    transport: Arc::clone(&self.transport),
                }))
            }
            health => {
                warn!(
                    trace_id = %trace.trace_id,
                    provider_class = "hardened_out_of_process",
                    reason_code = %health.reason_code(),
                    "WASM hardened provider refused session creation"
                );
                Err(ApplicationAbiError::RuntimeUnavailable(health.safe_reason()))
            }
        }
    }
}

/// Execution session that maps commands to daemon envelopes.
#[derive(Debug, Clone)]
struct HardenedOutOfProcessWasmExecutionSession {
    session_id: String,
    request: WasmRuntimeSessionRequest,
    transport: Arc<dyn WasmHardenedTransport>,
}

#[async_trait]
impl WasmExecutionSession for HardenedOutOfProcessWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        let availability = WasmRuntimeAvailability {
            state: "available".into(),
            reason: None,
            capabilities: HardenedOutOfProcessWasmRuntimeProvider {
                transport: Arc::clone(&self.transport),
            }
            .capabilities(),
            diagnostics: None,
            metadata: Default::default(),
        };
        WasmRuntimeProviderDescriptor {
            runtime_kind: PackageRuntimeKind::WasmComponent,
            provider_class: "hardened_out_of_process".into(),
            capabilities: availability.capabilities.clone(),
            default_profile: WasmExecutionProfile::default_wasm_component(),
            availability,
            diagnostics: None,
            metadata: Default::default(),
        }
    }

    fn diagnostics(&self) -> WasmRuntimeDiagnostics {
        WasmRuntimeDiagnostics::new(
            self.request.profile.runtime_kind.clone(),
            "ready",
            "WASM hardened out-of-process session is ready",
            self.request.trace.clone(),
        )
    }

    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let Some(trace) = non_empty_trace(command.trace.clone()) else {
            warn!(
                session_id = %self.session_id,
                provider_class = "hardened_out_of_process",
                reason_code = "missing_trace",
                "WASM hardened provider rejected untraceable command"
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        if self.request.profile.resources.max_wall_time_ms == Some(0) {
            return Ok(self.fail_closed_result(
                "timeout",
                "hardened daemon dispatch timed out before execution",
                trace,
                ApplicationHostCommandStatus::RuntimeUnavailable {
                    reason: "hardened daemon dispatch timed out".into(),
                },
            ));
        }

        let operation = command
            .metadata
            .get("wasm.operation")
            .cloned()
            .unwrap_or_else(|| command.import.to_string());
        let envelope = WasmHardenedProviderEnvelope::new(
            trace.trace_id.clone(),
            self.session_id.clone(),
            operation,
        )
        .diagnostics_level("sanitized");
        info!(
            session_id = %self.session_id,
            trace_id = %trace.trace_id,
            provider_class = "hardened_out_of_process",
            "WASM hardened provider dispatching daemon envelope"
        );
        let response = self.transport.dispatch(envelope).await;
        Ok(self.map_response(response, trace))
    }

    fn lifecycle_state(&self) -> WasmLifecycleState {
        WasmLifecycleState::Started
    }

    async fn transition_lifecycle(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmLifecycleTransitionResult, ApplicationAbiError> {
        let trace = command
            .require_trace()
            .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
        let mut metadata = command.metadata;
        metadata.insert("provider_class".into(), "hardened_out_of_process".into());
        metadata.insert("session_id".into(), self.session_id.clone());
        Ok(WasmLifecycleTransitionResult::completed(
            command.operation,
            WasmLifecycleState::Started,
            WasmLifecycleState::Started,
            Some(trace),
            metadata,
        ))
    }

    async fn checkpoint(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
        let trace = command
            .require_trace()
            .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
        Ok(WasmCheckpointMemento {
            checkpoint_id: format!("hardened-checkpoint-{}", self.session_id),
            session_id: self.session_id.clone(),
            application_id: self.request.application_id.clone(),
            ability_id: self.request.ability_id.clone(),
            lifecycle_state: WasmLifecycleState::Started,
            artifact: self.request.artifact.clone(),
            artifact_digest_prefix: "hardened".into(),
            abi_version: self.request.profile.abi_version.clone(),
            created_at: Utc::now(),
            trace: Some(trace),
            metadata: sanitize_wasm_lifecycle_metadata(command.metadata),
        })
    }

    async fn restore(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError> {
        let trace = request
            .trace
            .ok_or(ApplicationAbiError::MissingTraceContext)?;
        Ok(WasmRestoreReport {
            status: WasmLifecycleOperationStatus::Completed,
            reason_code: WasmLifecycleReasonCode::Completed.as_code().into(),
            checkpoint_id: request.checkpoint.checkpoint_id,
            lifecycle_state: request.checkpoint.lifecycle_state,
            trace: Some(trace),
            metadata: sanitize_wasm_lifecycle_metadata(request.metadata),
        })
    }

    async fn upgrade(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError> {
        let trace = request
            .trace
            .ok_or(ApplicationAbiError::MissingTraceContext)?;
        Ok(WasmUpgradeReport {
            status: WasmLifecycleOperationStatus::Completed,
            reason_code: WasmLifecycleReasonCode::Completed.as_code().into(),
            source_artifact: self.request.artifact.clone(),
            source_artifact_digest_prefix: String::new(),
            target_artifact: request.target_artifact,
            target_artifact_digest_prefix: request
                .target_artifact_digest
                .chars()
                .take(12)
                .collect(),
            abi_compatible: true,
            trace: Some(trace),
            metadata: sanitize_wasm_lifecycle_metadata(request.metadata),
        })
    }

    async fn rollback(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError> {
        let restore = self
            .restore(WasmRestoreRequest {
                checkpoint: request.checkpoint,
                trace: request.trace,
                metadata: request.metadata,
            })
            .await?;
        Ok(WasmRollbackReport {
            status: restore.status,
            reason_code: restore.reason_code,
            checkpoint_id: restore.checkpoint_id,
            restored_lifecycle_state: restore.lifecycle_state,
            trace: restore.trace,
            metadata: restore.metadata,
        })
    }
}

impl HardenedOutOfProcessWasmExecutionSession {
    fn map_response(
        &self,
        response: WasmHardenedProviderResponse,
        trace: TraceContext,
    ) -> ApplicationHostCommandResult {
        let expected_trace = trace.trace_id.as_str();
        if response.trace_id != expected_trace || response.request_id != self.session_id {
            warn!(
                session_id = %self.session_id,
                trace_id = %trace.trace_id,
                provider_class = "hardened_out_of_process",
                reason_code = "malformed_response",
                "WASM hardened provider rejected malformed daemon response"
            );
            return self.fail_closed_result(
                "malformed_response",
                "hardened daemon returned malformed response",
                trace,
                ApplicationHostCommandStatus::RuntimeUnavailable {
                    reason: "hardened daemon returned malformed response".into(),
                },
            );
        }
        let mut result = match response.status {
            ApplicationHostCommandStatus::Ok => {
                ApplicationHostCommandResult::ok(serde_json::Value::Null, Some(trace))
            }
            status => ApplicationHostCommandResult {
                status,
                output: serde_json::Value::Null,
                trace: Some(trace),
                policy: None,
                metadata: Default::default(),
            },
        };
        result.metadata.extend(response.metadata);
        result
            .metadata
            .insert("reason_code".into(), sanitize_label(response.reason_code));
        result
            .metadata
            .insert("provider_class".into(), "hardened_out_of_process".into());
        result
            .metadata
            .insert("session_id".into(), self.session_id.clone());
        if !response.diagnostics.is_empty() {
            result.metadata.insert(
                "diagnostics".into(),
                sanitize_hardened_text(response.diagnostics.join(";")),
            );
        }
        result
    }

    fn fail_closed_result(
        &self,
        reason_code: &str,
        reason: &str,
        trace: TraceContext,
        status: ApplicationHostCommandStatus,
    ) -> ApplicationHostCommandResult {
        let mut result = ApplicationHostCommandResult {
            status,
            output: serde_json::Value::Null,
            trace: Some(trace),
            policy: None,
            metadata: Default::default(),
        };
        result
            .metadata
            .insert("reason_code".into(), reason_code.into());
        result
            .metadata
            .insert("provider_class".into(), "hardened_out_of_process".into());
        result
            .metadata
            .insert("session_id".into(), self.session_id.clone());
        result
            .metadata
            .insert("diagnostics".into(), sanitize_hardened_text(reason));
        result
    }
}
