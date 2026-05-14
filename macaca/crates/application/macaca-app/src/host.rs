//! ApplicationHost facade for Application ABI v0.
//!
//! The host facade is the only application-facing path for ABI imports.  It
//! uses the Facade + Command patterns: applications submit structured commands,
//! while the host validates trace requirements, emits logs, and delegates to a
//! pluggable backend.  The default backend is intentionally conservative and
//! returns structured unavailable results for services not wired in Phase 05.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationImport,
};
use tracing::{info, warn};

use crate::genui::GenUiRuntime;

/// Backend contract used by [`ApplicationHost`] to keep routing pluggable.
///
/// Production web/framework code can supply a backend that writes to EventLog,
/// task services, or app-scoped storage.  Tests and early adopters can use the
/// default unavailable backend without pulling in `macaca-web`.
#[async_trait]
pub trait ApplicationHostBackend: Send + Sync {
    /// Dispatch one trace-validated ABI command.
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError>;
}

/// Null Object backend used when no concrete system service has been wired.
#[derive(Debug, Default)]
pub struct UnavailableApplicationHostBackend;

#[async_trait]
impl ApplicationHostBackend for UnavailableApplicationHostBackend {
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let trace = command.trace.clone();
        let reason = match command.import {
            ApplicationImport::AgentDelegate => {
                "agent delegation backend is not wired for this ApplicationHost"
            }
            ApplicationImport::TaskCreateGoal => {
                "task create-goal backend is not wired for this ApplicationHost"
            }
            ApplicationImport::TaskQuery => {
                "task query backend is not wired for this ApplicationHost"
            }
            ApplicationImport::TraceEmit => "trace backend is not wired for this ApplicationHost",
            ApplicationImport::StorageGet | ApplicationImport::StorageSet => {
                "app-scoped storage backend is not wired for this ApplicationHost"
            }
            ApplicationImport::CapabilityRequest => {
                "capability policy backend is not wired for this ApplicationHost"
            }
            ApplicationImport::UiRender => {
                "GenUI renderer backend is not wired for this ApplicationHost"
            }
            ApplicationImport::PaymentCreateIntent => {
                "payment intent backend belongs to a later Store/payment phase"
            }
            ApplicationImport::ServiceCall => {
                "generic service-call backend is not wired for this ApplicationHost"
            }
            ApplicationImport::Custom(_) => "custom host import is unsupported by this host",
        };
        Ok(ApplicationHostCommandResult::unavailable(reason, trace))
    }
}

/// Application-facing host facade.
pub struct ApplicationHost<B> {
    backend: B,
}

impl<B> ApplicationHost<B>
where
    B: ApplicationHostBackend,
{
    /// Create a host facade around a backend implementation.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Dispatch one host import after enforcing Route C trace requirements.
    ///
    /// Every import currently requires trace context because host imports are
    /// observable system actions.  Rejecting before backend dispatch prevents
    /// untraceable service calls from leaking into lower layers.
    pub async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        if command.trace.is_none() {
            warn!(
                import = %command.import,
                "application host command rejected because trace context is missing"
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        }

        info!(
            import = %command.import,
            "application host command accepted for backend dispatch"
        );
        if matches!(command.import, ApplicationImport::UiRender) {
            info!("application host received GenUI render command");
            // Validate the UiIntent payload before delegating to the backend.
            // This keeps the ABI host boundary traceable and prevents unsafe
            // UI schema from leaking into presentation shells.
            let intent = serde_json::from_value(command.payload.clone()).map_err(|error| {
                ApplicationAbiError::InvalidDeclaration(format!(
                    "invalid GenUI intent payload: {error}"
                ))
            })?;
            GenUiRuntime::new()
                .render_command(intent, command.trace.clone().expect("trace checked above"))
                .map_err(|error| {
                    ApplicationAbiError::InvalidDeclaration(format!(
                        "invalid GenUI render command: {error}"
                    ))
                })?;
        }

        self.backend.dispatch(command).await
    }
}

impl ApplicationHost<UnavailableApplicationHostBackend> {
    /// Create a host that validates commands but reports unavailable backends.
    pub fn unavailable() -> Self {
        Self::new(UnavailableApplicationHostBackend)
    }
}

/// Helper used by tests and simple callers to inspect unavailable results.
pub fn is_unavailable(result: &ApplicationHostCommandResult) -> bool {
    matches!(
        result.status,
        ApplicationHostCommandStatus::Unavailable { .. }
            | ApplicationHostCommandStatus::RuntimeUnavailable { .. }
            | ApplicationHostCommandStatus::Unsupported { .. }
            | ApplicationHostCommandStatus::DisabledByPolicy { .. }
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use macaca_proto::{ApplicationImport, TraceContext};

    use super::*;

    #[tokio::test]
    async fn application_abi_host_rejects_untraceable_commands() {
        let host = ApplicationHost::unavailable();
        let command =
            ApplicationHostCommand::without_trace(ApplicationImport::TraceEmit, json!({}));

        let error = host.dispatch(command).await.unwrap_err();

        assert_eq!(error, ApplicationAbiError::MissingTraceContext);
    }

    #[tokio::test]
    async fn application_abi_host_returns_structured_unavailable() {
        let host = ApplicationHost::unavailable();
        let command = ApplicationHostCommand::with_trace(
            ApplicationImport::PaymentCreateIntent,
            json!({"amount": "1"}),
            TraceContext::new("trace-host"),
        );

        let result = host.dispatch(command).await.unwrap();

        assert!(is_unavailable(&result));
    }
}
