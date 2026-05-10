use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    ContractAbiRef, ContractAddress, ContractCallRequest, ContractDeployRequest,
    ContractEventSubscription, ContractFunctionRef, ContractReadRequest, EvmChainId, EvmError,
    EvmExecutionPolicy, EvmRequestId, EvmTransactionId, GasPolicy, TransactionReceiptQuery,
};

use crate::evm::{DefaultEvmPolicyEngine, EvmFacade};
use crate::evm_adapter::MockEvmAdapter;
use crate::evm_event::InMemoryEvmTraceEventSink;

fn approved_policy() -> EvmExecutionPolicy {
    EvmExecutionPolicy {
        explicit_approval_required: true,
        explicit_approval_granted: true,
        signing_allowed: true,
        payment_allowed: true,
        gas_allowed: true,
        region_allowed: true,
        module_enabled: true,
        compliance_allowed: true,
        metadata: BTreeMap::new(),
    }
}

fn deploy_request() -> ContractDeployRequest {
    ContractDeployRequest {
        request_id: EvmRequestId::new("deploy.test"),
        chain_id: EvmChainId::new("chain.test"),
        wallet_id: Some("wallet.test".into()),
        artifact_ref: "artifact.digest".into(),
        abi_ref: ContractAbiRef::new("abi.test"),
        constructor_args_digest: Some("ctor.digest".into()),
        gas_policy: GasPolicy::default(),
        session_id: Some("session.evm".into()),
        task_id: Some("task.evm".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

fn call_request() -> ContractCallRequest {
    ContractCallRequest {
        request_id: EvmRequestId::new("call.test"),
        chain_id: EvmChainId::new("chain.test"),
        wallet_id: Some("wallet.test".into()),
        contract_address: ContractAddress::new("contract.test"),
        abi_ref: ContractAbiRef::new("abi.test"),
        function: ContractFunctionRef::new("mutate"),
        args_digest: Some("args.digest".into()),
        value: None,
        gas_policy: GasPolicy::default(),
        session_id: Some("session.evm".into()),
        task_id: Some("task.evm".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

fn facade_with_mock(sink: Arc<InMemoryEvmTraceEventSink>) -> EvmFacade {
    EvmFacade::new(
        Arc::new(MockEvmAdapter),
        Arc::new(DefaultEvmPolicyEngine::new()),
    )
    .with_event_sink(sink)
}

#[tokio::test]
async fn evm_unavailable_returns_structured_unavailable() {
    let facade = EvmFacade::unavailable();
    assert!(!facade.availability().await.unwrap().is_available());
    let result = facade
        .deploy_contract(deploy_request(), approved_policy())
        .await;
    assert!(matches!(result, Err(EvmError::Unavailable(_))));
}

#[tokio::test]
async fn evm_deploy_and_call_require_policy_approval() {
    let sink = Arc::new(InMemoryEvmTraceEventSink::new());
    let facade = facade_with_mock(sink.clone());
    let result = facade
        .call_contract(call_request(), EvmExecutionPolicy::default())
        .await;
    assert!(matches!(result, Err(EvmError::ApprovalRequired(_))));
    assert!(sink
        .events()
        .await
        .iter()
        .any(|event| event.status == "denied"));
}

#[tokio::test]
async fn evm_mock_adapter_marks_simulated_outputs_and_emits_events() {
    let sink = Arc::new(InMemoryEvmTraceEventSink::new());
    let facade = facade_with_mock(sink.clone());
    let deployed = facade
        .deploy_contract(deploy_request(), approved_policy())
        .await
        .unwrap();
    assert_eq!(deployed.status, "mock_deployed");
    assert_eq!(
        deployed.metadata.get("provenance").map(String::as_str),
        Some("simulated")
    );

    let called = facade
        .call_contract(call_request(), approved_policy())
        .await
        .unwrap();
    assert_eq!(called.status, "mock_called");

    let read = facade
        .read_contract(
            ContractReadRequest {
                request_id: EvmRequestId::new("read.test"),
                chain_id: EvmChainId::new("chain.test"),
                contract_address: ContractAddress::new("contract.test"),
                abi_ref: ContractAbiRef::new("abi.test"),
                function: ContractFunctionRef::new("view"),
                args_digest: None,
                session_id: Some("session.evm".into()),
                task_id: Some("task.evm".into()),
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
            approved_policy(),
        )
        .await
        .unwrap();
    assert_eq!(read.status, "ok");

    let event = facade
        .subscribe_contract_events(
            ContractEventSubscription {
                request_id: EvmRequestId::new("sub.test"),
                chain_id: EvmChainId::new("chain.test"),
                contract_address: ContractAddress::new("contract.test"),
                event_filter: None,
                session_id: Some("session.evm".into()),
                task_id: Some("task.evm".into()),
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
            approved_policy(),
        )
        .await
        .unwrap();
    assert_eq!(event.event_name, "mock_event");

    let estimate = facade
        .estimate_gas(EvmRequestId::new("gas.test"), approved_policy())
        .await
        .unwrap();
    assert_eq!(estimate.estimated_gas, "21000");

    let receipt = facade
        .get_transaction_receipt(
            TransactionReceiptQuery {
                request_id: EvmRequestId::new("receipt.test"),
                chain_id: EvmChainId::new("chain.test"),
                transaction_id: EvmTransactionId::new("tx.test"),
                session_id: Some("session.evm".into()),
                task_id: Some("task.evm".into()),
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
            approved_policy(),
        )
        .await
        .unwrap();
    assert_eq!(receipt.status, "mock_settled");

    let events = sink.events().await;
    assert!(events
        .iter()
        .any(|event| event.operation == "deploy_contract"));
    assert!(events
        .iter()
        .any(|event| event.operation == "call_contract"));
    assert!(events.iter().any(|event| event.receipt_id.is_some()));
}

#[tokio::test]
async fn evm_region_policy_blocks_execution_before_adapter() {
    let sink = Arc::new(InMemoryEvmTraceEventSink::new());
    let facade = facade_with_mock(sink);
    let mut policy = approved_policy();
    policy.region_allowed = false;
    let result = facade.deploy_contract(deploy_request(), policy).await;
    assert!(matches!(result, Err(EvmError::RegionBlocked(_))));
}
