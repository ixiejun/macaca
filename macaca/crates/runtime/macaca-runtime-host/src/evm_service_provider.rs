//! Runtime-host EVM optional service provider.
//!
//! The provider exposes EVM as a ServiceRuntime-owned optional service.  It
//! intentionally implements only unavailable and mock/dev strategies here; real nodes, RPC providers,
//! wallets, and EVM execution engines remain future adapters.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, ContractCallRequest, ContractCallResult, ContractDeployRequest,
    ContractDeployResult, ContractEvent, ContractEventSubscription, ContractReadRequest,
    ContractReadResult, EvmAvailability, EvmAvailabilityCommand, EvmCallAdmission,
    EvmContractCallCommand, EvmContractDeployCommand, EvmContractReadCommand, EvmDeployAdmission,
    EvmEventSubscribeCommand, EvmEventSubscriptionAdmission, EvmGasEstimateCommand,
    EvmReadAdmission, EvmReceiptGetCommand, EvmServiceSnapshot, EvmTransactionReceipt, GasEstimate,
    KernelServiceId, OptionalProviderDescriptor, OptionalServiceRedactionSpec, ServiceCallResult,
    ServiceCapability, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
    TransactionReceiptQuery, EVM_AVAILABILITY_COMMAND, EVM_CONTRACT_CALL_COMMAND,
    EVM_CONTRACT_DEPLOY_COMMAND, EVM_CONTRACT_READ_COMMAND, EVM_EVENT_SUBSCRIBE_COMMAND,
    EVM_GAS_ESTIMATE_COMMAND, EVM_RECEIPT_GET_COMMAND, EVM_SERVICE_ID, EVM_SNAPSHOT_COMMAND,
};
use tracing::{info, warn};

/// Strategy boundary implemented by unavailable, mock, and future providers.
#[async_trait]
pub trait EvmProviderStrategy: Send + Sync {
    fn descriptor(&self) -> OptionalProviderDescriptor;
    async fn availability(&self) -> EvmAvailability;
    async fn deploy_contract(
        &self,
        request: ContractDeployRequest,
    ) -> ServiceResult<EvmDeployAdmission>;
    async fn call_contract(&self, request: ContractCallRequest) -> ServiceResult<EvmCallAdmission>;
    async fn read_contract(&self, request: ContractReadRequest) -> ServiceResult<EvmReadAdmission>;
    async fn estimate_gas(
        &self,
        request_id: macaca_proto::EvmRequestId,
    ) -> ServiceResult<GasEstimate>;
    async fn get_receipt(
        &self,
        request: TransactionReceiptQuery,
    ) -> ServiceResult<EvmTransactionReceipt>;
    async fn subscribe_events(
        &self,
        request: ContractEventSubscription,
    ) -> ServiceResult<EvmEventSubscriptionAdmission>;
}

/// Null Object provider used when EVM is absent or disabled.
#[derive(Debug, Clone)]
pub struct UnavailableEvmProvider {
    reason: String,
}

impl UnavailableEvmProvider {
    /// Create an unavailable provider with a bounded diagnostic reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Default for UnavailableEvmProvider {
    fn default() -> Self {
        Self::new("optional evm service is unavailable")
    }
}

#[async_trait]
impl EvmProviderStrategy for UnavailableEvmProvider {
    fn descriptor(&self) -> OptionalProviderDescriptor {
        OptionalProviderDescriptor::unavailable(EVM_SERVICE_ID, self.reason.clone())
    }

    async fn availability(&self) -> EvmAvailability {
        EvmAvailability::unavailable("unavailable", self.reason.clone())
    }

    async fn deploy_contract(
        &self,
        _request: ContractDeployRequest,
    ) -> ServiceResult<EvmDeployAdmission> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn call_contract(
        &self,
        _request: ContractCallRequest,
    ) -> ServiceResult<EvmCallAdmission> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn read_contract(
        &self,
        _request: ContractReadRequest,
    ) -> ServiceResult<EvmReadAdmission> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn estimate_gas(
        &self,
        _request_id: macaca_proto::EvmRequestId,
    ) -> ServiceResult<GasEstimate> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn get_receipt(
        &self,
        _request: TransactionReceiptQuery,
    ) -> ServiceResult<EvmTransactionReceipt> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }

    async fn subscribe_events(
        &self,
        _request: ContractEventSubscription,
    ) -> ServiceResult<EvmEventSubscriptionAdmission> {
        Err(ServiceError::ServiceUnavailable(self.reason.clone()))
    }
}

/// Deterministic no-network provider used for tests and local development.
#[derive(Debug, Clone, Default)]
pub struct MockEvmProvider;

#[async_trait]
impl EvmProviderStrategy for MockEvmProvider {
    fn descriptor(&self) -> OptionalProviderDescriptor {
        OptionalProviderDescriptor::mock(EVM_SERVICE_ID)
    }

    async fn availability(&self) -> EvmAvailability {
        let mut availability = EvmAvailability::available(vec![
            "deploy".into(),
            "call".into(),
            "read".into(),
            "estimate_gas".into(),
            "receipt".into(),
            "subscribe".into(),
        ]);
        availability
            .metadata
            .insert("real_chain".into(), "false".into());
        availability
    }

    async fn deploy_contract(
        &self,
        request: ContractDeployRequest,
    ) -> ServiceResult<EvmDeployAdmission> {
        Ok(EvmDeployAdmission {
            result: Some(ContractDeployResult {
                request_id: request.request_id.clone(),
                chain_id: request.chain_id,
                contract_address: macaca_proto::ContractAddress::new(format!(
                    "mock.contract.{}",
                    request.request_id
                )),
                transaction_id: Some(macaca_proto::EvmTransactionId::new(format!(
                    "mock.tx.{}",
                    request.request_id
                ))),
                status: "mock_deployed".into(),
                recorded_at: Utc::now(),
                metadata: mock_metadata(),
            }),
            provider: self.descriptor(),
            status: "mock_deployed".into(),
            reason: Some("mock provider does not deploy to a real chain".into()),
            admitted_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }

    async fn call_contract(&self, request: ContractCallRequest) -> ServiceResult<EvmCallAdmission> {
        Ok(EvmCallAdmission {
            result: Some(ContractCallResult {
                request_id: request.request_id.clone(),
                chain_id: request.chain_id,
                contract_address: request.contract_address,
                transaction_id: Some(macaca_proto::EvmTransactionId::new(format!(
                    "mock.tx.{}",
                    request.request_id
                ))),
                status: "mock_called".into(),
                result_digest: Some(format!("mock-result:{}", request.function)),
                recorded_at: Utc::now(),
                metadata: mock_metadata(),
            }),
            provider: self.descriptor(),
            status: "mock_called".into(),
            reason: Some("mock provider does not execute a real contract call".into()),
            admitted_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }

    async fn read_contract(&self, request: ContractReadRequest) -> ServiceResult<EvmReadAdmission> {
        Ok(EvmReadAdmission {
            result: Some(ContractReadResult {
                request_id: request.request_id.clone(),
                chain_id: request.chain_id,
                contract_address: request.contract_address,
                status: "mock_read".into(),
                result_digest: Some(format!("mock-read:{}", request.function)),
                responded_at: Utc::now(),
                metadata: mock_metadata(),
            }),
            provider: self.descriptor(),
            status: "mock_read".into(),
            reason: Some("mock provider does not read a real chain".into()),
            responded_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }

    async fn estimate_gas(
        &self,
        request_id: macaca_proto::EvmRequestId,
    ) -> ServiceResult<GasEstimate> {
        Ok(GasEstimate {
            request_id,
            chain_id: macaca_proto::EvmChainId::new("mock.chain"),
            estimated_gas: "21000".into(),
            status: "mock_estimated".into(),
            estimated_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }

    async fn get_receipt(
        &self,
        request: TransactionReceiptQuery,
    ) -> ServiceResult<EvmTransactionReceipt> {
        Ok(EvmTransactionReceipt {
            receipt_id: macaca_proto::EvmReceiptId::new(format!(
                "mock.receipt.{}",
                request.request_id
            )),
            request_id: request.request_id,
            chain_id: request.chain_id,
            transaction_id: request.transaction_id,
            status: "mock_settled".into(),
            recorded_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }

    async fn subscribe_events(
        &self,
        request: ContractEventSubscription,
    ) -> ServiceResult<EvmEventSubscriptionAdmission> {
        Ok(EvmEventSubscriptionAdmission {
            event: Some(ContractEvent {
                subscription_id: request.request_id,
                chain_id: request.chain_id,
                contract_address: request.contract_address,
                event_name: "mock_event".into(),
                payload_digest: Some("mock-event-payload".into()),
                observed_at: Utc::now(),
                metadata: mock_metadata(),
            }),
            provider: self.descriptor(),
            status: "mock_subscribed".into(),
            reason: Some("mock provider does not subscribe to a real chain".into()),
            admitted_at: Utc::now(),
            metadata: mock_metadata(),
        })
    }
}

/// Runtime-host EVM service provider registered in ServiceRuntime.
pub struct EvmSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Arc<dyn EvmProviderStrategy>,
}

impl EvmSystemServiceProvider {
    /// Create a provider from an explicit strategy.
    pub fn new(provider: Arc<dyn EvmProviderStrategy>) -> Self {
        Self {
            descriptor: evm_service_descriptor(),
            provider,
        }
    }

    /// Create the default absent-safe provider.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableEvmProvider::default()))
    }

    /// Create the explicit mock/dev provider used by tests and local dev.
    pub fn mock() -> Self {
        Self::new(Arc::new(MockEvmProvider))
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        if trace.trace_id.trim().is_empty() {
            return Err(ServiceError::AdapterFailure(
                "evm trace context is required".into(),
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
impl SystemService for EvmSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %self.descriptor.id,
            provider_class = ?descriptor.provider_class,
            real_chain = descriptor.real_chain,
            "evm optional service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "evm optional service command accepted"
        );
        match command.name.as_str() {
            EVM_AVAILABILITY_COMMAND => {
                let typed: EvmAvailabilityCommand = decode(command.payload)?;
                Self::result(self.provider.availability().await, typed.trace)
            }
            EVM_CONTRACT_DEPLOY_COMMAND => {
                let typed: EvmContractDeployCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                let result = self.provider.deploy_contract(typed.request).await?;
                info!(trace_id = %typed.trace.trace_id, status = %result.status, "evm deploy command completed");
                Self::result(result, typed.trace)
            }
            EVM_CONTRACT_CALL_COMMAND => {
                let typed: EvmContractCallCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                let result = self.provider.call_contract(typed.request).await?;
                info!(trace_id = %typed.trace.trace_id, status = %result.status, "evm call command completed");
                Self::result(result, typed.trace)
            }
            EVM_CONTRACT_READ_COMMAND => {
                let typed: EvmContractReadCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                Self::result(
                    self.provider.read_contract(typed.request).await?,
                    typed.trace,
                )
            }
            EVM_GAS_ESTIMATE_COMMAND => {
                let typed: EvmGasEstimateCommand = decode(command.payload)?;
                Self::result(
                    self.provider.estimate_gas(typed.request_id).await?,
                    typed.trace,
                )
            }
            EVM_RECEIPT_GET_COMMAND => {
                let typed: EvmReceiptGetCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                Self::result(self.provider.get_receipt(typed.request).await?, typed.trace)
            }
            EVM_EVENT_SUBSCRIBE_COMMAND => {
                let typed: EvmEventSubscribeCommand = decode(command.payload)?;
                Self::check_metadata(&typed.request.metadata)?;
                Self::result(
                    self.provider.subscribe_events(typed.request).await?,
                    typed.trace,
                )
            }
            EVM_SNAPSHOT_COMMAND => {
                let typed: macaca_proto::EvmSnapshotCommand = decode(command.payload)?;
                let availability = self.provider.availability().await;
                let provider = self.provider.descriptor();
                let snapshot = EvmServiceSnapshot {
                    service_id: EVM_SERVICE_ID.into(),
                    health: if availability.is_available() {
                        "healthy".into()
                    } else {
                        "unavailable".into()
                    },
                    diagnostics: provider.diagnostics.clone(),
                    availability,
                    provider,
                    captured_at: Utc::now(),
                    metadata: BTreeMap::new(),
                };
                Self::result(snapshot, typed.trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported EVM service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "evm optional service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "evm optional service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let availability = self.provider.availability().await;
        if availability.is_available() {
            Ok(ServiceHealth::Healthy)
        } else {
            warn!(state = %availability.state, "evm optional service health unavailable");
            Ok(ServiceHealth::Unavailable {
                reason: availability
                    .reason_message
                    .unwrap_or_else(|| "evm unavailable".into()),
            })
        }
    }
}

/// Build the provider-neutral descriptor for EVM Service.
pub fn evm_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(EVM_SERVICE_ID),
        ServiceType::new("evm"),
        TraceSchemaRef::new("trace.system_service.evm.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.evm.availability"),
            "Reports optional EVM provider availability.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.evm.contract"),
            "Admits provider-neutral EVM contract operations.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.evm.snapshot"),
            "Reports sanitized EVM service snapshots.",
        ),
    ];
    descriptor.health = ServiceHealth::Unknown;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec![
        "evm.availability".into(),
        "evm.contract".into(),
        "evm.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

fn mock_metadata() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("mock_only".into(), "true".into()),
        ("development_only".into(), "true".into()),
        ("real_chain".into(), "false".into()),
    ])
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}
