use std::collections::BTreeMap;

use chrono::Utc;

use crate::*;

fn identities() -> (AgentIdentity, AgentIdentity) {
    (
        AgentIdentity::new("requester.agent"),
        AgentIdentity::new("provider.agent"),
    )
}

fn terms() -> PaymentTerms {
    PaymentTerms {
        amount: PaymentAmount::new(
            "1.25",
            PaymentAsset::new(PaymentRail::new("custom_rail"), "CUSTOM"),
        ),
        billing_unit: "operation".into(),
        expires_at: Some(Utc::now()),
        requires_explicit_approval: false,
        metadata: BTreeMap::from([("custom".into(), "preserved".into())]),
    }
}

fn quote() -> QuoteResponse {
    let (requester, provider) = identities();
    let capability = RemoteCapabilityDescriptor {
        capability_id: CapabilityId::new("capability.quote"),
        provider: provider.clone(),
        operation: "quote".into(),
        description: Some("provider-neutral fixture".into()),
        metadata: BTreeMap::new(),
    };
    QuoteResponse {
        quote_id: QuoteId::new("quote.one"),
        request: QuoteRequest {
            requester,
            provider,
            capability,
            operation: "execute".into(),
            session_id: Some("session.one".into()),
            task_id: Some("task.one".into()),
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        },
        terms: terms(),
        quoted_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

#[test]
fn quote_terms_intent_receipt_and_proof_round_trip() {
    let quote = quote();
    let decoded_quote: QuoteResponse =
        serde_json::from_str(&serde_json::to_string(&quote).unwrap()).unwrap();
    assert_eq!(
        decoded_quote.terms.amount.asset.rail.as_str(),
        "custom_rail"
    );
    assert_eq!(decoded_quote.terms.amount.asset.code, "CUSTOM");
    assert_eq!(decoded_quote.terms.billing_unit, "operation");

    let now = Utc::now();
    let intent = PaymentIntent {
        intent_id: PaymentIntentId::new("intent.one"),
        quote_id: quote.quote_id.clone(),
        requester: quote.request.requester.clone(),
        provider: quote.request.provider.clone(),
        capability_id: quote.request.capability.capability_id.clone(),
        amount: quote.terms.amount.clone(),
        state: PaymentIntentState::quoted(),
        session_id: quote.request.session_id.clone(),
        task_id: quote.request.task_id.clone(),
        created_at: now,
        updated_at: now,
        metadata: BTreeMap::new(),
    };
    let decoded_intent: PaymentIntent =
        serde_json::from_str(&serde_json::to_string(&intent).unwrap()).unwrap();
    assert_eq!(decoded_intent.state, PaymentIntentState::quoted());

    let receipt = PaymentReceipt {
        receipt_id: PaymentReceiptId::new("receipt.one"),
        quote_id: quote.quote_id,
        intent_id: intent.intent_id.clone(),
        requester: intent.requester,
        provider: intent.provider,
        capability_id: intent.capability_id,
        amount: intent.amount,
        status: "settled".into(),
        session_id: intent.session_id,
        task_id: intent.task_id,
        issued_at: now,
        metadata: BTreeMap::new(),
    };
    let decoded_receipt: PaymentReceipt =
        serde_json::from_str(&serde_json::to_string(&receipt).unwrap()).unwrap();
    assert_eq!(decoded_receipt.status, "settled");

    let proof = ExecutionProof {
        proof_id: ExecutionProofId::new("proof.one"),
        intent_id: decoded_receipt.intent_id,
        receipt_id: Some(decoded_receipt.receipt_id),
        operation: "local_simulated_execution".into(),
        status: "ok".into(),
        recorded_at: now,
        metadata: BTreeMap::from([("evidence".into(), "bounded".into())]),
    };
    let decoded_proof: ExecutionProof =
        serde_json::from_str(&serde_json::to_string(&proof).unwrap()).unwrap();
    assert_eq!(decoded_proof.metadata["evidence"], "bounded");
}

#[test]
fn custom_rail_asset_and_state_are_preserved() {
    let asset = PaymentAsset::new(PaymentRail::new("enterprise_private_rail"), "CREDIT_X");
    assert_eq!(asset.rail.as_str(), "enterprise_private_rail");
    assert_eq!(asset.code, "CREDIT_X");

    let state = PaymentIntentState::new("provider_custom_hold");
    let decoded: PaymentIntentState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(decoded.as_str(), "provider_custom_hold");
}

#[test]
fn canonical_state_machine_accepts_only_declared_phase_09_paths() {
    assert!(PaymentIntentState::created().can_transition_to(&PaymentIntentState::quoted()));
    assert!(PaymentIntentState::quoted().can_transition_to(&PaymentIntentState::pending_approval()));
    assert!(PaymentIntentState::approved().can_transition_to(&PaymentIntentState::failed()));
    assert!(!PaymentIntentState::created().can_transition_to(&PaymentIntentState::settled()));
}
