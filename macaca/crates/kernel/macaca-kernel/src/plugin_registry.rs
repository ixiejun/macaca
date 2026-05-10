//! Kernel-owned Plugin Registry invariants.
//!
//! The registry is deliberately narrow: it owns plugin identity, lifecycle
//! state, service descriptor ownership, capability descriptor ownership, and
//! cleanup.  It never executes plugin code and never implements gateway,
//! driver, memory, skill, MCP, payment, or compliance behavior.  Those remain
//! replaceable services/plugins outside the microkernel.

use std::collections::BTreeMap;

use macaca_proto::{
    CapabilityId, KernelServiceId, PluginError, PluginId, PluginLifecycleState, PluginManifest,
    PluginProvidedCapability, PluginProvidedService,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Registry entry for one plugin.
///
/// The entry stores the manifest and the descriptor ownership set as data.  It
/// does not hold executable handles.  Future WASM/native/process plugin hosts
/// should live behind runtime-host proxies and only publish descriptors here.
#[derive(Debug, Clone)]
pub struct PluginRegistryEntry {
    pub manifest: PluginManifest,
    pub state: PluginLifecycleState,
    pub services: Vec<PluginProvidedService>,
    pub capabilities: Vec<PluginProvidedCapability>,
    pub last_error: Option<String>,
}

impl PluginRegistryEntry {
    /// Create an installed entry from a validated manifest.
    fn installed(manifest: PluginManifest) -> Self {
        Self {
            services: manifest.provides_services.clone(),
            capabilities: manifest.provides_capabilities.clone(),
            manifest,
            state: PluginLifecycleState::Installed,
            last_error: None,
        }
    }
}

/// Snapshot returned to diagnostics and tests.
#[derive(Debug, Clone)]
pub struct PluginRegistrySnapshot {
    pub plugins: Vec<PluginRegistryEntry>,
    pub service_owners: BTreeMap<KernelServiceId, PluginId>,
    pub capability_owners: BTreeMap<CapabilityId, PluginId>,
}

/// In-memory implementation of the kernel plugin registry.
///
/// A later persistence-backed implementation can keep the same facade.  The
/// current data structure uses ownership maps so uninstall cleanup is atomic
/// and queryable without scanning every manifest on the hot path.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: RwLock<BTreeMap<PluginId, PluginRegistryEntry>>,
    service_owners: RwLock<BTreeMap<KernelServiceId, PluginId>>,
    capability_owners: RwLock<BTreeMap<CapabilityId, PluginId>>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a plugin manifest into registry state.
    ///
    /// Installation records identity and descriptor ownership but does not make
    /// the plugin runnable.  Runtime-host must still advance lifecycle states
    /// and perform permission/resource checks before any service is considered
    /// running.
    pub async fn install(&self, manifest: PluginManifest) -> Result<(), PluginError> {
        let plugin_id = manifest.plugin_id.clone();
        let mut plugins = self.plugins.write().await;
        if plugins.contains_key(&plugin_id) {
            warn!(plugin_id = %plugin_id, "plugin install rejected duplicate id");
            return Err(PluginError::DuplicatePlugin(plugin_id.to_string()));
        }
        plugins.insert(plugin_id.clone(), PluginRegistryEntry::installed(manifest));
        info!(plugin_id = %plugin_id, "plugin installed in registry");
        Ok(())
    }

    /// Register service and capability descriptors for an installed plugin.
    pub async fn register(&self, plugin_id: &PluginId) -> Result<(), PluginError> {
        self.transition(plugin_id, PluginLifecycleState::Registered)
            .await?;
        let entry = self.entry(plugin_id).await?;

        let mut service_owners = self.service_owners.write().await;
        for service in &entry.services {
            service_owners.insert(service.service.id.clone(), plugin_id.clone());
        }

        let mut capability_owners = self.capability_owners.write().await;
        for capability in &entry.capabilities {
            capability_owners.insert(capability.id.clone(), plugin_id.clone());
        }

        info!(
            plugin_id = %plugin_id,
            services = entry.services.len(),
            capabilities = entry.capabilities.len(),
            "plugin descriptors registered"
        );
        Ok(())
    }

    /// Transition a plugin through the typed lifecycle state machine.
    pub async fn transition(
        &self,
        plugin_id: &PluginId,
        next: PluginLifecycleState,
    ) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().await;
        let entry = plugins.get_mut(plugin_id).ok_or_else(|| {
            PluginError::Unavailable(format!("plugin not installed: {plugin_id}"))
        })?;
        let previous = entry.state.clone();
        if !is_valid_transition(&previous, &next) {
            warn!(
                plugin_id = %plugin_id,
                from = ?previous,
                to = ?next,
                "plugin lifecycle transition rejected"
            );
            return Err(PluginError::InvalidLifecycleTransition {
                from: previous,
                to: next,
            });
        }
        entry.state = next.clone();
        info!(
            plugin_id = %plugin_id,
            from = ?previous,
            to = ?next,
            "plugin lifecycle transition accepted"
        );
        Ok(())
    }

    /// Mark a plugin as failed while preserving the last structured error.
    pub async fn fail(&self, plugin_id: &PluginId, reason: impl Into<String>) {
        let reason = reason.into();
        let mut plugins = self.plugins.write().await;
        if let Some(entry) = plugins.get_mut(plugin_id) {
            entry.state = PluginLifecycleState::Failed;
            entry.last_error = Some(reason.clone());
            warn!(plugin_id = %plugin_id, error = %reason, "plugin marked failed");
        }
    }

    /// Uninstall a plugin and atomically remove every owned descriptor.
    pub async fn uninstall(&self, plugin_id: &PluginId) -> Result<(), PluginError> {
        self.transition(plugin_id, PluginLifecycleState::Uninstalled)
            .await?;
        let entry = self.entry(plugin_id).await?;

        let mut service_owners = self.service_owners.write().await;
        for service in &entry.services {
            service_owners.remove(&service.service.id);
        }

        let mut capability_owners = self.capability_owners.write().await;
        for capability in &entry.capabilities {
            capability_owners.remove(&capability.id);
        }

        self.plugins.write().await.remove(plugin_id);
        info!(plugin_id = %plugin_id, "plugin uninstalled and descriptors removed");
        Ok(())
    }

    /// Return a stable diagnostic snapshot sorted by plugin id.
    pub async fn snapshot(&self) -> PluginRegistrySnapshot {
        PluginRegistrySnapshot {
            plugins: self.plugins.read().await.values().cloned().collect(),
            service_owners: self.service_owners.read().await.clone(),
            capability_owners: self.capability_owners.read().await.clone(),
        }
    }

    async fn entry(&self, plugin_id: &PluginId) -> Result<PluginRegistryEntry, PluginError> {
        self.plugins
            .read()
            .await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::Unavailable(format!("plugin not installed: {plugin_id}")))
    }
}

/// Lifecycle Specification for Plugin Runtime v0.
///
/// Keeping this rule as a standalone function makes the state machine easy to
/// audit and avoids lifecycle if/else chains leaking into callers.
pub fn is_valid_transition(from: &PluginLifecycleState, to: &PluginLifecycleState) -> bool {
    use PluginLifecycleState::*;
    matches!(
        (from, to),
        (Installed, Registered)
            | (Registered, Starting)
            | (Starting, Running)
            | (Running, Stopping)
            | (Stopping, Stopped)
            | (Stopped, Uninstalled)
            | (Failed, Uninstalled)
            | (Installed, Failed)
            | (Registered, Failed)
            | (Starting, Failed)
            | (Stopping, Failed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        DeveloperId, PluginProvidedService, PluginRuntimeDeclaration, PluginRuntimeKind,
        PluginServiceCategory, PluginVersion, ServiceDescriptor, ServiceType, TraceSchemaRef,
    };

    fn manifest(id: &str) -> PluginManifest {
        let service = ServiceDescriptor::new(
            KernelServiceId::new(format!("service.{id}")),
            ServiceType::new("gateway"),
            TraceSchemaRef::new("trace.plugin.gateway"),
        );
        let mut manifest = PluginManifest::new(
            PluginId::new(id),
            PluginVersion::new("1.0.0"),
            DeveloperId::new("developer.local"),
            PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
        );
        manifest.provides_services.push(PluginProvidedService::new(
            service,
            PluginServiceCategory::Gateway,
        ));
        manifest
    }

    #[tokio::test]
    async fn duplicate_plugin_id_is_rejected() {
        let registry = PluginRegistry::new();
        registry.install(manifest("plugin.one")).await.unwrap();
        let err = registry.install(manifest("plugin.one")).await.unwrap_err();
        assert!(matches!(err, PluginError::DuplicatePlugin(_)));
    }

    #[tokio::test]
    async fn register_and_uninstall_cleans_descriptors() {
        let registry = PluginRegistry::new();
        let plugin_id = PluginId::new("plugin.cleanup");
        registry
            .install(manifest(plugin_id.as_str()))
            .await
            .unwrap();
        registry.register(&plugin_id).await.unwrap();
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.service_owners.len(), 1);

        registry
            .transition(&plugin_id, PluginLifecycleState::Starting)
            .await
            .unwrap();
        registry
            .transition(&plugin_id, PluginLifecycleState::Running)
            .await
            .unwrap();
        registry
            .transition(&plugin_id, PluginLifecycleState::Stopping)
            .await
            .unwrap();
        registry
            .transition(&plugin_id, PluginLifecycleState::Stopped)
            .await
            .unwrap();
        registry.uninstall(&plugin_id).await.unwrap();

        let snapshot = registry.snapshot().await;
        assert!(snapshot.plugins.is_empty());
        assert!(snapshot.service_owners.is_empty());
    }

    #[tokio::test]
    async fn invalid_transition_is_rejected() {
        let registry = PluginRegistry::new();
        let plugin_id = PluginId::new("plugin.invalid");
        registry
            .install(manifest(plugin_id.as_str()))
            .await
            .unwrap();
        let err = registry
            .transition(&plugin_id, PluginLifecycleState::Running)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            PluginError::InvalidLifecycleTransition { .. }
        ));
    }

    #[tokio::test]
    async fn failed_state_preserves_error() {
        let registry = PluginRegistry::new();
        let plugin_id = PluginId::new("plugin.failed");
        registry
            .install(manifest(plugin_id.as_str()))
            .await
            .unwrap();
        registry.fail(&plugin_id, "boom").await;
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.plugins[0].state, PluginLifecycleState::Failed);
        assert_eq!(snapshot.plugins[0].last_error.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn single_plugin_registers_multiple_services_and_capabilities() {
        let registry = PluginRegistry::new();
        let plugin_id = PluginId::new("plugin.multi");
        let mut manifest = manifest(plugin_id.as_str());
        let second_service = ServiceDescriptor::new(
            KernelServiceId::new("service.plugin.multi.second"),
            ServiceType::new("context"),
            TraceSchemaRef::new("trace.plugin.context"),
        );
        manifest.provides_services.push(PluginProvidedService::new(
            second_service,
            PluginServiceCategory::Context,
        ));
        manifest
            .provides_capabilities
            .push(PluginProvidedCapability {
                id: CapabilityId::new("capability.plugin.multi.first"),
                service: Some(KernelServiceId::new(format!("service.{}", plugin_id))),
                category: PluginServiceCategory::Gateway,
                description: "First capability".into(),
                required_permissions: Vec::new(),
                metadata: BTreeMap::new(),
            });
        manifest
            .provides_capabilities
            .push(PluginProvidedCapability {
                id: CapabilityId::new("capability.plugin.multi.second"),
                service: Some(KernelServiceId::new("service.plugin.multi.second")),
                category: PluginServiceCategory::Context,
                description: "Second capability".into(),
                required_permissions: Vec::new(),
                metadata: BTreeMap::new(),
            });

        registry.install(manifest).await.unwrap();
        registry.register(&plugin_id).await.unwrap();

        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.service_owners.len(), 2);
        assert_eq!(snapshot.capability_owners.len(), 2);
    }

    #[tokio::test]
    async fn failed_plugin_can_be_uninstalled_for_cleanup() {
        let registry = PluginRegistry::new();
        let plugin_id = PluginId::new("plugin.failed-cleanup");
        registry
            .install(manifest(plugin_id.as_str()))
            .await
            .unwrap();
        registry.register(&plugin_id).await.unwrap();
        registry.fail(&plugin_id, "boom").await;
        registry.uninstall(&plugin_id).await.unwrap();

        let snapshot = registry.snapshot().await;
        assert!(snapshot.plugins.is_empty());
        assert!(snapshot.service_owners.is_empty());
    }
}
