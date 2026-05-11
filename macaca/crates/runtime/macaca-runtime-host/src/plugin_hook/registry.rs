//! In-memory Plugin Hook Registry.
//!
//! The registry stores hook descriptors and active ownership.  It never invokes
//! plugin code.  Keeping storage separate from execution follows the
//! contract-first model: Macaca can discover hook participation and reason
//! about priority, policy, and audit shape before any executable host exists.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    PluginError, PluginHookDeactivateCommand, PluginHookDescriptor, PluginHookEvent,
    PluginHookName, PluginHookQueryCommand, PluginHookRegisterCommand, PluginHookRegistration,
    PluginHookRegistrySnapshot, PluginId, TraceContext,
};
use tokio::sync::RwLock;
use tracing::info;

/// Builder for hook registry strategies.
#[derive(Default)]
pub struct PluginHookRegistryBuilder {
    event_sink: Option<Arc<dyn PluginHookEventSink>>,
}

impl PluginHookRegistryBuilder {
    /// Install an event sink for tests or audit collectors.
    pub fn event_sink(mut self, event_sink: Arc<dyn PluginHookEventSink>) -> Self {
        self.event_sink = Some(event_sink);
        self
    }

    /// Build an in-memory registry.
    pub fn build(self) -> PluginHookRegistry {
        PluginHookRegistry {
            event_sink: self
                .event_sink
                .unwrap_or_else(|| Arc::new(NoopPluginHookEventSink)),
            hooks: RwLock::new(BTreeMap::new()),
        }
    }
}

/// Audit sink boundary for registry and runner events.
pub trait PluginHookEventSink: Send + Sync {
    /// Emit one audit-safe event.  Implementations must not enrich it with raw
    /// hook payloads or secrets.
    fn emit(&self, event: PluginHookEvent);
}

/// Null Object event sink used when no collector is configured.
pub struct NoopPluginHookEventSink;

impl PluginHookEventSink for NoopPluginHookEventSink {
    fn emit(&self, _event: PluginHookEvent) {}
}

/// Runtime-host-owned hook registry.
pub struct PluginHookRegistry {
    event_sink: Arc<dyn PluginHookEventSink>,
    hooks: RwLock<BTreeMap<(PluginHookName, PluginId), PluginHookRegistration>>,
}

impl Default for PluginHookRegistry {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl PluginHookRegistry {
    /// Start building a registry with replaceable audit strategies.
    pub fn builder() -> PluginHookRegistryBuilder {
        PluginHookRegistryBuilder::default()
    }

    /// Register descriptors for one plugin.
    pub async fn register(
        &self,
        command: PluginHookRegisterCommand,
    ) -> Result<Vec<PluginHookRegistration>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            descriptors = command.descriptors.len(),
            "plugin hook registry registering descriptors"
        );
        let mut hooks = self.hooks.write().await;
        let mut registered = Vec::new();
        for descriptor in command.descriptors {
            validate_descriptor(&command.plugin_id, &descriptor)?;
            let registration = PluginHookRegistration {
                plugin_id: command.plugin_id.clone(),
                hook_name: descriptor.hook_name.clone(),
                kind: descriptor.kind.clone(),
                active: true,
                registered_at: Utc::now(),
                descriptor,
            };
            hooks.insert(
                (
                    registration.hook_name.clone(),
                    registration.plugin_id.clone(),
                ),
                registration.clone(),
            );
            self.event_sink.emit(PluginHookEvent::new(
                Some(registration.plugin_id.clone()),
                Some(registration.hook_name.clone()),
                Some(registration.kind.clone()),
                "register",
                "accepted",
                command.trace.clone(),
            ));
            registered.push(registration);
        }
        Ok(registered)
    }

    /// Deactivate all hook descriptors owned by a plugin.
    pub async fn deactivate(
        &self,
        command: PluginHookDeactivateCommand,
    ) -> Result<Vec<PluginHookEvent>, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            reason = %command.reason,
            "plugin hook registry deactivating plugin hooks"
        );
        let mut hooks = self.hooks.write().await;
        let mut events = Vec::new();
        for registration in hooks.values_mut() {
            if registration.plugin_id == command.plugin_id && registration.active {
                registration.active = false;
                let mut event = PluginHookEvent::new(
                    Some(registration.plugin_id.clone()),
                    Some(registration.hook_name.clone()),
                    Some(registration.kind.clone()),
                    "deactivate",
                    "accepted",
                    command.trace.clone(),
                );
                event
                    .metadata
                    .insert("reason".into(), command.reason.clone());
                self.event_sink.emit(event.clone());
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Query hook registrations deterministically.
    pub async fn query(
        &self,
        command: PluginHookQueryCommand,
    ) -> Result<Vec<PluginHookRegistration>, PluginError> {
        let mut records: Vec<_> = self
            .hooks
            .read()
            .await
            .values()
            .filter(|registration| command.include_inactive || registration.active)
            .filter(|registration| {
                command
                    .hook_name
                    .as_ref()
                    .is_none_or(|hook_name| hook_name == &registration.hook_name)
            })
            .filter(|registration| {
                command
                    .plugin_id
                    .as_ref()
                    .is_none_or(|plugin_id| plugin_id == &registration.plugin_id)
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| {
            left.descriptor
                .priority
                .cmp(&right.descriptor.priority)
                .then_with(|| left.plugin_id.cmp(&right.plugin_id))
                .then_with(|| left.hook_name.cmp(&right.hook_name))
        });
        Ok(records)
    }

    /// Return a deterministic registry snapshot.
    pub async fn snapshot(&self, trace: TraceContext) -> PluginHookRegistrySnapshot {
        info!(
            trace_id = %trace.trace_id,
            "plugin hook registry snapshot requested"
        );
        PluginHookRegistrySnapshot {
            generated_at: Utc::now(),
            hooks: self
                .query(PluginHookQueryCommand {
                    hook_name: None,
                    plugin_id: None,
                    include_inactive: true,
                    trace,
                })
                .await
                .unwrap_or_default(),
        }
    }
}

fn validate_descriptor(
    owner: &PluginId,
    descriptor: &PluginHookDescriptor,
) -> Result<(), PluginError> {
    if &descriptor.plugin_id != owner {
        return Err(PluginError::Unavailable(
            "hook descriptor plugin id must match registration owner".into(),
        ));
    }
    if descriptor.trace_schema.as_str().is_empty() {
        return Err(PluginError::Unavailable(
            "hook descriptor trace schema cannot be empty".into(),
        ));
    }
    Ok(())
}
