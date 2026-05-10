//! Provider-neutral Optional EVM / DApp Module contracts for Route C Phase 11.
//!
//! This module intentionally defines data contracts only. It does not encode
//! one chain, provider, wallet, token, DApp, ABI codec, RPC transport, or
//! contract runtime. Runtime crates can compose these value objects with
//! optional facades and adapters so Macaca remains usable when EVM support is
//! absent, disabled, or replaced by a user-provided implementation.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

macro_rules! evm_string_value {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Create a normalized provider-neutral value after trimming whitespace.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into().trim().to_string())
            }

            /// Return the raw string so adapters can map it to provider-specific values.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Report whether the identifier is empty after normalization.
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

evm_string_value!(
    EvmChainId,
    "Provider-neutral EVM-compatible chain identifier."
);
evm_string_value!(
    EvmRequestId,
    "Stable EVM request identifier used for trace and audit."
);
evm_string_value!(
    ContractAddress,
    "Provider-neutral contract address or address-like identifier."
);
evm_string_value!(
    ContractAbiRef,
    "Reference to ABI metadata without embedding provider-specific ABI logic."
);
evm_string_value!(
    ContractFunctionRef,
    "Reference to a contract function or method."
);
evm_string_value!(
    EvmTransactionId,
    "Stable transaction identifier returned by an EVM adapter."
);
evm_string_value!(
    EvmReceiptId,
    "Stable receipt identifier used for replay and audit."
);

/// Bounded value policy for gas, fees, and execution limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPolicy {
    pub max_gas: Option<String>,
    pub max_fee: Option<String>,
    pub priority_fee: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for GasPolicy {
    fn default() -> Self {
        Self {
            max_gas: None,
            max_fee: None,
            priority_fee: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Replaceable policy input evaluated before any adapter executes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmExecutionPolicy {
    pub explicit_approval_required: bool,
    pub explicit_approval_granted: bool,
    pub signing_allowed: bool,
    pub payment_allowed: bool,
    pub gas_allowed: bool,
    pub region_allowed: bool,
    pub module_enabled: bool,
    pub compliance_allowed: bool,
    pub metadata: BTreeMap<String, String>,
}

impl Default for EvmExecutionPolicy {
    fn default() -> Self {
        Self {
            explicit_approval_required: true,
            explicit_approval_granted: false,
            signing_allowed: true,
            payment_allowed: true,
            gas_allowed: true,
            region_allowed: true,
            module_enabled: true,
            compliance_allowed: true,
            metadata: BTreeMap::new(),
        }
    }
}

/// Availability state for the optional EVM/DApp module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmAvailability {
    pub state: String,
    pub capabilities: Vec<String>,
    pub reason_code: Option<String>,
    pub reason_message: Option<String>,
    pub checked_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl EvmAvailability {
    /// Build an available state with provider-neutral capability names.
    pub fn available(capabilities: Vec<String>) -> Self {
        Self {
            state: "available".into(),
            capabilities,
            reason_code: None,
            reason_message: None,
            checked_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Build a structured unavailable state without panicking callers.
    pub fn unavailable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            state: "unavailable".into(),
            capabilities: Vec::new(),
            reason_code: Some(code.into()),
            reason_message: Some(message.into()),
            checked_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether an adapter can execute commands.
    pub fn is_available(&self) -> bool {
        self.state == "available"
    }
}

/// Command asking an adapter to deploy a contract artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDeployRequest {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub wallet_id: Option<String>,
    pub artifact_ref: String,
    pub abi_ref: ContractAbiRef,
    pub constructor_args_digest: Option<String>,
    pub gas_policy: GasPolicy,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Memento returned after a deploy command succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDeployResult {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub transaction_id: Option<EvmTransactionId>,
    pub status: String,
    pub recorded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for a state-changing contract call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCallRequest {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub wallet_id: Option<String>,
    pub contract_address: ContractAddress,
    pub abi_ref: ContractAbiRef,
    pub function: ContractFunctionRef,
    pub args_digest: Option<String>,
    pub value: Option<String>,
    pub gas_policy: GasPolicy,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Memento returned after a state-changing call succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCallResult {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub transaction_id: Option<EvmTransactionId>,
    pub status: String,
    pub result_digest: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for bounded read-only contract state access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractReadRequest {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub abi_ref: ContractAbiRef,
    pub function: ContractFunctionRef,
    pub args_digest: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Bounded read response safe for trace and UI surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractReadResult {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub status: String,
    pub result_digest: Option<String>,
    pub responded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for contract event subscription setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractEventSubscription {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub event_filter: Option<String>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Redacted event emitted by a provider adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractEvent {
    pub subscription_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub contract_address: ContractAddress,
    pub event_name: String,
    pub payload_digest: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Deterministic gas estimate result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasEstimate {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub estimated_gas: String,
    pub status: String,
    pub estimated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Query command for transaction receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionReceiptQuery {
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub transaction_id: EvmTransactionId,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Provider-normalized receipt memento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmTransactionReceipt {
    pub receipt_id: EvmReceiptId,
    pub request_id: EvmRequestId,
    pub chain_id: EvmChainId,
    pub transaction_id: EvmTransactionId,
    pub status: String,
    pub recorded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Structured EVM errors used by optional module boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum EvmError {
    #[error("evm unavailable: {0}")]
    Unavailable(String),
    #[error("evm disabled by policy: {0}")]
    DisabledByPolicy(String),
    #[error("evm region blocked: {0}")]
    RegionBlocked(String),
    #[error("evm approval required: {0}")]
    ApprovalRequired(String),
    #[error("evm policy denied: {0}")]
    PolicyDenied(String),
    #[error("evm adapter failed: {0}")]
    AdapterFailed(String),
}
