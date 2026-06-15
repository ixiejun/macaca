//! Task-level A2A payment context contracts.
//!
//! These types let task services attach requester/provider/capability/quote
//! and payment-intent metadata to existing task flows without changing task
//! execution semantics. Payment execution remains behind kernel/service
//! facades, while the task layer carries auditable context for trace replay.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::{
    AgentIdentity, CapabilityId, PaymentAmount, PaymentIntentId, QuoteId,
    RemoteCapabilityDescriptor,
};
use serde::{Deserialize, Serialize};

/// Serializable references to quote and payment intent artifacts.
///
/// References are optional because a task may be created before quote
/// negotiation begins. The same context can then be enriched after the A2A
/// coordinator creates quote and intent records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct A2APaymentReferences {
    pub quote_id: Option<QuoteId>,
    pub intent_id: Option<PaymentIntentId>,
    pub quoted_amount: Option<PaymentAmount>,
    pub metadata: BTreeMap<String, String>,
}

/// Trace metadata carried with an A2A task request.
///
/// The fields intentionally mirror the trace/audit payload required by Phase
/// 09 while excluding secrets and provider credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATaskTraceContext {
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requester_id: String,
    pub provider_id: String,
    pub capability_id: CapabilityId,
    pub operation: String,
    pub created_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command-like task context for an A2A capability request.
///
/// Existing task execution can store or forward this structure as metadata.
/// It does not call adapters, enforce payment, or alter worker scheduling by
/// itself, which keeps the A2A task path additive and no-network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATaskRequestContext {
    pub requester: AgentIdentity,
    pub provider: AgentIdentity,
    pub capability: RemoteCapabilityDescriptor,
    pub operation: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub payment: A2APaymentReferences,
    pub trace: A2ATaskTraceContext,
    pub metadata: BTreeMap<String, String>,
}

impl A2ATaskRequestContext {
    /// Build a context before quote/intent creation.
    pub fn new(
        requester: AgentIdentity,
        provider: AgentIdentity,
        capability: RemoteCapabilityDescriptor,
        operation: impl Into<String>,
    ) -> Self {
        let operation = operation.into();
        let trace = A2ATaskTraceContext {
            session_id: None,
            task_id: None,
            requester_id: requester.id.to_string(),
            provider_id: provider.id.to_string(),
            capability_id: capability.capability_id.clone(),
            operation: operation.clone(),
            created_at: Utc::now(),
            metadata: BTreeMap::new(),
        };
        Self {
            requester,
            provider,
            capability,
            operation,
            session_id: None,
            task_id: None,
            payment: A2APaymentReferences::default(),
            trace,
            metadata: BTreeMap::new(),
        }
    }

    /// Attach session and task scope used by EventLog and audit queries.
    pub fn with_scope(mut self, session_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let task_id = task_id.into();
        self.session_id = Some(session_id.clone());
        self.task_id = Some(task_id.clone());
        self.trace.session_id = Some(session_id);
        self.trace.task_id = Some(task_id);
        self
    }

    /// Attach quote and intent references after coordinator negotiation.
    pub fn with_payment_references(
        mut self,
        quote_id: QuoteId,
        intent_id: PaymentIntentId,
        amount: PaymentAmount,
    ) -> Self {
        self.payment.quote_id = Some(quote_id);
        self.payment.intent_id = Some(intent_id);
        self.payment.quoted_amount = Some(amount);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{PaymentAsset, PaymentRail};

    #[test]
    fn a2a_task_context_is_serializable_without_real_payment() {
        let provider = AgentIdentity::new("provider");
        let context = A2ATaskRequestContext::new(
            AgentIdentity::new("requester"),
            provider.clone(),
            RemoteCapabilityDescriptor {
                capability_id: CapabilityId::new("cap.task"),
                provider,
                operation: "quote".into(),
                description: None,
                metadata: BTreeMap::new(),
            },
            "execute",
        )
        .with_scope("session.task", "task.task")
        .with_payment_references(
            QuoteId::new("quote.task"),
            PaymentIntentId::new("intent.task"),
            PaymentAmount::new(
                "1",
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            ),
        );

        let decoded: A2ATaskRequestContext =
            serde_json::from_str(&serde_json::to_string(&context).unwrap()).unwrap();
        assert_eq!(decoded.session_id.as_deref(), Some("session.task"));
        assert_eq!(decoded.task_id.as_deref(), Some("task.task"));
        assert_eq!(decoded.payment.quote_id.unwrap().as_str(), "quote.task");
        assert_eq!(decoded.payment.intent_id.unwrap().as_str(), "intent.task");
        assert_eq!(decoded.trace.capability_id, CapabilityId::new("cap.task"));
    }
}
