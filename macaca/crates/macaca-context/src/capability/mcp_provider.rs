//! [`McpContextProvider`]: MCP server/tool summaries as capability context.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use super::model::McpCapabilityCatalog;
use super::render::mcp_catalog_to_candidate;
use crate::{
    ContextComposeContext, ContextProvider, ContextProviderDiagnostics, ContextProviderOutcome,
    ContextProviderStage,
};

/// Emits fenced, **untrusted** MCP capability summaries (transport + advertised tools).
#[derive(Clone)]
pub struct McpContextProvider {
    catalog: Arc<McpCapabilityCatalog>,
}

impl McpContextProvider {
    #[must_use]
    pub fn new(catalog: Arc<McpCapabilityCatalog>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl ContextProvider for McpContextProvider {
    fn provider_id(&self) -> &'static str {
        "mcp_capability_catalog"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::CapabilityIndex
    }

    async fn contribute(
        &self,
        _ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        let mut diagnostics = Vec::new();
        for c in &self.catalog.collisions {
            diagnostics.push(ContextProviderDiagnostics {
                provider_id: self.provider_id().to_string(),
                message: format!(
                    "tool name `{}` duplicated across MCP servers {:?}",
                    c.local_key, c.claimants
                ),
            });
        }

        Ok(ContextProviderOutcome {
            candidates: if let Some(c) = mcp_catalog_to_candidate(&self.catalog) {
                vec![c]
            } else {
                vec![]
            },
            diagnostics,
            active_recall_report: None,
        })
    }
}

#[must_use]
pub fn mcp_capability_provider_arc(catalog: Arc<McpCapabilityCatalog>) -> Arc<dyn ContextProvider> {
    Arc::new(McpContextProvider::new(catalog))
}
