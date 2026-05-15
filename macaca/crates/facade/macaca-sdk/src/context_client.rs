//! SDK Context client facade for Route C S5.
//!
//! The Context client is intentionally a thin command facade.  Context engine
//! selection, active recall orchestration, and provider inventory are owned by
//! Context Service implementations, not by Web/CLI/framework presentation code.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    ContextActiveRecallCommand, ContextActiveRecallServiceResult, ContextAssembleCommand,
    ContextAssembleServiceResult, ContextEngineInventoryCommand, ContextProviderInventoryCommand,
    ContextServiceSnapshot, ContextServiceSnapshotCommand, CONTEXT_ACTIVE_RECALL_COMMAND,
    CONTEXT_ASSEMBLE_COMMAND, CONTEXT_ENGINE_INVENTORY_COMMAND, CONTEXT_PROVIDER_INVENTORY_COMMAND,
    CONTEXT_SERVICE_ID, CONTEXT_SNAPSHOT_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Context client consumed by serviceized callers.
#[async_trait]
pub trait SystemContextClient: Send + Sync {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult>;
    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult>;
    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value>;
    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value>;
    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot>;
}

/// Null-object Context client used before a runtime is wired.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemContextClient;

#[async_trait]
impl SystemContextClient for UnavailableSystemContextClient {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk context client unavailable for assemble");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk context client unavailable for active recall");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for provider inventory");
        Ok(serde_json::json!({"providers": [], "status": "unavailable"}))
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for engine inventory");
        Ok(serde_json::json!({"engines": [], "status": "unavailable"}))
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for snapshot");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }
}

/// Runtime-backed Context client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedContextClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedContextClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemContextClient for ServiceBackedContextClient {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult> {
        let trace = command.trace.clone();
        call(&*self.service, CONTEXT_ASSEMBLE_COMMAND, trace, command).await
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_ACTIVE_RECALL_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_PROVIDER_INVENTORY_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_ENGINE_INVENTORY_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        let trace = command.trace.clone();
        call(&*self.service, CONTEXT_SNAPSHOT_COMMAND, trace, command).await
    }
}

async fn call<T, R>(
    service: &dyn SystemServiceClient,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // Context commands already validate their own typed trace.  The service
    // runtime also requires the outer generic envelope to carry that trace so
    // trace-required middleware can admit the call without inspecting payloads.
    let result = service
        .call_service(
            &ServiceCallCommand::new(
                CONTEXT_SERVICE_ID,
                command_name,
                serde_json::to_value(command)?,
            )?
            .with_trace(trace),
        )
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use macaca_context::{ContextEngineSelection, ContextServiceScope};
    use macaca_proto::{ApplicationId, LlmMessage, LlmOptions, TraceContext};

    use crate::{ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult};

    use super::*;

    #[derive(Default)]
    struct CapturingServiceClient {
        captured: Mutex<Option<ServiceCallCommand>>,
    }

    #[async_trait]
    impl SystemServiceClient for CapturingServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: Vec::new(),
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            *self.captured.lock().expect("capture mutex poisoned") = Some(command.clone());
            Err(MacacaError::Config("stop after capture".into()))
        }
    }

    #[tokio::test]
    async fn service_backed_context_client_preserves_trace_on_outer_service_call() {
        let service = Arc::new(CapturingServiceClient::default());
        let client = ServiceBackedContextClient::new(service.clone());
        let trace = TraceContext::new("trace-context-envelope");
        let scope = ContextServiceScope::new(
            ApplicationId::new(),
            "session-context-trace",
            "agent-context-trace",
        )
        .expect("valid context service scope");
        let command = ContextAssembleCommand::new(
            scope,
            trace.clone(),
            "test-model",
            vec![LlmMessage::user("assemble context")],
            LlmOptions::default(),
            ContextEngineSelection::legacy(),
        )
        .expect("valid context assemble command");

        let result = client.assemble(command).await;

        assert!(result.is_err());
        let captured = service
            .captured
            .lock()
            .expect("capture mutex poisoned")
            .clone()
            .expect("service call should be captured");
        assert_eq!(captured.service_id, CONTEXT_SERVICE_ID);
        assert_eq!(captured.command_name, CONTEXT_ASSEMBLE_COMMAND);
        assert_eq!(
            captured.trace.as_ref().map(|trace| trace.trace_id.as_str()),
            Some("trace-context-envelope")
        );
    }
}
