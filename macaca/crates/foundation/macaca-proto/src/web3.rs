//! Provider-neutral Optional Web3 Node Module contracts.
//!
//! These contracts describe Web3 availability, wallet/signing commands,
//! transaction commands, chain queries, and audit-friendly outcomes without
//! binding Macaca to any concrete chain, node provider, wallet provider, RPC
//! transport, token, DApp, or payment route. Runtime crates use these value
//! objects behind optional service facades so base OS flows continue to work
//! when Web3 is absent.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

macro_rules! web3_string_value {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Create a normalized string-backed value after trimming whitespace.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into().trim().to_string())
            }

            /// Return the raw provider-neutral identifier.
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

web3_string_value!(
    WalletId,
    "Stable wallet identifier scoped by a Web3 provider or module."
);
web3_string_value!(ChainId, "Provider-neutral chain or network identifier.");
web3_string_value!(
    Web3Address,
    "Provider-neutral account or contract address value."
);
web3_string_value!(
    Web3RequestId,
    "Stable Web3 request identifier used for trace and audit."
);
web3_string_value!(
    Web3TransactionId,
    "Stable transaction identifier returned by an adapter."
);
web3_string_value!(
    Web3ReceiptId,
    "Stable transaction receipt identifier for replay and audit."
);

/// String-backed Web3 capability kind advertised by optional modules.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Web3CapabilityKind(String);

impl Web3CapabilityKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_lowercase())
    }

    pub fn wallet() -> Self {
        Self("wallet".into())
    }

    pub fn signing() -> Self {
        Self("signing".into())
    }

    pub fn transaction() -> Self {
        Self("transaction".into())
    }

    pub fn chain_query() -> Self {
        Self("chain_query".into())
    }

    pub fn payment_adapter() -> Self {
        Self("payment_adapter".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Web3CapabilityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reason a Web3 module or capability is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3UnavailableReason {
    pub code: String,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl Web3UnavailableReason {
    /// Build a structured reason without relying on provider-specific errors.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Availability state for an optional Web3 module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3Availability {
    pub state: String,
    pub capabilities: Vec<Web3CapabilityKind>,
    pub reason: Option<Web3UnavailableReason>,
    pub checked_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl Web3Availability {
    pub fn available(capabilities: Vec<Web3CapabilityKind>) -> Self {
        Self {
            state: "available".into(),
            capabilities,
            reason: None,
            checked_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn unavailable(reason: Web3UnavailableReason) -> Self {
        Self {
            state: "unavailable".into(),
            capabilities: Vec::new(),
            reason: Some(reason),
            checked_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn disabled_by_policy(reason: Web3UnavailableReason) -> Self {
        let mut value = Self::unavailable(reason);
        value.state = "disabled_by_policy".into();
        value
    }

    pub fn region_blocked(reason: Web3UnavailableReason) -> Self {
        let mut value = Self::unavailable(reason);
        value.state = "region_blocked".into();
        value
    }

    pub fn is_available(&self) -> bool {
        self.state == "available"
    }
}

/// Policy input used before signing or transaction execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningPolicy {
    pub explicit_approval_required: bool,
    pub explicit_approval_granted: bool,
    pub region_allowed: bool,
    pub module_enabled: bool,
    pub max_fee: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for SigningPolicy {
    fn default() -> Self {
        Self {
            explicit_approval_required: true,
            explicit_approval_granted: false,
            region_allowed: true,
            module_enabled: true,
            max_fee: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Command asking a wallet/signing provider to sign bounded payload metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningRequest {
    pub request_id: Web3RequestId,
    pub wallet_id: WalletId,
    pub chain_id: ChainId,
    pub address: Web3Address,
    pub operation: String,
    pub payload_digest: String,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Decision produced by policy before a signing adapter may execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningDecision {
    pub request_id: Web3RequestId,
    pub allowed: bool,
    pub reason: String,
    pub decided_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Bounded signing proof returned by mock or future real adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningProof {
    pub request_id: Web3RequestId,
    pub proof: String,
    pub algorithm: String,
    pub created_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command asking a Web3 adapter to submit or simulate a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub request_id: Web3RequestId,
    pub wallet_id: WalletId,
    pub chain_id: ChainId,
    pub from: Web3Address,
    pub to: Option<Web3Address>,
    pub value: Option<String>,
    pub asset: Option<String>,
    pub method: String,
    pub payload_digest: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Receipt memento produced after a transaction adapter succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub receipt_id: Web3ReceiptId,
    pub request_id: Web3RequestId,
    pub transaction_id: Web3TransactionId,
    pub chain_id: ChainId,
    pub status: String,
    pub recorded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command asking a Web3 adapter to query chain state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainQueryRequest {
    pub request_id: Web3RequestId,
    pub chain_id: ChainId,
    pub method: String,
    pub params_digest: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Bounded chain query response safe for trace and shell surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainQueryResponse {
    pub request_id: Web3RequestId,
    pub chain_id: ChainId,
    pub status: String,
    pub result_digest: Option<String>,
    pub responded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Structured Web3 errors used by optional module boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum Web3Error {
    #[error("web3 unavailable: {0}")]
    Unavailable(String),
    #[error("web3 disabled by policy: {0}")]
    DisabledByPolicy(String),
    #[error("web3 region blocked: {0}")]
    RegionBlocked(String),
    #[error("web3 approval required: {0}")]
    ApprovalRequired(String),
    #[error("web3 adapter failed: {0}")]
    AdapterFailed(String),
}
