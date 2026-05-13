//! Null Object WASM runtime provider.
//!
//! The unavailable provider is used when no real runtime is configured.  It
//! preserves trace validation, structured diagnostics, and fail-closed command
//! results without attempting to load or execute guest code.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, PackageRuntimeKind,
    WasmRuntimeAvailability, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmRuntimeUnavailableReason,
};
use tracing::{info, warn};

use super::diagnostics::{non_empty_trace, session_id_from_request};
use super::traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};

/// Null Object provider returned when the optional WASM runtime is absent.
#[derive(Debug, Clone)]
pub struct UnavailableWasmRuntimeProvider {
    reason: String,
}

impl UnavailableWasmRuntimeProvider {
    /// Create an unavailable provider with a bounded diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        let reason = WasmRuntimeUnavailableReason::provider_missing(reason).message;
        Self { reason }
    }

    fn unavailable_reason(&self) -> WasmRuntimeUnavailableReason {
        WasmRuntimeUnavailableReason::provider_missing(self.reason.clone())
    }
}

impl Default for UnavailableWasmRuntimeProvider {
    fn default() -> Self {
        Self::new("WASM runtime provider is not installed")
    }
}

#[async_trait]
impl WasmApplicationRuntimeProvider for UnavailableWasmRuntimeProvider {
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        WasmRuntimeProviderDescriptor::unavailable_default()
    }

    async fn availability(
        &self,
        trace: Option<macaca_proto::TraceContext>,
    ) -> WasmRuntimeAvailability {
        let trace_id = trace.as_ref().map(|value| value.trace_id.as_str());
        warn!(
            trace_id = trace_id.unwrap_or("none"),
            runtime_kind = %PackageRuntimeKind::WasmComponent,
            reason_code = "provider_missing",
            "WASM runtime provider reported unavailable availability"
        );
        WasmRuntimeAvailability::unavailable(
            PackageRuntimeKind::WasmComponent,
            self.unavailable_reason(),
            trace,
        )
    }

    async fn create_session(
        &self,
        request: WasmRuntimeSessionRequest,
    ) -> Result<Box<dyn WasmExecutionSession>, ApplicationAbiError> {
        request.validate()?;
        let session_id =
            session_id_from_request(&request).ok_or(ApplicationAbiError::MissingTraceContext)?;
        let trace_id = request
            .trace
            .as_ref()
            .map(|trace| trace.trace_id.as_str())
            .unwrap_or("none");
        warn!(
            trace_id = trace_id,
            application_id = %request.application_id,
            ability_id = %request.ability_id,
            runtime_kind = %request.profile.runtime_kind,
            reason_code = "provider_missing",
            "WASM runtime session created as unavailable Null Object"
        );
        Ok(Box::new(UnavailableWasmExecutionSession {
            session_id,
            request,
            reason: self.unavailable_reason(),
        }))
    }
}

/// Unavailable session that rejects every command without guest execution.
#[derive(Debug, Clone)]
pub struct UnavailableWasmExecutionSession {
    session_id: String,
    request: WasmRuntimeSessionRequest,
    reason: WasmRuntimeUnavailableReason,
}

impl UnavailableWasmExecutionSession {
    fn runtime_kind(&self) -> PackageRuntimeKind {
        self.request.profile.runtime_kind.clone()
    }
}

#[async_trait]
impl WasmExecutionSession for UnavailableWasmExecutionSession {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn descriptor(&self) -> WasmRuntimeProviderDescriptor {
        WasmRuntimeProviderDescriptor::unavailable_default()
    }

    fn diagnostics(&self) -> WasmRuntimeDiagnostics {
        WasmRuntimeDiagnostics::new(
            self.runtime_kind(),
            self.reason.code.clone(),
            self.reason.message.clone(),
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
                import = %command.import,
                runtime_kind = %self.runtime_kind(),
                reason_code = "missing_trace",
                "WASM runtime session rejected untraceable host command"
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        info!(
            session_id = %self.session_id,
            trace_id = %trace.trace_id,
            application_id = %self.request.application_id,
            ability_id = %self.request.ability_id,
            import = %command.import,
            runtime_kind = %self.runtime_kind(),
            reason_code = %self.reason.code,
            "WASM runtime unavailable session returned fail-closed command result"
        );
        let mut result = ApplicationHostCommandResult::runtime_unavailable(
            self.reason.message.clone(),
            Some(trace),
        );
        result
            .metadata
            .insert("runtime_kind".into(), self.runtime_kind().to_string());
        result
            .metadata
            .insert("reason_code".into(), self.reason.code.clone());
        result
            .metadata
            .insert("session_id".into(), self.session_id.clone());
        result
            .metadata
            .insert("application_id".into(), self.request.application_id.clone());
        result
            .metadata
            .insert("ability_id".into(), self.request.ability_id.clone());
        Ok(result)
    }
}
