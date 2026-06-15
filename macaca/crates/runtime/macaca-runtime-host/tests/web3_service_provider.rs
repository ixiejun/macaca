//! Integration tests for the Web3 optional service provider.
//!
//! These tests live outside the provider module so the production file stays
//! below the Agent OS 500-line limit while still exercising the public service
//! contract exactly as Web/SDK consumers invoke it through `service.call`.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_proto::{
    ChainId, ChainQueryRequest, ServiceCommand, ServiceCommandName, SigningRequest, TraceContext,
    TransactionRequest, Web3Address, Web3AvailabilityCommand, Web3ChainQueryCommand, Web3RequestId,
    Web3SigningRequestCommand, Web3SnapshotCommand, Web3TransactionPrepareCommand,
    Web3WalletListCommand, WEB3_AVAILABILITY_COMMAND, WEB3_CHAIN_QUERY_COMMAND,
    WEB3_SIGNING_REQUEST_COMMAND, WEB3_SNAPSHOT_COMMAND, WEB3_TRANSACTION_PREPARE_COMMAND,
    WEB3_WALLET_LIST_COMMAND,
};
use macaca_runtime_host::Web3SystemServiceProvider;

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

/// Deterministic signing request used across unavailable and mock provider scenarios.
fn signing_request() -> SigningRequest {
    SigningRequest {
        request_id: Web3RequestId::new("request.sign"),
        wallet_id: macaca_proto::WalletId::new("wallet.mock"),
        chain_id: ChainId::new("chain.mock"),
        address: Web3Address::new("address.mock"),
        operation: "sign_digest".into(),
        payload_digest: "digest".into(),
        session_id: Some("session.web3".into()),
        task_id: Some("task.web3".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn web3_service_unavailable_returns_structured_unavailable_snapshot() {
    let service = Web3SystemServiceProvider::unavailable();
    service.start().await.unwrap();

    let availability_cmd = Web3AvailabilityCommand {
        trace: TraceContext::new("trace.web3.unavailable.availability"),
    };
    let availability_result = service
        .call(service_command(
            WEB3_AVAILABILITY_COMMAND,
            availability_cmd.trace.clone(),
            &availability_cmd,
        ))
        .await
        .unwrap();
    let availability: macaca_proto::Web3Availability =
        serde_json::from_value(availability_result.output).unwrap();
    assert!(!availability.is_available());

    let snapshot_cmd = Web3SnapshotCommand {
        trace: TraceContext::new("trace.web3.unavailable.snapshot"),
    };
    let snapshot_result = service
        .call(service_command(
            WEB3_SNAPSHOT_COMMAND,
            snapshot_cmd.trace.clone(),
            &snapshot_cmd,
        ))
        .await
        .unwrap();
    let snapshot: macaca_proto::Web3ServiceSnapshot =
        serde_json::from_value(snapshot_result.output).unwrap();
    assert_eq!(snapshot.health, "unavailable");
    assert!(!snapshot.availability.is_available());
}

#[tokio::test]
async fn web3_service_unavailable_signing_fails_closed_without_panic() {
    let service = Web3SystemServiceProvider::unavailable();
    let command = Web3SigningRequestCommand {
        trace: TraceContext::new("trace.web3.unavailable.sign"),
        request: signing_request(),
    };
    let error = service
        .call(service_command(
            WEB3_SIGNING_REQUEST_COMMAND,
            command.trace.clone(),
            &command,
        ))
        .await
        .expect_err("unavailable provider must reject signing");
    assert!(
        error
            .to_string()
            .to_ascii_lowercase()
            .contains("unavailable"),
        "expected unavailable diagnostic, got: {error}"
    );
}

#[tokio::test]
async fn web3_service_mock_signing_transaction_and_query_succeed() {
    let service = Web3SystemServiceProvider::mock();
    service.start().await.unwrap();

    let sign_cmd = Web3SigningRequestCommand {
        trace: TraceContext::new("trace.web3.mock.sign"),
        request: signing_request(),
    };
    let sign_result = service
        .call(service_command(
            WEB3_SIGNING_REQUEST_COMMAND,
            sign_cmd.trace.clone(),
            &sign_cmd,
        ))
        .await
        .unwrap();
    let admission: macaca_proto::Web3SigningAdmission =
        serde_json::from_value(sign_result.output).unwrap();
    assert_eq!(admission.status, "mock_signed");
    assert!(admission.proof.is_some());

    let tx_cmd = Web3TransactionPrepareCommand {
        trace: TraceContext::new("trace.web3.mock.tx"),
        request: TransactionRequest {
            request_id: Web3RequestId::new("request.tx"),
            wallet_id: macaca_proto::WalletId::new("wallet.mock"),
            chain_id: ChainId::new("chain.mock"),
            from: Web3Address::new("address.from"),
            to: Some(Web3Address::new("address.to")),
            value: Some("1".into()),
            asset: Some("UNIT".into()),
            method: "transfer".into(),
            payload_digest: Some("tx.digest".into()),
            session_id: Some("session.web3".into()),
            task_id: Some("task.web3".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let tx_result = service
        .call(service_command(
            WEB3_TRANSACTION_PREPARE_COMMAND,
            tx_cmd.trace.clone(),
            &tx_cmd,
        ))
        .await
        .unwrap();
    let preparation: macaca_proto::Web3TransactionPreparation =
        serde_json::from_value(tx_result.output).unwrap();
    assert_eq!(preparation.status, "mock_prepared");
    assert!(preparation.receipt.is_some());

    let query_cmd = Web3ChainQueryCommand {
        trace: TraceContext::new("trace.web3.mock.query"),
        request: ChainQueryRequest {
            request_id: Web3RequestId::new("request.query"),
            chain_id: ChainId::new("chain.mock"),
            method: "read_state".into(),
            params_digest: None,
            session_id: Some("session.web3".into()),
            task_id: Some("task.web3".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
    };
    let query_result = service
        .call(service_command(
            WEB3_CHAIN_QUERY_COMMAND,
            query_cmd.trace.clone(),
            &query_cmd,
        ))
        .await
        .unwrap();
    let response: macaca_proto::ChainQueryResponse =
        serde_json::from_value(query_result.output).unwrap();
    assert_eq!(response.status, "mock_ok");
}

#[tokio::test]
async fn web3_service_mock_wallet_list_returns_provider_views() {
    let service = Web3SystemServiceProvider::mock();
    let command = Web3WalletListCommand {
        trace: TraceContext::new("trace.web3.mock.wallets"),
    };
    let result = service
        .call(service_command(
            WEB3_WALLET_LIST_COMMAND,
            command.trace.clone(),
            &command,
        ))
        .await
        .unwrap();
    let wallets: Vec<macaca_proto::Web3WalletView> = serde_json::from_value(result.output).unwrap();
    assert_eq!(wallets.len(), 1);
    assert_eq!(wallets[0].wallet_id, "mock.wallet");
}

#[tokio::test]
async fn web3_service_rejects_command_without_trace() {
    let service = Web3SystemServiceProvider::mock();
    let command = ServiceCommand::without_trace(
        ServiceCommandName::new(WEB3_SNAPSHOT_COMMAND),
        serde_json::to_value(Web3SnapshotCommand {
            trace: TraceContext::new("trace.unused"),
        })
        .unwrap(),
    );
    assert!(service.call(command).await.is_err());
}

#[tokio::test]
async fn web3_service_unavailable_health_reports_structured_reason() {
    let service = Web3SystemServiceProvider::unavailable();
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
