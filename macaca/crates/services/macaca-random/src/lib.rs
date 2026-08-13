//! Randomness service for Macaca Agent OS.
//!
//! The crate owns provider-neutral service dispatch and replaceable providers.
//! Runtime-host composition chooses the provider; SDK, shells, applications,
//! and the kernel only see typed service commands. The service never logs raw
//! generated values, seeds, or provider payloads.

pub mod local_provider;
pub mod service_contract;

pub use local_provider::{DeterministicRandomProvider, HostRandomProvider};
pub use service_contract::{RandomService, UnavailableRandomProvider};
