//! Runtime-host WASM Application Runtime Provider boundary.
//!
//! The module applies a small set of stable infrastructure patterns.  The
//! public traits are the Strategy and Abstract Factory boundary used by host
//! wiring.  [`UnavailableWasmRuntimeProvider`] is the Null Object fallback used
//! when execution is disabled.  [`DefaultInProcessWasmRuntimeProvider`] is the
//! default in-process strategy backed by private adapter/cache modules so no
//! concrete engine detail leaks into `macaca-proto`, the SDK, or application
//! framework call sites.

mod certification;
mod certification_hardened;
mod compile_cache;
mod default_provider;
mod diagnostics;
mod engine_adapter;
mod errors;
mod guest_harness;
mod guest_harness_support;
mod host_import_bridge;
mod lifecycle_support;
mod registry;
mod sandbox_guard;
mod traits;
mod unavailable;

pub use certification::{
    WasmCertificationFixtureSet, WasmCertificationHarness, WasmCertificationProfile,
    WasmCertificationReport, WasmCertificationStatus, WasmConformanceFixtureKind,
};
pub use certification_hardened::{
    WasmHardenedProviderEnvelope, WasmHardenedProviderMockAdapter, WasmHardenedProviderResponse,
};
pub use default_provider::DefaultInProcessWasmRuntimeProvider;
pub use guest_harness::{
    WasmExampleFixtureKind, WasmGuestHarnessFixture, WasmGuestHarnessReport,
    WasmGuestRuntimeHarness, WasmMockHostOutcome, WasmToolchainFixtureReport,
};
pub use host_import_bridge::{WasmHostImportBridge, WasmHostImportBridgeConfig};
pub use registry::WasmRuntimeProviderRegistry;
pub use traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};
pub use unavailable::{UnavailableWasmExecutionSession, UnavailableWasmRuntimeProvider};

#[cfg(test)]
mod certification_tests;
#[cfg(test)]
mod guest_harness_tests;
#[cfg(test)]
mod host_import_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod tests;
