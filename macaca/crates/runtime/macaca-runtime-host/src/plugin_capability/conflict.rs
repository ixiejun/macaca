//! Conflict-policy strategies for Plugin Capability Registry v1.
//!
//! Conflict policy is modeled as a Strategy so deployments can replace slot
//! rules later without changing registry storage.  The default policy fails
//! closed for exclusive slots, which covers tool names, gateway routes, HTTP
//! routes, CLI commands, provider slots, and custom exclusive namespaces.

use async_trait::async_trait;
use macaca_proto::{
    PluginCapabilityConflictReport, PluginCapabilityDescriptor, PluginCapabilityOwnership,
    PluginError,
};

/// Replaceable conflict policy boundary.
#[async_trait]
pub trait PluginCapabilityConflictPolicy: Send + Sync {
    /// Validate one candidate descriptor against active ownership records.
    async fn check(
        &self,
        candidate: &PluginCapabilityDescriptor,
        active: &[PluginCapabilityOwnership],
    ) -> Result<(), PluginError>;
}

/// Default fail-closed conflict policy for exclusive capability slots.
#[derive(Debug, Default, Clone)]
pub struct FailClosedSlotConflictPolicy;

#[async_trait]
impl PluginCapabilityConflictPolicy for FailClosedSlotConflictPolicy {
    async fn check(
        &self,
        candidate: &PluginCapabilityDescriptor,
        active: &[PluginCapabilityOwnership],
    ) -> Result<(), PluginError> {
        if let Some(conflict) = detect_slot_conflict(candidate, active) {
            return Err(PluginError::Unavailable(format!(
                "capability conflict in slot {}:{} between {} ({}) and {} ({})",
                conflict.namespace,
                conflict.key,
                conflict.rejected_plugin_id,
                conflict.rejected_capability_id,
                conflict.existing_plugin_id,
                conflict.existing_capability_id
            )));
        }
        Ok(())
    }
}

/// Build a structured conflict report for the first exclusive-slot conflict.
pub fn detect_slot_conflict(
    candidate: &PluginCapabilityDescriptor,
    active: &[PluginCapabilityOwnership],
) -> Option<PluginCapabilityConflictReport> {
    for candidate_slot in candidate.slots.iter().filter(|slot| slot.exclusive) {
        for owner in active {
            if !owner.active {
                continue;
            }
            if owner.capability_id == candidate.capability_id {
                continue;
            }
            let Some(existing_slot) = owner.descriptor.slots.iter().find(|slot| {
                slot.exclusive
                    && slot.namespace == candidate_slot.namespace
                    && slot.key == candidate_slot.key
            }) else {
                continue;
            };
            return Some(PluginCapabilityConflictReport {
                namespace: existing_slot.namespace.clone(),
                key: existing_slot.key.clone(),
                rejected_plugin_id: candidate.plugin_id.clone(),
                rejected_capability_id: candidate.capability_id.clone(),
                existing_plugin_id: owner.plugin_id.clone(),
                existing_capability_id: owner.capability_id.clone(),
                reason: "exclusive capability slot is already owned".into(),
            });
        }
    }
    None
}
