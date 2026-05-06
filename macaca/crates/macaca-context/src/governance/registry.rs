//! **Registry + Abstract Factory** for context providers.
//!
//! Runtimes (web, CLI, gateway) register neutral [`ContextProviderFactory`] implementations under
//! stable string ids (`"profile_files"`, `"capability_skills"`, ...). Selection is always driven by
//! configuration — never by embedding application or workflow labels in code paths
//! (see OpenSpec `context-governance-runtime`).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use macaca_proto::MacacaResult;

pub use crate::catalog::descriptor::ProviderFamilyDescriptor;
use crate::catalog::descriptor::MACACA_CONTEXT_CRATE_SEMVER;
use crate::composer::{ContextProvider, ContextProviderStage};
/// Input envelope passed to factories. Intentionally small and JSON-friendly so higher layers can
/// forward opaque extension payloads without coupling crates to each provider’s concrete ctor args.
#[derive(Debug, Clone)]
pub struct ProviderFactoryInput {
    pub agent_name: String,
    pub session_id: Option<String>,
    pub params: serde_json::Value,
}

/// Abstract factory for families of [`ContextProvider`].
///
/// Each factory is responsible for one **family** (e.g., profile bootstrap) so operators can replace
/// entire families behind a single registry id.
#[async_trait]
pub trait ContextProviderFactory: Send + Sync {
    /// Stable registry key (e.g., `"agent_profile_files"`).
    fn family_id(&self) -> &'static str;

    /// Composer stage for providers minted by this factory (used for sort ordering).
    fn stage_hint(&self) -> ContextProviderStage {
        ContextProviderStage::StableProfile
    }

    /// Operator-visible metadata — safe for JSON dumps; override when the plugin has its own semver.
    fn descriptor(&self) -> ProviderFamilyDescriptor {
        ProviderFamilyDescriptor {
            family_id: self.family_id().to_string(),
            implementation_version: MACACA_CONTEXT_CRATE_SEMVER.to_string(),
            capability_tags: Vec::new(),
        }
    }

    /// Produce a ready-to-run provider instance.
    async fn build_provider(
        &self,
        input: &ProviderFactoryInput,
    ) -> MacacaResult<Arc<dyn ContextProvider>>;
}

/// Thread-safe in-process registry (first milestone: local only; no RPC/WASM in this crate).
#[derive(Default)]
pub struct ContextProviderRegistry {
    factories: RwLock<HashMap<String, Arc<dyn ContextProviderFactory>>>,
}

impl ContextProviderRegistry {
    /// Empty registry — call sites may [`register`](Self::register) builtins or user hooks at boot.
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Registers or replaces a factory under [`ContextProviderFactory::family_id`].
    pub fn register(&self, factory: Arc<dyn ContextProviderFactory>) {
        let id = factory.family_id().to_string();
        let mut map = self.factories.write().expect("registry lock poisoned");
        map.insert(id, factory);
    }

    /// Looks up a factory for configuration-driven wiring.
    pub fn get(&self, family_id: &str) -> Option<Arc<dyn ContextProviderFactory>> {
        let map = self.factories.read().expect("registry lock poisoned");
        map.get(family_id).cloned()
    }

    /// Lists registered ids in lexicographic order (stable UI / diagnostics).
    pub fn list_family_ids(&self) -> Vec<String> {
        let map = self.factories.read().expect("registry lock poisoned");
        let mut ids: Vec<_> = map.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Returns [`ProviderFamilyDescriptor`] rows for HTTP/CLI snapshots.
    pub fn list_registered_descriptors(&self) -> Vec<ProviderFamilyDescriptor> {
        let map = self.factories.read().expect("registry lock poisoned");
        let mut rows: Vec<_> = map.values().map(|f| f.descriptor()).collect();
        rows.sort_by(|a, b| a.family_id.cmp(&b.family_id));
        rows
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use macaca_proto::MacacaResult;

    use crate::composer::{
        ContextComposeContext, ContextProvider, ContextProviderOutcome, ContextProviderStage,
    };

    use super::{ContextProviderFactory, ContextProviderRegistry, ProviderFactoryInput};

    /// No-op provider minted by [`TestFactory`]; proves factories can vend neutral instances.
    struct NopProvider {
        id: &'static str,
    }

    #[async_trait]
    impl ContextProvider for NopProvider {
        fn provider_id(&self) -> &str {
            self.id
        }

        fn stage(&self) -> ContextProviderStage {
            ContextProviderStage::StableProfile
        }

        async fn contribute(
            &self,
            _ctx: &ContextComposeContext<'_>,
        ) -> MacacaResult<ContextProviderOutcome> {
            Ok(ContextProviderOutcome::default())
        }
    }

    struct AlphaFactory;

    #[async_trait]
    impl ContextProviderFactory for AlphaFactory {
        fn family_id(&self) -> &'static str {
            "alpha"
        }

        async fn build_provider(
            &self,
            _input: &ProviderFactoryInput,
        ) -> MacacaResult<Arc<dyn ContextProvider>> {
            Ok(Arc::new(NopProvider { id: "alpha-built" }))
        }
    }

    struct BetaFactory;

    #[async_trait]
    impl ContextProviderFactory for BetaFactory {
        fn family_id(&self) -> &'static str {
            "beta"
        }

        async fn build_provider(
            &self,
            _input: &ProviderFactoryInput,
        ) -> MacacaResult<Arc<dyn ContextProvider>> {
            Ok(Arc::new(NopProvider { id: "beta-built" }))
        }
    }

    #[tokio::test]
    async fn registry_roundtrip_and_sorted_listing() {
        let reg = ContextProviderRegistry::new();
        reg.register(Arc::new(AlphaFactory));
        reg.register(Arc::new(BetaFactory));
        assert_eq!(
            reg.list_family_ids(),
            vec!["alpha".to_string(), "beta".to_string()]
        );

        let a = reg.get("alpha").expect("alpha registered");
        let p = a
            .build_provider(&ProviderFactoryInput {
                agent_name: "x".into(),
                session_id: None,
                params: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert_eq!(p.provider_id(), "alpha-built");
    }
}
