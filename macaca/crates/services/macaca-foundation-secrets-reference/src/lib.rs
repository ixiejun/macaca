//! Provider-neutral secret-reference service contracts.
//!
//! Composition roots select concrete adapters behind this boundary. SDKs,
//! applications, and WASM guests receive only reference metadata, handles,
//! leases, and sanitized diagnostics; raw secret values never cross this API.

pub mod mock_provider;
pub mod service_contract;

pub use mock_provider::MockSecretsReferenceProvider;
pub use service_contract::{
    SecretsReferenceProviderFactory, SecretsReferenceService, UnavailableSecretsReferenceProvider,
};

#[cfg(test)]
mod tests;
