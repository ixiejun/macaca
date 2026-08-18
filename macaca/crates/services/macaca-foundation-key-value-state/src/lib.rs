//! Provider-neutral key-value state service contracts and resource decorators.
//!
//! Composition selects embedded, remote, mock, or unavailable Strategies here.
//! SDKs, applications, and shells issue only traced service commands and never
//! receive database clients, raw values, or provider-native handles.

pub mod embedded_provider;
pub mod mock_provider;
pub mod resource_lease;
pub mod service_contract;

pub use embedded_provider::EmbeddedKeyValueStateProvider;
pub use mock_provider::MockKeyValueStateProvider;
pub use resource_lease::{KeyValueResourceLease, KeyValueResourceLedger};
pub use service_contract::{
    KeyValueStateProviderFactory, KeyValueStateService, UnavailableKeyValueStateProvider,
};

#[cfg(test)]
mod resource_lease_tests;

#[cfg(test)]
mod tests;
