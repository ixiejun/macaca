//! SDK facade and registry adapters.

use async_trait::async_trait;

use macaca_proto::{AgentId, AgentManifest, MacacaResult};

use crate::config::AgentConfig;
use crate::spec::AgentSpec;

/// Registry boundary used by the SDK facade.
#[async_trait]
pub trait AgentRegistryApi: Send + Sync {
    /// Register a runtime agent and manifest.
    async fn register_manifest(&self, manifest: AgentManifest) -> MacacaResult<AgentId>;
}

/// Facade for SDK agent declaration and registration.
pub struct MacacaSdk<R> {
    registry: R,
}

impl<R> MacacaSdk<R>
where
    R: AgentRegistryApi,
{
    /// Create a facade over a registry adapter.
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Register an already-built agent spec.
    pub async fn register_spec(&self, spec: AgentSpec) -> MacacaResult<AgentId> {
        let manifest = spec.manifest();
        self.registry.register_manifest(manifest).await
    }

    /// Build and register an agent from config.
    pub async fn register_config(&self, config: AgentConfig) -> MacacaResult<AgentId> {
        self.register_spec(AgentSpec::from_config(config)?).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct RecordingRegistry {
        manifests: tokio::sync::Mutex<Vec<AgentManifest>>,
    }

    impl RecordingRegistry {
        fn new() -> Self {
            Self {
                manifests: tokio::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AgentRegistryApi for Arc<RecordingRegistry> {
        async fn register_manifest(&self, manifest: AgentManifest) -> MacacaResult<AgentId> {
            let id = manifest.id;
            self.manifests.lock().await.push(manifest);
            Ok(id)
        }
    }

    #[tokio::test]
    async fn facade_registers_config_manifest() {
        let registry = Arc::new(RecordingRegistry::new());
        let sdk = MacacaSdk::new(Arc::clone(&registry));
        let config = AgentConfig::from_yaml(
            r#"
name: facade-agent
capabilities:
  - name: test
prompt_template: "Hello"
"#,
        )
        .unwrap();

        let id = sdk.register_config(config).await.unwrap();
        let manifests = registry.manifests.lock().await;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, id);
        assert_eq!(manifests[0].name, "facade-agent");
    }
}
