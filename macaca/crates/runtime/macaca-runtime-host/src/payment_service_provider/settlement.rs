//! Payment intent state-machine execution.
//!
//! Settlement is separated from command dispatch so the provider uses a clear
//! State-pattern boundary: commands select an operation, while this module owns
//! valid transition sequencing, proof/receipt persistence, and audit logging.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_persist::PaymentStateTransition;
use macaca_proto::{
    PaymentIntent, PaymentIntentState, PaymentLifecycleEventView, PaymentSettlementResult,
    ServiceResult, TraceContext,
};
use tracing::info;

use super::support::a2a_error;
use super::PaymentSystemServiceProvider;
use crate::payment_admission::{
    PaymentAmountSpec, PaymentRedactionSpec, PaymentScopeSpec, PaymentTransitionSpec,
};

impl PaymentSystemServiceProvider {
    pub(super) async fn transition(
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

    pub(super) async fn settle_intent(
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
