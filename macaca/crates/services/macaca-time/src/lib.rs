//! Replaceable time-service contracts and built-in providers.

mod local_provider;
mod service_contract;

pub use local_provider::{FrozenTimeProvider, HostTimeProvider};
pub use service_contract::{TimeService, UnavailableTimeProvider};
