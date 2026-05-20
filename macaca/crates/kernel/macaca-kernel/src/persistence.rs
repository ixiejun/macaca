//! Provider-neutral persistence ports owned by the kernel.
//!
//! The kernel must describe *what* durable behavior it needs without naming
//! any concrete database crate.  These ports use the Repository pattern for
//! storage, the Memento pattern for replayable state snapshots, and the Null
//! Object pattern for callers that intentionally run without durability.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{
    A2AError, ExecutionProof, MacacaResult, PaymentIntentId, PaymentIntentState, PaymentReceipt,
    QuoteId, QuoteResponse,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Generic key-value repository used by kernel audit and execution recovery.
///
/// The key namespace is deliberately string-based because audit, queues, and
/// fork recovery already own their stable prefixes.  Concrete stores remain
/// outside the kernel and only need to implement these four primitive
/// operations.  This keeps persistence replaceable while preserving traceable
/// kernel recovery semantics.
#[async_trait]
pub trait KernelPersistencePort: Send + Sync {
    /// Load one binary memento by key.
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>>;

    /// Upsert one binary memento by key.
    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()>;

    /// Delete one memento by key.  Missing keys are expected to be harmless.
    async fn delete(&self, key: &str) -> MacacaResult<()>;

    /// List keys under one prefix so restart recovery can replay stored items.
    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>>;

    /// Human-readable backend label for diagnostics and audit logs.
    fn backend_name(&self) -> &'static str {
        "kernel-persistence-port"
    }
}

/// Null Object persistence port for intentionally non-durable construction.
///
/// Reads return empty data and writes are acknowledged with a warning.  This is
/// useful for service-client-only or test paths that do not need persistence,
/// while still making the missing durable backend visible in logs.
#[derive(Debug, Default)]
pub struct UnavailableKernelPersistencePort;

#[async_trait]
impl KernelPersistencePort for UnavailableKernelPersistencePort {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        debug!(
            key,
            "kernel persistence get skipped because no durable backend is configured"
        );
        Ok(None)
    }

    async fn set(&self, key: &str, _value: &[u8]) -> MacacaResult<()> {
        warn!(
            key,
            "kernel persistence set skipped because no durable backend is configured"
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> MacacaResult<()> {
        debug!(
            key,
            "kernel persistence delete skipped because no durable backend is configured"
        );
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        debug!(
            prefix,
            "kernel persistence list skipped because no durable backend is configured"
        );
        Ok(Vec::new())
    }

    fn backend_name(&self) -> &'static str {
        "unavailable-kernel-persistence"
    }
}

/// Ordered A2A payment transition stored for audit and replay.
///
/// The deprecated kernel A2A coordinator only needs a provider-neutral memento
/// describing the lifecycle transition.  Payment service implementations can
/// persist the same shape without the kernel depending on a concrete payment
/// store crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelPaymentStateTransition {
    pub intent_id: PaymentIntentId,
    pub from: Option<PaymentIntentState>,
    pub to: PaymentIntentState,
    pub operation: String,
    pub status: String,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl KernelPaymentStateTransition {
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

/// Repository abstraction for deprecated kernel A2A payment artifacts.
///
/// New payment flows should use payment services.  This port only preserves the
/// old coordinator contract during migration and keeps quote, transition,
/// receipt, and proof persistence provider-neutral.
#[async_trait]
#[deprecated(
    note = "Use PaymentSystemServiceProvider plus SystemPaymentClient for new payment flows"
)]
pub trait KernelPaymentStorePort: Send + Sync {
    /// Persist a quote snapshot and update lookup indexes when present.
    async fn put_quote(&self, quote: QuoteResponse) -> Result<(), A2AError>;

    /// Read a quote snapshot by quote id.
    async fn get_quote(&self, quote_id: &QuoteId) -> Result<Option<QuoteResponse>, A2AError>;

    /// Append one intent transition in chronological order.
    async fn append_transition(
        &self,
        transition: KernelPaymentStateTransition,
    ) -> Result<(), A2AError>;

    /// Return all transitions for one intent in append order.
    async fn transitions_by_intent(
        &self,
        intent_id: &PaymentIntentId,
    ) -> Result<Vec<KernelPaymentStateTransition>, A2AError>;

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

/// Deterministic in-memory payment store for deprecated kernel A2A tests.
///
/// The indexes mirror the durable repository contract, which prevents tests
/// from relying on whole-application scans and keeps migration behavior close
/// to the eventual service-owned implementation.
#[derive(Default)]
#[deprecated(note = "Use a service-owned payment store for production payment flows")]
pub struct InMemoryKernelPaymentStore {
    quotes: RwLock<HashMap<QuoteId, QuoteResponse>>,
    transitions: RwLock<BTreeMap<PaymentIntentId, Vec<KernelPaymentStateTransition>>>,
    receipts: RwLock<HashMap<PaymentIntentId, PaymentReceipt>>,
    receipt_session_index: RwLock<BTreeMap<String, Vec<PaymentIntentId>>>,
    receipt_task_index: RwLock<BTreeMap<String, Vec<PaymentIntentId>>>,
    proofs: RwLock<BTreeMap<PaymentIntentId, Vec<ExecutionProof>>>,
}

#[allow(deprecated)]
impl InMemoryKernelPaymentStore {
    /// Create an empty in-memory repository.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a value only once so repeated writes stay idempotent for callers.
    fn push_unique<T: Eq + Clone>(list: &mut Vec<T>, value: T) {
        if !list.contains(&value) {
            list.push(value);
        }
    }
}

#[async_trait]
#[allow(deprecated)]
impl KernelPaymentStorePort for InMemoryKernelPaymentStore {
    async fn put_quote(&self, quote: QuoteResponse) -> Result<(), A2AError> {
        info!(
            quote_id = %quote.quote_id,
            requester_id = %quote.request.requester.id,
            provider_id = %quote.request.provider.id,
            capability_id = %quote.request.capability.capability_id,
            "deprecated kernel A2A quote snapshot persisted"
        );
        self.quotes
            .write()
            .await
            .insert(quote.quote_id.clone(), quote);
        Ok(())
    }

    async fn get_quote(&self, quote_id: &QuoteId) -> Result<Option<QuoteResponse>, A2AError> {
        Ok(self.quotes.read().await.get(quote_id).cloned())
    }

    async fn append_transition(
        &self,
        transition: KernelPaymentStateTransition,
    ) -> Result<(), A2AError> {
        info!(
            intent_id = %transition.intent_id,
            to = %transition.to,
            operation = %transition.operation,
            status = %transition.status,
            "deprecated kernel A2A state transition appended"
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
    ) -> Result<Vec<KernelPaymentStateTransition>, A2AError> {
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
            "deprecated kernel A2A receipt persisted"
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
            "deprecated kernel A2A execution proof persisted"
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
