//! Runtime toolkit name index as capability context.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use super::model::RuntimeToolCapabilityCatalog;
use super::render::runtime_tool_catalog_to_candidate;
use crate::{ContextComposeContext, ContextProvider, ContextProviderOutcome, ContextProviderStage};

#[derive(Clone)]
pub struct RuntimeToolCapabilityProvider {
    catalog: Arc<RuntimeToolCapabilityCatalog>,
}

impl RuntimeToolCapabilityProvider {
    #[must_use]
    pub fn new(catalog: Arc<RuntimeToolCapabilityCatalog>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl ContextProvider for RuntimeToolCapabilityProvider {
    fn provider_id(&self) -> &'static str {
        "runtime_tool_capability_catalog"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::CapabilityIndex
    }

    async fn contribute(&self, _ctx: &ContextComposeContext<'_>) -> MacacaResult<ContextProviderOutcome> {
        Ok(ContextProviderOutcome {
            candidates: if let Some(c) = runtime_tool_catalog_to_candidate(&self.catalog) {
                vec![c]
            } else {
                vec![]
            },
            diagnostics: Vec::new(),
            active_recall_report: None,
        })
    }
}

#[must_use]
pub fn runtime_tool_capability_provider_arc(
    catalog: Arc<RuntimeToolCapabilityCatalog>,
) -> Arc<dyn ContextProvider> {
    Arc::new(RuntimeToolCapabilityProvider::new(catalog))
}
