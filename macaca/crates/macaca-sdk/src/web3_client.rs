//! SDK Web3 Service client for Route C S11.
//!
//! This focused client is the upper-consumer Facade for optional Web3
//! capability.  Web, CLI, Gateway, Application Framework, and agent-facing code
//! call this trait instead of constructing kernel facades, runtime-host
//! providers, wallet adapters, chain clients, or RPC clients.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ChainQueryResponse, MacacaError, MacacaResult, TraceContext, Web3Availability,
    Web3AvailabilityCommand, Web3ChainQueryCommand, Web3ServiceSnapshot, Web3SigningAdmission,
    Web3SigningRequestCommand, Web3SnapshotCommand, Web3TransactionPreparation,
    Web3TransactionPrepareCommand, Web3WalletListCommand, Web3WalletView,
    WEB3_AVAILABILITY_COMMAND, WEB3_CHAIN_QUERY_COMMAND, WEB3_SERVICE_ID,
    WEB3_SIGNING_REQUEST_COMMAND, WEB3_SNAPSHOT_COMMAND, WEB3_TRANSACTION_PREPARE_COMMAND,
    WEB3_WALLET_LIST_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Web3 client consumed by upper shells and SDK users.
#[async_trait]
pub trait SystemWeb3Client: Send + Sync {
    async fn availability(
        &self,
        command: Web3AvailabilityCommand,
    ) -> MacacaResult<Web3Availability>;
    async fn wallets(&self, command: Web3WalletListCommand) -> MacacaResult<Vec<Web3WalletView>>;
    async fn request_signing(
        &self,
        command: Web3SigningRequestCommand,
    ) -> MacacaResult<Web3SigningAdmission>;
    async fn prepare_transaction(
        &self,
        command: Web3TransactionPrepareCommand,
    ) -> MacacaResult<Web3TransactionPreparation>;
    async fn query_chain(&self, command: Web3ChainQueryCommand)
        -> MacacaResult<ChainQueryResponse>;
    async fn snapshot(&self, command: Web3SnapshotCommand) -> MacacaResult<Web3ServiceSnapshot>;
}

/// Null-object Web3 client used when Web3 Service is not installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemWeb3Client;

#[async_trait]
impl SystemWeb3Client for UnavailableSystemWeb3Client {
    async fn availability(
        &self,
        command: Web3AvailabilityCommand,
    ) -> MacacaResult<Web3Availability> {
        info!(trace_id = %command.trace.trace_id, "sdk web3 client returning unavailable availability");
        Ok(Web3ServiceSnapshot::unavailable("Web3 service is unavailable").availability)
    }

    async fn wallets(&self, _command: Web3WalletListCommand) -> MacacaResult<Vec<Web3WalletView>> {
        Ok(Vec::new())
    }

    async fn request_signing(
        &self,
        command: Web3SigningRequestCommand,
    ) -> MacacaResult<Web3SigningAdmission> {
        warn!(trace_id = %command.trace.trace_id, "sdk web3 client unavailable for signing");
        Err(MacacaError::Config("Web3 service is unavailable".into()))
    }

    async fn prepare_transaction(
        &self,
        command: Web3TransactionPrepareCommand,
    ) -> MacacaResult<Web3TransactionPreparation> {
        warn!(trace_id = %command.trace.trace_id, "sdk web3 client unavailable for transaction preparation");
        Err(MacacaError::Config("Web3 service is unavailable".into()))
    }

    async fn query_chain(
        &self,
        _command: Web3ChainQueryCommand,
    ) -> MacacaResult<ChainQueryResponse> {
        Err(MacacaError::Config("Web3 service is unavailable".into()))
    }

    async fn snapshot(&self, command: Web3SnapshotCommand) -> MacacaResult<Web3ServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk web3 client returning unavailable snapshot");
        Ok(Web3ServiceSnapshot::unavailable(
            "Web3 service is unavailable",
        ))
    }
}

/// Runtime-backed Web3 client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedWeb3Client {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedWeb3Client {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemWeb3Client for ServiceBackedWeb3Client {
    async fn availability(
        &self,
        command: Web3AvailabilityCommand,
    ) -> MacacaResult<Web3Availability> {
        call(
            &self.service,
            WEB3_AVAILABILITY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn wallets(&self, command: Web3WalletListCommand) -> MacacaResult<Vec<Web3WalletView>> {
        call(
            &self.service,
            WEB3_WALLET_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn request_signing(
        &self,
        command: Web3SigningRequestCommand,
    ) -> MacacaResult<Web3SigningAdmission> {
        call(
            &self.service,
            WEB3_SIGNING_REQUEST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn prepare_transaction(
        &self,
        command: Web3TransactionPrepareCommand,
    ) -> MacacaResult<Web3TransactionPreparation> {
        call(
            &self.service,
            WEB3_TRANSACTION_PREPARE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn query_chain(
        &self,
        command: Web3ChainQueryCommand,
    ) -> MacacaResult<ChainQueryResponse> {
        call(
            &self.service,
            WEB3_CHAIN_QUERY_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(&self, command: Web3SnapshotCommand) -> MacacaResult<Web3ServiceSnapshot> {
        call(
            &self.service,
            WEB3_SNAPSHOT_COMMAND,
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
        WEB3_SERVICE_ID,
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

    struct EchoWeb3ServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoWeb3ServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![WEB3_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, WEB3_SERVICE_ID);
            assert_eq!(command.command_name, WEB3_SNAPSHOT_COMMAND);
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(Web3ServiceSnapshot::unavailable("test")).unwrap(),
            })
        }
    }

    #[tokio::test]
    async fn service_backed_web3_client_dispatches_snapshot_command() {
        let client = ServiceBackedWeb3Client::new(Arc::new(EchoWeb3ServiceClient));
        let snapshot = client
            .snapshot(Web3SnapshotCommand {
                trace: TraceContext::new("trace-sdk-web3-snapshot"),
            })
            .await
            .unwrap();
        assert_eq!(snapshot.service_id, WEB3_SERVICE_ID);
    }

    #[tokio::test]
    async fn unavailable_web3_client_fails_closed_for_signing() {
        let client = UnavailableSystemWeb3Client;
        let request = macaca_proto::SigningRequest {
            request_id: macaca_proto::Web3RequestId::new("signing.request"),
            wallet_id: macaca_proto::WalletId::new("wallet"),
            chain_id: macaca_proto::ChainId::new("chain"),
            address: macaca_proto::Web3Address::new("address"),
            operation: "sign".into(),
            payload_digest: "digest".into(),
            session_id: None,
            task_id: None,
            requested_at: chrono::Utc::now(),
            metadata: Default::default(),
        };
        let err = client
            .request_signing(Web3SigningRequestCommand {
                trace: TraceContext::new("trace-sdk-web3-sign"),
                request,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Web3 service is unavailable"));
    }
}
