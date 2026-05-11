//! SDK Plugin Capability Registry client.
//!
//! Shells and external SDK users use this Facade instead of depending on
//! runtime-host registry internals.  The client speaks generic Route C service
//! calls, which keeps the capability registry replaceable by local, remote, or
//! test implementations.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, PluginCapabilityCallCommand, PluginCapabilityCallResult,
    PluginCapabilityDeactivateCommand, PluginCapabilityDiscoverCommand, PluginCapabilityEvent,
    PluginCapabilityInspectCommand, PluginCapabilityOwnership, PluginCapabilityQueryCommand,
    PluginCapabilityRegisterCommand, PLUGIN_CAPABILITY_CALL_COMMAND,
    PLUGIN_CAPABILITY_DEACTIVATE_COMMAND, PLUGIN_CAPABILITY_DISCOVER_COMMAND,
    PLUGIN_CAPABILITY_INSPECT_COMMAND, PLUGIN_CAPABILITY_QUERY_COMMAND,
    PLUGIN_CAPABILITY_REGISTER_COMMAND, PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Plugin Capability client consumed by shells and SDK users.
#[async_trait]
pub trait SystemPluginCapabilityClient: Send + Sync {
    async fn discover(
        &self,
        command: PluginCapabilityDiscoverCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>>;
    async fn register(
        &self,
        command: PluginCapabilityRegisterCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>>;
    async fn deactivate(
        &self,
        command: PluginCapabilityDeactivateCommand,
    ) -> MacacaResult<Vec<PluginCapabilityEvent>>;
    async fn query(
        &self,
        command: PluginCapabilityQueryCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>>;
    async fn inspect(
        &self,
        command: PluginCapabilityInspectCommand,
    ) -> MacacaResult<Option<PluginCapabilityOwnership>>;
    async fn call(
        &self,
        command: PluginCapabilityCallCommand,
    ) -> MacacaResult<PluginCapabilityCallResult>;
}

/// Null-object client used when the registry service is not configured.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemPluginCapabilityClient;

#[async_trait]
impl SystemPluginCapabilityClient for UnavailableSystemPluginCapabilityClient {
    async fn discover(
        &self,
        command: PluginCapabilityDiscoverCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk plugin capability client returning empty unavailable discovery"
        );
        Ok(Vec::new())
    }

    async fn register(
        &self,
        command: PluginCapabilityRegisterCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin capability client unavailable for register"
        );
        Err(unavailable())
    }

    async fn deactivate(
        &self,
        command: PluginCapabilityDeactivateCommand,
    ) -> MacacaResult<Vec<PluginCapabilityEvent>> {
        warn!(
            trace_id = %command.trace.trace_id,
            plugin_id = %command.plugin_id,
            "sdk plugin capability client unavailable for deactivate"
        );
        Err(unavailable())
    }

    async fn query(
        &self,
        command: PluginCapabilityQueryCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk plugin capability client returning empty unavailable query"
        );
        Ok(Vec::new())
    }

    async fn inspect(
        &self,
        command: PluginCapabilityInspectCommand,
    ) -> MacacaResult<Option<PluginCapabilityOwnership>> {
        info!(
            trace_id = %command.trace.trace_id,
            capability_id = %command.capability_id,
            "sdk plugin capability client unavailable for inspect"
        );
        Ok(None)
    }

    async fn call(
        &self,
        command: PluginCapabilityCallCommand,
    ) -> MacacaResult<PluginCapabilityCallResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            capability_id = %command.capability_id,
            "sdk plugin capability client unavailable for call"
        );
        Err(unavailable())
    }
}

/// Runtime-backed client implemented over generic service dispatch.
#[derive(Clone)]
pub struct ServiceBackedPluginCapabilityClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedPluginCapabilityClient {
    /// Create a service-backed Plugin Capability client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemPluginCapabilityClient for ServiceBackedPluginCapabilityClient {
    async fn discover(
        &self,
        command: PluginCapabilityDiscoverCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        call(&*self.service, PLUGIN_CAPABILITY_DISCOVER_COMMAND, command).await
    }

    async fn register(
        &self,
        command: PluginCapabilityRegisterCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        call(&*self.service, PLUGIN_CAPABILITY_REGISTER_COMMAND, command).await
    }

    async fn deactivate(
        &self,
        command: PluginCapabilityDeactivateCommand,
    ) -> MacacaResult<Vec<PluginCapabilityEvent>> {
        call(
            &*self.service,
            PLUGIN_CAPABILITY_DEACTIVATE_COMMAND,
            command,
        )
        .await
    }

    async fn query(
        &self,
        command: PluginCapabilityQueryCommand,
    ) -> MacacaResult<Vec<PluginCapabilityOwnership>> {
        call(&*self.service, PLUGIN_CAPABILITY_QUERY_COMMAND, command).await
    }

    async fn inspect(
        &self,
        command: PluginCapabilityInspectCommand,
    ) -> MacacaResult<Option<PluginCapabilityOwnership>> {
        call(&*self.service, PLUGIN_CAPABILITY_INSPECT_COMMAND, command).await
    }

    async fn call(
        &self,
        command: PluginCapabilityCallCommand,
    ) -> MacacaResult<PluginCapabilityCallResult> {
        call(&*self.service, PLUGIN_CAPABILITY_CALL_COMMAND, command).await
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
            PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID,
            command_name,
            serde_json::to_value(command)?,
        )?)
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

fn unavailable() -> MacacaError {
    MacacaError::Config("Plugin Capability Registry service is unavailable".into())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use macaca_proto::{
        CapabilityId, MacacaResult, PluginCapabilityKind, PluginCapabilityQueryCommand, PluginId,
        TraceContext,
    };

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoCapabilityServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoCapabilityServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, PLUGIN_CAPABILITY_REGISTRY_SERVICE_ID);
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::json!([]),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_client_queries_capabilities() {
        let client =
            ServiceBackedPluginCapabilityClient::new(Arc::new(EchoCapabilityServiceClient));
        let output = client
            .query(PluginCapabilityQueryCommand {
                kind: Some(PluginCapabilityKind::Tool),
                plugin_id: Some(PluginId::new("plugin.fixture")),
                slot_namespace: None,
                slot_key: None,
                include_inactive: false,
                trace: TraceContext::new("trace-sdk-query"),
            })
            .await
            .unwrap();
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn unavailable_client_returns_none_for_inspect() {
        let client = UnavailableSystemPluginCapabilityClient;
        let output = client
            .inspect(PluginCapabilityInspectCommand {
                capability_id: CapabilityId::new("capability.none"),
                trace: TraceContext::new("trace-sdk-inspect"),
            })
            .await
            .unwrap();
        assert!(output.is_none());
    }
}
