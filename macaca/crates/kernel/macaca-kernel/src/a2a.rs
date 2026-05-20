//! Kernel-level A2A payment coordinator facade for Route C Phase 09.
//!
//! The coordinator is a Facade/Mediator over protocol contracts, payment
//! policy, adapter strategy, event observers, and payment persistence. It does not implement a
//! real payment provider. The default adapter is a deterministic local
//! simulator that exercises the same state, audit, and receipt path required
//! by future replaceable payment services.

#![allow(deprecated)]

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    A2AError, ApprovalPolicy, BudgetPolicy, ExecutionProof, ExecutionProofId, PaymentIntent,
    PaymentIntentId, PaymentIntentState, PaymentRail, PaymentReceipt, PaymentReceiptId,
    PaymentTerms, QuoteId, QuoteRequest, QuoteResponse,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::a2a_event::{A2APaymentEvent, A2APaymentEventSink, NoopA2APaymentEventSink};
use crate::payment_policy::{DefaultPaymentPolicyEngine, PaymentPolicyEngine};
use crate::persistence::{KernelPaymentStateTransition, KernelPaymentStorePort};

/// Strategy boundary for A2A/payment adapters.
///
/// Real adapters introduced in later phases must implement this trait and
/// remain behind policy evaluation. Phase 09 ships only local simulation so
/// tests can run without network, wallet, chain, or billing provider access.
#[async_trait]
#[deprecated(
    note = "Use PaymentSystemServiceProvider plus SystemPaymentClient for new Payment/A2A call paths"
)]
pub trait A2AProtocolAdapter: Send + Sync {
    /// Return whether this adapter is configured for execution.
    fn is_configured(&self) -> bool;

    /// Create a provider-neutral quote for one request.
    async fn quote(&self, request: QuoteRequest) -> Result<QuoteResponse, A2AError>;

    /// Execute an approved intent and return a receipt plus execution proof.
    async fn execute(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(PaymentReceipt, ExecutionProof), A2AError>;
}

/// Deterministic local adapter used to validate the A2A payment pipeline.
///
/// The adapter produces synthetic quote/receipt/proof identifiers and never
/// performs network I/O. It is still policy-gated by the coordinator so future
/// real adapters can reuse the same execution contract.
#[deprecated(
    note = "Use LocalSimulatedPaymentAdapter behind PaymentSystemServiceProvider for new Payment/A2A call paths"
)]
pub struct LocalSimulatedA2AAdapter {
    terms: PaymentTerms,
}

impl LocalSimulatedA2AAdapter {
    /// Create a local adapter with explicit terms supplied by tests/runtime.
    pub fn new(terms: PaymentTerms) -> Self {
        Self { terms }
    }

    fn synthetic_id(prefix: &str) -> String {
        format!("{prefix}.{}", Uuid::new_v4())
    }
}

#[async_trait]
#[allow(deprecated)]
impl A2AProtocolAdapter for LocalSimulatedA2AAdapter {
    fn is_configured(&self) -> bool {
        true
    }

    async fn quote(&self, request: QuoteRequest) -> Result<QuoteResponse, A2AError> {
        info!(
            requester_id = %request.requester.id,
            provider_id = %request.provider.id,
            capability_id = %request.capability.capability_id,
            "local simulated A2A quote generated"
        );
        Ok(QuoteResponse {
            quote_id: QuoteId::new(Self::synthetic_id("quote")),
            request,
            terms: self.terms.clone(),
            quoted_at: Utc::now(),
            metadata: BTreeMap::from([("simulation".into(), "true".into())]),
        })
    }

    async fn execute(
        &self,
        intent: &PaymentIntent,
    ) -> Result<(PaymentReceipt, ExecutionProof), A2AError> {
        info!(
            intent_id = %intent.intent_id,
            quote_id = %intent.quote_id,
            rail = %intent.amount.asset.rail,
            "local simulated A2A execution started"
        );
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
            operation: "local_simulated_execution".into(),
            status: "ok".into(),
            recorded_at: Utc::now(),
            metadata: BTreeMap::from([("simulation".into(), "true".into())]),
        };
        Ok((receipt, proof))
    }
}

/// Runtime-facing facade for quote, intent, policy, execution, and receipt.
#[async_trait]
#[deprecated(note = "Use SystemPaymentClient for new Payment/A2A call paths")]
pub trait A2APaymentFacade: Send + Sync {
    /// Execute the full Phase 09 local pipeline and return a receipt.
    async fn request_and_pay(
        &self,
        request: QuoteRequest,
        budget: BudgetPolicy,
        approval: ApprovalPolicy,
    ) -> Result<PaymentReceipt, A2AError>;
}

/// Concrete coordinator that composes adapter, policy, and persistence.
#[deprecated(
    note = "Use PaymentSystemServiceProvider plus SystemPaymentClient for new Payment/A2A call paths"
)]
pub struct A2ACoordinator {
    adapter: Arc<dyn A2AProtocolAdapter>,
    policy: Arc<dyn PaymentPolicyEngine>,
    store: Arc<dyn KernelPaymentStorePort>,
    event_sink: Arc<dyn A2APaymentEventSink>,
}

#[allow(deprecated)]
impl A2ACoordinator {
    /// Create a coordinator with explicit strategy dependencies.
    pub fn new(
        adapter: Arc<dyn A2AProtocolAdapter>,
        policy: Arc<dyn PaymentPolicyEngine>,
        store: Arc<dyn KernelPaymentStorePort>,
    ) -> Self {
        info!("deprecated kernel A2A coordinator initialized with provider-neutral store port");
        Self {
            adapter,
            policy,
            store,
            event_sink: Arc::new(NoopA2APaymentEventSink),
        }
    }

    /// Create a coordinator using the default conservative policy.
    pub fn with_default_policy(
        adapter: Arc<dyn A2AProtocolAdapter>,
        store: Arc<dyn KernelPaymentStorePort>,
    ) -> Self {
        Self::new(adapter, Arc::new(DefaultPaymentPolicyEngine::new()), store)
    }

    /// Replace the default no-op event sink with a trace/audit observer.
    pub fn with_event_sink(mut self, event_sink: Arc<dyn A2APaymentEventSink>) -> Self {
        self.event_sink = event_sink;
        self
    }

    fn new_intent_from_quote(quote: &QuoteResponse) -> PaymentIntent {
        let now = Utc::now();
        PaymentIntent {
            intent_id: PaymentIntentId::new(format!("intent.{}", Uuid::new_v4())),
            quote_id: quote.quote_id.clone(),
            requester: quote.request.requester.clone(),
            provider: quote.request.provider.clone(),
            capability_id: quote.request.capability.capability_id.clone(),
            amount: quote.terms.amount.clone(),
            state: PaymentIntentState::created(),
            session_id: quote.request.session_id.clone(),
            task_id: quote.request.task_id.clone(),
            created_at: now,
            updated_at: now,
            metadata: BTreeMap::new(),
        }
    }

    async fn transition(
        &self,
        intent: &mut PaymentIntent,
        next: PaymentIntentState,
        operation: &str,
    ) -> Result<(), A2AError> {
        if !intent.state.can_transition_to(&next) {
            warn!(
                intent_id = %intent.intent_id,
                from = %intent.state,
                to = %next,
                "invalid payment state transition rejected"
            );
            return Err(A2AError::InvalidTransition {
                from: intent.state.to_string(),
                to: next.to_string(),
            });
        }
        let previous = intent.state.clone();
        intent.state = next.clone();
        intent.updated_at = Utc::now();
        self.store
            .append_transition(KernelPaymentStateTransition::new(
                intent.intent_id.clone(),
                Some(previous),
                next.clone(),
                operation,
                "ok",
            ))
            .await?;
        info!(
            intent_id = %intent.intent_id,
            state = %next,
            operation,
            "payment intent transitioned"
        );
        self.event_sink
            .emit(Self::event_from_intent(intent, operation, next.as_str()))
            .await?;
        Ok(())
    }

    fn event_from_intent(intent: &PaymentIntent, operation: &str, status: &str) -> A2APaymentEvent {
        A2APaymentEvent {
            requester_id: intent.requester.id.to_string(),
            provider_id: intent.provider.id.to_string(),
            capability_id: intent.capability_id.to_string(),
            quote_id: Some(intent.quote_id.to_string()),
            intent_id: Some(intent.intent_id.to_string()),
            amount: Some(intent.amount.quantity.clone()),
            asset: Some(intent.amount.asset.code.clone()),
            operation: operation.into(),
            status: status.into(),
            session_id: intent.session_id.clone(),
            task_id: intent.task_id.clone(),
            timestamp: Utc::now(),
            error_code: None,
            metadata: std::collections::BTreeMap::new(),
        }
    }
}

#[async_trait]
#[allow(deprecated)]
impl A2APaymentFacade for A2ACoordinator {
    async fn request_and_pay(
        &self,
        request: QuoteRequest,
        budget: BudgetPolicy,
        approval: ApprovalPolicy,
    ) -> Result<PaymentReceipt, A2AError> {
        info!(
            requester_id = %request.requester.id,
            provider_id = %request.provider.id,
            capability_id = %request.capability.capability_id,
            "A2A payment pipeline started"
        );

        let quote = self.adapter.quote(request).await?;
        self.event_sink
            .emit(A2APaymentEvent {
                requester_id: quote.request.requester.id.to_string(),
                provider_id: quote.request.provider.id.to_string(),
                capability_id: quote.request.capability.capability_id.to_string(),
                quote_id: Some(quote.quote_id.to_string()),
                intent_id: None,
                amount: Some(quote.terms.amount.quantity.clone()),
                asset: Some(quote.terms.amount.asset.code.clone()),
                operation: "quote".into(),
                status: "ok".into(),
                session_id: quote.request.session_id.clone(),
                task_id: quote.request.task_id.clone(),
                timestamp: Utc::now(),
                error_code: None,
                metadata: std::collections::BTreeMap::new(),
            })
            .await?;
        self.store.put_quote(quote.clone()).await?;
        let mut intent = Self::new_intent_from_quote(&quote);
        self.store
            .append_transition(KernelPaymentStateTransition::new(
                intent.intent_id.clone(),
                None,
                PaymentIntentState::created(),
                "create_intent",
                "ok",
            ))
            .await?;
        self.transition(&mut intent, PaymentIntentState::quoted(), "quote_intent")
            .await?;
        self.transition(
            &mut intent,
            PaymentIntentState::pending_approval(),
            "evaluate_policy",
        )
        .await?;

        self.policy
            .evaluate(&intent, &budget, &approval, self.adapter.is_configured())?;
        self.transition(
            &mut intent,
            PaymentIntentState::approved(),
            "approve_intent",
        )
        .await?;
        self.transition(
            &mut intent,
            PaymentIntentState::executing(),
            "execute_adapter",
        )
        .await?;

        let (receipt, proof) = self.adapter.execute(&intent).await.map_err(|error| {
            warn!(
                intent_id = %intent.intent_id,
                error = %error,
                "A2A payment adapter execution failed"
            );
            error
        })?;
        self.transition(&mut intent, PaymentIntentState::settled(), "settle_intent")
            .await?;
        self.store.put_execution_proof(proof).await?;
        self.store.put_receipt(receipt.clone()).await?;
        self.transition(
            &mut intent,
            PaymentIntentState::receipt_recorded(),
            "record_receipt",
        )
        .await?;

        let event = Self::event_from_intent(&intent, "record_receipt", "ok");
        info!(
            requester_id = %event.requester_id,
            provider_id = %event.provider_id,
            capability_id = %event.capability_id,
            intent_id = ?event.intent_id,
            quote_id = ?event.quote_id,
            session_id = ?event.session_id,
            task_id = ?event.task_id,
            "A2A payment pipeline completed"
        );
        Ok(receipt)
    }
}

/// Build local simulated terms used by tests and no-network scaffolds.
pub fn local_simulated_terms(
    quantity: impl Into<String>,
    asset_code: impl Into<String>,
) -> PaymentTerms {
    PaymentTerms {
        amount: macaca_proto::PaymentAmount::new(
            quantity,
            macaca_proto::PaymentAsset::new(PaymentRail::local_simulated(), asset_code),
        ),
        billing_unit: "operation".into(),
        expires_at: None,
        requires_explicit_approval: false,
        metadata: std::collections::BTreeMap::from([("simulation".into(), "true".into())]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a_event::InMemoryA2APaymentEventSink;
    use crate::persistence::InMemoryKernelPaymentStore;
    use macaca_proto::{
        AgentIdentity, CapabilityId, PaymentAmount, PaymentAsset, RemoteCapabilityDescriptor,
    };

    fn request() -> QuoteRequest {
        let provider = AgentIdentity::new("provider");
        QuoteRequest {
            requester: AgentIdentity::new("requester"),
            provider: provider.clone(),
            capability: RemoteCapabilityDescriptor {
                capability_id: CapabilityId::new("cap.a2a"),
                provider,
                operation: "quote".into(),
                description: None,
                metadata: BTreeMap::new(),
            },
            operation: "execute".into(),
            session_id: Some("session.a2a".into()),
            task_id: Some("task.a2a".into()),
            requested_at: Utc::now(),
            metadata: std::collections::BTreeMap::new(),
        }
    }

    fn budget() -> BudgetPolicy {
        BudgetPolicy {
            max_amount: Some(PaymentAmount::new(
                "2",
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            auto_approve_local_simulated_below: Some(PaymentAmount::new(
                "2",
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            metadata: std::collections::BTreeMap::new(),
        }
    }

    fn approval() -> ApprovalPolicy {
        ApprovalPolicy {
            explicit_approval_required: false,
            explicit_approval_granted: false,
            approver: None,
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn a2a_payment_local_simulation_produces_receipt() {
        let store = Arc::new(InMemoryKernelPaymentStore::new());
        let coordinator = A2ACoordinator::with_default_policy(
            Arc::new(LocalSimulatedA2AAdapter::new(local_simulated_terms(
                "1", "UNIT",
            ))),
            store.clone(),
        );
        let receipt = coordinator
            .request_and_pay(request(), budget(), approval())
            .await
            .unwrap();
        assert_eq!(receipt.status, "settled");
        assert_eq!(
            store
                .receipt_by_intent(&receipt.intent_id)
                .await
                .unwrap()
                .unwrap()
                .receipt_id,
            receipt.receipt_id
        );
    }

    #[tokio::test]
    async fn a2a_payment_events_are_trace_audit_compatible() {
        let store = Arc::new(InMemoryKernelPaymentStore::new());
        let sink = Arc::new(InMemoryA2APaymentEventSink::new());
        let coordinator = A2ACoordinator::with_default_policy(
            Arc::new(LocalSimulatedA2AAdapter::new(local_simulated_terms(
                "1", "UNIT",
            ))),
            store,
        )
        .with_event_sink(sink.clone());

        let receipt = coordinator
            .request_and_pay(request(), budget(), approval())
            .await
            .unwrap();
        let events = sink.events().await;
        assert!(events.iter().any(|event| event.operation == "quote"));
        assert!(events
            .iter()
            .any(|event| event.operation == "record_receipt"
                && event.intent_id.as_deref() == Some(receipt.intent_id.as_str())
                && event.session_id.as_deref() == Some("session.a2a")
                && event.task_id.as_deref() == Some("task.a2a")));
        assert!(events.iter().all(|event| event.error_code.is_none()));
    }

    #[tokio::test]
    async fn invalid_transition_is_rejected_without_mutating_state() {
        let store = Arc::new(InMemoryKernelPaymentStore::new());
        let coordinator = A2ACoordinator::with_default_policy(
            Arc::new(LocalSimulatedA2AAdapter::new(local_simulated_terms(
                "1", "UNIT",
            ))),
            store,
        );
        let quote = coordinator.adapter.quote(request()).await.unwrap();
        let mut intent = A2ACoordinator::new_intent_from_quote(&quote);
        let result = coordinator
            .transition(
                &mut intent,
                PaymentIntentState::settled(),
                "invalid_direct_settle",
            )
            .await;
        assert!(matches!(result, Err(A2AError::InvalidTransition { .. })));
        assert_eq!(intent.state, PaymentIntentState::created());
    }
}
