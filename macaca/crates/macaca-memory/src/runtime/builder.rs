use std::sync::Arc;

use crate::core::active_recall::ActiveRecallCapability;
use crate::core::facade::MemoryFacade;
use crate::governance::KnowledgeCompileCapability;

use super::facade::ComposedMemoryRuntime;

/// Small builder for already-constructed runtime components.
pub struct MemoryRuntimeBuilder<F, A, K> {
    facade: Arc<F>,
    active_recall: Arc<A>,
    knowledge: Arc<K>,
    runtime_id: String,
    provider_profile: Option<String>,
}

impl<F, A, K> MemoryRuntimeBuilder<F, A, K> {
    pub fn new(facade: Arc<F>, active_recall: Arc<A>, knowledge: Arc<K>) -> Self {
        Self {
            facade,
            active_recall,
            knowledge,
            runtime_id: "memory-runtime".into(),
            provider_profile: None,
        }
    }

    pub fn runtime_id(mut self, runtime_id: impl Into<String>) -> Self {
        self.runtime_id = runtime_id.into();
        self
    }

    pub fn provider_profile(mut self, provider_profile: impl Into<String>) -> Self {
        self.provider_profile = Some(provider_profile.into());
        self
    }

    pub fn build(self) -> ComposedMemoryRuntime<F, A, K> {
        ComposedMemoryRuntime::new(self.facade, self.active_recall, self.knowledge)
            .with_runtime_id(self.runtime_id)
            .with_provider_profile(self.provider_profile)
    }
}

impl<F, A, K> From<MemoryRuntimeBuilder<F, A, K>> for ComposedMemoryRuntime<F, A, K>
where
    F: MemoryFacade + Send + Sync,
    A: ActiveRecallCapability + Send + Sync,
    K: KnowledgeCompileCapability + Send + Sync,
{
    fn from(builder: MemoryRuntimeBuilder<F, A, K>) -> Self {
        builder.build()
    }
}
