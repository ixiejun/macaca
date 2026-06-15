//! Runtime-host Web3 optional service provider.
//!
//! This provider is the production service boundary for optional Web3
//! capability.  It uses the Facade pattern at the `SystemService` boundary,
//! Strategy for replaceable unavailable/mock/future adapters, Null Object for
//! absent-safe behavior, Command for typed dispatch, Specification for
//! admission checks, and Observer-style tracing logs for lifecycle nodes.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_proto::{
    ChainQueryRequest, ChainQueryResponse, CleanupPolicy, KernelServiceId,
    OptionalProviderDescriptor, OptionalServiceRedactionSpec, ServiceCallResult, ServiceCapability,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope,
    ServiceType, SigningProof, SigningRequest, TraceContext, TraceSchemaRef, TransactionReceipt,
    TransactionRequest, Web3Availability, Web3AvailabilityCommand, Web3ChainQueryCommand,
    Web3ServiceSnapshot, Web3SigningAdmission, Web3SigningRequestCommand, Web3SnapshotCommand,
    Web3TransactionPreparation, Web3TransactionPrepareCommand, Web3UnavailableReason,
    Web3WalletListCommand, Web3WalletView, WEB3_AVAILABILITY_COMMAND, WEB3_CHAIN_QUERY_COMMAND,
    WEB3_SERVICE_ID, WEB3_SIGNING_REQUEST_COMMAND, WEB3_SNAPSHOT_COMMAND,
    WEB3_TRANSACTION_PREPARE_COMMAND, WEB3_WALLET_LIST_COMMAND,
};
use tracing::{info, warn};

/// Strategy boundary implemented by unavailable, mock, and future providers.
#[async_trait]
pub trait Web3ProviderStrategy: Send + Sync {
    /// Return a sanitized provider descriptor safe for snapshots and logs.
    fn descriptor(&self) -> OptionalProviderDescriptor;
    /// Report provider-neutral Web3 availability.
    async fn availability(&self) -> Web3Availability;
    /// Return wallet descriptors without exposing private keys or credentials.
    async fn list_wallets(&self) -> ServiceResult<Vec<Web3WalletView>>;
    /// Admit or execute a bounded signing request.
    async fn request_signing(&self, request: SigningRequest)
        -> ServiceResult<Web3SigningAdmission>;
    /// Admit or simulate transaction preparation.
    async fn prepare_transaction(
        &self,
        request: TransactionRequest,
    ) -> ServiceResult<Web3TransactionPreparation>;
    /// Execute a bounded read-only chain query.
    async fn query_chain(&self, request: ChainQueryRequest) -> ServiceResult<ChainQueryResponse>;
}

/// Null Object provider used when Web3 is absent or disabled.
#[derive(Debug, Clone)]
pub struct UnavailableWeb3Provider {
    reason: String,
}

impl UnavailableWeb3Provider {
    /// Create an unavailable provider with a bounded diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableWeb3Provider {
    fn default() -> Self {
        Self::new("optional web3 service is unavailable")
    }
}

#[async_trait]
impl Web3ProviderStrategy for UnavailableWeb3Provider {
    fn descriptor(&self) -> OptionalProviderDescriptor {
        OptionalProviderDescriptor::unavailable(WEB3_SERVICE_ID, self.reason.clone())
    }

    async fn availability(&self) -> Web3Availability {
        Web3Availability::unavailable(Web3UnavailableReason::new(
            "unavailable",
            self.reason.clone(),
        ))
    }

    async fn list_wallets(&self) -> ServiceResult<Vec<Web3WalletView>> {
        Ok(Vec::new())
    }

    async fn request_signing(
        &self,
        _request: SigningRequest,
    ) -> ServiceResult<Web3SigningAdmission> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn prepare_transaction(
        &self,
        _request: TransactionRequest,
    ) -> ServiceResult<Web3TransactionPreparation> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn query_chain(&self, _request: ChainQueryRequest) -> ServiceResult<ChainQueryResponse> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }
}

/// Deterministic no-network provider used for tests and local development.
#[derive(Debug, Clone, Default)]
pub struct MockWeb3Provider;

#[async_trait]
impl Web3ProviderStrategy for MockWeb3Provider {
    fn descriptor(&self) -> OptionalProviderDescriptor {
        OptionalProviderDescriptor::mock(WEB3_SERVICE_ID)
    }

    async fn availability(&self) -> Web3Availability {
        let mut availability = Web3Availability::available(vec![
            macaca_proto::Web3CapabilityKind::wallet(),
            macaca_proto::Web3CapabilityKind::signing(),
            macaca_proto::Web3CapabilityKind::transaction(),
            macaca_proto::Web3CapabilityKind::chain_query(),
        ]);
        availability
            .metadata
            .insert("real_chain".into(), "false".into());
        availability
    }

    async fn list_wallets(&self) -> ServiceResult<Vec<Web3WalletView>> {
        Ok(vec![Web3WalletView {
            wallet_id: "mock.wallet".into(),
            label: Some("Mock development wallet".into()),
            capabilities: vec!["signing".into(), "transaction".into()],
            provider: self.descriptor(),
            metadata: BTreeMap::from([("real_chain".into(), "false".into())]),
        }])
    }

    async fn request_signing(
        &self,
        request: SigningRequest,
    ) -> ServiceResult<Web3SigningAdmission> {
        Ok(Web3SigningAdmission {
            proof: Some(SigningProof {
                request_id: request.request_id,
                proof: format!("mock-proof:{}", request.payload_digest),
                algorithm: "mock".into(),
                created_at: Utc::now(),
                metadata: BTreeMap::from([("real_chain".into(), "false".into())]),
            }),
            provider: self.descriptor(),
            status: "mock_signed".into(),
            reason: Some("mock provider does not sign real payloads".into()),
            admitted_at: Utc::now(),
            metadata: BTreeMap::from([("mock_only".into(), "true".into())]),
        })
    }

    async fn prepare_transaction(
        &self,
        request: TransactionRequest,
    ) -> ServiceResult<Web3TransactionPreparation> {
        Ok(Web3TransactionPreparation {
            receipt: Some(TransactionReceipt {
                receipt_id: macaca_proto::Web3ReceiptId::new(format!(
                    "mock.receipt.{}",
                    request.request_id
                )),
                request_id: request.request_id.clone(),
                transaction_id: macaca_proto::Web3TransactionId::new(format!(
                    "mock.tx.{}",
                    request.request_id
                )),
                chain_id: request.chain_id,
                status: "mock_prepared".into(),
                recorded_at: Utc::now(),
                metadata: BTreeMap::from([("real_chain".into(), "false".into())]),
            }),
            provider: self.descriptor(),
            status: "mock_prepared".into(),
            reason: Some("mock provider does not broadcast transactions".into()),
            prepared_at: Utc::now(),
            metadata: BTreeMap::from([("mock_only".into(), "true".into())]),
        })
    }

    async fn query_chain(&self, request: ChainQueryRequest) -> ServiceResult<ChainQueryResponse> {
        Ok(ChainQueryResponse {
            request_id: request.request_id,
            chain_id: request.chain_id,
            status: "mock_ok".into(),
            result_digest: Some(format!("mock-chain-query:{}", request.method)),
            responded_at: Utc::now(),
            metadata: BTreeMap::from([("real_chain".into(), "false".into())]),
        })
    }
}

/// Runtime-host Web3 service provider registered in ServiceRuntime.
pub struct Web3SystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Arc<dyn Web3ProviderStrategy>,
}

impl Web3SystemServiceProvider {
    /// Create a provider from an explicit strategy.
    pub fn new(provider: Arc<dyn Web3ProviderStrategy>) -> Self {
        Self {
            descriptor: web3_service_descriptor(),
            provider,
        }
    }

    /// Create the default absent-safe provider.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableWeb3Provider::default()))
    }

    /// Create the explicit mock/dev provider used by tests and local dev.
    pub fn mock() -> Self {
        Self::new(Arc::new(MockWeb3Provider))
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        if trace.trace_id.trim().is_empty() {
            return Err(ServiceError::AdapterFailure(
                "web3 trace context is required".into(),
            ));
        }
        Ok(trace)
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

    fn check_metadata(metadata: &BTreeMap<String, String>) -> ServiceResult<()> {
        OptionalServiceRedactionSpec::check_metadata(metadata).map_err(ServiceError::AdapterFailure)
    }
}

#[async_trait]
impl SystemService for Web3SystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %self.descriptor.id,
            provider_class = ?descriptor.provider_class,
            real_chain = descriptor.real_chain,
            "web3 optional service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "web3 optional service command accepted"
        );
        match command.name.as_str() {
            WEB3_AVAILABILITY_COMMAND => {
                let typed: Web3AvailabilityCommand = decode(command.payload)?;
                Self::result(self.provider.availability().await, typed.trace)
            }
            WEB3_WALLET_LIST_COMMAND => {
                let typed: Web3WalletListCommand = decode(command.payload)?;
                Self::result(self.provider.list_wallets().await?, typed.trace)
            }
            WEB3_SIGNING_REQUEST_COMMAND => {
                let typed: Web3SigningRequestCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                let result = self.provider.request_signing(typed.request).await?;
                info!(
                    trace_id = %typed.trace.trace_id,
                    status = %result.status,
                    "web3 signing request completed"
                );
                Self::result(result, typed.trace)
            }
            WEB3_TRANSACTION_PREPARE_COMMAND => {
                let typed: Web3TransactionPrepareCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                let result = self.provider.prepare_transaction(typed.request).await?;
                info!(
                    trace_id = %typed.trace.trace_id,
                    status = %result.status,
                    "web3 transaction preparation completed"
                );
                Self::result(result, typed.trace)
            }
            WEB3_CHAIN_QUERY_COMMAND => {
                let typed: Web3ChainQueryCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                Self::result(self.provider.query_chain(typed.request).await?, typed.trace)
            }
            WEB3_SNAPSHOT_COMMAND => {
                let typed: Web3SnapshotCommand = decode(command.payload)?;
                let availability = self.provider.availability().await;
                let provider = self.provider.descriptor();
                let wallets = self.provider.list_wallets().await.unwrap_or_default();
                let snapshot = Web3ServiceSnapshot {
                    service_id: WEB3_SERVICE_ID.into(),
                    health: if availability.is_available() {
                        "healthy".into()
                    } else {
                        "unavailable".into()
                    },
                    diagnostics: provider.diagnostics.clone(),
                    availability,
                    provider,
                    wallet_count: wallets.len(),
                    captured_at: Utc::now(),
                    metadata: BTreeMap::new(),
                };
                Self::result(snapshot, typed.trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Web3 service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "web3 optional service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "web3 optional service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let availability = self.provider.availability().await;
        if availability.is_available() {
            Ok(ServiceHealth::Healthy)
        } else {
            warn!(state = %availability.state, "web3 optional service health unavailable");
            Ok(ServiceHealth::Unavailable {
                reason: availability
                    .reason
                    .map(|reason| reason.message)
                    .unwrap_or_else(|| "web3 unavailable".into()),
            })
        }
    }
}

/// Build the provider-neutral descriptor for Web3 Service.
pub fn web3_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(WEB3_SERVICE_ID),
        ServiceType::new("web3"),
        TraceSchemaRef::new("trace.system_service.web3.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.web3.availability"),
            "Reports optional Web3 provider availability.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.web3.signing"),
            "Admits bounded Web3 signing requests.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.web3.transaction"),
            "Prepares provider-neutral Web3 transactions.",
        ),
    ];
    descriptor.health = ServiceHealth::Unknown;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec![
        "web3.availability".into(),
        "web3.signing".into(),
        "web3.transaction".into(),
        "web3.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}
