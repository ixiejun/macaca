use std::collections::BTreeMap;

use chrono::Utc;

use crate::{
    ContractAbiRef, ContractAddress, ContractCallRequest, ContractDeployRequest,
    ContractEventSubscription, ContractFunctionRef, ContractReadRequest, EvmAvailability,
    EvmChainId, EvmExecutionPolicy, EvmRequestId, EvmTransactionId, GasPolicy,
    TransactionReceiptQuery,
};

fn gas_policy() -> GasPolicy {
    GasPolicy {
        max_gas: Some("custom-gas-limit".into()),
        max_fee: Some("custom-fee-limit".into()),
        priority_fee: None,
        metadata: BTreeMap::from([("unit".into(), "provider-neutral".into())]),
    }
}

#[test]
fn evm_contract_commands_roundtrip_with_custom_metadata() {
    let deploy = ContractDeployRequest {
        request_id: EvmRequestId::new("deploy.1"),
        chain_id: EvmChainId::new("custom-chain"),
        wallet_id: Some("wallet.scoped".into()),
        artifact_ref: "artifact.digest".into(),
        abi_ref: ContractAbiRef::new("abi.ref"),
        constructor_args_digest: Some("args.digest".into()),
        gas_policy: gas_policy(),
        session_id: Some("session.evm".into()),
        task_id: Some("task.evm".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::from([("dapp".into(), "generic".into())]),
    };
    let decoded: ContractDeployRequest =
        serde_json::from_str(&serde_json::to_string(&deploy).unwrap()).unwrap();
    assert_eq!(decoded.chain_id.as_str(), "custom-chain");
    assert_eq!(decoded.abi_ref.as_str(), "abi.ref");
    assert_eq!(
        decoded.gas_policy.max_gas.as_deref(),
        Some("custom-gas-limit")
    );
    assert_eq!(decoded.metadata.get("dapp").unwrap(), "generic");
}

#[test]
fn evm_call_read_subscription_and_receipt_are_provider_neutral() {
    let call = ContractCallRequest {
        request_id: EvmRequestId::new("call.1"),
        chain_id: EvmChainId::new("chain.dynamic"),
        wallet_id: Some("wallet.dynamic".into()),
        contract_address: ContractAddress::new("contract.dynamic"),
        abi_ref: ContractAbiRef::new("abi.dynamic"),
        function: ContractFunctionRef::new("function.dynamic"),
        args_digest: Some("args.digest".into()),
        value: Some("1".into()),
        gas_policy: gas_policy(),
        session_id: None,
        task_id: None,
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let read = ContractReadRequest {
        request_id: EvmRequestId::new("read.1"),
        chain_id: call.chain_id.clone(),
        contract_address: call.contract_address.clone(),
        abi_ref: call.abi_ref.clone(),
        function: call.function.clone(),
        args_digest: None,
        session_id: None,
        task_id: None,
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let subscription = ContractEventSubscription {
        request_id: EvmRequestId::new("sub.1"),
        chain_id: call.chain_id.clone(),
        contract_address: call.contract_address.clone(),
        event_filter: Some("event.digest".into()),
        session_id: None,
        task_id: None,
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let receipt = TransactionReceiptQuery {
        request_id: EvmRequestId::new("receipt.1"),
        chain_id: call.chain_id.clone(),
        transaction_id: EvmTransactionId::new("tx.dynamic"),
        session_id: None,
        task_id: None,
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    assert_eq!(call.function.as_str(), "function.dynamic");
    assert_eq!(read.contract_address.as_str(), "contract.dynamic");
    assert_eq!(subscription.event_filter.as_deref(), Some("event.digest"));
    assert_eq!(receipt.transaction_id.as_str(), "tx.dynamic");
}

#[test]
fn evm_availability_and_policy_are_structured_data() {
    let unavailable = EvmAvailability::unavailable("not_installed", "optional evm absent");
    assert!(!unavailable.is_available());
    assert_eq!(unavailable.reason_code.as_deref(), Some("not_installed"));

    let policy = EvmExecutionPolicy::default();
    assert!(policy.signing_allowed);
    assert!(!policy.explicit_approval_granted);
}
