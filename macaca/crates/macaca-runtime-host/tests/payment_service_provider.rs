//! Integration tests for the Route C Payment Service provider.
//!
//! The tests live outside the provider module so the production file stays
//! below the Agent OS 500-line limit while still exercising the public service
//! contract exactly as Web/SDK consumers use it.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_persist::{InMemoryPaymentStore, PaymentStore};
use macaca_proto::{
    AgentIdentity, CapabilityId, PaymentIntentSettleCommand, PaymentQuoteCommand,
    PaymentSnapshotCommand as SnapshotCommand, QuoteRequest, RemoteCapabilityDescriptor,
    ServiceCommand, ServiceCommandName, TraceContext, PAYMENT_INTENT_SETTLE_COMMAND,
    PAYMENT_QUOTE_COMMAND, PAYMENT_SNAPSHOT_COMMAND,
};
use macaca_runtime_host::PaymentSystemServiceProvider;

fn quote_command() -> PaymentQuoteCommand {
    let provider = AgentIdentity::new("provider");
    PaymentQuoteCommand {
        trace: TraceContext::new("trace.payment.provider"),
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
            session_id: Some("session.payment".into()),
            task_id: Some("task.payment".into()),
            requested_at: chrono::Utc::now(),
            metadata: BTreeMap::new(),
        },
    }
}

fn service_command<T: serde::Serialize>(
    name: &str,
    trace: TraceContext,
    payload: &T,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        trace,
    )
}

#[tokio::test]
async fn payment_service_quote_persists_quote_snapshot() {
    let store = Arc::new(InMemoryPaymentStore::new());
    let service = PaymentSystemServiceProvider::local_simulated(
        store.clone(),
        macaca_kernel::local_simulated_terms("1", "UNIT"),
    );
    let command = quote_command();
    let result = service
        .call(service_command(
            PAYMENT_QUOTE_COMMAND,
            command.trace.clone(),
            &command,
        ))
        .await
        .unwrap();
    let quote: macaca_proto::QuoteResponse = serde_json::from_value(result.output).unwrap();
    assert!(store.get_quote(&quote.quote_id).await.unwrap().is_some());
}

#[tokio::test]
async fn payment_service_settle_records_receipt_and_proof() {
    let store = Arc::new(InMemoryPaymentStore::new());
    let service = PaymentSystemServiceProvider::local_simulated(
        store.clone(),
        macaca_kernel::local_simulated_terms("1", "UNIT"),
    );
    let quote_cmd = quote_command();
    let quote_result = service
        .call(service_command(
            PAYMENT_QUOTE_COMMAND,
            quote_cmd.trace.clone(),
            &quote_cmd,
        ))
        .await
        .unwrap();
    let quote: macaca_proto::QuoteResponse = serde_json::from_value(quote_result.output).unwrap();
    let intent = PaymentSystemServiceProvider::intent_from_quote(&quote);
    let settle = PaymentIntentSettleCommand {
        trace: TraceContext::new("trace.payment.settle"),
        intent,
    };
    let result = service
        .call(service_command(
            PAYMENT_INTENT_SETTLE_COMMAND,
            settle.trace.clone(),
            &settle,
        ))
        .await
        .unwrap();
    let settlement: macaca_proto::PaymentSettlementResult =
        serde_json::from_value(result.output).unwrap();
    assert_eq!(settlement.receipt.status, "settled");
    assert!(!store
        .execution_proofs_by_intent(&settlement.receipt.intent_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn payment_service_rejects_command_without_trace() {
    let service = PaymentSystemServiceProvider::local_simulated(
        Arc::new(InMemoryPaymentStore::new()),
        macaca_kernel::local_simulated_terms("1", "UNIT"),
    );
    let command = ServiceCommand::without_trace(
        ServiceCommandName::new(PAYMENT_SNAPSHOT_COMMAND),
        serde_json::to_value(SnapshotCommand {
            trace: TraceContext::new("trace.unused"),
        })
        .unwrap(),
    );
    assert!(service.call(command).await.is_err());
}
