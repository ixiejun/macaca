//! Provider-neutral Agent-to-Agent payment contracts for Route C Phase 09.
//!
//! This module is intentionally data-only. It models identities, quote
//! negotiation, payment intents, budget/approval policy inputs, receipts, and
//! execution proof without embedding any concrete payment provider, chain,
//! wallet, gateway, or application workflow. Runtime crates coordinate these
//! contracts through facades and adapters so future payment systems can be
//! plugged in without changing the protocol surface.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::CapabilityId;

macro_rules! string_value_object {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Create the value object after trimming surrounding whitespace.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into().trim().to_string())
            }

            /// Return the raw string used by storage, logs, and wire payloads.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Report whether the value carries no usable content.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_value_object!(
    AgentIdentityId,
    "Stable identity for an A2A requester or provider agent."
);
string_value_object!(
    QuoteId,
    "Stable quote identity returned by an A2A provider."
);
string_value_object!(
    PaymentIntentId,
    "Stable payment intent identity used across policy, execution, and audit."
);
string_value_object!(
    PaymentReceiptId,
    "Stable receipt identity recorded after a payment lifecycle completes."
);
string_value_object!(
    ExecutionProofId,
    "Stable execution proof identity used for replay and dispute evidence."
);

/// Agent identity visible to A2A negotiation and audit.
///
/// The `metadata` map is deliberately open-ended. Implementations may add
/// verified DID, enterprise account, local process, or plugin identity hints
/// without requiring kernel or protocol changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub id: AgentIdentityId,
    pub display_name: Option<String>,
    pub namespace: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AgentIdentity {
    /// Build a minimal identity with no provider-specific assumptions.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: AgentIdentityId::new(id),
            display_name: None,
            namespace: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Remote capability descriptor advertised by another agent or service.
///
/// Capabilities describe what can be requested. They do not decide policy or
/// routing; those remain runtime concerns handled by service registries,
/// payment policies, and adapter strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCapabilityDescriptor {
    pub capability_id: CapabilityId,
    pub provider: AgentIdentity,
    pub operation: String,
    pub description: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// String-backed payment rail.
///
/// Canonical constructors cover Phase 09 local simulation while custom values
/// preserve future rails, enterprise billing systems, and community adapters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PaymentRail(String);

impl PaymentRail {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_lowercase())
    }

    pub fn local_simulated() -> Self {
        Self("local_simulated".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_local_simulated(&self) -> bool {
        self.0 == "local_simulated"
    }
}

impl fmt::Display for PaymentRail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Provider-neutral asset code and payment rail tuple.
///
/// The asset code may represent test units, fiat-like accounting units,
/// enterprise credits, tokens, or future custom assets. The protocol never
/// interprets it as one concrete currency or chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentAsset {
    pub rail: PaymentRail,
    pub code: String,
    pub metadata: BTreeMap<String, String>,
}

impl PaymentAsset {
    /// Build an asset from a rail and arbitrary code.
    pub fn new(rail: PaymentRail, code: impl Into<String>) -> Self {
        Self {
            rail,
            code: code.into().trim().to_string(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Decimal-safe amount representation.
///
/// `quantity` is a string to avoid floating point rounding and to preserve
/// exact provider-specific precision until a concrete adapter validates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentAmount {
    pub quantity: String,
    pub asset: PaymentAsset,
}

impl PaymentAmount {
    /// Build an amount without imposing one fixed decimal precision.
    pub fn new(quantity: impl Into<String>, asset: PaymentAsset) -> Self {
        Self {
            quantity: quantity.into().trim().to_string(),
            asset,
        }
    }

    /// Parse the amount for policy comparison in Phase 09 deterministic tests.
    pub fn as_f64(&self) -> Result<f64, A2AError> {
        self.quantity
            .parse::<f64>()
            .map_err(|_| A2AError::InvalidAmount(self.quantity.clone()))
    }
}

/// Payment terms returned with a quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTerms {
    pub amount: PaymentAmount,
    pub billing_unit: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub requires_explicit_approval: bool,
    pub metadata: BTreeMap<String, String>,
}

/// Build provider-neutral local simulated payment terms for tests and scaffolds.
///
/// This factory centralizes the canonical `local_simulated` rail configuration so
/// bootstrap code, integration tests, and SDK fixtures share one contract shape
/// without depending on kernel-owned payment coordinators. The helper is
/// intentionally parameterised by quantity and asset code so different harnesses
/// can express budget scenarios while keeping rail semantics stable.
pub fn local_simulated_terms(
    quantity: impl Into<String>,
    asset_code: impl Into<String>,
) -> PaymentTerms {
    PaymentTerms {
        amount: PaymentAmount::new(
            quantity,
            PaymentAsset::new(PaymentRail::local_simulated(), asset_code),
        ),
        billing_unit: "operation".into(),
        expires_at: None,
        requires_explicit_approval: false,
        metadata: BTreeMap::from([("simulation".into(), "true".into())]),
    }
}

/// Command sent by a requester to ask a provider for priced capability terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub requester: AgentIdentity,
    pub provider: AgentIdentity,
    pub capability: RemoteCapabilityDescriptor,
    pub operation: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Provider response containing a quote snapshot and proposed terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote_id: QuoteId,
    pub request: QuoteRequest,
    pub terms: PaymentTerms,
    pub quoted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Canonical payment intent states with string-backed extensibility.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PaymentIntentState(String);

impl PaymentIntentState {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_lowercase())
    }

    pub fn created() -> Self {
        Self("created".into())
    }

    pub fn quoted() -> Self {
        Self("quoted".into())
    }

    pub fn pending_approval() -> Self {
        Self("pending_approval".into())
    }

    pub fn approved() -> Self {
        Self("approved".into())
    }

    pub fn executing() -> Self {
        Self("executing".into())
    }

    pub fn settled() -> Self {
        Self("settled".into())
    }

    pub fn receipt_recorded() -> Self {
        Self("receipt_recorded".into())
    }

    pub fn rejected() -> Self {
        Self("rejected".into())
    }

    pub fn failed() -> Self {
        Self("failed".into())
    }

    pub fn dispute_possible() -> Self {
        Self("dispute_possible".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate the canonical Phase 09 state machine.
    pub fn can_transition_to(&self, next: &Self) -> bool {
        matches!(
            (self.as_str(), next.as_str()),
            ("created", "quoted")
                | ("quoted", "pending_approval")
                | ("pending_approval", "approved")
                | ("approved", "executing")
                | ("executing", "settled")
                | ("settled", "receipt_recorded")
                | ("quoted", "rejected")
                | ("approved", "failed")
                | ("failed", "dispute_possible")
        )
    }
}

impl fmt::Display for PaymentIntentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Payment command evaluated by policy before adapter execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub intent_id: PaymentIntentId,
    pub quote_id: QuoteId,
    pub requester: AgentIdentity,
    pub provider: AgentIdentity,
    pub capability_id: CapabilityId,
    pub amount: PaymentAmount,
    pub state: PaymentIntentState,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Budget policy input used by kernel/service policy evaluators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetPolicy {
    pub max_amount: Option<PaymentAmount>,
    pub auto_approve_local_simulated_below: Option<PaymentAmount>,
    pub metadata: BTreeMap<String, String>,
}

/// Approval policy input that records whether explicit approval exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub explicit_approval_required: bool,
    pub explicit_approval_granted: bool,
    pub approver: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Immutable receipt recorded after execution/settlement simulation succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentReceipt {
    pub receipt_id: PaymentReceiptId,
    pub quote_id: QuoteId,
    pub intent_id: PaymentIntentId,
    pub requester: AgentIdentity,
    pub provider: AgentIdentity,
    pub capability_id: CapabilityId,
    pub amount: PaymentAmount,
    pub status: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Execution proof memento used for audit replay and dispute evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionProof {
    pub proof_id: ExecutionProofId,
    pub intent_id: PaymentIntentId,
    pub receipt_id: Option<PaymentReceiptId>,
    pub operation: String,
    pub status: String,
    pub recorded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Structured A2A/payment errors used across proto, policy, and persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum A2AError {
    #[error("invalid payment amount: {0}")]
    InvalidAmount(String),
    #[error("over budget: {0}")]
    OverBudget(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
    #[error("payment adapter unavailable: {0}")]
    AdapterUnavailable(String),
    #[error("invalid payment intent transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    #[error("payment store error: {0}")]
    Store(String),
    #[error("payment execution failed: {0}")]
    ExecutionFailed(String),
}
