//! Public provider/session traits for the WASM runtime boundary.
//!
//! These traits deliberately depend only on provider-neutral DTOs.  That keeps
//! application framework, SDK, and route wiring independent from a specific
//! WASM engine and lets runtime-host swap execution strategies through normal
//! dependency injection.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, TraceContext,
    WasmRuntimeAvailability, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest,
};

/// Strategy plus Abstract Factory boundary implemented by WASM providers.
#[async_trait]
pub trait WasmApplicationRuntimeProvider: Send + Sync {
    /// Return a sanitized descriptor safe for registries, telemetry, and tests.
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor;

    /// Return current availability with optional trace correlation.
    async fn availability(&self, trace: Option<TraceContext>) -> WasmRuntimeAvailability;

    /// Create an execution session from a validated provider-neutral request.
    async fn create_session(
        &self,
        request: WasmRuntimeSessionRequest,
    ) -> Result<Box<dyn WasmExecutionSession>, ApplicationAbiError>;
}

/// Execution session created by a WASM runtime provider.
#[async_trait]
pub trait WasmExecutionSession: Send + Sync + std::fmt::Debug {
    /// Return the stable session id used in logs and result metadata.
    fn session_id(&self) -> &str;

    /// Return the descriptor for the provider that created this session.
    fn descriptor(&self) -> WasmRuntimeProviderDescriptor;

    /// Return sanitized diagnostics describing the current session state.
    fn diagnostics(&self) -> WasmRuntimeDiagnostics;

    /// Dispatch an invocation command through the runtime session.
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError>;
}
