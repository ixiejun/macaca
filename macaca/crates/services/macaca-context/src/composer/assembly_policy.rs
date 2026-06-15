//! Policy bundle for [`super::facade::ContextFacade`] — keeps governance knobs separate from
//! transport-level context while allowing a single entrypoint for callers (Strategy objects).

use macaca_proto::config::{
    ContextGovernanceRuntimeConfig, ContextTrustGovernanceConfig, KnowledgeDigestContextConfig,
};

/// Full assembly policy: governance pipeline + optional trust promotions.
///
/// ### Why a struct instead of multiple `Option` parameters?
/// - **Facade** stays stable as new neutral policy dimensions appear (future retention rules, etc.).
/// - Call sites pass one value built from merged `ContextConfig` without positional confusion.
#[derive(Debug, Clone)]
pub struct ContextFacadeAssemblyPolicy {
    /// Timeouts, redaction, deny-prefix — evaluated inside the governed provider pipeline when
    /// `enabled` is true.
    pub governance: ContextGovernanceRuntimeConfig,
    /// Optional: when `promotions` is empty, treat as `None` at the facade (no trust work).
    pub trust_governance: Option<ContextTrustGovernanceConfig>,
    /// Knowledge digest provider + digest-vs-raw merge (`ContextConfig::knowledge_digest`).
    pub knowledge_digest: KnowledgeDigestContextConfig,
}

impl Default for ContextFacadeAssemblyPolicy {
    fn default() -> Self {
        Self {
            governance: ContextGovernanceRuntimeConfig::default(),
            trust_governance: None,
            knowledge_digest: KnowledgeDigestContextConfig::default(),
        }
    }
}

impl ContextFacadeAssemblyPolicy {
    /// Build from merged runtime/app context configuration.
    pub fn from_context_config_parts(
        governance: ContextGovernanceRuntimeConfig,
        trust: ContextTrustGovernanceConfig,
        knowledge_digest: KnowledgeDigestContextConfig,
    ) -> Self {
        let trust_governance = if trust.promotions.is_empty() {
            None
        } else {
            Some(trust)
        };
        Self {
            governance,
            trust_governance,
            knowledge_digest,
        }
    }
}
