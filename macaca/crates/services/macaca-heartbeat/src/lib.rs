//! Heartbeat service crate for Macaca Agent OS.
//!
//! This crate owns the service-facing Heartbeat contract.  It does not own task
//! planning, agent execution, notifications, or business workflows.  Concrete
//! providers may coalesce wake requests and evaluate gates, but they must
//! dispatch work through declared service capabilities rather than importing
//! shell or application-specific logic.

pub mod local_provider;
pub mod service_contract;

pub use local_provider::*;
pub use service_contract::*;
