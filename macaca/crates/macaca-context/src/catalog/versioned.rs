//! **Decorator** that attaches an immutable semver string to any [`crate::composer::ContextProvider`]
//! for diagnostics (`ProviderInvocationSummary::implementation_version`).

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;

use crate::composer::{
    ContextComposeContext, ContextProvider, ContextProviderOutcome, ContextProviderStage,
};

/// Wraps an inner provider and overrides [`ContextProvider::implementation_version`].
pub struct VersionedContextProvider {
    inner: Arc<dyn ContextProvider>,
    version: &'static str,
}

impl VersionedContextProvider {
    /// Prefer `crate::catalog::descriptor::MACACA_CONTEXT_CRATE_SEMVER` for built-ins.
    pub fn new(inner: Arc<dyn ContextProvider>, version: &'static str) -> Arc<dyn ContextProvider> {
        Arc::new(Self { inner, version })
    }
}

#[async_trait]
impl ContextProvider for VersionedContextProvider {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    fn stage(&self) -> ContextProviderStage {
        self.inner.stage()
    }

    fn implementation_version(&self) -> Option<&'static str> {
        Some(self.version)
    }

    async fn contribute(
        &self,
        ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        self.inner.contribute(ctx).await
    }
}
