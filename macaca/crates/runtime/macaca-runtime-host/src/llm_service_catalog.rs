//! Sanitized catalog state for `service.llm`.
//!
//! The catalog is a runtime-host Memento used by LLM service commands. It is
//! deliberately separate from concrete provider construction: composition roots
//! can describe configured providers, defaults, and unavailable reason codes
//! without exposing credentials, base URLs, raw provider errors, or prompt data
//! to applications and shells.

use std::collections::BTreeMap;

use macaca_llm::{LlmModelCatalogEntry, LlmProviderProtocolMetadata};
use macaca_proto::config::LlmConfig;

/// Provider-neutral profile used by the runtime-host Strategy wrapper.
///
/// The concrete `LlmProvider` trait intentionally stays small. This profile
/// supplies catalog and protocol metadata at the service boundary without
/// forcing every provider implementation to grow new methods immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmProviderProfile {
    pub provider_id: String,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub healthy: bool,
    pub capabilities: Vec<String>,
    pub protocol: LlmProviderProtocolMetadata,
}

impl LlmProviderProfile {
    /// Build a conservative profile for a generic provider.
    pub fn generic(provider_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        Self {
            provider_id,
            default_model: None,
            models: Vec::new(),
            healthy: true,
            capabilities: vec![
                "chat".into(),
                "model.list".into(),
                "route.resolve".into(),
                "continuation.validate".into(),
                "budget.status".into(),
                "degradation.explain".into(),
            ],
            protocol: LlmProviderProtocolMetadata::default(),
        }
    }

    /// Attach a default model visible through catalog and snapshot commands.
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        if !self.models.contains(&model) {
            self.models.push(model.clone());
        }
        self.default_model = Some(model);
        self
    }

    /// Attach provider protocol metadata.
    pub fn with_protocol(mut self, protocol: LlmProviderProtocolMetadata) -> Self {
        self.protocol = protocol;
        self
    }
}

/// Sanitized multi-provider catalog visible through `service.llm`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmProviderCatalogProfile {
    pub providers: Vec<LlmProviderProfile>,
    pub unavailable_reasons: BTreeMap<String, String>,
}

impl LlmProviderCatalogProfile {
    /// Build a catalog with one healthy provider.
    pub fn single(profile: LlmProviderProfile) -> Self {
        Self {
            providers: vec![profile],
            unavailable_reasons: BTreeMap::new(),
        }
    }

    /// Build a sanitized catalog from persisted LLM configuration.
    pub fn from_config(config: &LlmConfig) -> Self {
        let mut provider_names = config.providers.keys().cloned().collect::<Vec<_>>();
        provider_names.sort();

        let mut providers = Vec::with_capacity(provider_names.len());
        let mut unavailable_reasons = BTreeMap::new();
        for provider_id in provider_names {
            let Some(provider_config) = config.providers.get(&provider_id) else {
                continue;
            };
            let credential_available = provider_config
                .resolve_api_key()
                .map(|key| !key.trim().is_empty())
                .unwrap_or(false);
            let mut profile = LlmProviderProfile::generic(provider_id.clone());
            if let Some(default_model) = provider_config
                .default_model
                .as_ref()
                .filter(|model| !model.trim().is_empty())
            {
                profile = profile.with_default_model(default_model.clone());
            }
            if !credential_available {
                profile.healthy = false;
                unavailable_reasons.insert(provider_id.clone(), "credentials_unavailable".into());
            }
            providers.push(profile);
        }

        if providers.is_empty() {
            providers.push(LlmProviderProfile::generic("unavailable"));
            unavailable_reasons.insert("unavailable".into(), "provider_catalog_empty".into());
        }

        Self {
            providers,
            unavailable_reasons,
        }
    }

    /// Return catalog rows after applying the caller's disabled-row preference.
    pub fn visible_profiles(&self, include_disabled: bool) -> Vec<LlmProviderProfile> {
        self.providers
            .iter()
            .filter(|profile| include_disabled || profile.healthy)
            .cloned()
            .collect()
    }

    /// Read one provider profile by id for route diagnostics.
    pub(crate) fn provider(&self, provider_id: &str) -> Option<LlmProviderProfile> {
        self.providers
            .iter()
            .find(|profile| profile.provider_id == provider_id)
            .cloned()
    }

    /// Return the sanitized unavailable reason code for one provider.
    pub(crate) fn unavailable_reason(&self, provider_id: &str) -> Option<String> {
        self.unavailable_reasons.get(provider_id).cloned()
    }
}

/// Expand a provider profile into application-visible model rows.
pub(crate) fn catalog_models(
    profile: &LlmProviderProfile,
    unavailable_reason: Option<String>,
) -> Vec<LlmModelCatalogEntry> {
    let mut models = profile.models.clone();
    if models.is_empty() {
        if let Some(default_model) = &profile.default_model {
            models.push(default_model.clone());
        }
    }
    models
        .into_iter()
        .map(|model| LlmModelCatalogEntry {
            provider_id: profile.provider_id.clone(),
            display_name: model.clone(),
            model,
            available: profile.healthy && unavailable_reason.is_none(),
            unavailable_reason: unavailable_reason.clone(),
            service_tiers: if profile.protocol.supports_service_tiers {
                vec!["default".into()]
            } else {
                Vec::new()
            },
            reasoning_efforts: if profile.protocol.supports_reasoning {
                vec!["low".into(), "medium".into(), "high".into()]
            } else {
                Vec::new()
            },
            protocol: profile.protocol.clone(),
        })
        .collect()
}
