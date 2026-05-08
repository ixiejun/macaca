//! App runtime — manages the lifecycle of loaded applications.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use macaca_kernel::Kernel;
use macaca_proto::{AgentId, ApplicationId, MacacaError, MacacaResult};
use macaca_sdk::AgentConfig;
use macaca_sdk::MacacaSdk;

use crate::loader::AppLoader;
use crate::model::{AppLayer, AppManifest, AppStatus, LoadedApp};

/// Builder that incrementally validates and assembles application runtime
/// inputs before registration into [`AppRuntime`].
pub struct AppRuntimeBuilder {
    manifest: AppManifest,
    base_dir: PathBuf,
}

impl AppRuntimeBuilder {
    pub fn new(manifest: AppManifest, base_dir: impl Into<PathBuf>) -> Self {
        Self {
            manifest,
            base_dir: base_dir.into(),
        }
    }

    pub fn manifest(&self) -> &AppManifest {
        &self.manifest
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn validate(&self) -> MacacaResult<()> {
        AppLoader::validate_manifest(&self.manifest)?;
        if self.manifest.layer == AppLayer::L2Wasm {
            return Err(MacacaError::Config(
                "L2 WASM apps are not yet supported".into(),
            ));
        }
        Ok(())
    }

    pub fn resolve_agent_configs(&self) -> MacacaResult<Vec<AgentConfig>> {
        self.validate()?;
        AppLoader::resolve_agent_configs(&self.manifest, &self.base_dir)
    }

    pub fn assemble_loaded_app(self, agent_ids: Vec<AgentId>) -> MacacaResult<LoadedApp> {
        self.validate()?;
        Ok(LoadedApp {
            manifest: self.manifest,
            agent_ids,
            status: AppStatus::Running,
        })
    }
}

/// Factory for runtime builder creation. The default implementation is a thin
/// compatibility layer around current `macaca-app` startup assembly.
pub trait ApplicationRuntimeFactory: Send + Sync {
    fn build_runtime_builder(
        &self,
        manifest: AppManifest,
        base_dir: impl Into<PathBuf>,
    ) -> AppRuntimeBuilder;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultApplicationRuntimeFactory;

impl ApplicationRuntimeFactory for DefaultApplicationRuntimeFactory {
    fn build_runtime_builder(
        &self,
        manifest: AppManifest,
        base_dir: impl Into<PathBuf>,
    ) -> AppRuntimeBuilder {
        AppRuntimeBuilder::new(manifest, base_dir)
    }
}

/// Manages the lifecycle of loaded applications and their agents.
#[derive(Clone)]
pub struct AppRuntime {
    apps: Arc<RwLock<HashMap<ApplicationId, LoadedApp>>>,
}

impl AppRuntime {
    /// Create a new, empty app runtime.
    pub fn new() -> Self {
        Self {
            apps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load and start a declarative (L3) application from a manifest file.
    pub async fn start_app_from_file(
        &self,
        manifest_path: impl AsRef<Path>,
        kernel: &Kernel,
    ) -> MacacaResult<ApplicationId> {
        let path = manifest_path.as_ref();
        let manifest = AppLoader::load_manifest(path)?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        self.start_app(manifest, base_dir, kernel).await
    }

    /// Load and start an app from a parsed manifest.
    pub async fn start_app(
        &self,
        manifest: AppManifest,
        base_dir: impl AsRef<Path>,
        kernel: &Kernel,
    ) -> MacacaResult<ApplicationId> {
        let factory = DefaultApplicationRuntimeFactory;
        let builder = factory.build_runtime_builder(manifest, base_dir.as_ref().to_path_buf());
        let app_id = builder.manifest().id;

        {
            let apps = self.apps.read().await;
            if apps.contains_key(&app_id) {
                return Err(MacacaError::Agent(format!(
                    "App '{}' ({}) is already loaded",
                    builder.manifest().name,
                    app_id
                )));
            }
        }

        let configs = builder.resolve_agent_configs()?;

        let mut agent_ids = Vec::new();
        let sdk = MacacaSdk::for_kernel(kernel);
        for config in configs {
            let id = sdk.register_config(config).await?;
            agent_ids.push(id);
        }

        let name = builder.manifest().name.clone();
        let loaded = builder.assemble_loaded_app(agent_ids)?;

        self.apps.write().await.insert(app_id, loaded);
        tracing::info!(app = %name, id = %app_id, "app started");
        Ok(app_id)
    }

    /// Stop a running app. Unregisters its agents from the kernel.
    pub async fn stop_app(&self, app_id: &ApplicationId, kernel: &Kernel) -> MacacaResult<()> {
        let mut apps = self.apps.write().await;
        let app = apps
            .get_mut(app_id)
            .ok_or_else(|| MacacaError::NotFound(format!("App {} not found", app_id)))?;

        if app.status == AppStatus::Stopped {
            return Ok(());
        }

        for agent_id in &app.agent_ids {
            if let Err(e) = kernel.unregister_agent(agent_id).await {
                tracing::warn!(agent_id = ?agent_id, error = %e, "failed to unregister agent");
            }
        }

        app.status = AppStatus::Stopped;
        tracing::info!(app = %app_id, "app stopped");
        Ok(())
    }

    /// Remove a stopped app from the runtime entirely.
    pub async fn remove_app(&self, app_id: &ApplicationId) -> MacacaResult<()> {
        let mut apps = self.apps.write().await;
        let app = apps
            .get(app_id)
            .ok_or_else(|| MacacaError::NotFound(format!("App {} not found", app_id)))?;

        if app.status == AppStatus::Running {
            return Err(MacacaError::Agent(format!(
                "App '{}' is still running. Stop it first.",
                app.manifest.name
            )));
        }

        apps.remove(app_id);
        Ok(())
    }

    /// List all loaded apps.
    pub async fn list_apps(&self) -> Vec<(ApplicationId, String, AppStatus)> {
        let apps = self.apps.read().await;
        apps.iter()
            .map(|(id, app)| (*id, app.manifest.name.clone(), app.status))
            .collect()
    }

    /// Get the agent ids for a loaded app.
    pub async fn app_agents(&self, app_id: &ApplicationId) -> MacacaResult<Vec<AgentId>> {
        let apps = self.apps.read().await;
        let app = apps
            .get(app_id)
            .ok_or_else(|| MacacaError::NotFound(format!("App {} not found", app_id)))?;
        Ok(app.agent_ids.clone())
    }

    /// Get the status of a loaded app.
    pub async fn app_status(&self, app_id: &ApplicationId) -> MacacaResult<AppStatus> {
        let apps = self.apps.read().await;
        let app = apps
            .get(app_id)
            .ok_or_else(|| MacacaError::NotFound(format!("App {} not found", app_id)))?;
        Ok(app.status)
    }

    /// Find an app by name. Returns the first match.
    pub async fn find_by_name(&self, name: &str) -> Option<ApplicationId> {
        let apps = self.apps.read().await;
        apps.iter()
            .find(|(_, app)| app.manifest.name == name)
            .map(|(id, _)| *id)
    }

    /// Number of loaded apps.
    pub async fn app_count(&self) -> usize {
        self.apps.read().await.len()
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use macaca_kernel::{KernelBuilder, KernelProviderCompat};
    use macaca_llm::LlmProvider;
    use macaca_proto::config::KernelConfig;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult as Res, TokenUsage};
    use macaca_tools::DefaultToolSet;

    use crate::model::{AgentSource, CapabilityRef, InlineAgentConfig};

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }
        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> Res<LlmResponse> {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    fn make_kernel() -> Kernel {
        let config = KernelConfig {
            max_agents: 64,
            heartbeat_interval_ms: 5000,
            agent_timeout_ms: 30000,
        };
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        KernelBuilder::from_compat(
            config,
            KernelProviderCompat::new(llm, Box::new(DefaultToolSet::new())),
        )
        .build()
    }

    fn inline_manifest(name: &str) -> AppManifest {
        AppManifest {
            id: ApplicationId::new(),
            name: name.into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: format!("{name}-agent"),
                capabilities: vec![CapabilityRef {
                    name: "test".into(),
                    description: "test cap".into(),
                }],
                prompt_template: "You are a test agent.".into(),
                model: "mock".into(),
                permission_level: "user".into(),
                allowed_tools: vec![],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
        }
    }

    #[tokio::test]
    async fn start_and_list_app() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("app1");
        let expected_id = manifest.id;

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        assert_eq!(app_id, expected_id);

        let apps = runtime.list_apps().await;
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].0, expected_id);
        assert_eq!(apps[0].1, "app1");
        assert_eq!(apps[0].2, AppStatus::Running);
    }

    #[tokio::test]
    async fn start_duplicate_app_fails() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("dup");

        runtime
            .start_app(manifest.clone(), ".", &kernel)
            .await
            .unwrap();
        let err = runtime.start_app(manifest, ".", &kernel).await.unwrap_err();
        assert!(err.to_string().contains("already loaded"));
    }

    #[tokio::test]
    async fn stop_app() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("stop-test");

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        runtime.stop_app(&app_id, &kernel).await.unwrap();

        let status = runtime.app_status(&app_id).await.unwrap();
        assert_eq!(status, AppStatus::Stopped);
    }

    #[tokio::test]
    async fn stop_already_stopped_is_ok() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("stop2");

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        runtime.stop_app(&app_id, &kernel).await.unwrap();
        // Stopping again is a no-op
        runtime.stop_app(&app_id, &kernel).await.unwrap();
    }

    #[tokio::test]
    async fn remove_stopped_app() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("rm-test");

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        runtime.stop_app(&app_id, &kernel).await.unwrap();
        runtime.remove_app(&app_id).await.unwrap();
        assert_eq!(runtime.app_count().await, 0);
    }

    #[tokio::test]
    async fn remove_running_app_fails() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("rm-running");

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        let err = runtime.remove_app(&app_id).await.unwrap_err();
        assert!(err.to_string().contains("still running"));
    }

    #[tokio::test]
    async fn app_agents_returns_ids() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("agents-test");

        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        let agents = runtime.app_agents(&app_id).await.unwrap();
        assert_eq!(agents.len(), 1);

        // Also verify the kernel has the agent
        assert_eq!(kernel.agent_count().await, 1);
    }

    #[tokio::test]
    async fn stop_nonexistent_app_fails() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let fake_id = ApplicationId::new();
        let err = runtime.stop_app(&fake_id, &kernel).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn wasm_app_not_supported() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = AppManifest {
            id: ApplicationId::new(),
            name: "wasm-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L2Wasm,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
        };
        let err = runtime.start_app(manifest, ".", &kernel).await.unwrap_err();
        assert!(err.to_string().contains("WASM"));
    }

    #[tokio::test]
    async fn start_app_from_file() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let dir = std::env::temp_dir().join("macaca_app_runtime_test");
        std::fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join("app.yaml");
        std::fs::write(
            &manifest_path,
            r#"
name: runtime-file-app
layer: L3Declarative
agents:
  - name: runtime-file-agent
    prompt_template: "You are runtime-file-agent."
    model: "mock"
    capabilities:
      - name: assist
"#,
        )
        .unwrap();

        let app_id = runtime
            .start_app_from_file(&manifest_path, &kernel)
            .await
            .unwrap();
        let status = runtime.app_status(&app_id).await.unwrap();
        assert_eq!(status, AppStatus::Running);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runtime_builder_preserves_manifest_validation_and_assembly() {
        let manifest = inline_manifest("builder-app");
        let builder = AppRuntimeBuilder::new(manifest.clone(), ".");
        builder.validate().unwrap();
        let loaded = builder.assemble_loaded_app(vec![AgentId::new()]).unwrap();
        assert_eq!(loaded.manifest.name, manifest.name);
        assert_eq!(loaded.status, AppStatus::Running);
        assert_eq!(loaded.agent_ids.len(), 1);
    }

    #[tokio::test]
    async fn native_app_no_agents() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = AppManifest {
            id: ApplicationId::new(),
            name: "native-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L1Native,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: None,
            resources: None,
            context: None,
        };
        let app_id = runtime.start_app(manifest, ".", &kernel).await.unwrap();
        let agents = runtime.app_agents(&app_id).await.unwrap();
        assert!(agents.is_empty());
        assert_eq!(kernel.agent_count().await, 0);
    }

    #[tokio::test]
    async fn find_by_name() {
        let runtime = AppRuntime::new();
        let kernel = make_kernel();
        let manifest = inline_manifest("findme");
        let expected_id = manifest.id;

        runtime.start_app(manifest, ".", &kernel).await.unwrap();
        let found = runtime.find_by_name("findme").await;
        assert_eq!(found, Some(expected_id));
        assert!(runtime.find_by_name("nonexistent").await.is_none());
    }
}
