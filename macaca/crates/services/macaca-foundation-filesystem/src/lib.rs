//! Provider-neutral filesystem service contracts and deterministic test providers.
//!
//! Runtime composition selects a local, remote, plugin, mock, or unavailable
//! provider behind this crate. Application code, SDK Facades, and shells issue
//! only traced commands and never receive a host path or native file handle.

pub mod mock_provider;
pub mod service_contract;

pub use mock_provider::MockFilesystemProvider;
pub use service_contract::{FilesystemService, UnavailableFilesystemProvider};

#[cfg(test)]
mod tests;
