//! Runtime-host WASM Application Runtime Provider boundary.
//!
//! The module applies a small set of stable infrastructure patterns.  The
//! public traits are the Strategy and Abstract Factory boundary used by host
//! wiring.  [`UnavailableWasmRuntimeProvider`] is the Null Object fallback used
//! when execution is disabled.  [`DefaultInProcessWasmRuntimeProvider`] is the
//! default in-process strategy backed by private adapter/cache modules so no
//! concrete engine detail leaks into `macaca-proto`, the SDK, or application
//! framework call sites.

mod compile_cache;
mod default_provider;
mod diagnostics;
mod engine_adapter;
mod errors;
mod registry;
mod sandbox_guard;
mod traits;
mod unavailable;

pub use default_provider::DefaultInProcessWasmRuntimeProvider;
pub use registry::WasmRuntimeProviderRegistry;
pub use traits::{WasmApplicationRuntimeProvider, WasmExecutionSession};
pub use unavailable::{UnavailableWasmExecutionSession, UnavailableWasmRuntimeProvider};

#[cfg(test)]
mod tests;
