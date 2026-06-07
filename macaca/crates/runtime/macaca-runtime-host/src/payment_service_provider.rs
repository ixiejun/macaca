//! Runtime-host provider for the Route C Payment Service.
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
use macaca_proto::{
    A2AError, CleanupPolicy, KernelServiceId, PaymentIntent, PaymentIntentApproveCommand,
    PaymentIntentCreateCommand, PaymentIntentId, PaymentIntentSettleCommand, PaymentIntentState,
    PaymentLifecycleEventView, PaymentPolicyDecisionView, PaymentPolicyEvaluateCommand,
    PaymentProofListCommand, PaymentQuoteCommand, PaymentReceiptListScope, PaymentServiceSnapshot,
    PaymentSettlementResult, PaymentSnapshotCommand, PaymentTerms, PaymentTransitionListCommand,
    PaymentTransitionView, QuoteResponse, ServiceCallResult, ServiceCapability, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType,
    TraceContext, TraceSchemaRef, PAYMENT_INTENT_APPROVE_COMMAND, PAYMENT_INTENT_CREATE_COMMAND,
    PAYMENT_INTENT_SETTLE_COMMAND, PAYMENT_POLICY_EVALUATE_COMMAND, PAYMENT_PROOF_LIST_COMMAND,
    PAYMENT_QUOTE_COMMAND, PAYMENT_RECEIPT_GET_COMMAND, PAYMENT_RECEIPT_LIST_COMMAND,
    PAYMENT_SERVICE_ID, PAYMENT_SNAPSHOT_COMMAND, PAYMENT_TRANSITION_LIST_COMMAND,
};
use tracing::info;

use crate::payment_admission::{
    PaymentAmountSpec, PaymentRedactionSpec, PaymentScopeSpec, PaymentTraceSpec,
    PaymentTransitionSpec,
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

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        PaymentTraceSpec::check(&trace)?;
        Ok(trace)
    }

    fn result<T: serde::Serialize>(
        value: T,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(value)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn transition(
        &self,
        intent: &mut PaymentIntent,
        next: PaymentIntentState,
        operation: &str,
        trace: &TraceContext,
    ) -> ServiceResult<PaymentLifecycleEventView> {
        PaymentTransitionSpec::check(&intent.state, &next)?;
        let previous = intent.state.clone();
        intent.state = next.clone();
        intent.updated_at = Utc::now();
        self.store
            .append_transition(PaymentStateTransition::new(
                intent.intent_id.clone(),
                Some(previous),
                next.clone(),
                operation,
                "ok",
            ))
            .await
            .map_err(a2a_error)?;
        info!(
            intent_id = %intent.intent_id,
            state = %next,
            operation,
            "payment service intent transition appended"
        );
        Ok(PaymentLifecycleEventView {
            trace: trace.clone(),
            operation: operation.into(),
            status: "ok".into(),
            quote_id: Some(intent.quote_id.clone()),
            intent_id: Some(intent.intent_id.clone()),
            session_id: intent.session_id.clone(),
            task_id: intent.task_id.clone(),
            reason: None,
            metadata: BTreeMap::new(),
        })
    }

    async fn settle_intent(
        &self,
        mut intent: PaymentIntent,
        trace: &TraceContext,
    ) -> ServiceResult<PaymentSettlementResult> {
        PaymentScopeSpec::check_intent(&intent)?;
        PaymentAmountSpec::check_intent(&intent)?;
        PaymentRedactionSpec::check_metadata(&intent.metadata)?;
        let mut events = Vec::new();
        let normal_path = [
            (PaymentIntentState::quoted(), "quote_intent"),
            (PaymentIntentState::pending_approval(), "evaluate_policy"),
            (PaymentIntentState::approved(), "approve_intent"),
            (PaymentIntentState::executing(), "execute_adapter"),
        ];
        let start_index = normal_path
            .iter()
            .position(|(state, _)| *state == intent.state)
            .map(|index| index + 1)
            .unwrap_or(0);
        for (state, operation) in normal_path.into_iter().skip(start_index) {
            events.push(
                self.transition(&mut intent, state, operation, trace)
                    .await?,
            );
        }
        let (receipt, proof) = self.adapter.settle(&intent).await.map_err(a2a_error)?;
        events.push(
            self.transition(
                &mut intent,
                PaymentIntentState::settled(),
                "settle_intent",
                trace,
            )
            .await?,
        );
        self.store
            .put_execution_proof(proof.clone())
            .await
            .map_err(a2a_error)?;
        self.store
            .put_receipt(receipt.clone())
            .await
            .map_err(a2a_error)?;
        events.push(
            self.transition(
                &mut intent,
                PaymentIntentState::receipt_recorded(),
                "record_receipt",
                trace,
            )
            .await?,
        );
        info!(
            intent_id = %intent.intent_id,
            receipt_id = %receipt.receipt_id,
            "payment service settlement completed"
        );
        Ok(PaymentSettlementResult {
            receipt,
            proof,
            events,
        })
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
        let trace = Self::trace(&command)?;
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
                Self::result(quote, typed.trace)
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
                Self::result(intent, typed.trace)
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
                Self::result(decision, typed.trace)
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
                Self::result(intent, typed.trace)
            }
            PAYMENT_INTENT_SETTLE_COMMAND => {
                let typed: PaymentIntentSettleCommand = decode(command.payload)?;
                let result = self.settle_intent(typed.intent, &typed.trace).await?;
                Self::result(result, typed.trace)
            }
            PAYMENT_RECEIPT_GET_COMMAND => {
                let typed: macaca_proto::PaymentReceiptGetCommand = decode(command.payload)?;
                let receipt = self
                    .store
                    .receipt_by_intent(&typed.intent_id)
                    .await
                    .map_err(a2a_error)?;
                Self::result(receipt, typed.trace)
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
                Self::result(receipts, typed.trace)
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
                Self::result(views, typed.trace)
            }
            PAYMENT_PROOF_LIST_COMMAND => {
                let typed: PaymentProofListCommand = decode(command.payload)?;
                let proofs = self
                    .store
                    .execution_proofs_by_intent(&typed.intent_id)
                    .await
                    .map_err(a2a_error)?;
                Self::result(proofs, typed.trace)
            }
            PAYMENT_SNAPSHOT_COMMAND => {
                let typed: PaymentSnapshotCommand = decode(command.payload)?;
                Self::result(
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

/// Build the provider-neutral descriptor for Payment Service.
pub fn payment_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(PAYMENT_SERVICE_ID),
        ServiceType::new("payment"),
        TraceSchemaRef::new("trace.system_service.payment.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.quote"),
            "Negotiates provider-neutral A2A payment quotes.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.settle"),
            "Settles approved local simulated payment intents.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.payment.snapshot"),
            "Reports sanitized Payment service snapshots.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec![
        "payment.quote".into(),
        "payment.intent.settle".into(),
        "payment.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

fn a2a_error(error: A2AError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
