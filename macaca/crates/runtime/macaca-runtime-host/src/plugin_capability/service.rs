//! Plugin Capability Registry service facade.
//!
//! This facade composes discovery, registry storage, conflict policy, and a
//! descriptor-safe call skeleton.  It logs every major decision point while
//! keeping concrete capability behavior outside the registry.

use std::sync::Arc;

use macaca_proto::{
    PluginCapabilityCallCommand, PluginCapabilityCallResult, PluginCapabilityDeactivateCommand,
    PluginCapabilityDiscoverCommand, PluginCapabilityEvent, PluginCapabilityInspectCommand,
    PluginCapabilityOwnership, PluginCapabilityQueryCommand, PluginCapabilityRegisterCommand,
    PluginCapabilityRegistrySnapshot, PluginControlRecord, PluginError, TraceContext,
};
use tracing::{info, warn};

use crate::plugin_capability::admission::{
    CapabilityCallAdmissionChain, CapabilityCallAdmissionContext,
};
use crate::plugin_capability::discovery::discover_manifest_capabilities;
use crate::plugin_capability::registry::PluginCapabilityRegistry;
use crate::plugin_control::PluginRepository;

/// Builder for Plugin Capability Service strategies.
#[derive(Default)]
pub struct PluginCapabilityServiceBuilder {
    registry: Option<Arc<PluginCapabilityRegistry>>,
    repository: Option<Arc<dyn PluginRepository>>,
    call_admission: Option<CapabilityCallAdmissionChain>,
}

impl PluginCapabilityServiceBuilder {
    /// Use an externally managed registry.
    pub fn registry(mut self, registry: Arc<PluginCapabilityRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Attach a plugin repository for contract-first manifest discovery.
    pub fn repository(mut self, repository: Arc<dyn PluginRepository>) -> Self {
        self.repository = Some(repository);
        self
    }

    /// Replace the default capability-call admission chain.
    pub fn call_admission(mut self, admission: CapabilityCallAdmissionChain) -> Self {
        self.call_admission = Some(admission);
        self
    }

    /// Build the service facade.
    pub fn build(self) -> PluginCapabilityService {
        PluginCapabilityService {
            registry: self.registry.unwrap_or_default(),
            repository: self.repository,
            call_admission: self.call_admission.unwrap_or_default(),
        }
    }
}

/// Runtime-host-owned Plugin Capability Registry facade.
pub struct PluginCapabilityService {
    registry: Arc<PluginCapabilityRegistry>,
    repository: Option<Arc<dyn PluginRepository>>,
    call_admission: CapabilityCallAdmissionChain,
}

impl PluginCapabilityService {
    /// Start building the service from explicit strategies.
    pub fn builder() -> PluginCapabilityServiceBuilder {
        PluginCapabilityServiceBuilder::default()
    }

    /// Create a deterministic in-memory service with no repository discovery.
    pub fn in_memory() -> Self {
        Self::builder().build()
    }

    /// Discover capability descriptors from repository manifests.
    pub async fn discover(
        &self,
        command: PluginCapabilityDiscoverCommand,
    ) -> Result<Vec<PluginCapabilityOwnership>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            "plugin capability discovery requested"
        );
        if let Some(repository) = &self.repository {
            for record in repository.list(true).await? {
                if command
                    .plugin_id
                    .as_ref()
                    .is_some_and(|plugin_id| plugin_id != &record.plugin_id)
                {
                    continue;
                }
                self.register_manifest_record(record, command.trace.clone())
                    .await?;
            }
        }
        self.registry
            .query(PluginCapabilityQueryCommand {
                kind: None,
                plugin_id: command.plugin_id,
                slot_namespace: None,
                slot_key: None,
                include_inactive: command.include_inactive,
                trace: command.trace,
            })
            .await
    }

    /// Register explicit descriptors for one plugin.
    pub async fn register(
        &self,
        command: PluginCapabilityRegisterCommand,
    ) -> Result<Vec<PluginCapabilityOwnership>, PluginError> {
        self.registry.register(command).await
    }

    /// Deactivate all descriptors owned by one plugin.
    pub async fn deactivate(
        &self,
        command: PluginCapabilityDeactivateCommand,
    ) -> Result<Vec<PluginCapabilityEvent>, PluginError> {
        self.registry.deactivate(command).await
    }

    /// Query capability ownership.
    pub async fn query(
        &self,
        command: PluginCapabilityQueryCommand,
    ) -> Result<Vec<PluginCapabilityOwnership>, PluginError> {
        self.registry.query(command).await
    }

    /// Inspect one capability.
    pub async fn inspect(
        &self,
        command: PluginCapabilityInspectCommand,
    ) -> Result<Option<PluginCapabilityOwnership>, PluginError> {
        self.registry.inspect(command).await
    }

    /// Attempt a descriptor-safe capability call.
    ///
    /// v1 does not execute external plugin code.  Calls are admitted through a
    /// traceable envelope and then return structured unavailable unless a later
    /// execution-plane proposal installs a safe host for the descriptor.
    pub async fn call(
        &self,
        command: PluginCapabilityCallCommand,
    ) -> Result<PluginCapabilityCallResult, PluginError> {
        let ownership = self
            .registry
            .inspect(PluginCapabilityInspectCommand {
                capability_id: command.capability_id.clone(),
                trace: command.trace.clone(),
            })
            .await?
            .ok_or_else(|| {
                PluginError::Unavailable(format!("capability not found: {}", command.capability_id))
            })?;
        self.call_admission
            .authorize(&CapabilityCallAdmissionContext {
                command: &command,
                ownership: &ownership,
            })
            .await?;
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %ownership.plugin_id,
            capability_id = %ownership.capability_id,
            kind = %ownership.kind,
            "plugin capability call reached descriptor-safe unavailable skeleton"
        );
        Err(PluginError::Unavailable(format!(
            "capability execution host is unavailable for {}",
            ownership.capability_id
        )))
    }

    /// Return registry diagnostics.
    pub async fn snapshot(&self, trace: TraceContext) -> PluginCapabilityRegistrySnapshot {
        self.registry.snapshot(trace).await
    }

    async fn register_manifest_record(
        &self,
        record: PluginControlRecord,
        trace: TraceContext,
    ) -> Result<(), PluginError> {
        let descriptors = discover_manifest_capabilities(&record.manifest);
        if descriptors.is_empty() {
            return Ok(());
        }
        self.registry
            .register(PluginCapabilityRegisterCommand {
                plugin_id: record.plugin_id,
                descriptors,
                trace,
                metadata: Default::default(),
            })
            .await?;
        Ok(())
    }
}
