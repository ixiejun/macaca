//! SystemFacade extension for application execution clients.
//!
//! This module keeps the main `SystemFacade` type from growing into a God
//! object while still exposing a discoverable SDK entrypoint.  The extension
//! applies the Facade pattern: callers receive focused application-execution
//! clients, while provider construction and service registration remain owned
//! by runtime-host composition roots.

use std::sync::Arc;

use crate::application_execution_client::{
    ServiceBackedApplicationExecutionClient, UnavailableSystemApplicationExecutionClient,
};
use crate::service_client::SystemServiceClient;
use crate::system_facade::SystemFacade;

/// Extension methods that attach application execution clients to
/// `SystemFacade` without importing runtime-host internals into the SDK.
pub trait SystemApplicationExecutionFacadeExt {
    /// Build the Null Object client used when no service dispatcher is wired.
    ///
    /// Returning a structured unavailable client is intentional.  It keeps
    /// shells and applications on the same typed protocol even before the
    /// runtime-host provider stack is available.
    fn unavailable_application_execution_client(
        &self,
    ) -> UnavailableSystemApplicationExecutionClient;

    /// Build a service-backed client from an injected generic service client.
    ///
    /// The dispatcher is supplied by an approved composition root.  This SDK
    /// helper only wraps it in a focused Facade and never constructs providers,
    /// EventLog instances, runtimes, transports, or application workflows.
    fn application_execution_client_from_service(
        service: Arc<dyn SystemServiceClient>,
    ) -> ServiceBackedApplicationExecutionClient;
}

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
    SystemApplicationExecutionFacadeExt
    for SystemFacade<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
{
    fn unavailable_application_execution_client(
        &self,
    ) -> UnavailableSystemApplicationExecutionClient {
        UnavailableSystemApplicationExecutionClient::new()
    }

    fn application_execution_client_from_service(
        service: Arc<dyn SystemServiceClient>,
    ) -> ServiceBackedApplicationExecutionClient {
        ServiceBackedApplicationExecutionClient::new(service)
    }
}
