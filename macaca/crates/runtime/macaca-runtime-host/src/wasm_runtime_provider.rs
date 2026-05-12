//! Runtime-host WASM Application Runtime Provider contract.
//!
//! This module owns the execution-plane side of the WASM runtime boundary.  It
//! intentionally does not integrate a real engine or load guest bytes.  The
//! traits model an Abstract Factory plus Strategy boundary for future runtime
//! providers, while [`UnavailableWasmRuntimeProvider`] is the Null Object used
//! by the base OS when no optional WASM runtime module is installed.  All
//! methods preserve traceability and log only bounded, sanitized metadata.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, PackageRuntimeKind,
    TraceContext, WasmRuntimeAvailability, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmRuntimeUnavailableReason,
};
use tracing::{info, warn};

/// Strategy boundary implemented by unavailable and future real providers.
///
/// The provider is also an Abstract Factory: it reports a descriptor and
/// creates execution sessions from validated provider-neutral requests.  The
/// trait depends only on `macaca-proto` DTOs so concrete engines, process
/// supervisors, remote transports, and provider-specific handles cannot leak
/// into Application Framework or SDK call sites.
#[async_trait]
pub trait WasmApplicationRuntimeProvider: Send + Sync {
    /// Return the provider descriptor safe for registry snapshots and logs.
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor;

    /// Return current availability with optional trace correlation.
    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability;

    /// Create a runtime session or fail closed before any guest code can run.
    async fn create_session(
        &self,
        request: WasmRuntimeSessionRequest,
    ) -> Result<Box<dyn WasmExecutionSession>, ApplicationAbiError>;
}

/// Execution session created by a WASM runtime provider.
///
/// Sessions are the Bridge between host import commands and future engine
/// execution.  Current unavailable sessions never execute guest code; future
/// providers can implement the same trait while preserving trace validation,
/// resource profile visibility, and sanitized diagnostics.
#[async_trait]
pub trait WasmExecutionSession: Send + Sync + std::fmt::Debug {
    /// Return a stable session id derived from traced request metadata.
    fn session_id(&self) -> &str;

    /// Return the descriptor for the provider that created this session.
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor;

    /// Return diagnostics associated with the current session state.
    fn diagnostics(&self) -> WasmRuntimeDiagnostics;

    /// Dispatch a host import command through the session.
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError>;
}

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

    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability {
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
        let trace = request
            .trace
            .clone()
            .ok_or(ApplicationAbiError::MissingTraceContext)?;
        let session_id = format!(
            "{}:{}:{}",
            trace.trace_id, request.application_id, request.ability_id
        );
        warn!(
            trace_id = %trace.trace_id,
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

/// Unavailable session that rejects every command without executing guest code.
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
        let Some(trace) = command.trace.clone() else {
            warn!(
                session_id = %self.session_id,
                import = %command.import,
                runtime_kind = %self.runtime_kind(),
                reason_code = "missing_trace",
                "WASM runtime session rejected untraceable host command"
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        if trace.trace_id.trim().is_empty() {
            return Err(ApplicationAbiError::MissingTraceContext);
        }

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

#[cfg(test)]
mod tests {
    use macaca_proto::{
        ApplicationHostCommand, ApplicationHostCommandStatus, ApplicationImport, TraceContext,
        WasmExecutionProfile, WasmRuntimeArtifactRef, WasmRuntimeSessionRequest,
    };
    use serde_json::json;

    use super::{UnavailableWasmRuntimeProvider, WasmApplicationRuntimeProvider};

    #[tokio::test]
    async fn unavailable_wasm_runtime_provider_rejects_session_without_trace() {
        let provider = UnavailableWasmRuntimeProvider::default();
        let request = WasmRuntimeSessionRequest {
            trace: None,
            application_id: "fixture.application".into(),
            ability_id: "main".into(),
            artifact: WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
            profile: WasmExecutionProfile::default_wasm_component(),
            metadata: Default::default(),
        };

        let error = provider.create_session(request).await.unwrap_err();

        assert_eq!(
            error,
            macaca_proto::ApplicationAbiError::MissingTraceContext
        );
    }

    #[tokio::test]
    async fn unavailable_wasm_runtime_provider_creates_unavailable_session() {
        let provider = UnavailableWasmRuntimeProvider::default();
        let request = WasmRuntimeSessionRequest::new(
            TraceContext::new("trace-wasm-provider"),
            "fixture.application",
            "main",
            WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
            WasmExecutionProfile::default_wasm_component(),
        )
        .unwrap();

        let session = provider.create_session(request).await.unwrap();
        let command = ApplicationHostCommand::with_trace(
            ApplicationImport::ServiceCall,
            json!({"raw_payload": "must_not_escape"}),
            TraceContext::new("trace-wasm-command"),
        );
        let result = session.dispatch(command).await.unwrap();

        assert!(matches!(
            result.status,
            ApplicationHostCommandStatus::RuntimeUnavailable { .. }
        ));
        assert_eq!(
            result.metadata.get("reason_code").map(String::as_str),
            Some("provider_missing")
        );
        assert!(!result
            .metadata
            .values()
            .any(|value| value.contains("must_not_escape")));
    }

    #[tokio::test]
    async fn unavailable_wasm_runtime_provider_reports_sanitized_availability() {
        let provider = UnavailableWasmRuntimeProvider::new("raw payload contains API_KEY");
        let availability = provider.availability(None).await;

        assert!(!availability.is_available());
        let diagnostics = availability.diagnostics.unwrap();
        assert!(diagnostics.is_sanitized());
        assert_eq!(diagnostics.reason_code, "provider_missing");
    }
}
