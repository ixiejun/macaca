//! Payment adapter strategies for the Route C Payment Service.
//!
//! This module contains only provider-facing settlement behavior.  The runtime
//! service provider owns command dispatch, admission control, policy checks and
//! persistence.  Splitting the adapter keeps the service provider small and
//! makes future network, wallet, chain, gateway, or billing adapters replaceable
//! without changing the Payment Service contract.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    A2AError, ExecutionProof, ExecutionProofId, PaymentIntent, PaymentReceipt, PaymentReceiptId,
    PaymentTerms, QuoteResponse,
};
use tracing::info;

/// Replaceable adapter Strategy for Payment Service settlement behavior.
///
/// Implementations must remain provider-neutral at this layer: the caller gives
/// a normalized A2A quote or intent, and the adapter returns normalized receipts
/// and execution proofs.  Provider-specific authentication, wallets, gateways,
/// and chain clients belong behind this trait, not in Web or kernel code.
#[async_trait]
pub trait PaymentAdapterStrategy: Send + Sync {
    /// Report whether this adapter can execute settlement.
    ///
    /// Policy evaluation uses this as a conservative fail-closed guard.  A
    /// provider may be registered but still unavailable because credentials,
    /// wallet state, network, or entitlement setup is missing.
    fn is_configured(&self) -> bool;

    /// Return quote terms for one provider-neutral A2A request.
    async fn quote(&self, request: macaca_proto::QuoteRequest) -> Result<QuoteResponse, A2AError>;

    /// Execute settlement for one approved intent.
    async fn settle(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(PaymentReceipt, ExecutionProof), A2AError>;
}

/// Deterministic no-network adapter used for local tests and bootstrap.
///
/// The local adapter intentionally does not perform any network or wallet I/O.
/// It still emits the same normalized quote, receipt, and proof structures as a
/// real adapter so upper layers can be verified without installing a payment
/// provider.  Production adapters can replace this Strategy through dependency
/// injection.
pub struct LocalSimulatedPaymentAdapter {
    terms: PaymentTerms,
}

impl LocalSimulatedPaymentAdapter {
    /// Create a local simulated adapter with explicit quote terms.
    pub fn new(terms: PaymentTerms) -> Self {
        Self { terms }
    }

    /// Produce deterministic-shape identifiers without adding runtime deps.
    ///
    /// The timestamp is sufficient for local bootstrap/test traces.  Durable
    /// provider integrations should use provider-issued identifiers or a real
    /// ID generator behind their adapter Strategy.
    fn synthetic_id(prefix: &str) -> String {
        format!(
            "{prefix}.{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    }
}

#[async_trait]
impl PaymentAdapterStrategy for LocalSimulatedPaymentAdapter {
    fn is_configured(&self) -> bool {
        true
    }

    async fn quote(&self, request: macaca_proto::QuoteRequest) -> Result<QuoteResponse, A2AError> {
        info!(
            requester_id = %request.requester.id,
            provider_id = %request.provider.id,
            capability_id = %request.capability.capability_id,
            "payment service local simulated quote generated"
        );
        Ok(QuoteResponse {
            quote_id: macaca_proto::QuoteId::new(Self::synthetic_id("quote")),
            request,
            terms: self.terms.clone(),
            quoted_at: Utc::now(),
            metadata: BTreeMap::from([("simulation".into(), "true".into())]),
        })
    }

    async fn settle(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(PaymentReceipt, ExecutionProof), A2AError> {
        let receipt_id = PaymentReceiptId::new(Self::synthetic_id("receipt"));
        let receipt = PaymentReceipt {
            receipt_id: receipt_id.clone(),
            quote_id: intent.quote_id.clone(),
            intent_id: intent.intent_id.clone(),
            requester: intent.requester.clone(),
            provider: intent.provider.clone(),
            capability_id: intent.capability_id.clone(),
            amount: intent.amount.clone(),
            status: "settled".into(),
            session_id: intent.session_id.clone(),
            task_id: intent.task_id.clone(),
            issued_at: Utc::now(),
            metadata: BTreeMap::from([("simulation".into(), "true".into())]),
        };
        let proof = ExecutionProof {
            proof_id: ExecutionProofId::new(Self::synthetic_id("proof")),
            intent_id: intent.intent_id.clone(),
            receipt_id: Some(receipt_id),
            operation: "local_simulated_settlement".into(),
            status: "ok".into(),
            recorded_at: Utc::now(),
            metadata: BTreeMap::from([("simulation".into(), "true".into())]),
        };
        Ok((receipt, proof))
    }
}
