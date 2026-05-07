//! Payment persistence contracts for A2A Payment v0.
//!
//! The repository keeps payment artifacts as mementos: quote snapshots,
//! payment intent transitions, receipts, and execution proofs are appended or
//! upserted without coupling callers to one database engine. The in-memory
//! adapter is deterministic and mirrors the contract future durable backends
//! must satisfy.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::info;

use macaca_proto::{
    A2AError, ExecutionProof, PaymentIntentId, PaymentIntentState, PaymentReceipt, QuoteId,
    QuoteResponse,
};

/// Ordered state transition stored for audit and replay.
///
/// The store records both the target state and the operation that caused it so
/// trace consumers can reconstruct the lifecycle without reading runtime logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentStateTransition {
    pub intent_id: PaymentIntentId,
    pub from: Option<PaymentIntentState>,
    pub to: PaymentIntentState,
    pub operation: String,
    pub status: String,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PaymentStateTransition {
    /// Create a transition memento at the current wall-clock time.
    pub fn new(
        intent_id: PaymentIntentId,
        from: Option<PaymentIntentState>,
        to: PaymentIntentState,
        operation: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            intent_id,
            from,
            to,
            operation: operation.into(),
            status: status.into(),
            reason: None,
            timestamp: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Repository abstraction for A2A payment artifacts.
///
/// Runtime services use this trait instead of a concrete database. That keeps
/// A2A payment replaceable and lets later durable stores preserve the same
/// quote/transition/receipt/proof indexes.
#[async_trait]
pub trait PaymentStore: Send + Sync {
    /// Persist a quote snapshot and update session/task indexes when present.
    async fn put_quote(&self, quote: QuoteResponse) -> Result<(), A2AError>;

    /// Read a quote snapshot by quote id.
    async fn get_quote(&self, quote_id: &QuoteId) -> Result<Option<QuoteResponse>, A2AError>;

    /// Append one intent transition in chronological order.
    async fn append_transition(&self, transition: PaymentStateTransition) -> Result<(), A2AError>;

    /// Return all transitions for one intent in append order.
    async fn transitions_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Vec<PaymentStateTransition>, A2AError>;

    /// Persist an immutable payment receipt and update query indexes.
    async fn put_receipt(&self, receipt: PaymentReceipt) -> Result<(), A2AError>;

    /// Return receipts associated with one session id.
    async fn receipts_by_session(&self, session_id: &str) -> Result<Vec<PaymentReceipt>, A2AError>;

    /// Return receipts associated with one task id.
    async fn receipts_by_task(&self, task_id: &str) -> Result<Vec<PaymentReceipt>, A2AError>;

    /// Return the receipt for one payment intent when available.
    async fn receipt_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Option<PaymentReceipt>, A2AError>;

    /// Persist execution evidence emitted by an adapter or simulator.
    async fn put_execution_proof(&self, proof: ExecutionProof) -> Result<(), A2AError>;

    /// Return execution proof records associated with one intent.
    async fn execution_proofs_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Vec<ExecutionProof>, A2AError>;
}

/// Deterministic in-memory implementation for tests and local scaffolds.
///
/// The adapter uses separate indexes instead of scanning every artifact. This
/// keeps the behavior close to a production store and prevents future callers
/// from depending on application-wide reads.
#[derive(Default)]
pub struct InMemoryPaymentStore {
    quotes: RwLock<HashMap<QuoteId, QuoteResponse>>,
    quote_session_index: RwLock<BTreeMap<String, Vec<QuoteId>>>,
    quote_task_index: RwLock<BTreeMap<String, Vec<QuoteId>>>,
    transitions: RwLock<BTreeMap<PaymentIntentId, Vec<PaymentStateTransition>>>,
    receipts: RwLock<HashMap<PaymentIntentId, PaymentReceipt>>,
    receipt_session_index: RwLock<BTreeMap<String, Vec<PaymentIntentId>>>,
    receipt_task_index: RwLock<BTreeMap<String, Vec<PaymentIntentId>>>,
    proofs: RwLock<BTreeMap<PaymentIntentId, Vec<ExecutionProof>>>,
}

impl InMemoryPaymentStore {
    /// Create an empty payment store.
    pub fn new() -> Self {
        Self::default()
    }

    fn push_unique<T: Eq + Clone>(list: &mut Vec<T>, value: T) {
        if !list.contains(&value) {
            list.push(value);
        }
    }
}

#[async_trait]
impl PaymentStore for InMemoryPaymentStore {
    async fn put_quote(&self, quote: QuoteResponse) -> Result<(), A2AError> {
        info!(
            quote_id = %quote.quote_id,
            requester_id = %quote.request.requester.id,
            provider_id = %quote.request.provider.id,
            capability_id = %quote.request.capability.capability_id,
            "payment quote snapshot persisted"
        );

        if let Some(session_id) = &quote.request.session_id {
            let mut index = self.quote_session_index.write().await;
            Self::push_unique(
                index.entry(session_id.clone()).or_default(),
                quote.quote_id.clone(),
            );
        }
        if let Some(task_id) = &quote.request.task_id {
            let mut index = self.quote_task_index.write().await;
            Self::push_unique(
                index.entry(task_id.clone()).or_default(),
                quote.quote_id.clone(),
            );
        }
        self.quotes
            .write()
            .await
            .insert(quote.quote_id.clone(), quote);
        Ok(())
    }

    async fn get_quote(&self, quote_id: &QuoteId) -> Result<Option<QuoteResponse>, A2AError> {
        Ok(self.quotes.read().await.get(quote_id).cloned())
    }

    async fn append_transition(&self, transition: PaymentStateTransition) -> Result<(), A2AError> {
        info!(
            intent_id = %transition.intent_id,
            to = %transition.to,
            operation = %transition.operation,
            status = %transition.status,
            "payment state transition appended"
        );
        self.transitions
            .write()
            .await
            .entry(transition.intent_id.clone())
            .or_default()
            .push(transition);
        Ok(())
    }

    async fn transitions_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Vec<PaymentStateTransition>, A2AError> {
        Ok(self
            .transitions
            .read()
            .await
            .get(intent_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn put_receipt(&self, receipt: PaymentReceipt) -> Result<(), A2AError> {
        info!(
            receipt_id = %receipt.receipt_id,
            intent_id = %receipt.intent_id,
            status = %receipt.status,
            "payment receipt persisted"
        );
        if let Some(session_id) = &receipt.session_id {
            let mut index = self.receipt_session_index.write().await;
            Self::push_unique(
                index.entry(session_id.clone()).or_default(),
                receipt.intent_id.clone(),
            );
        }
        if let Some(task_id) = &receipt.task_id {
            let mut index = self.receipt_task_index.write().await;
            Self::push_unique(
                index.entry(task_id.clone()).or_default(),
                receipt.intent_id.clone(),
            );
        }
        self.receipts
            .write()
            .await
            .insert(receipt.intent_id.clone(), receipt);
        Ok(())
    }

    async fn receipts_by_session(&self, session_id: &str) -> Result<Vec<PaymentReceipt>, A2AError> {
        let ids = self
            .receipt_session_index
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        let receipts = self.receipts.read().await;
        Ok(ids
            .iter()
            .filter_map(|intent_id| receipts.get(intent_id).cloned())
            .collect())
    }

    async fn receipts_by_task(&self, task_id: &str) -> Result<Vec<PaymentReceipt>, A2AError> {
        let ids = self
            .receipt_task_index
            .read()
            .await
            .get(task_id)
            .cloned()
            .unwrap_or_default();
        let receipts = self.receipts.read().await;
        Ok(ids
            .iter()
            .filter_map(|intent_id| receipts.get(intent_id).cloned())
            .collect())
    }

    async fn receipt_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Option<PaymentReceipt>, A2AError> {
        Ok(self.receipts.read().await.get(intent_id).cloned())
    }

    async fn put_execution_proof(&self, proof: ExecutionProof) -> Result<(), A2AError> {
        info!(
            proof_id = %proof.proof_id,
            intent_id = %proof.intent_id,
            status = %proof.status,
            "payment execution proof persisted"
        );
        self.proofs
            .write()
            .await
            .entry(proof.intent_id.clone())
            .or_default()
            .push(proof);
        Ok(())
    }

    async fn execution_proofs_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Vec<ExecutionProof>, A2AError> {
        Ok(self
            .proofs
            .read()
            .await
            .get(intent_id)
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        AgentIdentity, CapabilityId, PaymentAmount, PaymentAsset, PaymentIntentState, PaymentRail,
        PaymentReceiptId, PaymentTerms, QuoteRequest, RemoteCapabilityDescriptor,
    };

    fn quote() -> QuoteResponse {
        let provider = AgentIdentity::new("provider");
        QuoteResponse {
            quote_id: QuoteId::new("quote.store"),
            request: QuoteRequest {
                requester: AgentIdentity::new("requester"),
                provider: provider.clone(),
                capability: RemoteCapabilityDescriptor {
                    capability_id: CapabilityId::new("cap.store"),
                    provider,
                    operation: "quote".into(),
                    description: None,
                    metadata: BTreeMap::new(),
                },
                operation: "execute".into(),
                session_id: Some("session.store".into()),
                task_id: Some("task.store".into()),
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
            terms: PaymentTerms {
                amount: PaymentAmount::new(
                    "1",
                    PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
                ),
                billing_unit: "call".into(),
                expires_at: None,
                requires_explicit_approval: false,
                metadata: BTreeMap::new(),
            },
            quoted_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    fn receipt(intent_id: PaymentIntentId) -> PaymentReceipt {
        let quote = quote();
        PaymentReceipt {
            receipt_id: PaymentReceiptId::new("receipt.store"),
            quote_id: quote.quote_id,
            intent_id,
            requester: quote.request.requester,
            provider: quote.request.provider,
            capability_id: quote.request.capability.capability_id,
            amount: quote.terms.amount,
            status: "settled".into(),
            session_id: Some("session.store".into()),
            task_id: Some("task.store".into()),
            issued_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn quote_write_and_read_work() {
        let store = InMemoryPaymentStore::new();
        let quote = quote();
        store.put_quote(quote.clone()).await.unwrap();
        assert_eq!(store.get_quote(&quote.quote_id).await.unwrap(), Some(quote));
    }

    #[tokio::test]
    async fn state_transitions_are_ordered() {
        let store = InMemoryPaymentStore::new();
        let intent_id = PaymentIntentId::new("intent.transitions");
        store
            .append_transition(PaymentStateTransition::new(
                intent_id.clone(),
                None,
                PaymentIntentState::created(),
                "create_intent",
                "ok",
            ))
            .await
            .unwrap();
        store
            .append_transition(PaymentStateTransition::new(
                intent_id.clone(),
                Some(PaymentIntentState::created()),
                PaymentIntentState::quoted(),
                "quote_intent",
                "ok",
            ))
            .await
            .unwrap();
        let transitions = store.transitions_by_intent(&intent_id).await.unwrap();
        assert_eq!(transitions[0].to, PaymentIntentState::created());
        assert_eq!(transitions[1].to, PaymentIntentState::quoted());
    }

    #[tokio::test]
    async fn receipt_and_execution_proof_queries_work() {
        let store = InMemoryPaymentStore::new();
        let intent_id = PaymentIntentId::new("intent.receipt");
        store.put_receipt(receipt(intent_id.clone())).await.unwrap();
        assert_eq!(
            store
                .receipts_by_session("session.store")
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(store.receipts_by_task("task.store").await.unwrap().len(), 1);

        let proof = ExecutionProof {
            proof_id: macaca_proto::ExecutionProofId::new("proof.store"),
            intent_id: intent_id.clone(),
            receipt_id: None,
            operation: "simulate".into(),
            status: "ok".into(),
            recorded_at: Utc::now(),
            metadata: BTreeMap::new(),
        };
        store.put_execution_proof(proof).await.unwrap();
        assert_eq!(
            store
                .execution_proofs_by_intent(&intent_id)
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
