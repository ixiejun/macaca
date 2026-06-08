//! Remote-agent application execution provider strategy.
//!
//! Remote agents are external runtime participants that execute under a
//! Macaca-issued scoped lease.  This module keeps remote-agent concerns behind
//! an Adapter/Strategy boundary: registration, health, selection, lease issue,
//! transport dispatch, gateway validation, and control delivery are all generic
//! protocol operations.  No kernel, shell, or application-specific code is
//! imported here.
//!
//! Module layout (Facade + Registry + Adapter + Strategy):
//! - `registration.rs` — admission DTOs validated before registry insert
//! - `transport.rs` — transport Adapter trait and unavailable null object
//! - `registry.rs` — lease-aware agent Registry with capability selection
//! - `provider.rs` — ApplicationExecutionProvider Strategy implementation

mod provider;
mod registration;
mod registry;
mod transport;

pub use provider::RemoteAgentApplicationExecutionProvider;
pub use registration::{RemoteAgentRegistration, RemoteAgentTransportMetadata};
pub use registry::RemoteAgentRegistry;
pub use transport::{
    RemoteAgentExecutionTransport, UnavailableRemoteAgentExecutionTransport,
};
