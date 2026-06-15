//! Provider-neutral payment policy strategy for A2A intents.
//!
//! This module defines the policy boundary used by runtime adapters and
//! coordination services. Keeping this in `macaca-proto` allows both
//! kernel and runtime-host crates to share one implementation without creating
//! cross-crate dependency cycles.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::{A2AError, ApprovalPolicy, BudgetPolicy, PaymentIntent};

/// Outcome of one payment policy evaluation.
///
/// Callers can persist and emit this data directly through audit/trace paths
/// without parsing log text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentPolicyDecision {
    pub allowed: bool,
    pub reason: String,
    pub decided_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl PaymentPolicyDecision {
    /// Build an allow decision with optional audit metadata.
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a deny decision with optional audit metadata.
    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            decided_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Strategy interface for budget and approval checks.
///
/// Implementations must remain side-effect free. The policy only decides
/// whether execution may proceed; stores and adapters handle side effects.
pub trait PaymentPolicyEngine: Send + Sync {
    /// Evaluate one intent against budget, approval, and adapter availability.
    fn evaluate(
        &self,
        intent: &PaymentIntent,
        budget: &BudgetPolicy,
        approval: &ApprovalPolicy,
        adapter_configured: bool,
    ) -> Result<PaymentPolicyDecision, A2AError>;
}

/// Default conservative payment policy.
///
/// Local simulated intents can be auto-approved under a configured threshold.
/// Non-simulated rails require adapter availability and explicit approval.
#[derive(Debug, Default, Clone)]
pub struct DefaultPaymentPolicyEngine;

impl DefaultPaymentPolicyEngine {
    /// Create the default policy engine.
    pub fn new() -> Self {
        Self
    }

    fn exceeds_limit(intent: &PaymentIntent, budget: &BudgetPolicy) -> Result<bool, A2AError> {
        let Some(max_amount) = &budget.max_amount else {
            return Ok(false);
        };
        Ok(intent.amount.as_f64()? > max_amount.as_f64()?)
    }

    fn under_local_auto_approve(
        intent: &PaymentIntent,
        budget: &BudgetPolicy,
    ) -> Result<bool, A2AError> {
        let Some(threshold) = &budget.auto_approve_local_simulated_below else {
            return Ok(false);
        };
        Ok(intent.amount.asset.rail.is_local_simulated()
            && intent.amount.as_f64()? <= threshold.as_f64()?)
    }
}

impl PaymentPolicyEngine for DefaultPaymentPolicyEngine {
    fn evaluate(
        &self,
        intent: &PaymentIntent,
        budget: &BudgetPolicy,
        approval: &ApprovalPolicy,
        adapter_configured: bool,
    ) -> Result<PaymentPolicyDecision, A2AError> {
        info!(
            intent_id = %intent.intent_id,
            quote_id = %intent.quote_id,
            requester_id = %intent.requester.id,
            provider_id = %intent.provider.id,
            rail = %intent.amount.asset.rail,
            amount = %intent.amount.quantity,
            "payment policy evaluation started"
        );

        if Self::exceeds_limit(intent, budget)? {
            let reason = "payment intent exceeds configured budget";
            warn!(
                intent_id = %intent.intent_id,
                reason,
                "payment policy rejected over-budget intent"
            );
            return Err(A2AError::OverBudget(reason.into()));
        }

        if Self::under_local_auto_approve(intent, budget)? {
            info!(
                intent_id = %intent.intent_id,
                "payment policy auto-approved local simulated intent"
            );
            return Ok(PaymentPolicyDecision::allow(
                "local simulated payment under auto-approval threshold",
            ));
        }

        if !intent.amount.asset.rail.is_local_simulated() && !adapter_configured {
            let reason = "real payment adapter is not configured";
            warn!(
                intent_id = %intent.intent_id,
                reason,
                "payment policy marked adapter unavailable"
            );
            return Err(A2AError::AdapterUnavailable(reason.into()));
        }

        if approval.explicit_approval_required && !approval.explicit_approval_granted {
            let reason = "explicit approval required before payment execution";
            warn!(
                intent_id = %intent.intent_id,
                reason,
                "payment policy requires explicit approval"
            );
            return Err(A2AError::ApprovalRequired(reason.into()));
        }

        info!(
            intent_id = %intent.intent_id,
            "payment policy evaluation passed"
        );
        Ok(PaymentPolicyDecision::allow("payment policy passed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentIdentity, CapabilityId, PaymentAmount, PaymentAsset, PaymentIntentId,
        PaymentIntentState, PaymentRail, QuoteId,
    };

    fn intent(quantity: &str, rail: PaymentRail) -> PaymentIntent {
        let now = Utc::now();
        PaymentIntent {
            intent_id: PaymentIntentId::new("intent.policy"),
            quote_id: QuoteId::new("quote.policy"),
            requester: AgentIdentity::new("requester"),
            provider: AgentIdentity::new("provider"),
            capability_id: CapabilityId::new("cap.policy"),
            amount: PaymentAmount::new(quantity, PaymentAsset::new(rail, "UNIT")),
            state: PaymentIntentState::pending_approval(),
            session_id: None,
            task_id: None,
            created_at: now,
            updated_at: now,
            metadata: BTreeMap::new(),
        }
    }

    fn budget(max: &str, threshold: &str) -> BudgetPolicy {
        BudgetPolicy {
            max_amount: Some(PaymentAmount::new(
                max,
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            auto_approve_local_simulated_below: Some(PaymentAmount::new(
                threshold,
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            )),
            metadata: BTreeMap::new(),
        }
    }

    fn approval(required: bool, granted: bool) -> ApprovalPolicy {
        ApprovalPolicy {
            explicit_approval_required: required,
            explicit_approval_granted: granted,
            approver: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn over_budget_is_rejected() {
        let engine = DefaultPaymentPolicyEngine::new();
        let result = engine.evaluate(
            &intent("10", PaymentRail::local_simulated()),
            &budget("2", "2"),
            &approval(false, false),
            true,
        );
        assert!(matches!(result, Err(A2AError::OverBudget(_))));
    }

    #[test]
    fn local_simulation_under_threshold_is_auto_approved() {
        let engine = DefaultPaymentPolicyEngine::new();
        let decision = engine
            .evaluate(
                &intent("1", PaymentRail::local_simulated()),
                &budget("2", "2"),
                &approval(false, false),
                true,
            )
            .unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn real_adapter_without_configuration_is_unavailable() {
        let engine = DefaultPaymentPolicyEngine::new();
        let result = engine.evaluate(
            &intent("1", PaymentRail::new("external_rail")),
            &budget("2", "0"),
            &approval(true, true),
            false,
        );
        assert!(matches!(result, Err(A2AError::AdapterUnavailable(_))));
    }

    #[test]
    fn real_adapter_requires_explicit_approval() {
        let engine = DefaultPaymentPolicyEngine::new();
        let result = engine.evaluate(
            &intent("1", PaymentRail::new("external_rail")),
            &budget("2", "0"),
            &approval(true, false),
            true,
        );
        assert!(matches!(result, Err(A2AError::ApprovalRequired(_))));
    }
}
