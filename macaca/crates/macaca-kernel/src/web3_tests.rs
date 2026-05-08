use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_proto::{
    ChainId, ChainQueryRequest, SigningPolicy, SigningRequest, TransactionRequest, WalletId,
    Web3Address, Web3Error, Web3RequestId,
};

use crate::web3::{DefaultWeb3PolicyEngine, MockWeb3Adapter, Web3Facade};
use crate::web3_event::InMemoryWeb3TraceEventSink;

fn approved_policy() -> SigningPolicy {
    SigningPolicy {
        explicit_approval_required: true,
        explicit_approval_granted: true,
        region_allowed: true,
        module_enabled: true,
        max_fee: None,
        metadata: BTreeMap::new(),
    }
}

fn signing_request() -> SigningRequest {
    SigningRequest {
        request_id: Web3RequestId::new("request.sign"),
        wallet_id: WalletId::new("wallet.mock"),
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
async fn web3_unavailable_returns_structured_unavailable() {
    let facade = Web3Facade::unavailable();
    assert!(!facade.availability().await.unwrap().is_available());
    let result = facade.sign(signing_request(), approved_policy()).await;
    assert!(matches!(result, Err(Web3Error::Unavailable(_))));
}

#[tokio::test]
async fn web3_signing_requires_policy_approval() {
    let facade = Web3Facade::new(
        Arc::new(MockWeb3Adapter),
        Arc::new(DefaultWeb3PolicyEngine::new()),
    );
    let result = facade
        .sign(signing_request(), SigningPolicy::default())
        .await;
    assert!(matches!(result, Err(Web3Error::ApprovalRequired(_))));
}

#[tokio::test]
async fn web3_mock_signing_transaction_and_query_emit_events() {
    let sink = Arc::new(InMemoryWeb3TraceEventSink::new());
    let facade = Web3Facade::new(
        Arc::new(MockWeb3Adapter),
        Arc::new(DefaultWeb3PolicyEngine::new()),
    )
    .with_event_sink(sink.clone());
    let proof = facade
        .sign(signing_request(), approved_policy())
        .await
        .unwrap();
    assert_eq!(proof.algorithm, "mock");

    let tx = TransactionRequest {
        request_id: Web3RequestId::new("request.tx"),
        wallet_id: WalletId::new("wallet.mock"),
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
    };
    let receipt = facade
        .submit_transaction(tx, approved_policy())
        .await
        .unwrap();
    assert_eq!(receipt.status, "mock_settled");

    let response = facade
        .query_chain(ChainQueryRequest {
            request_id: Web3RequestId::new("request.query"),
            chain_id: ChainId::new("chain.mock"),
            method: "read_state".into(),
            params_digest: None,
            session_id: Some("session.web3".into()),
            task_id: Some("task.web3".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        })
        .await
        .unwrap();
    assert_eq!(response.status, "ok");
    let events = sink.events().await;
    assert!(events.iter().any(|event| event.capability == "signing"));
    assert!(events.iter().any(|event| event.receipt_id.is_some()));
}

#[tokio::test]
async fn web3_region_policy_blocks_execution() {
    let facade = Web3Facade::new(
        Arc::new(MockWeb3Adapter),
        Arc::new(DefaultWeb3PolicyEngine::new()),
    );
    let mut policy = approved_policy();
    policy.region_allowed = false;
    let result = facade.sign(signing_request(), policy).await;
    assert!(matches!(result, Err(Web3Error::RegionBlocked(_))));
}
