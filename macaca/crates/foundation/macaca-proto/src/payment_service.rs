//! Provider-neutral Payment Service contracts for Route C S10.
//!
//! The module describes the service-facing command and result surface for
//! A2A/payment lifecycle work. It intentionally reuses the A2A value objects
//! from `a2a.rs` so S10 serviceization does not redefine quote, intent,
//! receipt, or proof semantics. Runtime crates execute these commands through
//! ServiceRuntime; SDK and presentation shells consume the same data-only
//! contracts without depending on concrete payment providers.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ApprovalPolicy, BudgetPolicy, ExecutionProof, PaymentIntent, PaymentIntentId,
    PaymentIntentState, PaymentReceipt, QuoteId, QuoteRequest, QuoteResponse, TraceContext,
};

/// Stable service id used by ServiceRuntime and SDK clients.
pub const PAYMENT_SERVICE_ID: &str = "macaca.payment";

/// Command name for quote negotiation.
pub const PAYMENT_QUOTE_COMMAND: &str = "payment.quote";
/// Command name for creating a payment intent from a quote snapshot.
pub const PAYMENT_INTENT_CREATE_COMMAND: &str = "payment.intent.create";
/// Command name for evaluating budget and approval policy.
pub const PAYMENT_POLICY_EVALUATE_COMMAND: &str = "payment.intent.evaluate_policy";
/// Command name for recording explicit payment approval.
pub const PAYMENT_INTENT_APPROVE_COMMAND: &str = "payment.intent.approve";
/// Command name for settlement adapter execution and receipt recording.
pub const PAYMENT_INTENT_SETTLE_COMMAND: &str = "payment.intent.settle";
/// Command name for reading a receipt by payment intent id.
pub const PAYMENT_RECEIPT_GET_COMMAND: &str = "payment.receipt.get";
/// Command name for listing receipts by session or task scope.
pub const PAYMENT_RECEIPT_LIST_COMMAND: &str = "payment.receipt.list";
/// Command name for listing ordered payment state transitions.
pub const PAYMENT_TRANSITION_LIST_COMMAND: &str = "payment.transition.list";
/// Command name for listing execution proof records.
pub const PAYMENT_PROOF_LIST_COMMAND: &str = "payment.proof.list";
/// Command name for a sanitized Payment Service snapshot.
pub const PAYMENT_SNAPSHOT_COMMAND: &str = "payment.snapshot";

/// Quote command accepted by Payment Service.
///
/// The command carries trace context at the typed layer because S10 requires
/// mutating service calls to be traceable before provider dispatch. The quote
/// request itself remains the provider-neutral A2A contract from `a2a.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentQuoteCommand {
    pub trace: TraceContext,
    pub request: QuoteRequest,
}

/// Command that creates a canonical payment intent from an immutable quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentCreateCommand {
    pub trace: TraceContext,
    pub quote: QuoteResponse,
}

/// Command that evaluates budget and approval rules for one intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentPolicyEvaluateCommand {
    pub trace: TraceContext,
    pub intent: PaymentIntent,
    pub budget: BudgetPolicy,
    pub approval: ApprovalPolicy,
}

/// Command that records explicit approval for an intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentApproveCommand {
    pub trace: TraceContext,
    pub intent: PaymentIntent,
    pub approval: ApprovalPolicy,
}

/// Command that settles an already approved intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentSettleCommand {
    pub trace: TraceContext,
    pub intent: PaymentIntent,
}

/// Command that reads a receipt by payment intent id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentReceiptGetCommand {
    pub trace: TraceContext,
    pub intent_id: PaymentIntentId,
}

/// Query scope for receipt list operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentReceiptListScope {
    Session(String),
    Task(String),
}

/// Command that lists receipts by session or task scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentReceiptListCommand {
    pub trace: TraceContext,
    pub scope: PaymentReceiptListScope,
}

/// Command that lists transition mementos for one intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTransitionListCommand {
    pub trace: TraceContext,
    pub intent_id: PaymentIntentId,
}

/// Redacted transition view returned by Payment Service.
///
/// Persistence stores the authoritative transition memento. The service view
/// mirrors only bounded fields needed by shells, trace replay, and audit
/// readers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTransitionView {
    pub intent_id: PaymentIntentId,
    pub from: Option<PaymentIntentState>,
    pub to: PaymentIntentState,
    pub operation: String,
    pub status: String,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command that lists execution proofs for one intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentProofListCommand {
    pub trace: TraceContext,
    pub intent_id: PaymentIntentId,
}

/// Read-only snapshot command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSnapshotCommand {
    pub trace: TraceContext,
}

/// Redacted policy decision view returned by Payment Service.
///
/// The view deliberately stores reason and bounded metadata only. It must not
/// carry raw provider responses, credentials, keys, wallet secrets, or
/// unbounded prompt/user payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentPolicyDecisionView {
    pub trace: TraceContext,
    pub allowed: bool,
    pub intent_id: Option<String>,
    pub reason: String,
    pub decided_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PaymentPolicyDecisionView {
    /// Build an allow decision view with a bounded reason string.
    pub fn allowed(
        trace: TraceContext,
        intent_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            trace,
            allowed: true,
            intent_id: Some(intent_id.into()),
            reason: reason.into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a deny decision view with a bounded reason string.
    pub fn denied(
        trace: TraceContext,
        intent_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            trace,
            allowed: false,
            intent_id: Some(intent_id.into()),
            reason: reason.into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Redacted lifecycle event view suitable for trace/audit surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLifecycleEventView {
    pub trace: TraceContext,
    pub operation: String,
    pub status: String,
    pub quote_id: Option<QuoteId>,
    pub intent_id: Option<PaymentIntentId>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Sanitized Payment Service snapshot.
///
/// Snapshots expose health, adapter availability, counts, and diagnostics so
/// shell tools can render state without seeing provider credentials or raw
/// settlement payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentServiceSnapshot {
    pub service_id: String,
    pub health: String,
    pub adapter_configured: bool,
    pub quote_count: usize,
    pub receipt_count: usize,
    pub proof_count: usize,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PaymentServiceSnapshot {
    /// Build an unavailable snapshot for Null Object clients and providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: PAYMENT_SERVICE_ID.into(),
            health: "unavailable".into(),
            adapter_configured: false,
            quote_count: 0,
            receipt_count: 0,
            proof_count: 0,
            diagnostics: vec![reason.into()],
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Result returned by settlement commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSettlementResult {
    pub receipt: PaymentReceipt,
    pub proof: ExecutionProof,
    pub events: Vec<PaymentLifecycleEventView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentIdentity, ApprovalPolicy, BudgetPolicy, CapabilityId, PaymentAmount, PaymentAsset,
        PaymentIntent, PaymentIntentId, PaymentIntentState, PaymentRail, QuoteId, QuoteRequest,
        RemoteCapabilityDescriptor, TraceContext,
    };
    use chrono::Utc;
    use std::collections::BTreeMap;

    fn quote_command() -> PaymentQuoteCommand {
        let provider = AgentIdentity::new("provider");
        PaymentQuoteCommand {
            trace: TraceContext::new("trace.payment.proto"),
            request: QuoteRequest {
                requester: AgentIdentity::new("requester"),
                provider: provider.clone(),
                capability: RemoteCapabilityDescriptor {
                    capability_id: CapabilityId::new("cap.payment"),
                    provider,
                    operation: "analyze".into(),
                    description: None,
                    metadata: BTreeMap::new(),
                },
                operation: "analyze".into(),
                session_id: Some("session.payment".into()),
                task_id: Some("task.payment".into()),
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
        }
    }

    #[test]
    fn payment_service_command_names_are_stable() {
        assert_eq!(PAYMENT_SERVICE_ID, "macaca.payment");
        assert_eq!(PAYMENT_QUOTE_COMMAND, "payment.quote");
        assert_eq!(PAYMENT_INTENT_SETTLE_COMMAND, "payment.intent.settle");
        assert_eq!(PAYMENT_SNAPSHOT_COMMAND, "payment.snapshot");
    }

    #[test]
    fn payment_quote_command_round_trips() {
        let command = quote_command();
        let encoded = serde_json::to_string(&command).unwrap();
        let decoded: PaymentQuoteCommand = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.trace.trace_id, "trace.payment.proto");
        assert_eq!(decoded.request.requester.id.as_str(), "requester");
        assert_eq!(
            decoded.request.capability.capability_id.as_str(),
            "cap.payment"
        );
    }

    #[test]
    fn payment_policy_decision_view_is_redacted_and_traceable() {
        let view = PaymentPolicyDecisionView::allowed(
            TraceContext::new("trace.payment.policy"),
            "intent.policy",
            "budget under limit",
        );
        assert!(view.allowed);
        assert_eq!(view.intent_id.as_deref(), Some("intent.policy"));
        assert_eq!(view.reason, "budget under limit");
        assert!(view.metadata.is_empty());
    }

    #[test]
    fn payment_snapshot_unavailable_is_explicit() {
        let snapshot = PaymentServiceSnapshot::unavailable("Payment service disabled");
        assert_eq!(snapshot.service_id, PAYMENT_SERVICE_ID);
        assert_eq!(snapshot.health, "unavailable");
        assert!(snapshot
            .diagnostics
            .iter()
            .any(|item| item.contains("Payment service disabled")));
    }

    #[test]
    fn policy_evaluate_command_reuses_existing_a2a_value_objects() {
        let quote = quote_command();
        let command = PaymentPolicyEvaluateCommand {
            trace: quote.trace.clone(),
            intent: PaymentIntent {
                intent_id: PaymentIntentId::new("intent.proto"),
                quote_id: QuoteId::new("quote.proto"),
                requester: quote.request.requester.clone(),
                provider: quote.request.provider.clone(),
                capability_id: quote.request.capability.capability_id.clone(),
                amount: PaymentAmount::new(
                    "1",
                    PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
                ),
                state: PaymentIntentState::pending_approval(),
                session_id: quote.request.session_id.clone(),
                task_id: quote.request.task_id.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
            budget: BudgetPolicy {
                max_amount: None,
                auto_approve_local_simulated_below: None,
                metadata: BTreeMap::new(),
            },
            approval: ApprovalPolicy {
                explicit_approval_required: false,
                explicit_approval_granted: false,
                approver: None,
                metadata: BTreeMap::new(),
            },
        };
        assert_eq!(
            command.intent.amount.asset.rail,
            PaymentRail::local_simulated()
        );
    }
}
