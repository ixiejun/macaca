//! In-memory Plugin Capability Registry v1.
//!
//! The registry stores descriptor ownership and indexes only.  It does not
//! execute plugin code, call service implementations, load providers, or apply
//! application-specific routing.  This keeps the Plugin Capability Plane a
//! control-plane registry rather than a new macro-kernel.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    CapabilityId, PluginCapabilityDeactivateCommand, PluginCapabilityDescriptor,
    PluginCapabilityEvent, PluginCapabilityInspectCommand, PluginCapabilityOwnership,
    PluginCapabilityQueryCommand, PluginCapabilityRegisterCommand,
    PluginCapabilityRegistrySnapshot, PluginError, PluginId, TraceContext,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::plugin_capability::conflict::{
    FailClosedSlotConflictPolicy, PluginCapabilityConflictPolicy,
};

/// Builder for capability registry strategies.
#[derive(Default)]
pub struct PluginCapabilityRegistryBuilder {
    conflict_policy: Option<Arc<dyn PluginCapabilityConflictPolicy>>,
}

impl PluginCapabilityRegistryBuilder {
    /// Replace the default conflict policy Strategy.
    pub fn conflict_policy(mut self, policy: Arc<dyn PluginCapabilityConflictPolicy>) -> Self {
        self.conflict_policy = Some(policy);
        self
    }

    /// Build a registry with safe defaults for omitted strategies.
    pub fn build(self) -> PluginCapabilityRegistry {
        PluginCapabilityRegistry {
            conflict_policy: self
                .conflict_policy
                .unwrap_or_else(|| Arc::new(FailClosedSlotConflictPolicy)),
            capabilities: RwLock::new(BTreeMap::new()),
        }
    }
}

/// Runtime-host-owned capability registry facade.
pub struct PluginCapabilityRegistry {
    conflict_policy: Arc<dyn PluginCapabilityConflictPolicy>,
    capabilities: RwLock<BTreeMap<CapabilityId, PluginCapabilityOwnership>>,
}

impl Default for PluginCapabilityRegistry {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl PluginCapabilityRegistry {
    /// Start building a registry from explicit strategies.
    pub fn builder() -> PluginCapabilityRegistryBuilder {
        PluginCapabilityRegistryBuilder::default()
    }

    /// Register descriptors for one plugin after conflict validation.
    pub async fn register(
        &self,
        command: PluginCapabilityRegisterCommand,
    ) -> Result<Vec<PluginCapabilityOwnership>, PluginError> {
        validate_descriptors(&command.plugin_id, &command.descriptors)?;
        let active = self.active_ownership().await;
        for descriptor in &command.descriptors {
            self.conflict_policy.check(descriptor, &active).await?;
        }

        let mut capabilities = self.capabilities.write().await;
        let mut registered = Vec::new();
        for descriptor in command.descriptors {
            let ownership = PluginCapabilityOwnership {
                capability_id: descriptor.capability_id.clone(),
                plugin_id: command.plugin_id.clone(),
                kind: descriptor.kind.clone(),
                service: descriptor.service.clone(),
                active: true,
                registered_at: Utc::now(),
                descriptor,
            };
            info!(
                trace_id = %command.trace.trace_id,
                plugin_id = %ownership.plugin_id,
                capability_id = %ownership.capability_id,
                kind = %ownership.kind,
                "plugin capability registered"
            );
            capabilities.insert(ownership.capability_id.clone(), ownership.clone());
            registered.push(ownership);
        }
        Ok(registered)
    }

    /// Deactivate every active capability owned by a plugin.
    pub async fn deactivate(
        &self,
        command: PluginCapabilityDeactivateCommand,
    ) -> Result<Vec<PluginCapabilityEvent>, PluginError> {
        let mut capabilities = self.capabilities.write().await;
        let mut events = Vec::new();
        for ownership in capabilities
            .values_mut()
            .filter(|owner| owner.plugin_id == command.plugin_id && owner.active)
        {
            ownership.active = false;
            info!(
                trace_id = %command.trace.trace_id,
                plugin_id = %ownership.plugin_id,
                capability_id = %ownership.capability_id,
                reason = %command.reason,
                "plugin capability deactivated"
            );
            events.push(PluginCapabilityEvent::new(
                Some(ownership.plugin_id.clone()),
                Some(ownership.capability_id.clone()),
                Some(ownership.kind.clone()),
                "plugin.capability.deactivate",
                "ok",
                command.trace.clone(),
            ));
        }
        Ok(events)
    }

    /// Query active or inactive capability ownership records deterministically.
    pub async fn query(
        &self,
        command: PluginCapabilityQueryCommand,
    ) -> Result<Vec<PluginCapabilityOwnership>, PluginError> {
        let mut records = self
            .capabilities
            .read()
            .await
            .values()
            .filter(|owner| command.include_inactive || owner.active)
            .filter(|owner| command.kind.as_ref().is_none_or(|kind| &owner.kind == kind))
            .filter(|owner| {
                command
                    .plugin_id
                    .as_ref()
                    .is_none_or(|plugin_id| &owner.plugin_id == plugin_id)
            })
            .filter(|owner| slot_matches(owner, &command))
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
        info!(
            trace_id = %command.trace.trace_id,
            records = records.len(),
            "plugin capability query completed"
        );
        Ok(records)
    }

    /// Inspect one capability by id.
    pub async fn inspect(
        &self,
        command: PluginCapabilityInspectCommand,
    ) -> Result<Option<PluginCapabilityOwnership>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            capability_id = %command.capability_id,
            "plugin capability inspect requested"
        );
        Ok(self
            .capabilities
            .read()
            .await
            .get(&command.capability_id)
            .cloned())
    }

    /// Return a deterministic snapshot for diagnostics and tests.
    pub async fn snapshot(&self, trace: TraceContext) -> PluginCapabilityRegistrySnapshot {
        let mut capabilities = self
            .capabilities
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        capabilities.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
        info!(
            trace_id = %trace.trace_id,
            capabilities = capabilities.len(),
            "plugin capability registry snapshot built"
        );
        PluginCapabilityRegistrySnapshot {
            generated_at: Utc::now(),
            capabilities,
            conflicts: Vec::new(),
        }
    }

    async fn active_ownership(&self) -> Vec<PluginCapabilityOwnership> {
        self.capabilities
            .read()
            .await
            .values()
            .filter(|owner| owner.active)
            .cloned()
            .collect()
    }
}

fn validate_descriptors(
    plugin_id: &PluginId,
    descriptors: &[PluginCapabilityDescriptor],
) -> Result<(), PluginError> {
    for descriptor in descriptors {
        if descriptor.capability_id.is_empty() {
            return Err(PluginError::Unavailable(
                "capability descriptor requires non-empty capability id".into(),
            ));
        }
        if descriptor.plugin_id != *plugin_id {
            warn!(
                expected_plugin_id = %plugin_id,
                descriptor_plugin_id = %descriptor.plugin_id,
                capability_id = %descriptor.capability_id,
                "plugin capability descriptor owner mismatch"
            );
            return Err(PluginError::Unavailable(
                "capability descriptor plugin id does not match registration owner".into(),
            ));
        }
        if descriptor.name.trim().is_empty() {
            return Err(PluginError::Unavailable(format!(
                "capability descriptor requires a name: {}",
                descriptor.capability_id
            )));
        }
    }
    Ok(())
}

fn slot_matches(owner: &PluginCapabilityOwnership, command: &PluginCapabilityQueryCommand) -> bool {
    match (&command.slot_namespace, &command.slot_key) {
        (None, None) => true,
        (Some(namespace), None) => owner
            .descriptor
            .slots
            .iter()
            .any(|slot| &slot.namespace == namespace),
        (Some(namespace), Some(key)) => owner
            .descriptor
            .slots
            .iter()
            .any(|slot| &slot.namespace == namespace && &slot.key == key),
        (None, Some(_)) => false,
    }
}
