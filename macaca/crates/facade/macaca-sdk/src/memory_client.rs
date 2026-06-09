//! SDK Memory client facade for Route C S5.
//!
//! The client keeps upper layers scoped and service-oriented.  It forwards
//! typed memory commands through the generic service client instead of exposing
//! vector stores, memory runtimes, or backend factories to Web/CLI/framework.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    MemoryDeleteRequest, MemoryFacade, MemoryForgetCommand, MemoryGetCommand, MemoryGetRequest,
    MemoryGetResult, MemoryPrefetchCommand, MemoryPrefetchRequest, MemoryRecallCommand,
    MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult, MemorySearchRequest,
    MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryStatusReport,
    MemoryWriteRequest, MEMORY_FORGET_COMMAND, MEMORY_GET_COMMAND, MEMORY_PREFETCH_COMMAND,
    MEMORY_RECALL_COMMAND, MEMORY_REMEMBER_COMMAND, MEMORY_SERVICE_ID, MEMORY_SNAPSHOT_COMMAND,
    MEMORY_STATUS_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Memory client consumed by serviceized callers.
#[async_trait]
pub trait SystemMemoryClient: Send + Sync {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult>;
    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult>;
    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult>;
    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult>;
    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()>;
    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport>;
    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot>;
}

/// Null-object Memory client used before a runtime is wired.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemMemoryClient;

#[async_trait]
impl SystemMemoryClient for UnavailableSystemMemoryClient {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for remember");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for recall");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for prefetch");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for get");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for forget");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        info!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for status");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for snapshot");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }
}

/// In-process Memory client that forwards typed SDK commands to a [`MemoryFacade`].
///
/// Bootstrap uses this adapter before the memory service is registered on the bus, and
/// context recall wiring can share the same facade handle without re-implementing command
/// translation inside each shell. Production request paths should prefer
/// [`ServiceBackedMemoryClient`] once the memory service is online.
#[derive(Clone)]
pub struct DirectFacadeMemoryClient {
    facade: Arc<dyn MemoryFacade>,
    runtime_label: String,
}

impl DirectFacadeMemoryClient {
    /// Wrap a memory facade with provider-neutral SDK command translation.
    pub fn new(facade: Arc<dyn MemoryFacade>) -> Self {
        info!(
            service_id = "memory",
            command = "direct_facade_client_construct",
            "Direct facade memory client wired for in-process bootstrap"
        );
        Self {
            facade,
            runtime_label: "direct-facade-memory-client".into(),
        }
    }
}

#[async_trait]
impl SystemMemoryClient for DirectFacadeMemoryClient {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        let id = self
            .facade
            .remember(
                MemoryWriteRequest::new(command.scope, command.content)
                    .layer(command.layer)
                    .metadata(command.metadata),
            )
            .await?;
        info!(
            service_id = "memory",
            command = MEMORY_REMEMBER_COMMAND,
            trace_id = %command.trace.trace_id,
            memory_id = ?id,
            "Direct facade memory client remember completed"
        );
        Ok(MemoryRememberResult {
            id,
            stored_at: chrono::Utc::now(),
        })
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        let entries = self
            .facade
            .search(MemorySearchRequest::new(
                command.scope,
                command.query,
                command.limit,
            ))
            .await?;
        info!(
            service_id = "memory",
            command = MEMORY_RECALL_COMMAND,
            trace_id = %command.trace.trace_id,
            count = entries.len(),
            "Direct facade memory client recall completed"
        );
        Ok(MemoryRecallResult::new(entries))
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        let entries = self
            .facade
            .prefetch(MemoryPrefetchRequest {
                scope: command.scope,
                query: command.query,
                limit: command.limit,
            })
            .await?;
        info!(
            service_id = "memory",
            command = MEMORY_PREFETCH_COMMAND,
            trace_id = %command.trace.trace_id,
            count = entries.len(),
            "Direct facade memory client prefetch completed"
        );
        Ok(MemoryRecallResult::new(entries))
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        let entry = self
            .facade
            .get(MemoryGetRequest {
                scope: command.scope,
                id: command.id,
            })
            .await?;
        Ok(MemoryGetResult::new(entry))
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        self.facade
            .delete(MemoryDeleteRequest {
                scope: command.scope,
                id: command.id,
            })
            .await
    }

    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        let _ = command.trace.trace_id.as_str();
        Ok(self.facade.status())
    }

    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        let status = self.facade.status();
        info!(
            service_id = "memory",
            command = MEMORY_SNAPSHOT_COMMAND,
            trace_id = %command.trace.trace_id,
            runtime_label = %self.runtime_label,
            "Direct facade memory client snapshot emitted"
        );
        Ok(MemoryServiceSnapshot::new(
            self.runtime_label.clone(),
            true,
            status.capabilities,
            None,
        ))
    }
}

/// Runtime-backed Memory client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedMemoryClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedMemoryClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemMemoryClient for ServiceBackedMemoryClient {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_REMEMBER_COMMAND, trace, command).await
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_RECALL_COMMAND, trace, command).await
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_PREFETCH_COMMAND, trace, command).await
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_GET_COMMAND, trace, command).await
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        let trace = command.trace.clone();
        let _: serde_json::Value =
            call(&*self.service, MEMORY_FORGET_COMMAND, trace, command).await?;
        Ok(())
    }

    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_STATUS_COMMAND, trace, command).await
    }

    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        let trace = command.trace.clone();
        call(&*self.service, MEMORY_SNAPSHOT_COMMAND, trace, command).await
    }
}

async fn call<T, R>(
    service: &dyn SystemServiceClient,
    command_name: &str,
    trace: TraceContext,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // Memory commands are trace-required service commands.  The command payload
    // keeps trace for provider/audit use, while the outer SDK service envelope
    // must also carry the same trace so ServiceRuntime can admit and audit the
    // call before dispatching to `service.memory`.
    let result = service
        .call_service(
            &ServiceCallCommand::new(
                MEMORY_SERVICE_ID,
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
    use super::*;

    use std::sync::Mutex;

    use chrono::Utc;
    use macaca_memory::MemoryScope;
    use macaca_proto::{ApplicationId, MemoryId};

    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct CapturingServiceClient {
        last_call: Mutex<Option<ServiceCallCommand>>,
        output: serde_json::Value,
    }

    #[async_trait]
    impl SystemServiceClient for CapturingServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![MEMORY_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            *self.last_call.lock().expect("capture mutex poisoned") = Some(command.clone());
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: self.output.clone(),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_memory_client_forwards_trace_to_service_envelope() {
        let expected_trace = TraceContext::new("memory-sdk-trace-forwarding-test");
        let scope = MemoryScope::session_shared(ApplicationId::new(), "session-a");
        let command =
            MemoryRememberCommand::new(scope, expected_trace.clone(), "sanitized derived memory")
                .expect("valid memory command");
        let service = Arc::new(CapturingServiceClient {
            last_call: Mutex::new(None),
            output: serde_json::to_value(MemoryRememberResult {
                id: MemoryId::new(),
                stored_at: Utc::now(),
            })
            .expect("serializable memory result"),
        });
        let client = ServiceBackedMemoryClient::new(service.clone());

        client
            .remember(command)
            .await
            .expect("service-backed memory remember should succeed");

        let captured = service
            .last_call
            .lock()
            .expect("capture mutex poisoned")
            .clone()
            .expect("service call should be captured");
        assert_eq!(captured.service_id, MEMORY_SERVICE_ID);
        assert_eq!(captured.command_name, MEMORY_REMEMBER_COMMAND);
        assert_eq!(
            captured.trace.map(|trace| trace.trace_id),
            Some(expected_trace.trace_id)
        );
    }
}
