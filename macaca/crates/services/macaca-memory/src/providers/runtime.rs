use super::config::MemoryProviderRuntimeConfig;
use super::registry::MemoryProviderRegistry;
use super::slot::MemoryProviderSlotKind;

/// Lightweight runtime status for provider orchestration.
#[derive(Debug, Clone, Default)]
pub struct MemoryProviderRuntimeStatus {
    pub profile: String,
    pub provider_count: usize,
    pub active_providers: Vec<String>,
    pub diagnostics: Vec<String>,
}

/// Provider runtime wrapper around the registry.
#[derive(Debug, Clone)]
pub struct MemoryProviderRuntime {
    registry: MemoryProviderRegistry,
}

impl MemoryProviderRuntime {
    pub fn new(config: MemoryProviderRuntimeConfig) -> Self {
        Self {
            registry: MemoryProviderRegistry::new(config),
        }
    }

    pub fn status(&self) -> MemoryProviderRuntimeStatus {
        let profile = self.registry.active_profile_name();
        let provider_count = self.registry.provider_count();
        MemoryProviderRuntimeStatus {
            profile: profile.clone(),
            provider_count,
            active_providers: vec![
                format!("profile:{profile}"),
                format!("slot:{:?}", MemoryProviderSlotKind::AgentPrivate),
                format!("slot:{:?}", MemoryProviderSlotKind::SessionShared),
            ],
            diagnostics: Vec::new(),
        }
    }
}
