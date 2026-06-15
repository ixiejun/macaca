//! Context engine configuration: recall, profiles, governance, and external adapters.
//!
//! All types are application-neutral serde surfaces consumed by `macaca-context` and web shells.

mod active_vector_memory;
mod agent_profile;
mod external_adapters;
mod governance;
mod knowledge_digest;
mod provider_families;
mod recall;

pub use active_vector_memory::*;
pub use agent_profile::*;
pub use external_adapters::*;
pub use governance::*;
pub use knowledge_digest::*;
pub use provider_families::*;
pub use recall::*;

use serde::{Deserialize, Serialize};

use super::workspace::WorkspaceGuideSourcesConfig;

/// Top-level context composer configuration section in [`super::MacacaConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_context_engine")]
    pub default_engine: String,
    #[serde(default = "default_context_fallback_engine")]
    pub fallback_engine: String,
    #[serde(default = "default_context_emit_reports")]
    pub emit_reports: bool,
    #[serde(default = "default_context_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_context_reserve_output_tokens")]
    pub reserve_output_tokens: u32,
    #[serde(default)]
    pub workspace_guides: WorkspaceGuideSourcesConfig,
    #[serde(default)]
    pub recall: ContextRecallRuntimeConfig,
    #[serde(default)]
    pub agent_profile: AgentProfileContextConfig,
    #[serde(default)]
    pub active_vector_memory: ActiveVectorMemoryContextConfig,
    /// Governed knowledge digest provider + digest-vs-raw merge (OpenSpec `knowledge-digest-context`).
    #[serde(default)]
    pub knowledge_digest: KnowledgeDigestContextConfig,
    /// Provider-runtime governance: timeouts, redaction, allow/deny, and failure isolation
    /// applied at the [`macaca_context::ContextFacade`] composer boundary.
    #[serde(default)]
    pub governance: ContextGovernanceRuntimeConfig,
    /// Ordered, configuration-driven selection of built-in or registry-backed provider families.
    /// Empty means: use the documented built-in default order inside the catalog assembler.
    #[serde(default)]
    pub provider_families: Vec<ContextProviderFamilyConfig>,
    /// Declarative external context adapters installed into the runtime at startup.
    ///
    /// These rows remain application-agnostic: they describe transport, safety, and fallback
    /// policy only. The web/runtime layer decides how to instantiate each transport and then
    /// overlays the resulting engine into the neutral `ContextEngineRegistry`.
    #[serde(default)]
    pub external_adapters: Vec<ContextExternalAdapterConfig>,
    /// Optional trust promotion rules (`ContextCandidate::trust`) evaluated after redaction/deny
    /// and before composer budgeting — application-neutral pattern matching only.
    #[serde(default)]
    pub trust_governance: ContextTrustGovernanceConfig,
}

fn default_context_engine() -> String {
    "passthrough".into()
}

fn default_context_fallback_engine() -> String {
    "passthrough".into()
}

fn default_context_emit_reports() -> bool {
    true
}

fn default_context_max_tokens() -> u32 {
    120_000
}

fn default_context_reserve_output_tokens() -> u32 {
    4_096
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            default_engine: default_context_engine(),
            fallback_engine: default_context_fallback_engine(),
            emit_reports: default_context_emit_reports(),
            max_tokens: default_context_max_tokens(),
            reserve_output_tokens: default_context_reserve_output_tokens(),
            workspace_guides: WorkspaceGuideSourcesConfig::default(),
            recall: ContextRecallRuntimeConfig::default(),
            agent_profile: AgentProfileContextConfig::default(),
            active_vector_memory: ActiveVectorMemoryContextConfig::default(),
            knowledge_digest: KnowledgeDigestContextConfig::default(),
            governance: ContextGovernanceRuntimeConfig::default(),
            provider_families: Vec::new(),
            external_adapters: Vec::new(),
            trust_governance: ContextTrustGovernanceConfig::default(),
        }
    }
}
