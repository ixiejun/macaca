//! Runtime-host provider for the Payment Service.
//!
//! The provider is a Mediator over provider-neutral A2A contracts, a payment
//! adapter Strategy, a provider-neutral payment policy Strategy, payment-store
//! Mementos, and trace/audit-friendly logs. It intentionally does not own
//! Store, Entitlement, optional chain modules, wallet, gateway, or application orchestration
//! semantics. Future payment providers can replace the adapter without
//! changing the service contract.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_persist::{PaymentStateTransition, PaymentStore};
mod descriptor;
mod settlement;
mod support;

pub use descriptor::payment_service_descriptor;
use support::{a2a_error, decode, result, trace};

use macaca_proto::{
    PaymentIntent, PaymentIntentApproveCommand, PaymentIntentCreateCommand, PaymentIntentId,
    PaymentIntentSettleCommand, PaymentIntentState, PaymentPolicyDecisionView,
    PaymentPolicyEvaluateCommand, PaymentProofListCommand, PaymentQuoteCommand,
    PaymentReceiptListScope, PaymentServiceSnapshot, PaymentSnapshotCommand, PaymentTerms,
    PaymentTransitionListCommand, PaymentTransitionView, QuoteResponse, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    PAYMENT_INTENT_APPROVE_COMMAND, PAYMENT_INTENT_CREATE_COMMAND, PAYMENT_INTENT_SETTLE_COMMAND,
    PAYMENT_POLICY_EVALUATE_COMMAND, PAYMENT_PROOF_LIST_COMMAND, PAYMENT_QUOTE_COMMAND,
    PAYMENT_RECEIPT_GET_COMMAND, PAYMENT_RECEIPT_LIST_COMMAND, PAYMENT_SERVICE_ID,
    PAYMENT_SNAPSHOT_COMMAND, PAYMENT_TRANSITION_LIST_COMMAND,
};
use tracing::info;

use crate::payment_admission::{
    PaymentAmountSpec, PaymentRedactionSpec, PaymentScopeSpec, PaymentTraceSpec,
};
use crate::payment_policy::{DefaultPaymentPolicyEngine, PaymentPolicyEngine};
use crate::{LocalSimulatedPaymentAdapter, PaymentAdapterStrategy};

/// Payment Service provider hosted by `ServiceRuntime`.
pub struct PaymentSystemServiceProvider {
    descriptor: ServiceDescriptor,
    adapter: Arc<dyn PaymentAdapterStrategy>,
    policy: Arc<dyn PaymentPolicyEngine>,
    store: Arc<dyn PaymentStore>,
}

impl PaymentSystemServiceProvider {
    /// Create a provider from explicit strategies.
    pub fn new(
        adapter: Arc<dyn PaymentAdapterStrategy>,
        policy: Arc<dyn PaymentPolicyEngine>,
        store: Arc<dyn PaymentStore>,
    ) -> Self {
        Self {
            descriptor: payment_service_descriptor(),
            adapter,
            policy,
            store,
        }
    }

    /// Create the built-in local simulated provider used by local hosts.
    pub fn local_simulated(store: Arc<dyn PaymentStore>, terms: PaymentTerms) -> Self {
        Self::new(
            Arc::new(LocalSimulatedPaymentAdapter::new(terms)),
            Arc::new(DefaultPaymentPolicyEngine::new()),
            store,
        )
    }

    /// Build the canonical intent skeleton used by service commands.
    pub fn intent_from_quote(quote: &QuoteResponse) -> PaymentIntent {
        let now = Utc::now();
        PaymentIntent {
            intent_id: PaymentIntentId::new(format!(
                "intent.{}",
                Utc::now().timestamp_nanos_opt().unwrap_or_default()
            )),
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
}

#[async_trait]
impl SystemService for PaymentSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            adapter_configured = self.adapter.is_configured(),
            "payment service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "payment service command accepted"
        );
        match command.name.as_str() {
            PAYMENT_QUOTE_COMMAND => {
                let typed: PaymentQuoteCommand = decode(command.payload)?;
                PaymentTraceSpec::check(&typed.trace)?;
                PaymentScopeSpec::check_quote(&typed.request)?;
                PaymentRedactionSpec::check_metadata(&typed.request.metadata)?;
                let quote = self.adapter.quote(typed.request).await.map_err(a2a_error)?;
                self.store
                    .put_quote(quote.clone())
                    .await
                    .map_err(a2a_error)?;
                result(quote, typed.trace)
            }
            PAYMENT_INTENT_CREATE_COMMAND => {
                let typed: PaymentIntentCreateCommand = decode(command.payload)?;
                let intent = Self::intent_from_quote(&typed.quote);
                self.store
                    .append_transition(PaymentStateTransition::new(
                        intent.intent_id.clone(),
                        None,
                        intent.state.clone(),
                        "create_intent",
                        "ok",
                    ))
                    .await
                    .map_err(a2a_error)?;
                result(intent, typed.trace)
            }
            PAYMENT_POLICY_EVALUATE_COMMAND => {
                let typed: PaymentPolicyEvaluateCommand = decode(command.payload)?;
                PaymentAmountSpec::check_intent(&typed.intent)?;
                let decision = self
                    .policy
                    .evaluate(
                        &typed.intent,
                        &typed.budget,
                        &typed.approval,
                        self.adapter.is_configured(),
                    )
                    .map(|decision| {
                        if decision.allowed {
                            PaymentPolicyDecisionView::allowed(
                                typed.trace.clone(),
                                typed.intent.intent_id.to_string(),
                                decision.reason,
                            )
                        } else {
                            PaymentPolicyDecisionView::denied(
                                typed.trace.clone(),
                                typed.intent.intent_id.to_string(),
                                decision.reason,
                            )
                        }
                    })
                    .map_err(a2a_error)?;
                result(decision, typed.trace)
            }
            PAYMENT_INTENT_APPROVE_COMMAND => {
                let typed: PaymentIntentApproveCommand = decode(command.payload)?;
                let mut intent = typed.intent;
                let _event = self
                    .transition(
                        &mut intent,
                        PaymentIntentState::approved(),
                        "approve_intent",
                        &typed.trace,
                    )
                    .await?;
                result(intent, typed.trace)
            }
            PAYMENT_INTENT_SETTLE_COMMAND => {
                let typed: PaymentIntentSettleCommand = decode(command.payload)?;
                let settlement = self.settle_intent(typed.intent, &typed.trace).await?;
                result(settlement, typed.trace)
            }
            PAYMENT_RECEIPT_GET_COMMAND => {
                let typed: macaca_proto::PaymentReceiptGetCommand = decode(command.payload)?;
                let receipt = self
                    .store
                    .receipt_by_intent(&typed.intent_id)
                    .await
                    .map_err(a2a_error)?;
                result(receipt, typed.trace)
            }
            PAYMENT_RECEIPT_LIST_COMMAND => {
                let typed: macaca_proto::PaymentReceiptListCommand = decode(command.payload)?;
                let receipts = match typed.scope {
                    PaymentReceiptListScope::Session(session_id) => self
                        .store
                        .receipts_by_session(&session_id)
                        .await
                        .map_err(a2a_error)?,
                    PaymentReceiptListScope::Task(task_id) => self
                        .store
                        .receipts_by_task(&task_id)
                        .await
                        .map_err(a2a_error)?,
                };
                result(receipts, typed.trace)
            }
            PAYMENT_TRANSITION_LIST_COMMAND => {
                let typed: PaymentTransitionListCommand = decode(command.payload)?;
                let views: Vec<PaymentTransitionView> = self
                    .store
                    .transitions_by_intent(&typed.intent_id)
                    .await
                    .map_err(a2a_error)?
                    .into_iter()
                    .map(|item| PaymentTransitionView {
                        intent_id: item.intent_id,
                        from: item.from,
                        to: item.to,
                        operation: item.operation,
                        status: item.status,
                        reason: item.reason,
                        timestamp: item.timestamp,
                        metadata: item.metadata,
                    })
                    .collect();
                result(views, typed.trace)
            }
            PAYMENT_PROOF_LIST_COMMAND => {
                let typed: PaymentProofListCommand = decode(command.payload)?;
                let proofs = self
                    .store
                    .execution_proofs_by_intent(&typed.intent_id)
                    .await
                    .map_err(a2a_error)?;
                result(proofs, typed.trace)
            }
            PAYMENT_SNAPSHOT_COMMAND => {
                let typed: PaymentSnapshotCommand = decode(command.payload)?;
                result(
                    PaymentServiceSnapshot {
                        service_id: PAYMENT_SERVICE_ID.into(),
                        health: "healthy".into(),
                        adapter_configured: self.adapter.is_configured(),
                        quote_count: 0,
                        receipt_count: 0,
                        proof_count: 0,
                        diagnostics: Vec::new(),
                        captured_at: Utc::now(),
                        metadata: BTreeMap::new(),
                    },
                    typed.trace,
                )
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Payment service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "payment service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "payment service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}
