//! Plugin Hook Bus service facade.
//!
//! The facade composes descriptor registration, deterministic lookup, bounded
//! runner execution, and traceable audit.  It is the runtime-host surface that
//! framework/services can call without depending on concrete plugin hosts.

use std::sync::Arc;

use macaca_proto::{
    PluginError, PluginHookDeactivateCommand, PluginHookEvent, PluginHookInvokeCommand,
    PluginHookInvokeResult, PluginHookQueryCommand, PluginHookRegisterCommand,
    PluginHookRegistration, PluginHookRegistrySnapshot, PluginHookSnapshotCommand,
};
use tracing::info;

use crate::plugin_hook::registry::{
    NoopPluginHookEventSink, PluginHookEventSink, PluginHookRegistry,
};
use crate::plugin_hook::runner::PluginHookRunner;

/// Builder for Hook Bus strategies.
#[derive(Default)]
pub struct PluginHookBusBuilder {
    registry: Option<Arc<PluginHookRegistry>>,
    runner: Option<Arc<PluginHookRunner>>,
    event_sink: Option<Arc<dyn PluginHookEventSink>>,
}

impl PluginHookBusBuilder {
    /// Use an externally managed registry.
    pub fn registry(mut self, registry: Arc<PluginHookRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Use an externally managed runner.
    pub fn runner(mut self, runner: Arc<PluginHookRunner>) -> Self {
        self.runner = Some(runner);
        self
    }

    /// Use an audit sink shared by registry and runner.
    pub fn event_sink(mut self, event_sink: Arc<dyn PluginHookEventSink>) -> Self {
        self.event_sink = Some(event_sink);
        self
    }

    /// Build a Hook Bus facade.
    pub fn build(self) -> PluginHookBus {
        let event_sink = self
            .event_sink
            .unwrap_or_else(|| Arc::new(NoopPluginHookEventSink));
        let registry = self.registry.unwrap_or_else(|| {
            Arc::new(
                PluginHookRegistry::builder()
                    .event_sink(Arc::clone(&event_sink))
                    .build(),
            )
        });
        let runner = self
            .runner
            .unwrap_or_else(|| Arc::new(PluginHookRunner::descriptor_only(event_sink)));
        PluginHookBus { registry, runner }
    }
}

/// Runtime-host-owned Hook Bus facade.
pub struct PluginHookBus {
    registry: Arc<PluginHookRegistry>,
    runner: Arc<PluginHookRunner>,
}

impl PluginHookBus {
    /// Start building a Hook Bus from explicit strategies.
    pub fn builder() -> PluginHookBusBuilder {
        PluginHookBusBuilder::default()
    }

    /// Create a deterministic in-memory Hook Bus.
    pub fn in_memory() -> Self {
        Self::builder().build()
    }

    /// Register hook descriptors.
    pub async fn register(
        &self,
        command: PluginHookRegisterCommand,
    ) -> Result<Vec<PluginHookRegistration>, PluginError> {
        self.registry.register(command).await
    }

    /// Deactivate all descriptors owned by one plugin.
    pub async fn deactivate(
        &self,
        command: PluginHookDeactivateCommand,
    ) -> Result<Vec<PluginHookEvent>, PluginError> {
        self.registry.deactivate(command).await
    }

    /// Query descriptors.
    pub async fn query(
        &self,
        command: PluginHookQueryCommand,
    ) -> Result<Vec<PluginHookRegistration>, PluginError> {
        self.registry.query(command).await
    }

    /// Invoke one hook point.
    pub async fn invoke(
        &self,
        command: PluginHookInvokeCommand,
    ) -> Result<PluginHookInvokeResult, PluginError> {
        info!(
            trace_id = %command.trace.trace_id,
            hook_name = %command.hook_name,
            "plugin hook bus invocation requested"
        );
        let registrations = self
            .registry
            .query(PluginHookQueryCommand {
                hook_name: Some(command.hook_name.clone()),
                plugin_id: None,
                include_inactive: false,
                trace: command.trace.clone(),
            })
            .await?;
        self.runner.run(command, registrations).await
    }

    /// Return registry diagnostics.
    pub async fn snapshot(&self, command: PluginHookSnapshotCommand) -> PluginHookRegistrySnapshot {
        self.registry.snapshot(command.trace).await
    }
}
