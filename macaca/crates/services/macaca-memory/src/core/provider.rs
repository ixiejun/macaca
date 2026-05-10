use serde::{Deserialize, Serialize};

use super::facade::MemoryFacade;
use super::status::MemoryCapabilitySet;

/// Provider metadata exposed to callers, diagnostics, and future registries.
///
/// The descriptor is intentionally lightweight: it tells the system which
/// provider is active and what it can do without leaking backend-specific
/// implementation details into every call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub capabilities: MemoryCapabilitySet,
}

impl MemoryProviderDescriptor {
    /// Create a descriptor for a concrete provider implementation.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        capabilities: MemoryCapabilitySet,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            capabilities,
        }
    }
}

/// A discoverable memory provider.
///
/// `MemoryProvider` extends the operational `MemoryFacade` with descriptor
/// metadata so routers or registries can reason about provider identity and
/// capabilities without coupling to a concrete type.
pub trait MemoryProvider: MemoryFacade {
    fn descriptor(&self) -> MemoryProviderDescriptor;
}
