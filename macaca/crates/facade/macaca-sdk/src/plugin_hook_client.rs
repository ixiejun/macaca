//! SDK Plugin Hook Bus client.
//!
//! Shells and future plugin SDK helpers use this Facade instead of importing
//! runtime-host internals.  The client only speaks generic Route C service
//! calls, keeping Hook Bus implementations replaceable by local, remote, or
//! test service runtimes.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, PluginHookDeactivateCommand, PluginHookEvent,
    PluginHookInvokeCommand, PluginHookInvokeResult, PluginHookQueryCommand,
    PluginHookRegisterCommand, PluginHookRegistration, PluginHookRegistrySnapshot,
    PluginHookSnapshotCommand, PLUGIN_HOOK_BUS_SERVICE_ID, PLUGIN_HOOK_DEACTIVATE_COMMAND,
    PLUGIN_HOOK_INVOKE_COMMAND, PLUGIN_HOOK_QUERY_COMMAND, PLUGIN_HOOK_REGISTER_COMMAND,
    PLUGIN_HOOK_SNAPSHOT_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Hook Bus client consumed by shells and SDK users.
#[async_trait]
pub trait SystemPluginHookClient: Send + Sync {
    async fn register(
        &self,
        command: PluginHookRegisterCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>>;
    async fn deactivate(
        &self,
        command: PluginHookDeactivateCommand,
    ) -> MacacaResult<Vec<PluginHookEvent>>;
    async fn query(
        &self,
        command: PluginHookQueryCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>>;
    async fn invoke(
        &self,
        command: PluginHookInvokeCommand,
    ) -> MacacaResult<PluginHookInvokeResult>;
    async fn snapshot(
        &self,
        command: PluginHookSnapshotCommand,
    ) -> MacacaResult<PluginHookRegistrySnapshot>;
}

/// Null-object client used when Hook Bus is not configured.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemPluginHookClient;

#[async_trait]
impl SystemPluginHookClient for UnavailableSystemPluginHookClient {
    async fn register(
        &self,
        command: PluginHookRegisterCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin hook client unavailable for register"
        );
        Err(unavailable())
    }

    async fn deactivate(
        &self,
        command: PluginHookDeactivateCommand,
    ) -> MacacaResult<Vec<PluginHookEvent>> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin hook client unavailable for deactivate"
        );
        Err(unavailable())
    }

    async fn query(
        &self,
        command: PluginHookQueryCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk plugin hook client returning empty unavailable query"
        );
        Ok(Vec::new())
    }

    async fn invoke(
        &self,
        command: PluginHookInvokeCommand,
    ) -> MacacaResult<PluginHookInvokeResult> {
        info!(
            trace_id = %command.trace.trace_id,
            hook_name = %command.hook_name,
            "sdk plugin hook client returning no-op unavailable invocation"
        );
        Ok(PluginHookInvokeResult {
            hook_name: command.hook_name,
            decision: macaca_proto::PluginHookDecision::Noop,
            outcomes: Vec::new(),
            trace: command.trace,
            metadata: Default::default(),
        })
    }

    async fn snapshot(
        &self,
        command: PluginHookSnapshotCommand,
    ) -> MacacaResult<PluginHookRegistrySnapshot> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk plugin hook client returning empty unavailable snapshot"
        );
        Ok(PluginHookRegistrySnapshot {
            generated_at: chrono::Utc::now(),
            hooks: Vec::new(),
        })
    }
}

/// Runtime-backed Hook Bus client implemented over generic service dispatch.
#[derive(Clone)]
pub struct ServiceBackedPluginHookClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedPluginHookClient {
    /// Create a service-backed Hook Bus client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemPluginHookClient for ServiceBackedPluginHookClient {
    async fn register(
        &self,
        command: PluginHookRegisterCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>> {
        call(&*self.service, PLUGIN_HOOK_REGISTER_COMMAND, command).await
    }

    async fn deactivate(
        &self,
        command: PluginHookDeactivateCommand,
    ) -> MacacaResult<Vec<PluginHookEvent>> {
        call(&*self.service, PLUGIN_HOOK_DEACTIVATE_COMMAND, command).await
    }

    async fn query(
        &self,
        command: PluginHookQueryCommand,
    ) -> MacacaResult<Vec<PluginHookRegistration>> {
        call(&*self.service, PLUGIN_HOOK_QUERY_COMMAND, command).await
    }

    async fn invoke(
        &self,
        command: PluginHookInvokeCommand,
    ) -> MacacaResult<PluginHookInvokeResult> {
        call(&*self.service, PLUGIN_HOOK_INVOKE_COMMAND, command).await
    }

    async fn snapshot(
        &self,
        command: PluginHookSnapshotCommand,
    ) -> MacacaResult<PluginHookRegistrySnapshot> {
        call(&*self.service, PLUGIN_HOOK_SNAPSHOT_COMMAND, command).await
    }
}

async fn call<T, R>(
    service: &dyn SystemServiceClient,
    command_name: &str,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let result = service
        .call_service(&ServiceCallCommand::new(
            PLUGIN_HOOK_BUS_SERVICE_ID,
            command_name,
            serde_json::to_value(command)?,
        )?)
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

fn unavailable() -> MacacaError {
    MacacaError::Config("Plugin Hook Bus service is unavailable".into())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use macaca_proto::{MacacaResult, PluginHookName, PluginHookQueryCommand, TraceContext};

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoHookServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoHookServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![PLUGIN_HOOK_BUS_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, PLUGIN_HOOK_BUS_SERVICE_ID);
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::json!([]),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_client_queries_hooks() {
        let client = ServiceBackedPluginHookClient::new(Arc::new(EchoHookServiceClient));
        let output = client
            .query(PluginHookQueryCommand {
                hook_name: Some(PluginHookName::BeforeToolCall),
                plugin_id: None,
                include_inactive: false,
                trace: TraceContext::new("trace-sdk-hook-query"),
            })
            .await
            .unwrap();
        assert!(output.is_empty());
    }
}
