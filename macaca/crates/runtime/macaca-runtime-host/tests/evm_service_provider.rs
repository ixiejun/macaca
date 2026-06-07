//! Integration tests for the Route C EVM optional service provider.
//!
//! These tests live outside the provider module so the production file stays
//! below the Agent OS 500-line limit while still exercising the public service
//! contract exactly as Web/SDK consumers invoke it through `service.call`.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_proto::{
    ContractAbiRef, ContractAddress, ContractCallRequest, ContractDeployRequest,
    ContractEventSubscription, ContractFunctionRef, ContractReadRequest, EvmAvailabilityCommand,
    EvmContractCallCommand, EvmContractDeployCommand, EvmContractReadCommand,
    EvmEventSubscribeCommand, EvmGasEstimateCommand, EvmReceiptGetCommand, EvmRequestId,
    EvmSnapshotCommand, EvmTransactionId, GasPolicy, ServiceCommand, ServiceCommandName,
    TraceContext, TransactionReceiptQuery, EVM_AVAILABILITY_COMMAND, EVM_CONTRACT_CALL_COMMAND,
    EVM_CONTRACT_DEPLOY_COMMAND, EVM_CONTRACT_READ_COMMAND, EVM_EVENT_SUBSCRIBE_COMMAND,
    EVM_GAS_ESTIMATE_COMMAND, EVM_RECEIPT_GET_COMMAND, EVM_SNAPSHOT_COMMAND,
};
use macaca_runtime_host::EvmSystemServiceProvider;

/// Build a typed `ServiceCommand` envelope with trace propagation for provider dispatch.
fn service_command<T: serde::Serialize>(
    name: &str,
    trace: TraceContext,
    payload: &T,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        trace,
    )
}

/// Provider-neutral deploy request reused across unavailable and mock scenarios.
fn deploy_request() -> ContractDeployRequest {
    ContractDeployRequest {
        request_id: EvmRequestId::new("request.deploy"),
        chain_id: macaca_proto::EvmChainId::new("chain.mock"),
        wallet_id: Some("wallet.mock".into()),
        artifact_ref: "artifact.digest".into(),
        abi_ref: ContractAbiRef::new("abi.mock"),
        constructor_args_digest: None,
        gas_policy: GasPolicy::default(),
        session_id: Some("session.evm".into()),
        task_id: Some("task.evm".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn evm_service_unavailable_returns_structured_unavailable_snapshot() {
    let service = EvmSystemServiceProvider::unavailable();
    service.start().await.unwrap();

    let availability_cmd = EvmAvailabilityCommand {
        trace: TraceContext::new("trace.evm.unavailable.availability"),
    };
    let availability_result = service
        .call(service_command(
            EVM_AVAILABILITY_COMMAND,
            availability_cmd.trace.clone(),
            &availability_cmd,
        ))
        .await
        .unwrap();
    let availability: macaca_proto::EvmAvailability =
        serde_json::from_value(availability_result.output).unwrap();
    assert!(!availability.is_available());

    let snapshot_cmd = EvmSnapshotCommand {
        trace: TraceContext::new("trace.evm.unavailable.snapshot"),
    };
    let snapshot_result = service
        .call(service_command(
            EVM_SNAPSHOT_COMMAND,
            snapshot_cmd.trace.clone(),
            &snapshot_cmd,
        ))
        .await
        .unwrap();
    let snapshot: macaca_proto::EvmServiceSnapshot =
        serde_json::from_value(snapshot_result.output).unwrap();
    assert_eq!(snapshot.health, "unavailable");
    assert!(!snapshot.availability.is_available());
}

#[tokio::test]
async fn evm_service_unavailable_deploy_fails_closed_without_panic() {
    let service = EvmSystemServiceProvider::unavailable();
    let command = EvmContractDeployCommand {
        trace: TraceContext::new("trace.evm.unavailable.deploy"),
        request: deploy_request(),
    };
    let error = service
        .call(service_command(
            EVM_CONTRACT_DEPLOY_COMMAND,
            command.trace.clone(),
            &command,
        ))
        .await
        .expect_err("unavailable provider must reject deploy");
    assert!(
        error.to_string().to_ascii_lowercase().contains("unavailable"),
        "expected unavailable diagnostic, got: {error}"
    );
}

#[tokio::test]
async fn evm_service_mock_deploy_call_read_gas_receipt_and_subscribe_succeed() {
    let service = EvmSystemServiceProvider::mock();
    service.start().await.unwrap();

    let deploy_cmd = EvmContractDeployCommand {
        trace: TraceContext::new("trace.evm.mock.deploy"),
        request: deploy_request(),
    };
    let deploy_result = service
        .call(service_command(
            EVM_CONTRACT_DEPLOY_COMMAND,
            deploy_cmd.trace.clone(),
            &deploy_cmd,
        ))
        .await
        .unwrap();
    let deploy: macaca_proto::EvmDeployAdmission =
        serde_json::from_value(deploy_result.output).unwrap();
    assert_eq!(deploy.status, "mock_deployed");
    assert!(deploy.result.is_some());

    let call_cmd = EvmContractCallCommand {
        trace: TraceContext::new("trace.evm.mock.call"),
        request: ContractCallRequest {
            request_id: EvmRequestId::new("request.call"),
            chain_id: macaca_proto::EvmChainId::new("chain.mock"),
            wallet_id: Some("wallet.mock".into()),
            contract_address: ContractAddress::new("contract.mock"),
            abi_ref: ContractAbiRef::new("abi.mock"),
            function: ContractFunctionRef::new("transfer"),
            args_digest: Some("args.digest".into()),
            value: Some("1".into()),
            gas_policy: GasPolicy::default(),
            session_id: Some("session.evm".into()),
            task_id: Some("task.evm".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let call_result = service
        .call(service_command(
            EVM_CONTRACT_CALL_COMMAND,
            call_cmd.trace.clone(),
            &call_cmd,
        ))
        .await
        .unwrap();
    let call: macaca_proto::EvmCallAdmission =
        serde_json::from_value(call_result.output).unwrap();
    assert_eq!(call.status, "mock_called");

    let read_cmd = EvmContractReadCommand {
        trace: TraceContext::new("trace.evm.mock.read"),
        request: ContractReadRequest {
            request_id: EvmRequestId::new("request.read"),
            chain_id: macaca_proto::EvmChainId::new("chain.mock"),
            contract_address: ContractAddress::new("contract.mock"),
            abi_ref: ContractAbiRef::new("abi.mock"),
            function: ContractFunctionRef::new("balanceOf"),
            args_digest: None,
            session_id: Some("session.evm".into()),
            task_id: Some("task.evm".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let read_result = service
        .call(service_command(
            EVM_CONTRACT_READ_COMMAND,
            read_cmd.trace.clone(),
            &read_cmd,
        ))
        .await
        .unwrap();
    let read: macaca_proto::EvmReadAdmission =
        serde_json::from_value(read_result.output).unwrap();
    assert_eq!(read.status, "mock_read");

    let gas_cmd = EvmGasEstimateCommand {
        trace: TraceContext::new("trace.evm.mock.gas"),
        request_id: EvmRequestId::new("request.gas"),
    };
    let gas_result = service
        .call(service_command(
            EVM_GAS_ESTIMATE_COMMAND,
            gas_cmd.trace.clone(),
            &gas_cmd,
        ))
        .await
        .unwrap();
    let gas: macaca_proto::GasEstimate = serde_json::from_value(gas_result.output).unwrap();
    assert_eq!(gas.estimated_gas, "21000");

    let receipt_cmd = EvmReceiptGetCommand {
        trace: TraceContext::new("trace.evm.mock.receipt"),
        request: TransactionReceiptQuery {
            request_id: EvmRequestId::new("request.receipt"),
            chain_id: macaca_proto::EvmChainId::new("chain.mock"),
            transaction_id: EvmTransactionId::new("tx.mock"),
            session_id: Some("session.evm".into()),
            task_id: Some("task.evm".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let receipt_result = service
        .call(service_command(
            EVM_RECEIPT_GET_COMMAND,
            receipt_cmd.trace.clone(),
            &receipt_cmd,
        ))
        .await
        .unwrap();
    let receipt: macaca_proto::EvmTransactionReceipt =
        serde_json::from_value(receipt_result.output).unwrap();
    assert_eq!(receipt.status, "mock_settled");

    let subscribe_cmd = EvmEventSubscribeCommand {
        trace: TraceContext::new("trace.evm.mock.subscribe"),
        request: ContractEventSubscription {
            request_id: EvmRequestId::new("request.subscribe"),
            chain_id: macaca_proto::EvmChainId::new("chain.mock"),
            contract_address: ContractAddress::new("contract.mock"),
            event_filter: Some("event.filter".into()),
            session_id: Some("session.evm".into()),
            task_id: Some("task.evm".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let subscribe_result = service
        .call(service_command(
            EVM_EVENT_SUBSCRIBE_COMMAND,
            subscribe_cmd.trace.clone(),
            &subscribe_cmd,
        ))
        .await
        .unwrap();
    let subscription: macaca_proto::EvmEventSubscriptionAdmission =
        serde_json::from_value(subscribe_result.output).unwrap();
    assert_eq!(subscription.status, "mock_subscribed");
    assert!(subscription.event.is_some());
}

#[tokio::test]
async fn evm_service_rejects_command_without_trace() {
    let service = EvmSystemServiceProvider::mock();
    let command = ServiceCommand::without_trace(
        ServiceCommandName::new(EVM_SNAPSHOT_COMMAND),
        serde_json::to_value(EvmSnapshotCommand {
            trace: TraceContext::new("trace.unused"),
        })
        .unwrap(),
    );
    assert!(service.call(command).await.is_err());
}

#[tokio::test]
async fn evm_service_unavailable_health_reports_structured_reason() {
    let service = EvmSystemServiceProvider::unavailable();
    let health = service.health().await.unwrap();
    match health {
        macaca_proto::ServiceHealth::Unavailable { reason } => {
            assert!(
                reason.to_ascii_lowercase().contains("unavailable"),
                "expected unavailable health reason, got: {reason}"
            );
        }
        other => panic!("expected unavailable health, got: {other:?}"),
    }
}
