//! SDK Payment Service client for Route C S10.
//!
//! The client is the upper-consumer Facade for Payment / A2A operations. Web,
//! CLI, Gateway, Application Framework, and future agent-facing code call this
//! trait instead of constructing kernel coordinators, runtime-host providers,
//! payment stores, or concrete adapter strategies.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ExecutionProof, MacacaError, MacacaResult, PaymentIntent, PaymentIntentApproveCommand,
    PaymentIntentCreateCommand, PaymentIntentSettleCommand, PaymentPolicyDecisionView,
    PaymentPolicyEvaluateCommand, PaymentProofListCommand, PaymentQuoteCommand, PaymentReceipt,
    PaymentReceiptGetCommand, PaymentReceiptListCommand, PaymentServiceSnapshot,
    PaymentSettlementResult, PaymentSnapshotCommand, PaymentTransitionListCommand,
    PaymentTransitionView, QuoteResponse, PAYMENT_INTENT_APPROVE_COMMAND,
    PAYMENT_INTENT_CREATE_COMMAND, PAYMENT_INTENT_SETTLE_COMMAND, PAYMENT_POLICY_EVALUATE_COMMAND,
    PAYMENT_PROOF_LIST_COMMAND, PAYMENT_QUOTE_COMMAND, PAYMENT_RECEIPT_GET_COMMAND,
    PAYMENT_RECEIPT_LIST_COMMAND, PAYMENT_SERVICE_ID, PAYMENT_SNAPSHOT_COMMAND,
    PAYMENT_TRANSITION_LIST_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Payment client consumed by upper shells and SDK users.
#[async_trait]
pub trait SystemPaymentClient: Send + Sync {
    async fn quote(&self, command: PaymentQuoteCommand) -> MacacaResult<QuoteResponse>;
    async fn create_intent(
        &self,
        command: PaymentIntentCreateCommand,
    ) -> MacacaResult<PaymentIntent>;
    async fn evaluate_policy(
        &self,
        command: PaymentPolicyEvaluateCommand,
    ) -> MacacaResult<PaymentPolicyDecisionView>;
    async fn approve(&self, command: PaymentIntentApproveCommand) -> MacacaResult<PaymentIntent>;
    async fn settle(
        &self,
        command: PaymentIntentSettleCommand,
    ) -> MacacaResult<PaymentSettlementResult>;
    async fn receipt(
        &self,
        command: PaymentReceiptGetCommand,
    ) -> MacacaResult<Option<PaymentReceipt>>;
    async fn receipts(
        &self,
        command: PaymentReceiptListCommand,
    ) -> MacacaResult<Vec<PaymentReceipt>>;
    async fn transitions(
        &self,
        command: PaymentTransitionListCommand,
    ) -> MacacaResult<Vec<PaymentTransitionView>>;
    async fn proofs(&self, command: PaymentProofListCommand) -> MacacaResult<Vec<ExecutionProof>>;
    async fn snapshot(
        &self,
        command: PaymentSnapshotCommand,
    ) -> MacacaResult<PaymentServiceSnapshot>;
}

/// Null-object Payment client used when Payment Service is not installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemPaymentClient;

#[async_trait]
impl SystemPaymentClient for UnavailableSystemPaymentClient {
    async fn quote(&self, command: PaymentQuoteCommand) -> MacacaResult<QuoteResponse> {
        warn!(trace_id = %command.trace.trace_id, "sdk payment client unavailable for quote");
        Err(MacacaError::Config("Payment service is unavailable".into()))
    }

    async fn create_intent(
        &self,
        _command: PaymentIntentCreateCommand,
    ) -> MacacaResult<PaymentIntent> {
        Err(MacacaError::Config("Payment service is unavailable".into()))
    }

    async fn evaluate_policy(
        &self,
        _command: PaymentPolicyEvaluateCommand,
    ) -> MacacaResult<PaymentPolicyDecisionView> {
        Err(MacacaError::Config("Payment service is unavailable".into()))
    }

    async fn approve(&self, _command: PaymentIntentApproveCommand) -> MacacaResult<PaymentIntent> {
        Err(MacacaError::Config("Payment service is unavailable".into()))
    }

    async fn settle(
        &self,
        _command: PaymentIntentSettleCommand,
    ) -> MacacaResult<PaymentSettlementResult> {
        Err(MacacaError::Config("Payment service is unavailable".into()))
    }

    async fn receipt(
        &self,
        _command: PaymentReceiptGetCommand,
    ) -> MacacaResult<Option<PaymentReceipt>> {
        Ok(None)
    }

    async fn receipts(
        &self,
        _command: PaymentReceiptListCommand,
    ) -> MacacaResult<Vec<PaymentReceipt>> {
        Ok(Vec::new())
    }

    async fn transitions(
        &self,
        _command: PaymentTransitionListCommand,
    ) -> MacacaResult<Vec<PaymentTransitionView>> {
        Ok(Vec::new())
    }

    async fn proofs(&self, _command: PaymentProofListCommand) -> MacacaResult<Vec<ExecutionProof>> {
        Ok(Vec::new())
    }

    async fn snapshot(
        &self,
        command: PaymentSnapshotCommand,
    ) -> MacacaResult<PaymentServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk payment client returning unavailable snapshot");
        Ok(PaymentServiceSnapshot::unavailable(
            "Payment service is unavailable",
        ))
    }
}

/// Runtime-backed Payment client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedPaymentClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedPaymentClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemPaymentClient for ServiceBackedPaymentClient {
    async fn quote(&self, command: PaymentQuoteCommand) -> MacacaResult<QuoteResponse> {
        call(
            &self.service,
            PAYMENT_QUOTE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn create_intent(
        &self,
        command: PaymentIntentCreateCommand,
    ) -> MacacaResult<PaymentIntent> {
        call(
            &self.service,
            PAYMENT_INTENT_CREATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn evaluate_policy(
        &self,
        command: PaymentPolicyEvaluateCommand,
    ) -> MacacaResult<PaymentPolicyDecisionView> {
        call(
            &self.service,
            PAYMENT_POLICY_EVALUATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn approve(&self, command: PaymentIntentApproveCommand) -> MacacaResult<PaymentIntent> {
        call(
            &self.service,
            PAYMENT_INTENT_APPROVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn settle(
        &self,
        command: PaymentIntentSettleCommand,
    ) -> MacacaResult<PaymentSettlementResult> {
        call(
            &self.service,
            PAYMENT_INTENT_SETTLE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn receipt(
        &self,
        command: PaymentReceiptGetCommand,
    ) -> MacacaResult<Option<PaymentReceipt>> {
        call(
            &self.service,
            PAYMENT_RECEIPT_GET_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn receipts(
        &self,
        command: PaymentReceiptListCommand,
    ) -> MacacaResult<Vec<PaymentReceipt>> {
        call(
            &self.service,
            PAYMENT_RECEIPT_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn transitions(
        &self,
        command: PaymentTransitionListCommand,
    ) -> MacacaResult<Vec<PaymentTransitionView>> {
        call(
            &self.service,
            PAYMENT_TRANSITION_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn proofs(&self, command: PaymentProofListCommand) -> MacacaResult<Vec<ExecutionProof>> {
        call(
            &self.service,
            PAYMENT_PROOF_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: PaymentSnapshotCommand,
    ) -> MacacaResult<PaymentServiceSnapshot> {
        call(
            &self.service,
            PAYMENT_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        PAYMENT_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{
        AgentIdentity, CapabilityId, PaymentQuoteCommand, PaymentSnapshotCommand, QuoteRequest,
        RemoteCapabilityDescriptor, TraceContext,
    };
    use std::collections::BTreeMap;

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoPaymentServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoPaymentServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![PAYMENT_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, PAYMENT_SERVICE_ID);
            assert_eq!(command.command_name, PAYMENT_QUOTE_COMMAND);
            let typed: PaymentQuoteCommand =
                serde_json::from_value(command.payload.clone()).unwrap();
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(QuoteResponse {
                    quote_id: macaca_proto::QuoteId::new("quote.sdk"),
                    request: typed.request,
                    terms: macaca_kernel::local_simulated_terms("1", "UNIT"),
                    quoted_at: Utc::now(),
                    metadata: BTreeMap::new(),
                })
                .unwrap(),
            })
        }
    }

    fn quote_command() -> PaymentQuoteCommand {
        let provider = AgentIdentity::new("provider");
        PaymentQuoteCommand {
            trace: TraceContext::new("trace-sdk-payment-quote"),
            request: QuoteRequest {
                requester: AgentIdentity::new("requester"),
                provider: provider.clone(),
                capability: RemoteCapabilityDescriptor {
                    capability_id: CapabilityId::new("cap.payment"),
                    provider,
                    operation: "execute".into(),
                    description: None,
                    metadata: BTreeMap::new(),
                },
                operation: "execute".into(),
                session_id: None,
                task_id: None,
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
        }
    }

    #[tokio::test]
    async fn service_backed_payment_client_dispatches_quote_command() {
        let client = ServiceBackedPaymentClient::new(Arc::new(EchoPaymentServiceClient));
        let quote = client.quote(quote_command()).await.unwrap();
        assert_eq!(quote.quote_id.as_str(), "quote.sdk");
    }

    #[tokio::test]
    async fn unavailable_payment_client_fails_closed_for_quote() {
        let client = UnavailableSystemPaymentClient;
        let err = client.quote(quote_command()).await.unwrap_err();
        assert!(err.to_string().contains("Payment service is unavailable"));
    }

    #[tokio::test]
    async fn unavailable_payment_client_returns_unavailable_snapshot() {
        let client = UnavailableSystemPaymentClient;
        let snapshot = client
            .snapshot(PaymentSnapshotCommand {
                trace: TraceContext::new("trace-sdk-payment-snapshot"),
            })
            .await
            .unwrap();
        assert_eq!(snapshot.health, "unavailable");
    }
}
