//! Optional EVM / DApp service facade for Route C Phase 11.
//!
//! The facade composes Null Object, Adapter, Strategy, Facade, Command,
//! Observer, and Memento patterns. Kernel code coordinates availability,
//! policy, trace, and adapter boundaries, but it never implements a concrete
//! chain, EVM runtime, Substrate/Frontier node, RPC provider, wallet, or
//! private-key provider. Missing EVM support is represented as structured data
//! so base OS flows keep working when the optional module is not installed.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{info, warn};

use macaca_proto::{
    ContractCallRequest, ContractCallResult, ContractDeployRequest, ContractDeployResult,
    ContractEvent, ContractEventSubscription, ContractReadRequest, ContractReadResult,
    EvmAvailability, EvmError, EvmExecutionPolicy, EvmRequestId, EvmTransactionReceipt,
    GasEstimate, TransactionReceiptQuery,
};

use crate::evm_adapter::UnavailableEvmAdapter;
use crate::evm_event::{EvmTraceEvent, EvmTraceEventSink, NoopEvmTraceEventSink};

/// Policy strategy for optional EVM commands.
pub trait EvmPolicyEngine: Send + Sync {
    /// Evaluate policy before a provider adapter can execute a command.
    fn evaluate(
        &self,
        request_id: &EvmRequestId,
        availability: &EvmAvailability,
        policy: &EvmExecutionPolicy,
        operation: &str,
        state_changing: bool,
    ) -> Result<(), EvmError>;
}

/// Conservative default policy used by the optional EVM facade.
#[derive(Debug, Default, Clone)]
pub struct DefaultEvmPolicyEngine;

impl DefaultEvmPolicyEngine {
    /// Create a stateless default policy engine.
    pub fn new() -> Self {
        Self
    }
}

impl EvmPolicyEngine for DefaultEvmPolicyEngine {
    fn evaluate(
        &self,
        request_id: &EvmRequestId,
        availability: &EvmAvailability,
        policy: &EvmExecutionPolicy,
        operation: &str,
        state_changing: bool,
    ) -> Result<(), EvmError> {
        info!(
            request_id = %request_id,
            operation,
            state_changing,
            availability = %availability.state,
            "evm policy evaluation started"
        );
        if !policy.region_allowed {
            return Err(EvmError::RegionBlocked(
                "evm request blocked by region policy".into(),
            ));
        }
        if !policy.module_enabled || !policy.compliance_allowed {
            return Err(EvmError::DisabledByPolicy(
                "evm module disabled by compliance policy".into(),
            ));
        }
        if !availability.is_available() {
            return Err(EvmError::Unavailable(
                "optional evm module is unavailable".into(),
            ));
        }
        if policy.explicit_approval_required && !policy.explicit_approval_granted {
            return Err(EvmError::ApprovalRequired(
                "explicit approval required before evm execution".into(),
            ));
        }
        if state_changing
            && (!policy.signing_allowed || !policy.payment_allowed || !policy.gas_allowed)
        {
            return Err(EvmError::PolicyDenied(
                "state-changing evm command denied by signing/payment/gas policy".into(),
            ));
        }
        Ok(())
    }
}

/// Replaceable provider boundary for EVM-compatible adapters.
#[async_trait]
pub trait EvmAdapter: Send + Sync {
    /// Return adapter availability and supported capability names.
    async fn availability(&self) -> EvmAvailability;

    /// Deploy a contract artifact after policy approval.
    async fn deploy_contract(
        &self,
        request: &ContractDeployRequest,
    ) -> Result<ContractDeployResult, EvmError>;

    /// Execute a state-changing contract call after policy approval.
    async fn call_contract(
        &self,
        request: &ContractCallRequest,
    ) -> Result<ContractCallResult, EvmError>;

    /// Read bounded contract state after availability and compliance checks.
    async fn read_contract(
        &self,
        request: &ContractReadRequest,
    ) -> Result<ContractReadResult, EvmError>;

    /// Subscribe to a bounded contract event stream.
    async fn subscribe_contract_events(
        &self,
        request: &ContractEventSubscription,
    ) -> Result<ContractEvent, EvmError>;

    /// Estimate gas for a deploy or call command without executing it.
    async fn estimate_gas(&self, request_id: &EvmRequestId) -> Result<GasEstimate, EvmError>;

    /// Return a normalized receipt memento.
    async fn get_transaction_receipt(
        &self,
        request: &TransactionReceiptQuery,
    ) -> Result<EvmTransactionReceipt, EvmError>;
}

/// Runtime-facing EVM facade.
#[deprecated(
    note = "Use SystemEvmClient through ServiceRuntime-backed SystemFacade for new EVM paths"
)]
pub struct EvmFacade {
    adapter: Arc<dyn EvmAdapter>,
    policy: Arc<dyn EvmPolicyEngine>,
    events: Arc<dyn EvmTraceEventSink>,
}

impl EvmFacade {
    /// Create a facade with explicit replaceable dependencies.
    pub fn new(adapter: Arc<dyn EvmAdapter>, policy: Arc<dyn EvmPolicyEngine>) -> Self {
        Self {
            adapter,
            policy,
            events: Arc::new(NoopEvmTraceEventSink),
        }
    }

    /// Create a facade backed by the Null Object unavailable service.
    pub fn unavailable() -> Self {
        Self::new(
            Arc::new(UnavailableEvmAdapter),
            Arc::new(DefaultEvmPolicyEngine::new()),
        )
    }

    /// Attach an event sink for trace/audit integration.
    pub fn with_event_sink(mut self, events: Arc<dyn EvmTraceEventSink>) -> Self {
        self.events = events;
        self
    }

    /// Return optional EVM availability without executing any contract logic.
    pub async fn availability(&self) -> Result<EvmAvailability, EvmError> {
        let availability = self.adapter.availability().await;
        info!(state = %availability.state, "evm availability checked");
        self.emit_event(
            "availability_check",
            &availability.state,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            availability.reason_code.clone(),
        )
        .await?;
        Ok(availability)
    }

    /// Deploy a contract only after availability and policy evaluation pass.
    pub async fn deploy_contract(
        &self,
        request: ContractDeployRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractDeployResult, EvmError> {
        self.evaluate_or_emit(
            &request.request_id,
            "deploy_contract",
            true,
            &policy,
            request.session_id.clone(),
            request.task_id.clone(),
        )
        .await?;
        let result = self.adapter.deploy_contract(&request).await?;
        self.emit_event(
            "deploy_contract",
            &result.status,
            Some(result.chain_id.to_string()),
            Some(result.request_id.to_string()),
            Some(result.contract_address.to_string()),
            result.transaction_id.as_ref().map(ToString::to_string),
            None,
            request.session_id,
            request.task_id,
            None,
        )
        .await?;
        Ok(result)
    }

    /// Execute a state-changing contract call only after policy approval.
    pub async fn call_contract(
        &self,
        request: ContractCallRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractCallResult, EvmError> {
        self.evaluate_or_emit(
            &request.request_id,
            "call_contract",
            true,
            &policy,
            request.session_id.clone(),
            request.task_id.clone(),
        )
        .await?;
        let result = self.adapter.call_contract(&request).await?;
        self.emit_event(
            "call_contract",
            &result.status,
            Some(result.chain_id.to_string()),
            Some(result.request_id.to_string()),
            Some(result.contract_address.to_string()),
            result.transaction_id.as_ref().map(ToString::to_string),
            None,
            request.session_id,
            request.task_id,
            None,
        )
        .await?;
        Ok(result)
    }

    /// Read bounded contract state after availability and compliance checks.
    pub async fn read_contract(
        &self,
        request: ContractReadRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractReadResult, EvmError> {
        self.evaluate_or_emit(
            &request.request_id,
            "read_contract",
            false,
            &policy,
            request.session_id.clone(),
            request.task_id.clone(),
        )
        .await?;
        let result = self.adapter.read_contract(&request).await?;
        self.emit_event(
            "read_contract",
            &result.status,
            Some(result.chain_id.to_string()),
            Some(result.request_id.to_string()),
            Some(result.contract_address.to_string()),
            None,
            None,
            request.session_id,
            request.task_id,
            None,
        )
        .await?;
        Ok(result)
    }

    /// Subscribe to contract events through the optional adapter boundary.
    pub async fn subscribe_contract_events(
        &self,
        request: ContractEventSubscription,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractEvent, EvmError> {
        self.evaluate_or_emit(
            &request.request_id,
            "subscribe_contract_events",
            false,
            &policy,
            request.session_id.clone(),
            request.task_id.clone(),
        )
        .await?;
        let event = self.adapter.subscribe_contract_events(&request).await?;
        self.emit_event(
            "subscribe_contract_events",
            "ok",
            Some(event.chain_id.to_string()),
            Some(event.subscription_id.to_string()),
            Some(event.contract_address.to_string()),
            None,
            None,
            request.session_id,
            request.task_id,
            None,
        )
        .await?;
        Ok(event)
    }

    /// Estimate gas after availability and compliance checks.
    pub async fn estimate_gas(
        &self,
        request_id: EvmRequestId,
        policy: EvmExecutionPolicy,
    ) -> Result<GasEstimate, EvmError> {
        self.evaluate_or_emit(&request_id, "estimate_gas", false, &policy, None, None)
            .await?;
        let estimate = self.adapter.estimate_gas(&request_id).await?;
        self.emit_event(
            "estimate_gas",
            &estimate.status,
            Some(estimate.chain_id.to_string()),
            Some(estimate.request_id.to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;
        Ok(estimate)
    }

    /// Return a normalized transaction receipt memento.
    pub async fn get_transaction_receipt(
        &self,
        request: TransactionReceiptQuery,
        policy: EvmExecutionPolicy,
    ) -> Result<EvmTransactionReceipt, EvmError> {
        self.evaluate_or_emit(
            &request.request_id,
            "get_transaction_receipt",
            false,
            &policy,
            request.session_id.clone(),
            request.task_id.clone(),
        )
        .await?;
        let receipt = self.adapter.get_transaction_receipt(&request).await?;
        self.emit_event(
            "get_transaction_receipt",
            &receipt.status,
            Some(receipt.chain_id.to_string()),
            Some(receipt.request_id.to_string()),
            None,
            Some(receipt.transaction_id.to_string()),
            Some(receipt.receipt_id.to_string()),
            request.session_id,
            request.task_id,
            None,
        )
        .await?;
        Ok(receipt)
    }

    async fn evaluate_or_emit(
        &self,
        request_id: &EvmRequestId,
        operation: &str,
        state_changing: bool,
        policy: &EvmExecutionPolicy,
        session_id: Option<String>,
        task_id: Option<String>,
    ) -> Result<(), EvmError> {
        let availability = self.availability().await?;
        match self
            .policy
            .evaluate(request_id, &availability, policy, operation, state_changing)
        {
            Ok(()) => Ok(()),
            Err(error) => {
                warn!(
                    request_id = %request_id,
                    operation,
                    error = %error,
                    "evm command rejected before adapter execution"
                );
                self.emit_event(
                    operation,
                    "denied",
                    None,
                    Some(request_id.to_string()),
                    None,
                    None,
                    None,
                    session_id,
                    task_id,
                    Some(evm_error_code(&error).into()),
                )
                .await?;
                Err(error)
            }
        }
    }

    async fn emit_event(
        &self,
        operation: &str,
        status: &str,
        chain_id: Option<String>,
        request_id: Option<String>,
        contract_address: Option<String>,
        transaction_id: Option<String>,
        receipt_id: Option<String>,
        session_id: Option<String>,
        task_id: Option<String>,
        error_code: Option<String>,
    ) -> Result<(), EvmError> {
        self.events
            .emit(EvmTraceEvent {
                chain_id,
                operation: operation.into(),
                status: status.into(),
                request_id,
                contract_address,
                transaction_id,
                receipt_id,
                session_id,
                task_id,
                timestamp: Utc::now(),
                error_code,
                metadata: BTreeMap::new(),
            })
            .await
    }
}

fn evm_error_code(error: &EvmError) -> &'static str {
    match error {
        EvmError::Unavailable(_) => "unavailable",
        EvmError::DisabledByPolicy(_) => "disabled_by_policy",
        EvmError::RegionBlocked(_) => "region_blocked",
        EvmError::ApprovalRequired(_) => "approval_required",
        EvmError::PolicyDenied(_) => "policy_denied",
        EvmError::AdapterFailed(_) => "adapter_failed",
    }
}
