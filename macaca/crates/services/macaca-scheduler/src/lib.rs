//! Scheduler service crate for Macaca Agent OS.
//!
//! This crate owns the service-facing Scheduler contract.  It deliberately does
//! not implement cron parsing, timer loops, persistence, or concrete dispatch.
//! Those runtime behaviors belong to replaceable providers registered by the
//! runtime-host composition root.  Keeping this crate contract-only makes it
//! safe for SDK, runtime, tests, and future plugin providers to depend on the
//! same boundary without importing shell or application-specific code.

pub mod service_contract;
pub mod local_provider;

pub use local_provider::*;
pub use service_contract::*;
