use serde::{Deserialize, Serialize};

use super::facade::MemoryFacade;
use super::status::MemoryCapabilitySet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderDescriptor {
    pub id: String,
    pub display_name: String,
    pub capabilities: MemoryCapabilitySet,
}

impl MemoryProviderDescriptor {
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

pub trait MemoryProvider: MemoryFacade {
    fn descriptor(&self) -> MemoryProviderDescriptor;
}
