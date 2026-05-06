//! [`SkillContextProvider`]: Composer stage that emits compact skill capability rows.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use super::model::SkillCapabilityCatalog;
use super::render::{skill_catalog_to_candidate, skill_missing_dependency_notes};
use crate::{
    ContextComposeContext, ContextProvider, ContextProviderDiagnostics, ContextProviderOutcome,
    ContextProviderStage,
};

/// Chain-of-responsibility node for Tier-1 **AgentSkills** catalogs.
///
/// The provider never reads disk; callers supply a neutral [`SkillCapabilityCatalog`] built by the host
/// from a frozen skill-runtime snapshot (Tier-1 catalog only).
#[derive(Clone)]
pub struct SkillContextProvider {
    catalog: Arc<SkillCapabilityCatalog>,
    ready_mcp_server_ids: Arc<Vec<String>>,
}

impl SkillContextProvider {
    #[must_use]
    pub fn new(catalog: Arc<SkillCapabilityCatalog>, ready_mcp_server_ids: Arc<Vec<String>>) -> Self {
        Self {
            catalog,
            ready_mcp_server_ids,
        }
    }
}

#[async_trait]
impl ContextProvider for SkillContextProvider {
    fn provider_id(&self) -> &'static str {
        "skills_capability_catalog"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::CapabilityIndex
    }

    async fn contribute(&self, _ctx: &ContextComposeContext<'_>) -> MacacaResult<ContextProviderOutcome> {
        let mut diagnostics = Vec::new();
        for note in skill_missing_dependency_notes(&self.catalog, &self.ready_mcp_server_ids) {
            diagnostics.push(ContextProviderDiagnostics {
                provider_id: self.provider_id().to_string(),
                message: note,
            });
        }
        Ok(ContextProviderOutcome {
            candidates: if let Some(c) = skill_catalog_to_candidate(&self.catalog) {
                vec![c]
            } else {
                vec![]
            },
            diagnostics,
            active_recall_report: None,
        })
    }
}

/// Wraps [`SkillContextProvider`] for dynamic dispatch.
#[must_use]
pub fn skill_capability_provider_arc(
    catalog: Arc<SkillCapabilityCatalog>,
    ready_mcp_server_ids: Arc<Vec<String>>,
) -> Arc<dyn ContextProvider> {
    Arc::new(SkillContextProvider::new(catalog, ready_mcp_server_ids))
}
