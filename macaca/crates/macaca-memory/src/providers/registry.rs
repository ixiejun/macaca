use std::collections::HashMap;

use macaca_proto::MacacaResult;

use super::config::{MemoryProviderProfileConfig, MemoryProviderRuntimeConfig};
use super::factory::{MemoryProviderFactory, MemoryProviderFactoryResult};
use super::slot::{
    MemoryProviderSlotBinding, MemoryProviderSlotKind, MemoryProviderSlotOverride,
    MemoryProviderSlotScope,
};

/// Registry that resolves provider ids from profile and slot overrides.
#[derive(Debug, Clone)]
pub struct MemoryProviderRegistry {
    runtime: MemoryProviderRuntimeConfig,
}

impl MemoryProviderRegistry {
    /// Build a registry from runtime configuration.
    pub fn new(runtime: MemoryProviderRuntimeConfig) -> Self {
        Self { runtime }
    }

    /// Resolve the profile that should be used for a given scope.
    pub fn resolve_profile(
        &self,
        agent_name: Option<&str>,
        session_id: Option<&str>,
    ) -> MemoryProviderProfileConfig {
        if let Some(agent_name) = agent_name {
            if let Some(profile) = self.runtime.profiles.agents.get(agent_name) {
                return profile.clone();
            }
        }
        if let Some(session_id) = session_id {
            if let Some(profile) = self.runtime.profiles.sessions.get(session_id) {
                return profile.clone();
            }
        }
        self.resolve_default_profile()
    }

    /// Resolve a provider id for one slot.
    pub fn resolve_binding(
        &self,
        profile: &MemoryProviderProfileConfig,
        slot: MemoryProviderSlotKind,
        overrides: &[MemoryProviderSlotOverride],
    ) -> Option<MemoryProviderSlotBinding> {
        let override_id = overrides
            .iter()
            .find(|override_item| override_item.slot == slot)
            .map(|override_item| override_item.provider_id.clone());
        let provider_id = override_id.or_else(|| self.profile_slot_id(profile, slot))?;
        Some(MemoryProviderSlotBinding::new(
            provider_id,
            slot,
            Self::scope_from_overrides(overrides, slot),
        ))
    }

    /// Return the currently selected top-level profile name.
    pub fn active_profile_name(&self) -> String {
        if self.runtime.profile.is_empty() {
            self.runtime.profiles.default_profile.clone()
        } else {
            self.runtime.profile.clone()
        }
    }

    /// Resolve the default profile referenced by the runtime.
    pub fn resolve_default_profile(&self) -> MemoryProviderProfileConfig {
        self.runtime
            .profiles
            .profiles
            .get(&self.active_profile_name())
            .cloned()
            .unwrap_or_default()
    }

    /// Resolve a provider config by id.
    pub fn provider_config(
        &self,
        provider_id: &str,
    ) -> Option<&super::config::MemoryProviderConfig> {
        self.runtime.providers.get(provider_id)
    }

    /// Return the provider count so runtime status can summarize the registry.
    pub fn provider_count(&self) -> usize {
        self.runtime.providers.len()
    }

    fn scope_from_overrides(
        overrides: &[MemoryProviderSlotOverride],
        slot: MemoryProviderSlotKind,
    ) -> MemoryProviderSlotScope {
        overrides
            .iter()
            .find(|override_item| override_item.slot == slot)
            .map(|override_item| match override_item.slot {
                MemoryProviderSlotKind::AgentPrivate | MemoryProviderSlotKind::Embedding => {
                    MemoryProviderSlotScope::Agent
                }
                MemoryProviderSlotKind::SessionShared
                | MemoryProviderSlotKind::VectorBackend
                | MemoryProviderSlotKind::ActiveRecall
                | MemoryProviderSlotKind::KnowledgeCompiler => MemoryProviderSlotScope::Session,
            })
            .unwrap_or(MemoryProviderSlotScope::Default)
    }

    fn profile_slot_id(
        &self,
        profile: &MemoryProviderProfileConfig,
        slot: MemoryProviderSlotKind,
    ) -> Option<String> {
        match slot {
            MemoryProviderSlotKind::AgentPrivate => profile.agent_private_provider.clone(),
            MemoryProviderSlotKind::SessionShared => profile.session_shared_provider.clone(),
            MemoryProviderSlotKind::Embedding => profile.embedding_provider.clone(),
            MemoryProviderSlotKind::VectorBackend => profile.vector_backend.clone(),
            MemoryProviderSlotKind::ActiveRecall => profile.active_recall.clone(),
            MemoryProviderSlotKind::KnowledgeCompiler => profile.knowledge_compiler.clone(),
        }
    }

    /// Return a provider config map so runtime adapters can inspect metadata.
    pub fn provider_configs(&self) -> &HashMap<String, super::config::MemoryProviderConfig> {
        &self.runtime.providers
    }
}

/// Errors returned by registry resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryProviderRegistryError {
    MissingProvider { slot: MemoryProviderSlotKind },
}

impl std::fmt::Display for MemoryProviderRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvider { slot } => write!(f, "missing provider for slot {slot:?}"),
        }
    }
}

impl std::error::Error for MemoryProviderRegistryError {}

/// A tiny helper that applies profile-driven selection to a factory.
pub fn resolve_provider<'a, F>(
    factory: &'a F,
    provider_id: &str,
    binding: MemoryProviderSlotBinding,
) -> MacacaResult<MemoryProviderFactoryResult<'a>>
where
    F: MemoryProviderFactory,
{
    factory.build(provider_id, binding)
}
