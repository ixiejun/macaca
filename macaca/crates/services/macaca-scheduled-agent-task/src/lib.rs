//! Scheduled Agent Task service crate for Macaca Agent OS.
//!
//! This crate owns the service-facing capability that turns durable user intent
//! into scheduled agent execution.  It deliberately sits between presentation
//! adapters/entry-agent tools and Scheduler: the service accepts prompt
//! material, stores it as an owned payload memento, registers a Scheduler job
//! with only an `AutonomyPayloadRef`, and later resolves that payload for
//! Runtime Host when a due run is dispatched to `service.agent_execution`.
//! Neither Scheduler nor Web/frontend should own prompt storage or
//! application-specific task semantics.

pub mod audit;
pub mod local_provider;
mod payload_store;
pub mod service_contract;

pub use audit::*;
pub use local_provider::*;
pub use service_contract::*;
