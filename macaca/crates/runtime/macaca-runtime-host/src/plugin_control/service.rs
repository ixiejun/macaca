//! Plugin Control Service facade.
//!
//! The service composes repository, admission chain, and Plugin Runtime v0 into
//! one narrow control-plane API.  The composition is intentionally explicit:
//! repository manages records, admission manages policy, runtime manages
//! descriptor-safe lifecycle transitions, and this Facade records trace/audit
//! events at the operation boundary.

use std::sync::Arc;

use chrono::Utc;
use macaca_kernel::PluginRegistry;
use macaca_proto::{
    PluginActivationState, PluginControlEvent, PluginControlRecord, PluginDiagnostics, PluginError,
    PluginHealth, PluginHealthSnapshot, PluginId, PluginInstallRequest, PluginInstallResult,
    PluginLifecycleState, PluginListCommand, PluginTargetCommand,
};
use tracing::{info, warn};

use crate::plugin::PluginRuntimeFacade;
use crate::plugin_capability::discovery::discover_manifest_capabilities;
use crate::plugin_capability::PluginCapabilityService;
use crate::plugin_control::admission::{AdmissionContext, PluginAdmissionChain};
use crate::plugin_control::metadata::{
    config_requirements, sanitized_metadata, secret_requirements,
};
use crate::plugin_control::repository::{InMemoryPluginRepository, PluginRepository};

/// Builder for the control service.
///
/// The Builder pattern keeps test composition and future host bootstrap code
/// readable while allowing repository/admission/runtime strategies to be
/// replaced without adding many constructors.
pub struct PluginControlServiceBuilder {
    repository: Option<Arc<dyn PluginRepository>>,
    admission: Option<PluginAdmissionChain>,
    runtime: Option<Arc<PluginRuntimeFacade>>,
    capability_service: Option<Arc<PluginCapabilityService>>,
    project_local_enabled: bool,
}

impl Default for PluginControlServiceBuilder {
    fn default() -> Self {
        Self {
            repository: None,
            admission: None,
            runtime: None,
            capability_service: None,
            project_local_enabled: false,
        }
    }
}

impl PluginControlServiceBuilder {
    /// Use a custom repository strategy.
    pub fn repository(mut self, repository: Arc<dyn PluginRepository>) -> Self {
        self.repository = Some(repository);
        self
    }

    /// Use a custom admission chain.
    pub fn admission(mut self, admission: PluginAdmissionChain) -> Self {
        self.admission = Some(admission);
        self
    }

    /// Use a custom plugin runtime facade.
    pub fn runtime(mut self, runtime: Arc<PluginRuntimeFacade>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Use a Plugin Capability Registry service for capability ownership sync.
    pub fn capability_service(mut self, capability_service: Arc<PluginCapabilityService>) -> Self {
        self.capability_service = Some(capability_service);
        self
    }

    /// Enable project-local plugin installs for explicit development hosts.
    pub fn allow_project_local(mut self) -> Self {
        self.project_local_enabled = true;
        self
    }

    /// Build the service with safe defaults for omitted strategies.
    pub fn build(self) -> PluginControlService {
        let runtime = self
            .runtime
            .unwrap_or_else(|| Arc::new(PluginRuntimeFacade::new(Arc::new(PluginRegistry::new()))));
        PluginControlService {
            repository: self
                .repository
                .unwrap_or_else(|| Arc::new(InMemoryPluginRepository::default())),
            admission: self.admission.unwrap_or_default(),
            runtime,
            capability_service: self.capability_service,
            project_local_enabled: self.project_local_enabled,
        }
    }
}

/// Runtime-host-owned Plugin Control Plane facade.
pub struct PluginControlService {
    repository: Arc<dyn PluginRepository>,
    admission: PluginAdmissionChain,
    runtime: Arc<PluginRuntimeFacade>,
    capability_service: Option<Arc<PluginCapabilityService>>,
    project_local_enabled: bool,
}

impl PluginControlService {
    /// Start building a service with explicit strategies.
    pub fn builder() -> PluginControlServiceBuilder {
        PluginControlServiceBuilder::default()
    }

    /// Create a service with deterministic in-memory state.
    pub fn in_memory() -> Self {
        Self::builder().build()
    }

    /// List plugin records, optionally including disabled entries.
    pub async fn list(
        &self,
        command: PluginListCommand,
    ) -> Result<Vec<PluginControlRecord>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            include_disabled = command.include_disabled,
            "plugin control list requested"
        );
        self.repository.list(command.include_disabled).await
    }

    /// Inspect one plugin record.
    pub async fn inspect(
        &self,
        command: PluginTargetCommand,
    ) -> Result<Option<PluginControlRecord>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "plugin control inspect requested"
        );
        self.repository.get(&command.plugin_id).await
    }

    /// Install a plugin from a manifest-bearing request.
    ///
    /// The first implementation requires the manifest in the request because
    /// real archive/git/store loading belongs to install-source strategies that
    /// can be added without changing this public method.
    pub async fn install(
        &self,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallResult, PluginError> {
        let manifest = request.manifest.clone().ok_or_else(|| {
            PluginError::Unavailable("plugin install request requires a manifest".into())
        })?;
        info!(
            trace_id = %request.trace.trace_id,
            plugin_id = %manifest.plugin_id,
            source_kind = ?request.location.source_kind,
            "plugin control install requested"
        );
        self.admission
            .authorize(&AdmissionContext {
                manifest: &manifest,
                location: &request.location,
                project_local_enabled: self.project_local_enabled,
            })
            .await?;

        let now = Utc::now();
        let mut record = PluginControlRecord {
            plugin_id: manifest.plugin_id.clone(),
            version: manifest.version.clone(),
            repository_id: request
                .metadata
                .get("repository_id")
                .cloned()
                .unwrap_or_else(|| "default".into()),
            source_kind: request.location.source_kind.clone(),
            activation_state: PluginActivationState::Disabled,
            lifecycle_state: PluginLifecycleState::Installed,
            health: PluginHealth::Unknown,
            enabled: false,
            installed_at: now,
            updated_at: now,
            manifest,
            config_requirements: config_requirements(&request.metadata),
            secret_requirements: secret_requirements(&request.metadata),
            metadata: sanitized_metadata(&request.metadata),
        };
        let mut events = vec![PluginControlEvent::new(
            Some(record.plugin_id.clone()),
            "plugin.install",
            "accepted",
            Some(record.activation_state.clone()),
            request.trace.clone(),
        )];

        if request.enable_after_install {
            record = self.enable_record(record, request.trace.clone()).await?;
            events.push(PluginControlEvent::new(
                Some(record.plugin_id.clone()),
                "plugin.enable",
                "ok",
                Some(record.activation_state.clone()),
                request.trace.clone(),
            ));
        }

        self.repository.put(record.clone(), "install").await?;
        Ok(PluginInstallResult { record, events })
    }

    /// Enable and register an installed plugin.
    pub async fn enable(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginControlRecord, PluginError> {
        let record = self.required_record(&command.plugin_id).await?;
        let record = self.enable_record(record, command.trace.clone()).await?;
        self.repository.put(record.clone(), "enable").await?;
        Ok(record)
    }

    /// Disable a plugin without deleting its repository record.
    pub async fn disable(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginControlRecord, PluginError> {
        let mut record = self.required_record(&command.plugin_id).await?;
        info!(
            trace_id = %command.trace.trace_id,
            plugin_id = %record.plugin_id,
            "plugin control disable requested"
        );
        record.enabled = false;
        record.activation_state = PluginActivationState::Disabled;
        record.updated_at = Utc::now();
        self.deactivate_capabilities(
            &record.plugin_id,
            command.trace.clone(),
            "plugin disabled by control plane",
        )
        .await?;
        self.repository.put(record.clone(), "disable").await?;
        Ok(record)
    }

    /// Start a registered plugin through Plugin Runtime v0.
    pub async fn start(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginControlRecord, PluginError> {
        let mut record = self.required_record(&command.plugin_id).await?;
        if !record.enabled {
            return Err(PluginError::Unavailable(format!(
                "plugin is disabled: {}",
                record.plugin_id
            )));
        }
        let event = self
            .runtime
            .start(&record.manifest, Some(command.trace.clone()))
            .await?;
        record.lifecycle_state = event.next_state.clone();
        record.activation_state = PluginActivationState::from_lifecycle(&event.next_state);
        record.health = PluginHealth::Healthy;
        record.updated_at = Utc::now();
        self.repository.put(record.clone(), "start").await?;
        Ok(record)
    }

    /// Stop a running plugin through Plugin Runtime v0.
    pub async fn stop(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginControlRecord, PluginError> {
        let mut record = self.required_record(&command.plugin_id).await?;
        let event = self
            .runtime
            .stop(&record.manifest, Some(command.trace.clone()))
            .await?;
        record.lifecycle_state = event.next_state.clone();
        record.activation_state = PluginActivationState::from_lifecycle(&event.next_state);
        record.health = PluginHealth::Unknown;
        record.updated_at = Utc::now();
        self.repository.put(record.clone(), "stop").await?;
        Ok(record)
    }

    /// Uninstall a plugin record and remove runtime descriptors when possible.
    pub async fn uninstall(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginControlEvent, PluginError> {
        let record = self.required_record(&command.plugin_id).await?;
        if matches!(
            record.lifecycle_state,
            PluginLifecycleState::Stopped | PluginLifecycleState::Failed
        ) {
            self.runtime
                .uninstall(&record.manifest, Some(command.trace.clone()))
                .await?;
        } else if record.enabled {
            warn!(
                plugin_id = %record.plugin_id,
                state = ?record.lifecycle_state,
                "plugin control uninstall skipped runtime cleanup because lifecycle is not stopped"
            );
        }
        self.deactivate_capabilities(
            &record.plugin_id,
            command.trace.clone(),
            "plugin uninstalled by control plane",
        )
        .await?;
        self.repository.remove(&record.plugin_id).await?;
        Ok(PluginControlEvent::new(
            Some(record.plugin_id),
            "plugin.uninstall",
            "ok",
            Some(PluginActivationState::Uninstalled),
            command.trace,
        ))
    }

    /// Return a deterministic health snapshot.
    pub async fn health(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginHealthSnapshot, PluginError> {
        let record = self.required_record(&command.plugin_id).await?;
        Ok(PluginHealthSnapshot {
            plugin_id: record.plugin_id,
            health: record.health,
            activation_state: record.activation_state,
            checked_at: Utc::now(),
            reason: None,
        })
    }

    /// Return diagnostics without secret/config values.
    pub async fn diagnostics(
        &self,
        command: PluginTargetCommand,
    ) -> Result<PluginDiagnostics, PluginError> {
        let record = self.required_record(&command.plugin_id).await?;
        Ok(PluginDiagnostics {
            plugin_id: record.plugin_id,
            repository_id: record.repository_id,
            source_kind: record.source_kind,
            activation_state: record.activation_state,
            lifecycle_state: record.lifecycle_state,
            health: record.health,
            config_requirements: record.config_requirements,
            secret_requirements: record.secret_requirements,
            generated_at: Utc::now(),
            warnings: Vec::new(),
        })
    }

    async fn enable_record(
        &self,
        mut record: PluginControlRecord,
        trace: macaca_proto::TraceContext,
    ) -> Result<PluginControlRecord, PluginError> {
        info!(
            trace_id = %trace.trace_id,
            plugin_id = %record.plugin_id,
            "plugin control enable requested"
        );
        let events = self
            .runtime
            .install_and_register(record.manifest.clone(), Some(trace.clone()))
            .await?;
        let next = events
            .last()
            .map(|event| event.next_state.clone())
            .unwrap_or(PluginLifecycleState::Registered);
        self.register_capabilities(&record, trace.clone()).await?;
        record.enabled = true;
        record.lifecycle_state = next.clone();
        record.activation_state = PluginActivationState::from_lifecycle(&next);
        record.updated_at = Utc::now();
        Ok(record)
    }

    async fn required_record(
        &self,
        plugin_id: &PluginId,
    ) -> Result<PluginControlRecord, PluginError> {
        self.repository
            .get(plugin_id)
            .await?
            .ok_or_else(|| PluginError::Unavailable(format!("plugin not found: {plugin_id}")))
    }

    async fn register_capabilities(
        &self,
        record: &PluginControlRecord,
        trace: macaca_proto::TraceContext,
    ) -> Result<(), PluginError> {
        let Some(capability_service) = &self.capability_service else {
            return Ok(());
        };
        let descriptors = discover_manifest_capabilities(&record.manifest);
        if descriptors.is_empty() {
            return Ok(());
        }
        info!(
            trace_id = %trace.trace_id,
            plugin_id = %record.plugin_id,
            descriptors = descriptors.len(),
            "plugin control syncing capability descriptors"
        );
        capability_service
            .register(macaca_proto::PluginCapabilityRegisterCommand {
                plugin_id: record.plugin_id.clone(),
                descriptors,
                trace,
                metadata: Default::default(),
            })
            .await?;
        Ok(())
    }

    async fn deactivate_capabilities(
        &self,
        plugin_id: &PluginId,
        trace: macaca_proto::TraceContext,
        reason: &str,
    ) -> Result<(), PluginError> {
        let Some(capability_service) = &self.capability_service else {
            return Ok(());
        };
        let events = capability_service
            .deactivate(macaca_proto::PluginCapabilityDeactivateCommand {
                plugin_id: plugin_id.clone(),
                trace,
                reason: reason.into(),
            })
            .await?;
        info!(
            plugin_id = %plugin_id,
            deactivated = events.len(),
            "plugin control synced capability deactivation"
        );
        Ok(())
    }
}
