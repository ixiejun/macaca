//! Public provider/session traits for the WASM runtime boundary.
//!
//! These traits deliberately depend only on provider-neutral DTOs.  That keeps
//! application framework, SDK, and route wiring independent from a specific
//! WASM engine and lets runtime-host swap execution strategies through normal
//! dependency injection.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult, TraceContext,
    WasmCheckpointMemento, WasmLifecycleCommand, WasmLifecycleState, WasmLifecycleTransitionResult,
    WasmRestoreReport, WasmRestoreRequest, WasmRollbackReport, WasmRollbackRequest,
    WasmRuntimeAvailability, WasmRuntimeDiagnostics, WasmRuntimeProviderDescriptor,
    WasmRuntimeSessionRequest, WasmUpgradeReport, WasmUpgradeRequest,
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

    /// Return the current provider-owned lifecycle state for this session.
    ///
    /// The lifecycle state is exposed as provider-neutral data so callers can
    /// audit long-running application behavior without depending on a concrete
    /// engine handle or process model.
    fn lifecycle_state(&self) -> WasmLifecycleState;

    /// Execute one typed lifecycle transition command.
    ///
    /// Implementations must fail closed for missing trace, unsupported engine
    /// operations, invalid transitions, and runtime failures.  The default
    /// provider maps supported operations to Application ABI exports, while
    /// unavailable providers return structured unavailable transition results.
    async fn transition_lifecycle(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmLifecycleTransitionResult, ApplicationAbiError>;

    /// Create a sanitized checkpoint memento for the current session.
    ///
    /// Checkpoints are metadata mementos only.  They must not contain raw guest
    /// memory, raw command payloads, environment variables, prompts, secrets, or
    /// provider-specific engine handles.
    async fn checkpoint(
        &self,
        command: WasmLifecycleCommand,
    ) -> Result<WasmCheckpointMemento, ApplicationAbiError>;

    /// Restore session lifecycle metadata from a compatible checkpoint memento.
    async fn restore(
        &self,
        request: WasmRestoreRequest,
    ) -> Result<WasmRestoreReport, ApplicationAbiError>;

    /// Evaluate an upgrade request against artifact and ABI compatibility metadata.
    async fn upgrade(
        &self,
        request: WasmUpgradeRequest,
    ) -> Result<WasmUpgradeReport, ApplicationAbiError>;

    /// Evaluate a rollback request against checkpoint and ABI compatibility metadata.
    async fn rollback(
        &self,
        request: WasmRollbackRequest,
    ) -> Result<WasmRollbackReport, ApplicationAbiError>;
}
