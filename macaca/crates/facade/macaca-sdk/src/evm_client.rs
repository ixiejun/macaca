//! SDK EVM Service client for Route C S11.
//!
//! This focused client keeps EVM optional module access behind SDK and
//! ServiceRuntime contracts.  Upper consumers never construct concrete EVM
//! providers, RPC clients, wallet clients, or kernel compatibility facades.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    EvmAvailability, EvmAvailabilityCommand, EvmCallAdmission, EvmContractCallCommand,
    EvmContractDeployCommand, EvmContractReadCommand, EvmDeployAdmission, EvmEventSubscribeCommand,
    EvmEventSubscriptionAdmission, EvmGasEstimateCommand, EvmReadAdmission, EvmReceiptGetCommand,
    EvmServiceSnapshot, EvmSnapshotCommand, EvmTransactionReceipt, GasEstimate, MacacaError,
    MacacaResult, TraceContext, EVM_AVAILABILITY_COMMAND, EVM_CONTRACT_CALL_COMMAND,
    EVM_CONTRACT_DEPLOY_COMMAND, EVM_CONTRACT_READ_COMMAND, EVM_EVENT_SUBSCRIBE_COMMAND,
    EVM_GAS_ESTIMATE_COMMAND, EVM_RECEIPT_GET_COMMAND, EVM_SERVICE_ID, EVM_SNAPSHOT_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused EVM client consumed by upper shells and SDK users.
#[async_trait]
pub trait SystemEvmClient: Send + Sync {
    async fn availability(&self, command: EvmAvailabilityCommand) -> MacacaResult<EvmAvailability>;
    async fn deploy(&self, command: EvmContractDeployCommand) -> MacacaResult<EvmDeployAdmission>;
    async fn call(&self, command: EvmContractCallCommand) -> MacacaResult<EvmCallAdmission>;
    async fn read(&self, command: EvmContractReadCommand) -> MacacaResult<EvmReadAdmission>;
    async fn estimate_gas(&self, command: EvmGasEstimateCommand) -> MacacaResult<GasEstimate>;
    async fn receipt(&self, command: EvmReceiptGetCommand) -> MacacaResult<EvmTransactionReceipt>;
    async fn subscribe(
        &self,
        command: EvmEventSubscribeCommand,
    ) -> MacacaResult<EvmEventSubscriptionAdmission>;
    async fn snapshot(&self, command: EvmSnapshotCommand) -> MacacaResult<EvmServiceSnapshot>;
}

/// Null-object EVM client used when EVM Service is not installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemEvmClient;

#[async_trait]
impl SystemEvmClient for UnavailableSystemEvmClient {
    async fn availability(&self, command: EvmAvailabilityCommand) -> MacacaResult<EvmAvailability> {
        info!(trace_id = %command.trace.trace_id, "sdk evm client returning unavailable availability");
        Ok(EvmServiceSnapshot::unavailable("EVM service is unavailable").availability)
    }

    async fn deploy(&self, command: EvmContractDeployCommand) -> MacacaResult<EvmDeployAdmission> {
        warn!(trace_id = %command.trace.trace_id, "sdk evm client unavailable for deploy");
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn call(&self, command: EvmContractCallCommand) -> MacacaResult<EvmCallAdmission> {
        warn!(trace_id = %command.trace.trace_id, "sdk evm client unavailable for call");
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn read(&self, _command: EvmContractReadCommand) -> MacacaResult<EvmReadAdmission> {
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn estimate_gas(&self, _command: EvmGasEstimateCommand) -> MacacaResult<GasEstimate> {
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn receipt(&self, _command: EvmReceiptGetCommand) -> MacacaResult<EvmTransactionReceipt> {
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn subscribe(
        &self,
        _command: EvmEventSubscribeCommand,
    ) -> MacacaResult<EvmEventSubscriptionAdmission> {
        Err(MacacaError::Config("EVM service is unavailable".into()))
    }

    async fn snapshot(&self, command: EvmSnapshotCommand) -> MacacaResult<EvmServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk evm client returning unavailable snapshot");
        Ok(EvmServiceSnapshot::unavailable(
            "EVM service is unavailable",
        ))
    }
}

/// Runtime-backed EVM client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedEvmClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedEvmClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemEvmClient for ServiceBackedEvmClient {
    async fn availability(&self, command: EvmAvailabilityCommand) -> MacacaResult<EvmAvailability> {
        call(
            &self.service,
            EVM_AVAILABILITY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn deploy(&self, command: EvmContractDeployCommand) -> MacacaResult<EvmDeployAdmission> {
        call(
            &self.service,
            EVM_CONTRACT_DEPLOY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn call(&self, command: EvmContractCallCommand) -> MacacaResult<EvmCallAdmission> {
        call(
            &self.service,
            EVM_CONTRACT_CALL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn read(&self, command: EvmContractReadCommand) -> MacacaResult<EvmReadAdmission> {
        call(
            &self.service,
            EVM_CONTRACT_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn estimate_gas(&self, command: EvmGasEstimateCommand) -> MacacaResult<GasEstimate> {
        call(
            &self.service,
            EVM_GAS_ESTIMATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn receipt(&self, command: EvmReceiptGetCommand) -> MacacaResult<EvmTransactionReceipt> {
        call(
            &self.service,
            EVM_RECEIPT_GET_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn subscribe(
        &self,
        command: EvmEventSubscribeCommand,
    ) -> MacacaResult<EvmEventSubscriptionAdmission> {
        call(
            &self.service,
            EVM_EVENT_SUBSCRIBE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(&self, command: EvmSnapshotCommand) -> MacacaResult<EvmServiceSnapshot> {
        call(
            &self.service,
            EVM_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        EVM_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoEvmServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoEvmServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![EVM_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, EVM_SERVICE_ID);
            assert_eq!(command.command_name, EVM_SNAPSHOT_COMMAND);
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(EvmServiceSnapshot::unavailable("test")).unwrap(),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_evm_client_dispatches_snapshot_command() {
        let client = ServiceBackedEvmClient::new(Arc::new(EchoEvmServiceClient));
        let snapshot = client
            .snapshot(EvmSnapshotCommand {
                trace: TraceContext::new("trace-sdk-evm-snapshot"),
            })
            .await
            .unwrap();
        assert_eq!(snapshot.service_id, EVM_SERVICE_ID);
    }
}
