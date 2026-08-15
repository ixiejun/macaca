//! Provider-neutral foundation configuration service.
//!
//! The crate exposes a Command boundary, an in-memory deterministic Strategy
//! for test/replay compositions, and a fail-closed Null Object. Runtime hosts
//! choose provider composition while applications only issue traced commands.

pub mod local_provider;
pub mod service_contract;

pub use local_provider::MockConfigProvider;
pub use service_contract::{ConfigService, UnavailableConfigProvider};

#[cfg(test)]
mod tests;
