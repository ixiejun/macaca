//! App runtime — manages the lifecycle of loaded applications.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use macaca_kernel::Kernel;
use macaca_proto::{
    agent_manifest_from_declarative_config, AgentConfig, AgentId, ApplicationId, MacacaError,
    MacacaResult,
};

use crate::domain_pack_catalog::{empty_domain_pack_catalog, SharedDomainPackCatalog};
use crate::loader::AppLoader;
use crate::model::{AppManifest, AppStatus, LoadedApp};
use crate::service_capability::expand_service_capabilities;

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

/// Factory for runtime builder creation. The default implementation adapts the
/// current `macaca-app` startup assembly into a replaceable construction seam.
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
    /// Installed domain-pack catalog injected by the composition root.
    ///
    /// Capability expansion during `bootstrap_manifest` reads this catalog so manifest
    /// `use_packs` declarations resolve against host-installed extensions rather
    /// than hardcoded OS defaults.
    domain_pack_catalog: SharedDomainPackCatalog,
}

impl AppRuntime {
    /// Create a new, empty app runtime with an empty domain-pack catalog.
    pub fn new() -> Self {
        Self::with_domain_pack_catalog(empty_domain_pack_catalog())
    }

    /// Create a runtime bound to a composition-root catalog instance.
    pub fn with_domain_pack_catalog(domain_pack_catalog: SharedDomainPackCatalog) -> Self {
        tracing::info!(
            catalog_configured = true,
            "AppRuntime constructed with injected domain-pack catalog"
        );
        Self {
            apps: Arc::new(RwLock::new(HashMap::new())),
            domain_pack_catalog,
        }
    }

    /// Return the shared catalog used for manifest capability expansion.
    pub fn domain_pack_catalog(&self) -> SharedDomainPackCatalog {
        Arc::clone(&self.domain_pack_catalog)
    }

    /// Bootstrap a declarative application from an on-disk manifest path.
    ///
    /// This is the provider-internal companion to the Application Service `start`
    /// command. External callers (shells, tools, HTTP handlers) MUST route through
    /// the traced service command; only the runtime-host Application Service
    /// provider may invoke this bootstrap entry directly.
    ///
    /// Uses the Builder pattern (`AppRuntimeBuilder`) to validate manifest inputs,
    /// resolve agent configs, and assemble a `LoadedApp` before kernel registration.
    pub async fn bootstrap_manifest_from_path(
        &self,
        manifest_path: impl AsRef<Path>,
        kernel: &Kernel,
    ) -> MacacaResult<ApplicationId> {
        let path = manifest_path.as_ref();
        tracing::info!(
            manifest_path = %path.display(),
            "AppRuntime bootstrap_manifest_from_path: loading manifest from disk"
        );
        let manifest = AppLoader::load_manifest(path)?;

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        self.bootstrap_manifest(manifest, base_dir, kernel).await
    }

    /// Bootstrap a running application from an already-parsed manifest.
    ///
    /// Orchestrates manifest validation, domain-pack capability expansion, agent
    /// kernel manifest registration, and in-memory `LoadedApp` insertion.
    /// Duplicate application IDs are rejected before any kernel side-effects occur.
    pub async fn bootstrap_manifest(
        &self,
        manifest: AppManifest,
        base_dir: impl AsRef<Path>,
        kernel: &Kernel,
    ) -> MacacaResult<ApplicationId> {
        let factory = DefaultApplicationRuntimeFactory;
        let builder = factory.build_runtime_builder(manifest, base_dir.as_ref().to_path_buf());
        let app_id = builder.manifest().id;

        tracing::info!(
            app = %builder.manifest().name,
            app_id = %app_id,
            "AppRuntime bootstrap_manifest: beginning application bootstrap"
        );

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
        let capabilities = expand_service_capabilities(
            builder.manifest().service_contract.as_ref(),
            self.domain_pack_catalog.as_ref(),
        );
        tracing::info!(
            app = %builder.manifest().name,
            resolved_pack_count = capabilities.resolved_packs.len(),
            unresolved_pack_count = capabilities.unresolved_packs.len(),
            effective_service_count = capabilities.services.len(),
            capabilities_hash = %capabilities.capabilities_hash,
            "Resolved application effective service capabilities"
        );
        let mut agent_ids = Vec::new();
        for config in configs {
            let manifest = agent_manifest_from_declarative_config(&config)?;
            tracing::info!(
                agent_name = %manifest.name,
                "AppRuntime bootstrap_manifest: registering declarative agent with kernel"
            );
            let id = kernel.register_agent(manifest).await?;
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
mod tests;
