//! Admission specifications for Route C Payment Service.
//!
//! These small Specification objects keep validation rules out of Web, CLI,
//! kernel compatibility helpers, and provider-specific adapters. The Payment
//! Service provider invokes them before adapter execution so every payment
//! lifecycle command is traceable, scoped, amount-valid, transition-safe, and
//! safe to log.

use std::collections::BTreeMap;

use macaca_proto::{
    PaymentIntent, PaymentIntentState, QuoteRequest, ServiceError, ServiceResult, TraceContext,
};

/// Specification requiring a non-empty trace id before service dispatch.
pub struct PaymentTraceSpec;

impl PaymentTraceSpec {
    /// Reject an empty trace context before a mutating payment command runs.
    pub fn check(trace: &TraceContext) -> ServiceResult<()> {
        if trace.trace_id.trim().is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment trace context is required".into(),
            ));
        }
        Ok(())
    }
}

/// Specification requiring requester, provider, and capability identity.
pub struct PaymentScopeSpec;

impl PaymentScopeSpec {
    /// Validate the scope carried by an A2A quote request.
    pub fn check_quote(request: &QuoteRequest) -> ServiceResult<()> {
        if request.requester.id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment requester identity is required".into(),
            ));
        }
        if request.provider.id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment provider identity is required".into(),
            ));
        }
        if request.capability.capability_id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment capability identity is required".into(),
            ));
        }
        Ok(())
    }

    /// Validate the scope carried by an already-created payment intent.
    pub fn check_intent(intent: &PaymentIntent) -> ServiceResult<()> {
        if intent.requester.id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment requester identity is required".into(),
            ));
        }
        if intent.provider.id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment provider identity is required".into(),
            ));
        }
        if intent.capability_id.is_empty() {
            return Err(ServiceError::AdapterFailure(
                "payment capability identity is required".into(),
            ));
        }
        Ok(())
    }
}

/// Specification requiring an amount that policy can parse deterministically.
pub struct PaymentAmountSpec;

impl PaymentAmountSpec {
    /// Validate that the intent amount is parseable by the A2A value object.
    pub fn check_intent(intent: &PaymentIntent) -> ServiceResult<()> {
        intent.amount.as_f64().map(|_| ()).map_err(|error| {
            ServiceError::AdapterFailure(format!("invalid payment amount: {error}"))
        })
    }
}

/// Specification guarding canonical payment state transitions.
pub struct PaymentTransitionSpec;

impl PaymentTransitionSpec {
    /// Validate one lifecycle transition before it is persisted or executed.
    pub fn check(from: &PaymentIntentState, to: &PaymentIntentState) -> ServiceResult<()> {
        if !from.can_transition_to(to) {
            return Err(ServiceError::AdapterFailure(format!(
                "invalid payment transition from {from} to {to}"
            )));
        }
        Ok(())
    }
}

/// Specification that protects logs, trace, and snapshots from secret keys.
pub struct PaymentRedactionSpec;

impl PaymentRedactionSpec {
    /// Reject metadata keys that are very likely to carry payment secrets.
    ///
    /// Values are never included in the error. This keeps admission failures
    /// auditable without copying secrets into logs or trace events.
    pub fn check_metadata(metadata: &BTreeMap<String, String>) -> ServiceResult<()> {
        for key in metadata.keys() {
            let normalized = key.to_ascii_lowercase();
            if normalized.contains("secret")
                || normalized.contains("private_key")
                || normalized.contains("api_key")
                || normalized.contains("credential")
                || normalized.contains("signed_payload")
            {
                return Err(ServiceError::AdapterFailure(format!(
                    "payment redaction rejected sensitive metadata key '{key}'"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{
        AgentIdentity, CapabilityId, PaymentAmount, PaymentAsset, PaymentIntent, PaymentIntentId,
        PaymentIntentState, PaymentRail, QuoteId, QuoteRequest, RemoteCapabilityDescriptor,
        TraceContext,
    };
    use std::collections::BTreeMap;

    fn quote_request(requester_id: &str) -> QuoteRequest {
        let provider = AgentIdentity::new("provider");
        QuoteRequest {
            requester: AgentIdentity::new(requester_id),
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
            requested_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    fn intent(quantity: &str, state: PaymentIntentState) -> PaymentIntent {
        let now = Utc::now();
        PaymentIntent {
            intent_id: PaymentIntentId::new("intent.payment"),
            quote_id: QuoteId::new("quote.payment"),
            requester: AgentIdentity::new("requester"),
            provider: AgentIdentity::new("provider"),
            capability_id: CapabilityId::new("cap.payment"),
            amount: PaymentAmount::new(
                quantity,
                PaymentAsset::new(PaymentRail::local_simulated(), "UNIT"),
            ),
            state,
            session_id: Some("session.payment".into()),
            task_id: Some("task.payment".into()),
            created_at: now,
            updated_at: now,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn payment_trace_spec_rejects_empty_trace() {
        let error = PaymentTraceSpec::check(&TraceContext::new("  ")).unwrap_err();
        assert!(error.to_string().contains("trace"));
    }

    #[test]
    fn payment_scope_spec_rejects_empty_requester() {
        let error = PaymentScopeSpec::check_quote(&quote_request("  ")).unwrap_err();
        assert!(error.to_string().contains("requester"));
    }

    #[test]
    fn payment_amount_spec_rejects_invalid_decimal() {
        let error = PaymentAmountSpec::check_intent(&intent(
            "not-a-number",
            PaymentIntentState::pending_approval(),
        ))
        .unwrap_err();
        assert!(error.to_string().contains("amount"));
    }

    #[test]
    fn payment_transition_spec_rejects_invalid_transition() {
        let error = PaymentTransitionSpec::check(
            &PaymentIntentState::created(),
            &PaymentIntentState::settled(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("transition"));
    }

    #[test]
    fn payment_redaction_spec_rejects_secret_keys() {
        let mut metadata = BTreeMap::new();
        metadata.insert("wallet_secret".into(), "value".into());
        let error = PaymentRedactionSpec::check_metadata(&metadata).unwrap_err();
        assert!(error.to_string().contains("redaction"));
    }
}
