//! Execution session that maps host commands to hardened daemon envelopes.
//!
//! The session never interprets application payloads.  It validates trace
//! context, applies resource timeout gates, forwards sanitized envelopes to the
//! transport Adapter, and maps daemon responses through the response mapper.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    sanitize_wasm_lifecycle_metadata, ApplicationAbiError, ApplicationHostCommand,
    ApplicationHostCommandResult, ApplicationHostCommandStatus, TraceContext,
    WasmCheckpointMemento, WasmExecutionProfile, WasmLifecycleCommand,
    WasmLifecycleOperationStatus, WasmLifecycleReasonCode, WasmLifecycleState,
    WasmLifecycleTransitionResult, WasmRestoreReport, WasmRestoreRequest, WasmRollbackReport,
    WasmRollbackRequest, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmUpgradeReport, WasmUpgradeRequest,
};
use tracing::{info, warn};

use super::response_mapper::{fail_closed_result, map_daemon_response};
use super::HardenedOutOfProcessWasmRuntimeProvider;
use super::super::diagnostics::non_empty_trace;
use super::super::hardened_transport::WasmHardenedTransport;
use super::super::telemetry::{
    emit_wasm_telemetry, WasmTelemetryEvent, WasmTelemetrySinkRef, WasmTelemetryStage,
};
use super::super::traits::WasmExecutionSession;
use super::super::WasmHardenedProviderEnvelope;

/// Execution session that maps commands to daemon envelopes.
#[derive(Debug, Clone)]
pub struct HardenedOutOfProcessWasmExecutionSession {
    pub(super) session_id: String,
    pub(super) request: WasmRuntimeSessionRequest,
    pub(super) transport: Arc<dyn WasmHardenedTransport>,
    pub(super) telemetry: Option<WasmTelemetrySinkRef>,
}

#[async_trait]
impl WasmExecutionSession for HardenedOutOfProcessWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        let availability = macaca_proto::WasmRuntimeAvailability {
            state: "available".into(),
            reason: None,
            capabilities: HardenedOutOfProcessWasmRuntimeProvider {
                transport: Arc::clone(&self.transport),
                telemetry: self.telemetry.clone(),
            }
            .capabilities(),
            diagnostics: None,
            metadata: Default::default(),
        };
        WasmRuntimeProviderDescriptor {
            runtime_kind: macaca_proto::PackageRuntimeKind::WasmComponent,
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
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(
                    WasmTelemetryStage::Daemon,
                    "rejected",
                    "hardened_out_of_process",
                )
                .session_id(self.session_id.clone())
                .reason_code("missing_trace"),
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        if self.request.profile.resources.max_wall_time_ms == Some(0) {
            emit_wasm_telemetry(
                self.telemetry.as_ref(),
                WasmTelemetryEvent::new(
                    WasmTelemetryStage::Daemon,
                    "timeout",
                    "hardened_out_of_process",
                )
                .trace_id(trace.trace_id.clone())
                .session_id(self.session_id.clone())
                .reason_code("timeout"),
            );
            return Ok(fail_closed_result(
                &self.session_id,
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
        emit_wasm_telemetry(
            self.telemetry.as_ref(),
            WasmTelemetryEvent::new(
                WasmTelemetryStage::Daemon,
                "dispatching",
                "hardened_out_of_process",
            )
            .trace_id(trace.trace_id.clone())
            .session_id(self.session_id.clone()),
        );
        let response = self.transport.dispatch(envelope).await;
        Ok(map_daemon_response(
            &self.session_id,
            &self.telemetry,
            response,
            trace,
        ))
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
