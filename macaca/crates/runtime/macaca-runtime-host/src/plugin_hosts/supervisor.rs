//! Host lifecycle supervisor skeleton.
//!
//! The supervisor is a Facade over host selection, command dispatch, health
//! probing, and traceable logging. It is deliberately non-invasive: v1 does not
//! allocate real sandbox resources or run external code, but every lifecycle
//! operation still flows through the same command shape future real hosts will
//! use.

use macaca_proto::{
    PluginHostCommand, PluginHostDescriptor, PluginHostHealthSnapshot, PluginHostOperation,
    PluginHostResult, TraceContext,
};
use tracing::info;

use super::{PluginHostRuntimeFactory, PluginHostRuntimeResult};

/// Supervises lifecycle commands for one host descriptor at a time.
#[derive(Debug, Clone, Default)]
pub struct PluginHostLifecycleSupervisor {
    factory: PluginHostRuntimeFactory,
}

impl PluginHostLifecycleSupervisor {
    /// Create a supervisor with the default host Abstract Factory.
    pub fn new() -> Self {
        Self {
            factory: PluginHostRuntimeFactory,
        }
    }

    /// Create a supervisor from an explicit factory for tests or embedding.
    pub fn with_factory(factory: PluginHostRuntimeFactory) -> Self {
        Self { factory }
    }

    /// Dispatch one lifecycle or invocation command to the selected host.
    pub async fn execute(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostResult> {
        let timeout = command.descriptor.timeout_policy.as_millis(5_000);
        info!(
            plugin_id = %command.descriptor.plugin_id,
            runtime_kind = %command.descriptor.runtime_kind,
            operation = %command.operation,
            trace_id = %command.trace.trace_id,
            timeout_millis = ?timeout,
            resource_lease_count = command.descriptor.resource_leases.len(),
            "plugin host supervisor dispatching command"
        );
        let host = self.factory.create(&command.descriptor);
        host.execute(command).await
    }

    /// Build and execute a command from operation primitives.
    pub async fn execute_operation(
        &self,
        descriptor: PluginHostDescriptor,
        operation: PluginHostOperation,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> PluginHostRuntimeResult<PluginHostResult> {
        self.execute(PluginHostCommand::new(
            descriptor, operation, payload, trace,
        ))
        .await
    }

    /// Probe host health through the selected host strategy.
    pub async fn health(
        &self,
        descriptor: PluginHostDescriptor,
        trace: TraceContext,
    ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
        let command = PluginHostCommand::new(
            descriptor,
            PluginHostOperation::HealthProbe,
            serde_json::json!({}),
            trace,
        );
        info!(
            plugin_id = %command.descriptor.plugin_id,
            runtime_kind = %command.descriptor.runtime_kind,
            trace_id = %command.trace.trace_id,
            "plugin host supervisor dispatching health probe"
        );
        let host = self.factory.create(&command.descriptor);
        host.health(command).await
    }
}
