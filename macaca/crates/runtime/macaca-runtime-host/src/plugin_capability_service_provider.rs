//! ServiceRuntime adapter for Plugin Capability Registry v1.
//!
//! The adapter translates generic Route C `ServiceCommand` envelopes into the
//! typed Plugin Capability Service facade.  It owns transport decoding only;
//! registry policy, conflict detection, and descriptor-safe call semantics stay
//! behind `PluginCapabilityService`.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    plugin_capability_registry_service_id, CleanupPolicy, ServiceCallResult, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceScope, ServiceType, TraceContext, TraceSchemaRef, PLUGIN_CAPABILITY_CALL_COMMAND,
    PLUGIN_CAPABILITY_DEACTIVATE_COMMAND, PLUGIN_CAPABILITY_DISCOVER_COMMAND,
    PLUGIN_CAPABILITY_INSPECT_COMMAND, PLUGIN_CAPABILITY_QUERY_COMMAND,
    PLUGIN_CAPABILITY_REGISTER_COMMAND,
};
use tracing::info;

use crate::plugin_capability::PluginCapabilityService;

/// ServiceRuntime provider for Plugin Capability Registry.
pub struct PluginCapabilitySystemServiceProvider {
    descriptor: ServiceDescriptor,
    service: Arc<PluginCapabilityService>,
}

impl PluginCapabilitySystemServiceProvider {
    /// Create a provider over an injected service facade.
    pub fn new(service: Arc<PluginCapabilityService>) -> Self {
        Self {
            descriptor: plugin_capability_service_descriptor(),
            service,
        }
    }

    /// Create a deterministic in-memory provider for local hosts and tests.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(PluginCapabilityService::in_memory()))
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result<T: serde::Serialize>(
        value: T,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(value)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

#[async_trait]
impl SystemService for PluginCapabilitySystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin capability service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "plugin capability service command accepted"
        );
        match command.name.as_str() {
            PLUGIN_CAPABILITY_DISCOVER_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.discover(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CAPABILITY_REGISTER_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.register(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CAPABILITY_DEACTIVATE_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.deactivate(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CAPABILITY_QUERY_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.query(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CAPABILITY_INSPECT_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.inspect(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CAPABILITY_CALL_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.service.call(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin capability service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin capability service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Build the provider-neutral service descriptor for capability registry.
pub fn plugin_capability_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        plugin_capability_registry_service_id(),
        ServiceType::new("plugin_capability_registry"),
        TraceSchemaRef::new("trace.plugin.capability_registry.v1"),
    );
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor
        .metadata
        .insert("phase".into(), "capability_registry_v1".into());
    descriptor
}

/// Build a typed service command for tests and local adapters.
pub fn plugin_capability_service_command<T: serde::Serialize>(
    command_name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceResult<ServiceCommand> {
    Ok(ServiceCommand::with_trace(
        ServiceCommandName::new(command_name),
        serde_json::to_value(payload)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        trace,
    ))
}

fn decode<T>(value: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

fn plugin_error(error: macaca_proto::PluginError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        CapabilityId, PluginCapabilityDescriptor, PluginCapabilityKind,
        PluginCapabilityQueryCommand, PluginCapabilityRegisterCommand, PluginId, TraceContext,
        TraceSchemaRef, PLUGIN_CAPABILITY_QUERY_COMMAND, PLUGIN_CAPABILITY_REGISTER_COMMAND,
    };

    use super::*;

    #[tokio::test]
    async fn provider_dispatches_register_and_query_commands() {
        let provider = PluginCapabilitySystemServiceProvider::in_memory();
        provider.start().await.unwrap();
        let descriptor = PluginCapabilityDescriptor::new(
            CapabilityId::new("capability.fixture"),
            PluginId::new("plugin.fixture"),
            PluginCapabilityKind::Tool,
            "fixture",
            TraceSchemaRef::new("trace.fixture"),
        );
        let register = plugin_capability_service_command(
            PLUGIN_CAPABILITY_REGISTER_COMMAND,
            PluginCapabilityRegisterCommand {
                plugin_id: PluginId::new("plugin.fixture"),
                descriptors: vec![descriptor],
                trace: TraceContext::new("trace-register"),
                metadata: Default::default(),
            },
            TraceContext::new("trace-register-envelope"),
        )
        .unwrap();
        provider.call(register).await.unwrap();

        let query = plugin_capability_service_command(
            PLUGIN_CAPABILITY_QUERY_COMMAND,
            PluginCapabilityQueryCommand {
                kind: None,
                plugin_id: Some(PluginId::new("plugin.fixture")),
                slot_namespace: None,
                slot_key: None,
                include_inactive: false,
                trace: TraceContext::new("trace-query"),
            },
            TraceContext::new("trace-query-envelope"),
        )
        .unwrap();
        let result = provider.call(query).await.unwrap();
        let records: Vec<macaca_proto::PluginCapabilityOwnership> =
            serde_json::from_value(result.output).unwrap();
        assert_eq!(records.len(), 1);
    }
}
