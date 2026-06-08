//! Transport Adapter boundary for remote application execution.
//!
//! Concrete transports (HTTP, IPC, message bus, etc.) implement this trait
//! outside the provider Strategy so dispatch semantics stay stable while wiring
//! changes per deployment environment.

use async_trait::async_trait;

use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionProviderLease, ServiceError, StartApplicationExecutionCommand,
};

use super::registration::RemoteAgentRegistration;

/// Transport adapter used to dispatch starts and controls to remote agents.
#[async_trait]
pub trait RemoteAgentExecutionTransport: Send + Sync {
    /// Dispatch a start command over the remote transport boundary.
    async fn start(
        &self,
        agent: RemoteAgentRegistration,
        lease: ApplicationExecutionProviderLease,
        command: StartApplicationExecutionCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError>;

    /// Deliver a control command to a leased remote agent.
    async fn control(
        &self,
        agent: RemoteAgentRegistration,
        lease: ApplicationExecutionProviderLease,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError>;
}

/// Null remote transport used when a composition root has no concrete adapter.
///
/// This Null Object preserves provider wiring while forcing explicit transport
/// configuration before any remote start or control can succeed.
#[derive(Debug, Default)]
pub struct UnavailableRemoteAgentExecutionTransport;

#[async_trait]
impl RemoteAgentExecutionTransport for UnavailableRemoteAgentExecutionTransport {
    async fn start(
        &self,
        _agent: RemoteAgentRegistration,
        _lease: ApplicationExecutionProviderLease,
        _command: StartApplicationExecutionCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError> {
        Err(ServiceError::ServiceUnavailable(
            "remote agent transport adapter is not configured".into(),
        ))
    }

    async fn control(
        &self,
        _agent: RemoteAgentRegistration,
        _lease: ApplicationExecutionProviderLease,
        _command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionCommandStatus, ServiceError> {
        Err(ServiceError::ServiceUnavailable(
            "remote agent transport adapter is not configured".into(),
        ))
    }
}
