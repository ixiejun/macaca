//! Runtime-host service provider for Plugin Control Plane v1.
//!
//! This adapter is the ServiceRuntime-facing boundary for plugin management.
//! It translates provider-neutral `ServiceCommand` values into typed
//! `PluginControlService` calls and serializes sanitized DTOs back into
//! `ServiceCallResult`.  The adapter does not implement repository policy or
//! plugin lifecycle semantics itself; those remain behind the injected control
//! service Facade.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    plugin_control_service_id, CleanupPolicy, ServiceCallResult, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceContext, TraceSchemaRef, PLUGIN_CONTROL_DIAGNOSTICS_COMMAND,
    PLUGIN_CONTROL_DISABLE_COMMAND, PLUGIN_CONTROL_ENABLE_COMMAND, PLUGIN_CONTROL_HEALTH_COMMAND,
    PLUGIN_CONTROL_INSPECT_COMMAND, PLUGIN_CONTROL_INSTALL_COMMAND, PLUGIN_CONTROL_LIST_COMMAND,
    PLUGIN_CONTROL_START_COMMAND, PLUGIN_CONTROL_STOP_COMMAND, PLUGIN_CONTROL_UNINSTALL_COMMAND,
};
use tracing::info;

use crate::plugin_control::PluginControlService;

/// ServiceRuntime adapter for the Plugin Control Service facade.
pub struct PluginControlSystemServiceProvider {
    descriptor: ServiceDescriptor,
    control: Arc<PluginControlService>,
}

impl PluginControlSystemServiceProvider {
    /// Create a provider over an injected control service.
    pub fn new(control: Arc<PluginControlService>) -> Self {
        Self {
            descriptor: plugin_control_service_descriptor(),
            control,
        }
    }

    /// Create a deterministic in-memory provider for local hosts and tests.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(PluginControlService::in_memory()))
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
impl SystemService for PluginControlSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin control service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "plugin control service command accepted"
        );
        match command.name.as_str() {
            PLUGIN_CONTROL_LIST_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.list(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_INSPECT_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.inspect(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_INSTALL_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.install(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_ENABLE_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.enable(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_DISABLE_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.disable(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_START_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.start(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_STOP_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.stop(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_UNINSTALL_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.uninstall(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_HEALTH_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self.control.health(typed).await.map_err(plugin_error)?;
                Self::result(output, trace)
            }
            PLUGIN_CONTROL_DIAGNOSTICS_COMMAND => {
                let typed = decode(command.payload)?;
                let output = self
                    .control
                    .diagnostics(typed)
                    .await
                    .map_err(plugin_error)?;
                Self::result(output, trace)
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin control service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            "plugin control service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Build the provider-neutral descriptor for Plugin Control Service.
pub fn plugin_control_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        plugin_control_service_id(),
        ServiceType::new("plugin_control"),
        TraceSchemaRef::new("trace.plugin.control.v1"),
    );
    descriptor.supported_scopes = vec![macaca_proto::ServiceScope::Global];
    descriptor
        .metadata
        .insert("phase".into(), "control_plane_v1".into());
    descriptor
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

/// Build a typed service command for Plugin Control tests and local adapters.
pub fn plugin_control_service_command<T: serde::Serialize>(
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

#[cfg(test)]
mod tests {
    use macaca_proto::{
        DeveloperId, PluginId, PluginInstallRequest, PluginInstallSourceKind,
        PluginPackageLocation, PluginRuntimeDeclaration, PluginRuntimeKind, PluginVersion,
        TraceContext, PLUGIN_CONTROL_INSTALL_COMMAND, PLUGIN_CONTROL_LIST_COMMAND,
    };

    use super::*;

    fn install_request() -> PluginInstallRequest {
        PluginInstallRequest {
            location: PluginPackageLocation::new(PluginInstallSourceKind::UserLocal, "/tmp/plugin"),
            manifest: Some(macaca_proto::PluginManifest::new(
                PluginId::new("plugin.provider.fixture"),
                PluginVersion::new("1.0.0"),
                DeveloperId::new("developer.fixture"),
                PluginRuntimeDeclaration::new(PluginRuntimeKind::DescriptorOnly, "1"),
            )),
            enable_after_install: false,
            trace: TraceContext::new("trace-provider-install"),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn provider_dispatches_install_and_list_commands() {
        let provider = PluginControlSystemServiceProvider::in_memory();
        provider.start().await.unwrap();
        let install = plugin_control_service_command(
            PLUGIN_CONTROL_INSTALL_COMMAND,
            install_request(),
            TraceContext::new("trace-install-envelope"),
        )
        .unwrap();
        provider.call(install).await.unwrap();

        let list = plugin_control_service_command(
            PLUGIN_CONTROL_LIST_COMMAND,
            macaca_proto::PluginListCommand {
                repository_id: None,
                include_disabled: true,
                trace: TraceContext::new("trace-list"),
            },
            TraceContext::new("trace-list-envelope"),
        )
        .unwrap();
        let result = provider.call(list).await.unwrap();
        let records: Vec<macaca_proto::PluginControlRecord> =
            serde_json::from_value(result.output).unwrap();
        assert_eq!(records.len(), 1);
    }
}
