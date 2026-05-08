//! SDK-facing Optional EVM / DApp facade for Route C Phase 11.
//!
//! The SDK facade constructs provider-neutral commands and delegates execution
//! to the kernel EVM facade. It does not instantiate wallets, RPC clients,
//! browser providers, Substrate/Frontier nodes, or concrete EVM adapters. This
//! preserves dependency inversion: applications describe intent while the OS
//! service boundary enforces policy, trace, audit, and module availability.

use std::sync::Arc;

use macaca_kernel::EvmFacade;
use macaca_proto::{
    ContractCallRequest, ContractCallResult, ContractDeployRequest, ContractDeployResult,
    ContractEvent, ContractEventSubscription, ContractReadRequest, ContractReadResult, EvmError,
    EvmExecutionPolicy, EvmRequestId, EvmTransactionReceipt, GasEstimate, TransactionReceiptQuery,
};

/// Thin SDK facade for optional EVM commands.
pub struct MacacaEvmSdk {
    facade: Arc<EvmFacade>,
}

impl MacacaEvmSdk {
    /// Create an SDK facade backed by a runtime-provided EVM facade.
    pub fn new(facade: Arc<EvmFacade>) -> Self {
        Self { facade }
    }

    /// Deploy a contract through the policy-aware service boundary.
    pub async fn deploy_contract(
        &self,
        request: ContractDeployRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractDeployResult, EvmError> {
        self.facade.deploy_contract(request, policy).await
    }

    /// Execute a state-changing call through the policy-aware service boundary.
    pub async fn call_contract(
        &self,
        request: ContractCallRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractCallResult, EvmError> {
        self.facade.call_contract(request, policy).await
    }

    /// Read bounded contract state through the optional service boundary.
    pub async fn read_contract(
        &self,
        request: ContractReadRequest,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractReadResult, EvmError> {
        self.facade.read_contract(request, policy).await
    }

    /// Subscribe to contract events through the optional service boundary.
    pub async fn subscribe_contract_events(
        &self,
        request: ContractEventSubscription,
        policy: EvmExecutionPolicy,
    ) -> Result<ContractEvent, EvmError> {
        self.facade.subscribe_contract_events(request, policy).await
    }

    /// Estimate gas without allowing SDK code to call providers directly.
    pub async fn estimate_gas(
        &self,
        request_id: EvmRequestId,
        policy: EvmExecutionPolicy,
    ) -> Result<GasEstimate, EvmError> {
        self.facade.estimate_gas(request_id, policy).await
    }

    /// Fetch a transaction receipt through the normalized service boundary.
    pub async fn get_transaction_receipt(
        &self,
        request: TransactionReceiptQuery,
        policy: EvmExecutionPolicy,
    ) -> Result<EvmTransactionReceipt, EvmError> {
        self.facade.get_transaction_receipt(request, policy).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;
    use macaca_kernel::{DefaultEvmPolicyEngine, MockEvmAdapter};
    use macaca_proto::{
        ContractAbiRef, ContractDeployRequest, EvmChainId, EvmExecutionPolicy, EvmRequestId,
        GasPolicy,
    };

    use super::*;

    #[tokio::test]
    async fn sdk_delegates_to_policy_aware_evm_facade() {
        let facade = Arc::new(EvmFacade::new(
            Arc::new(MockEvmAdapter),
            Arc::new(DefaultEvmPolicyEngine::new()),
        ));
        let sdk = MacacaEvmSdk::new(facade);
        let result = sdk
            .deploy_contract(
                ContractDeployRequest {
                    request_id: EvmRequestId::new("sdk.deploy"),
                    chain_id: EvmChainId::new("sdk.chain"),
                    wallet_id: Some("sdk.wallet".into()),
                    artifact_ref: "artifact.digest".into(),
                    abi_ref: ContractAbiRef::new("abi.sdk"),
                    constructor_args_digest: None,
                    gas_policy: GasPolicy::default(),
                    session_id: None,
                    task_id: None,
                    requested_at: Utc::now(),
                    metadata: BTreeMap::new(),
                },
                EvmExecutionPolicy {
                    explicit_approval_granted: true,
                    ..EvmExecutionPolicy::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(result.status, "mock_deployed");
    }
}
