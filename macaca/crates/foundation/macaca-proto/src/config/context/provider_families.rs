//! Configuration rows for built-in or registry-backed context provider families.

use serde::{Deserialize, Serialize};

/// One configured provider **family** row. `family_id` matches stable neutral keys consumed by
/// [`macaca_context::catalog`] (for example `agent_profile`, `skill_capability`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextProviderFamilyConfig {
    pub family_id: String,
    #[serde(default = "default_context_provider_family_enabled")]
    pub enabled: bool,
    /// Opaque JSON parameters forwarded to [`macaca_context::ProviderFactoryInput::params`].
    #[serde(default)]
    pub params: serde_json::Value,
}

fn default_context_provider_family_enabled() -> bool {
    true
}
