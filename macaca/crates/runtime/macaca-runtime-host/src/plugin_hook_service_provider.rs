//! ServiceRuntime adapter for Plugin Hook Bus v1.
//!
//! The adapter translates generic protocol `ServiceCommand` envelopes into the
//! typed Hook Bus facade.  It owns transport decoding only; hook ordering,
//! timeout/failure policy, descriptor-only execution, and audit semantics stay
//! behind `PluginHookBus`.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    plugin_hook_bus_service_id, CleanupPolicy, ServiceCallResult, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceScope, ServiceType, TraceContext, TraceSchemaRef, PLUGIN_HOOK_DEACTIVATE_COMMAND,
    PLUGIN_HOOK_INVOKE_COMMAND, PLUGIN_HOOK_QUERY_COMMAND, PLUGIN_HOOK_REGISTER_COMMAND,
    PLUGIN_HOOK_SNAPSHOT_COMMAND,
};
use tracing::info;

use crate::plugin_hook::PluginHookBus;

/// ServiceRuntime provider for Plugin Hook Bus.
pub struct PluginHookSystemServiceProvider {
    descriptor: ServiceDescriptor,
    bus: Arc<PluginHookBus>,
}

impl PluginHookSystemServiceProvider {
    /// Create a provider over an injected Hook Bus facade.
    pub fn new(bus: Arc<PluginHookBus>) -> Self {
        Self {
            descriptor: plugin_hook_service_descriptor(),
            bus,
        }
    }

    /// Create a deterministic in-memory provider for local hosts and tests.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(PluginHookBus::in_memory()))
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
impl SystemService for PluginHookSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin hook service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "plugin hook service command accepted"
        );
        match command.name.as_str() {
            PLUGIN_HOOK_REGISTER_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.bus.register(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_HOOK_DEACTIVATE_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.bus.deactivate(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_HOOK_QUERY_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.bus.query(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_HOOK_INVOKE_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.bus.invoke(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_HOOK_SNAPSHOT_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.bus.snapshot(typed).await;
                Self::result(output, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin hook service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin hook service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Build the provider-neutral service descriptor for Hook Bus.
pub fn plugin_hook_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        plugin_hook_bus_service_id(),
        ServiceType::new("plugin_hook_bus"),
        TraceSchemaRef::new("trace.plugin.hook_bus.v1"),
    );
    descriptor.supported_scopes = vec![ServiceScope::Global];
    descriptor
        .metadata
        .insert("phase".into(), "hook_bus_v1".into());
    descriptor
}

/// Build a typed service command for tests and local adapters.
pub fn plugin_hook_service_command<T: serde::Serialize>(
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
        PluginHookContext, PluginHookDescriptor, PluginHookInvokeCommand, PluginHookName,
        PluginHookQueryCommand, PluginHookRegisterCommand, PluginId, TraceContext, TraceSchemaRef,
        PLUGIN_HOOK_INVOKE_COMMAND, PLUGIN_HOOK_QUERY_COMMAND, PLUGIN_HOOK_REGISTER_COMMAND,
    };

    use super::*;

    #[tokio::test]
    async fn provider_dispatches_register_query_and_invoke_commands() {
        let provider = PluginHookSystemServiceProvider::in_memory();
        provider.start().await.unwrap();
        let descriptor = PluginHookDescriptor::observer(
            PluginId::new("plugin.fixture"),
            PluginHookName::BeforeToolCall,
            0,
            TraceSchemaRef::new("trace.plugin.hook.fixture"),
        );
        let register = plugin_hook_service_command(
            PLUGIN_HOOK_REGISTER_COMMAND,
            PluginHookRegisterCommand {
                plugin_id: PluginId::new("plugin.fixture"),
                descriptors: vec![descriptor],
                trace: TraceContext::new("trace-register"),
                metadata: Default::default(),
            },
            TraceContext::new("trace-register-envelope"),
        )
        .unwrap();
        provider.call(register).await.unwrap();

        let query = plugin_hook_service_command(
            PLUGIN_HOOK_QUERY_COMMAND,
            PluginHookQueryCommand {
                hook_name: Some(PluginHookName::BeforeToolCall),
                plugin_id: None,
                include_inactive: false,
                trace: TraceContext::new("trace-query"),
            },
            TraceContext::new("trace-query-envelope"),
        )
        .unwrap();
        let result = provider.call(query).await.unwrap();
        let records: Vec<macaca_proto::PluginHookRegistration> =
            serde_json::from_value(result.output).unwrap();
        assert_eq!(records.len(), 1);

        let invoke = plugin_hook_service_command(
            PLUGIN_HOOK_INVOKE_COMMAND,
            PluginHookInvokeCommand {
                hook_name: PluginHookName::BeforeToolCall,
                expected_kind: None,
                context: PluginHookContext::global(),
                payload: serde_json::json!({"tool": "generic"}),
                trace: TraceContext::new("trace-invoke"),
                metadata: Default::default(),
            },
            TraceContext::new("trace-invoke-envelope"),
        )
        .unwrap();
        let result = provider.call(invoke).await.unwrap();
        let hook_result: macaca_proto::PluginHookInvokeResult =
            serde_json::from_value(result.output).unwrap();
        assert_eq!(hook_result.outcomes.len(), 1);
    }
}
