//! Optional EVM adapter implementations for Route C Phase 11.
//!
//! This module keeps provider-facing behavior separate from the facade. The
//! unavailable adapter is a Null Object used when EVM is absent. The mock
//! adapter is deterministic and no-network, so tests can exercise command,
//! policy, trace, and receipt semantics without pretending to be real chain
//! evidence.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::Utc;

use macaca_proto::{
    ContractAddress, ContractCallRequest, ContractCallResult, ContractDeployRequest,
    ContractDeployResult, ContractEvent, ContractEventSubscription, ContractReadRequest,
    ContractReadResult, EvmAvailability, EvmError, EvmReceiptId, EvmRequestId, EvmTransactionId,
    EvmTransactionReceipt, GasEstimate, TransactionReceiptQuery,
};

use crate::evm::EvmAdapter;

/// Null Object adapter used when EVM is not installed.
#[deprecated(note = "Use EvmSystemServiceProvider plus SystemEvmClient for new EVM service paths")]
#[derive(Debug, Default)]
pub struct UnavailableEvmAdapter;

#[async_trait]
impl EvmAdapter for UnavailableEvmAdapter {
    async fn availability(&self) -> EvmAvailability {
        EvmAvailability::unavailable("not_installed", "optional evm module is not installed")
    }

    async fn deploy_contract(
        &self,
        _request: &ContractDeployRequest,
    ) -> Result<ContractDeployResult, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm deploy adapter is unavailable".into(),
        ))
    }

    async fn call_contract(
        &self,
        _request: &ContractCallRequest,
    ) -> Result<ContractCallResult, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm call adapter is unavailable".into(),
        ))
    }

    async fn read_contract(
        &self,
        _request: &ContractReadRequest,
    ) -> Result<ContractReadResult, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm read adapter is unavailable".into(),
        ))
    }

    async fn subscribe_contract_events(
        &self,
        _request: &ContractEventSubscription,
    ) -> Result<ContractEvent, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm subscription adapter is unavailable".into(),
        ))
    }

    async fn estimate_gas(&self, _request_id: &EvmRequestId) -> Result<GasEstimate, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm gas adapter is unavailable".into(),
        ))
    }

    async fn get_transaction_receipt(
        &self,
        _request: &TransactionReceiptQuery,
    ) -> Result<EvmTransactionReceipt, EvmError> {
        Err(EvmError::Unavailable(
            "optional evm receipt adapter is unavailable".into(),
        ))
    }
}

/// Deterministic no-network adapter used for tests and contract semantics.
#[deprecated(
    note = "Use MockEvmProvider through EvmSystemServiceProvider for new development/test paths"
)]
#[derive(Debug, Default)]
pub struct MockEvmAdapter;

#[async_trait]
impl EvmAdapter for MockEvmAdapter {
    async fn availability(&self) -> EvmAvailability {
        EvmAvailability::available(vec![
            "deploy".into(),
            "call".into(),
            "read".into(),
            "subscribe".into(),
            "estimate_gas".into(),
            "receipt".into(),
        ])
    }

    async fn deploy_contract(
        &self,
        request: &ContractDeployRequest,
    ) -> Result<ContractDeployResult, EvmError> {
        Ok(ContractDeployResult {
            request_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            contract_address: ContractAddress::new(format!("mock.contract.{}", request.request_id)),
            transaction_id: Some(EvmTransactionId::new(format!(
                "mock.tx.{}",
                request.request_id
            ))),
            status: "mock_deployed".into(),
            recorded_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }

    async fn call_contract(
        &self,
        request: &ContractCallRequest,
    ) -> Result<ContractCallResult, EvmError> {
        Ok(ContractCallResult {
            request_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            contract_address: request.contract_address.clone(),
            transaction_id: Some(EvmTransactionId::new(format!(
                "mock.tx.{}",
                request.request_id
            ))),
            status: "mock_called".into(),
            result_digest: Some(format!("mock-result:{}", request.function)),
            recorded_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }

    async fn read_contract(
        &self,
        request: &ContractReadRequest,
    ) -> Result<ContractReadResult, EvmError> {
        Ok(ContractReadResult {
            request_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            contract_address: request.contract_address.clone(),
            status: "ok".into(),
            result_digest: Some(format!("mock-read:{}", request.function)),
            responded_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }

    async fn subscribe_contract_events(
        &self,
        request: &ContractEventSubscription,
    ) -> Result<ContractEvent, EvmError> {
        Ok(ContractEvent {
            subscription_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            contract_address: request.contract_address.clone(),
            event_name: "mock_event".into(),
            payload_digest: Some("mock-event-payload".into()),
            observed_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }

    async fn estimate_gas(&self, request_id: &EvmRequestId) -> Result<GasEstimate, EvmError> {
        Ok(GasEstimate {
            request_id: request_id.clone(),
            chain_id: macaca_proto::EvmChainId::new("mock.chain"),
            estimated_gas: "21000".into(),
            status: "mock_estimated".into(),
            estimated_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }

    async fn get_transaction_receipt(
        &self,
        request: &TransactionReceiptQuery,
    ) -> Result<EvmTransactionReceipt, EvmError> {
        Ok(EvmTransactionReceipt {
            receipt_id: EvmReceiptId::new(format!("mock.receipt.{}", request.request_id)),
            request_id: request.request_id.clone(),
            chain_id: request.chain_id.clone(),
            transaction_id: request.transaction_id.clone(),
            status: "mock_settled".into(),
            recorded_at: Utc::now(),
            metadata: simulated_metadata(),
        })
    }
}

fn simulated_metadata() -> BTreeMap<String, String> {
    BTreeMap::from([("provenance".into(), "simulated".into())])
}
