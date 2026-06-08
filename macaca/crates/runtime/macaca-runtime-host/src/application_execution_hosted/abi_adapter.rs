//! Application ABI bridge for hosted execution (Adapter pattern).
//!
//! Converts `StartApplicationExecutionCommand` into bounded `ApplicationHostCommand`
//! envelopes using the existing Application ABI host runtime. Start dispatch uses the
//! generic WASM export-invoke command so app-specific execution behavior remains in
//! the component artifact and declared host imports rather than in Macaca OS service code.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationHostCommandStatus, ApplicationImport, ServiceError,
    StartApplicationExecutionCommand,
};
use tracing::info;

use crate::application_hosts::ApplicationHostRuntime;

use super::host_signal_translator::hosted_signals_from_host_result;
use super::types::{
    adapter_error, HostedApplicationExecutionAdapter, HostedApplicationExecutionOutcome,
};

/// Application ABI bridge for hosted execution.
///
/// This Adapter converts protocol start/control/resume commands into bounded host
/// commands. Production composition roots inject a real `ApplicationHostRuntime`
/// without teaching the service layer about WASM bytes, YAML workflows, or product domains.
pub struct ApplicationAbiHostedExecutionAdapter {
    host: Arc<dyn ApplicationHostRuntime>,
}

impl ApplicationAbiHostedExecutionAdapter {
    /// Build an ABI-backed hosted adapter wired to one application host runtime.
    pub fn new(host: Arc<dyn ApplicationHostRuntime>) -> Self {
        info!("application abi hosted execution adapter constructed");
        Self { host }
    }
}

#[async_trait]
impl HostedApplicationExecutionAdapter for ApplicationAbiHostedExecutionAdapter {
    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError> {
        // Child WASM host imports need the user-visible application-execution scope,
        // not the provider-private runtime session. Enrich trace before crossing into
        // Application Service so any WASM/YAML/GenUI application reuses the same
        // replay and audit correlation path.
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

        // Hosted execution starts are represented as a generic WASM export invocation.
        // Application Service resolves the app-scoped WASM session, invokes `app:start`,
        // and routes declared host commands through the host-import bridge.
        let mut host_command = macaca_proto::ApplicationHostCommand::with_trace(
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
                // A successful `app:start` only proves the application boundary accepted
                // the envelope — not a terminal task result. Return `Running` with a generic
                // checkpoint memento so replay/audit does not lie about unfinished work.
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
        let host_command = macaca_proto::ApplicationHostCommand::with_trace(
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
        snapshot: macaca_proto::ApplicationExecutionSnapshot,
    ) -> Result<HostedApplicationExecutionOutcome, ServiceError> {
        let host_command = macaca_proto::ApplicationHostCommand::with_trace(
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
