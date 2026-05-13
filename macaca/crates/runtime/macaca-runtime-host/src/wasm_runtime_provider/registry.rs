//! WASM runtime provider registry.
//!
//! The registry is intentionally tiny: it provides a single provider handle
//! behind the public trait and can later grow into weighted provider selection,
//! policy-aware routing, or remote provider discovery without changing callers.

use std::sync::Arc;

use super::traits::WasmApplicationRuntimeProvider;
use super::unavailable::UnavailableWasmRuntimeProvider;

/// Registry facade for runtime-host provider selection.
#[derive(Clone)]
pub struct WasmRuntimeProviderRegistry {
    provider: Arc<dyn WasmApplicationRuntimeProvider>,
}

impl std::fmt::Debug for WasmRuntimeProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeProviderRegistry")
            .field("provider_class", &self.provider.descriptor().provider_class)
            .finish()
    }
}

impl Default for WasmRuntimeProviderRegistry {
    fn default() -> Self {
        Self::new(Arc::new(UnavailableWasmRuntimeProvider::default()))
    }
}

impl WasmRuntimeProviderRegistry {
    /// Create a registry from an injected provider strategy.
    pub fn new(provider: Arc<dyn WasmApplicationRuntimeProvider>) -> Self {
        Self { provider }
    }

    /// Return the selected provider strategy.
    pub fn provider(&self) -> Arc<dyn WasmApplicationRuntimeProvider> {
        Arc::clone(&self.provider)
    }
}
