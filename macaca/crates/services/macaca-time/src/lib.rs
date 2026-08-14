//! Replaceable time-service contracts and built-in providers.

mod local_provider;
mod service_contract;
mod time_conversion;

pub use local_provider::{FrozenTimeProvider, HostTimeProvider};
pub use service_contract::{TimeService, UnavailableTimeProvider};
