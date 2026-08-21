//! Optional remote/replay bridge for the foundation session-state service.
//!
//! The bridge is intentionally owned by runtime-host: remote transports are
//! composition concerns, while callers continue to use the provider-neutral
//! `ServiceCommand`/`ServiceCallResult` contract. Implementations may use a
//! network, plugin, or replay log, but no transport handle or raw payload is
//! exposed to SDK, shell, or application layers.

use async_trait::async_trait;
use macaca_proto::{ServiceCallResult, ServiceCommand, ServiceError, ServiceResult};
use tracing::warn;

/// Runtime-host Strategy for a remote or replay-backed session-state service.
#[async_trait]
pub trait SessionStateRemoteStore: Send + Sync {
    /// Forward one already-traced, provider-neutral service command.
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult>;

    /// Return a bounded, sanitized transport diagnostic for health reporting.
    fn diagnostic(&self) -> SessionStateRemoteDiagnostic;
}

/// Sanitized bridge health facts; transport identity and endpoint details stay private.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateRemoteDiagnostic {
    pub available: bool,
    pub replay_supported: bool,
    pub reason_code: Option<String>,
}

/// Null Object Strategy used when no remote/replay module is installed.
#[derive(Debug, Clone)]
pub struct UnavailableSessionStateRemoteStore {
    reason_code: String,
}

impl UnavailableSessionStateRemoteStore {
    pub fn new(reason_code: impl Into<String>) -> Self {
        Self {
            reason_code: reason_code.into(),
        }
    }
}

#[async_trait]
impl SessionStateRemoteStore for UnavailableSessionStateRemoteStore {
    async fn call(&self, _command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        warn!(reason_code = %self.reason_code, "remote session state bridge unavailable");
        Err(ServiceError::ServiceUnavailable(self.reason_code.clone()))
    }

    fn diagnostic(&self) -> SessionStateRemoteDiagnostic {
        SessionStateRemoteDiagnostic {
            available: false,
            replay_supported: false,
            reason_code: Some(self.reason_code.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{ServiceCommandName, TraceContext};

    #[tokio::test]
    async fn absent_remote_module_fails_closed_without_transport_details() {
        let bridge = UnavailableSessionStateRemoteStore::new("remote_module_absent");
        let error = bridge
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("session_state.inspect_recovery"),
                serde_json::json!({"session": {"session_id": "opaque"}}),
                TraceContext::new("remote-unavailable"),
            ))
            .await
            .unwrap_err();
        assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
        assert_eq!(
            bridge.diagnostic().reason_code.as_deref(),
            Some("remote_module_absent")
        );
    }
}
