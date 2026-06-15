//! Integration tests for the Payment Service provider.
//!
//! The tests live outside the provider module so the production file stays
//! below the Agent OS 500-line limit while still exercising the public service
//! contract exactly as Web/SDK consumers use it.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_persist::{InMemoryPaymentStore, PaymentStore};
use macaca_proto::{
    local_simulated_terms, AgentIdentity, ApprovalPolicy, BudgetPolicy, CapabilityId,
    PaymentAmount, PaymentAsset, PaymentIntent, PaymentIntentApproveCommand, PaymentIntentId,
    PaymentIntentSettleCommand, PaymentIntentState, PaymentPolicyEvaluateCommand,
    PaymentQuoteCommand, PaymentRail, PaymentSnapshotCommand as SnapshotCommand, QuoteRequest,
    RemoteCapabilityDescriptor, ServiceCommand, ServiceCommandName, TraceContext,
    PAYMENT_INTENT_APPROVE_COMMAND, PAYMENT_INTENT_SETTLE_COMMAND, PAYMENT_POLICY_EVALUATE_COMMAND,
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

fn over_budget_intent() -> PaymentIntent {
    PaymentIntent {
        intent_id: PaymentIntentId::new("intent.over-budget"),
        quote_id: macaca_proto::QuoteId::new("quote.over-budget"),
        requester: AgentIdentity::new("requester"),
        provider: AgentIdentity::new("provider"),
        capability_id: CapabilityId::new("cap.payment"),
        amount: PaymentAmount::new(
            "10",
            PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
        ),
        state: PaymentIntentState::created(),
        session_id: Some("session.payment".into()),
        task_id: Some("task.payment".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn payment_service_quote_persists_quote_snapshot() {
    let store = Arc::new(InMemoryPaymentStore::new());
    let service = PaymentSystemServiceProvider::local_simulated(
        store.clone(),
        local_simulated_terms("1", "UNIT"),
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
        local_simulated_terms("1", "UNIT"),
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
        local_simulated_terms("1", "UNIT"),
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

#[tokio::test]
async fn payment_service_policy_denied_returns_structured_adapter_failure() {
    let service = PaymentSystemServiceProvider::local_simulated(
        Arc::new(InMemoryPaymentStore::new()),
        local_simulated_terms("1", "UNIT"),
    );
    let evaluate = PaymentPolicyEvaluateCommand {
        trace: TraceContext::new("trace.payment.policy.denied"),
        intent: over_budget_intent(),
        budget: BudgetPolicy {
            max_amount: Some(PaymentAmount::new(
                "2",
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            auto_approve_local_simulated_below: Some(PaymentAmount::new(
                "2",
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            metadata: BTreeMap::new(),
        },
        approval: ApprovalPolicy {
            explicit_approval_required: false,
            explicit_approval_granted: false,
            approver: None,
            metadata: BTreeMap::new(),
        },
    };
    let error = service
        .call(service_command(
            PAYMENT_POLICY_EVALUATE_COMMAND,
            evaluate.trace.clone(),
            &evaluate,
        ))
        .await
        .expect_err("over-budget policy evaluation must fail closed");
    assert!(
        error
            .to_string()
            .to_ascii_lowercase()
            .contains("over budget"),
        "expected over-budget reason, got: {error}"
    );
}

#[tokio::test]
async fn payment_service_rejects_invalid_intent_transition_without_mutating_store() {
    let store = Arc::new(InMemoryPaymentStore::new());
    let service = PaymentSystemServiceProvider::local_simulated(
        store.clone(),
        local_simulated_terms("1", "UNIT"),
    );
    let mut intent = over_budget_intent();
    intent.amount = PaymentAmount::new(
        "1",
        PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
    );
    let approve = PaymentIntentApproveCommand {
        trace: TraceContext::new("trace.payment.approve.invalid"),
        intent: intent.clone(),
        approval: ApprovalPolicy {
            explicit_approval_required: false,
            explicit_approval_granted: true,
            approver: Some("approver.test".into()),
            metadata: BTreeMap::new(),
        },
    };
    let error = service
        .call(service_command(
            PAYMENT_INTENT_APPROVE_COMMAND,
            approve.trace.clone(),
            &approve,
        ))
        .await
        .expect_err("created -> approved transition must be rejected");
    assert!(
        error
            .to_string()
            .to_ascii_lowercase()
            .contains("invalid payment transition"),
        "expected invalid transition diagnostic, got: {error}"
    );
    assert!(
        store
            .transitions_by_intent(&intent.intent_id)
            .await
            .unwrap()
            .is_empty(),
        "invalid transition must not append store mementos"
    );
}
