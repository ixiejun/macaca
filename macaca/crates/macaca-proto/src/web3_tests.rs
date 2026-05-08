use std::collections::BTreeMap;

use chrono::Utc;

use crate::*;

fn signing_request() -> SigningRequest {
    SigningRequest {
        request_id: Web3RequestId::new("request.sign"),
        wallet_id: WalletId::new("wallet.custom"),
        chain_id: ChainId::new("chain.custom"),
        address: Web3Address::new("address.custom"),
        operation: "sign_digest".into(),
        payload_digest: "digest.only".into(),
        session_id: Some("session.web3".into()),
        task_id: Some("task.web3".into()),
        requested_at: Utc::now(),
        metadata: BTreeMap::from([("network".into(), "custom".into())]),
    }
}

#[test]
fn web3_availability_signing_transaction_query_and_error_round_trip() {
    let availability = Web3Availability::available(vec![
        Web3CapabilityKind::signing(),
        Web3CapabilityKind::transaction(),
    ]);
    let decoded_availability: Web3Availability =
        serde_json::from_str(&serde_json::to_string(&availability).unwrap()).unwrap();
    assert!(decoded_availability.is_available());

    let request = signing_request();
    let decoded_request: SigningRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
    assert_eq!(decoded_request.chain_id.as_str(), "chain.custom");
    assert_eq!(decoded_request.wallet_id.as_str(), "wallet.custom");

    let decision = SigningDecision {
        request_id: request.request_id.clone(),
        allowed: true,
        reason: "approved".into(),
        decided_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let proof = SigningProof {
        request_id: request.request_id.clone(),
        proof: "bounded-proof".into(),
        algorithm: "mock".into(),
        created_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let tx = TransactionRequest {
        request_id: Web3RequestId::new("request.tx"),
        wallet_id: request.wallet_id.clone(),
        chain_id: request.chain_id.clone(),
        from: request.address.clone(),
        to: Some(Web3Address::new("address.to")),
        value: Some("1".into()),
        asset: Some("CUSTOM".into()),
        method: "transfer".into(),
        payload_digest: Some("tx.digest".into()),
        session_id: request.session_id.clone(),
        task_id: request.task_id.clone(),
        requested_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let receipt = TransactionReceipt {
        receipt_id: Web3ReceiptId::new("receipt.tx"),
        request_id: tx.request_id.clone(),
        transaction_id: Web3TransactionId::new("tx.custom"),
        chain_id: tx.chain_id.clone(),
        status: "settled".into(),
        recorded_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let query = ChainQueryResponse {
        request_id: Web3RequestId::new("query.one"),
        chain_id: tx.chain_id.clone(),
        status: "ok".into(),
        result_digest: Some("result.digest".into()),
        responded_at: Utc::now(),
        metadata: BTreeMap::new(),
    };
    let error = Web3Error::Unavailable("missing optional module".into());

    let _: SigningDecision =
        serde_json::from_str(&serde_json::to_string(&decision).unwrap()).unwrap();
    let _: SigningProof = serde_json::from_str(&serde_json::to_string(&proof).unwrap()).unwrap();
    let _: TransactionRequest = serde_json::from_str(&serde_json::to_string(&tx).unwrap()).unwrap();
    let _: TransactionReceipt =
        serde_json::from_str(&serde_json::to_string(&receipt).unwrap()).unwrap();
    let decoded_query: ChainQueryResponse =
        serde_json::from_str(&serde_json::to_string(&query).unwrap()).unwrap();
    let decoded_error: Web3Error =
        serde_json::from_str(&serde_json::to_string(&error).unwrap()).unwrap();
    assert_eq!(
        decoded_query.result_digest.as_deref(),
        Some("result.digest")
    );
    assert!(matches!(decoded_error, Web3Error::Unavailable(_)));
}

#[test]
fn unavailable_policy_and_region_states_are_structured() {
    let unavailable =
        Web3Availability::unavailable(Web3UnavailableReason::new("not_installed", "missing"));
    let disabled =
        Web3Availability::disabled_by_policy(Web3UnavailableReason::new("policy", "disabled"));
    let region = Web3Availability::region_blocked(Web3UnavailableReason::new("region", "blocked"));
    assert_eq!(unavailable.state, "unavailable");
    assert_eq!(disabled.state, "disabled_by_policy");
    assert_eq!(region.state, "region_blocked");
}

#[test]
fn custom_capability_kind_is_preserved() {
    let custom = Web3CapabilityKind::new("enterprise_query");
    let decoded: Web3CapabilityKind =
        serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
    assert_eq!(decoded.as_str(), "enterprise_query");
}
