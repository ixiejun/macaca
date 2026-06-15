//! ServiceRuntime-facing EVM Service contracts.
//!
//! `evm.rs` defines provider-neutral EVM value objects.  This module defines
//! the service command layer that SDK clients and runtime-host providers share.
//! The separation keeps EVM optional, replaceable, traceable, and safe to
//! expose in snapshots without leaking ABI, bytecode, keys, credentials, or
//! provider-specific response bodies.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ContractCallRequest, ContractCallResult, ContractDeployRequest, ContractDeployResult,
    ContractEvent, ContractEventSubscription, ContractReadRequest, ContractReadResult,
    EvmAvailability, OptionalProviderDescriptor, TraceContext, TransactionReceiptQuery,
};

/// Stable EVM service id used by ServiceRuntime, SDK clients, and snapshots.
pub const EVM_SERVICE_ID: &str = "macaca.evm";

/// Command name for optional EVM availability inspection.
pub const EVM_AVAILABILITY_COMMAND: &str = "evm.availability.get";
/// Command name for contract deploy admission.
pub const EVM_CONTRACT_DEPLOY_COMMAND: &str = "evm.contract.deploy";
/// Command name for state-changing contract call admission.
pub const EVM_CONTRACT_CALL_COMMAND: &str = "evm.contract.call";
/// Command name for bounded contract read admission.
pub const EVM_CONTRACT_READ_COMMAND: &str = "evm.contract.read";
/// Command name for gas estimate.
pub const EVM_GAS_ESTIMATE_COMMAND: &str = "evm.gas.estimate";
/// Command name for transaction receipt query.
pub const EVM_RECEIPT_GET_COMMAND: &str = "evm.receipt.get";
/// Command name for contract event subscription admission.
pub const EVM_EVENT_SUBSCRIBE_COMMAND: &str = "evm.event.subscribe";
/// Command name for sanitized EVM service snapshot.
pub const EVM_SNAPSHOT_COMMAND: &str = "evm.snapshot";

/// Read-only EVM availability command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmAvailabilityCommand {
    pub trace: TraceContext,
}

/// Mutating contract deploy command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmContractDeployCommand {
    pub trace: TraceContext,
    pub request: ContractDeployRequest,
}

/// Mutating contract call command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmContractCallCommand {
    pub trace: TraceContext,
    pub request: ContractCallRequest,
}

/// Read-only contract state command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmContractReadCommand {
    pub trace: TraceContext,
    pub request: ContractReadRequest,
}

/// Gas-estimate command.  It is read-only, but still traceable for audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmGasEstimateCommand {
    pub trace: TraceContext,
    pub request_id: crate::EvmRequestId,
}

/// Receipt query command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmReceiptGetCommand {
    pub trace: TraceContext,
    pub request: TransactionReceiptQuery,
}

/// Event subscription admission command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmEventSubscribeCommand {
    pub trace: TraceContext,
    pub request: ContractEventSubscription,
}

/// Sanitized snapshot command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmSnapshotCommand {
    pub trace: TraceContext,
}

/// Result returned by EVM deploy admission/execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmDeployAdmission {
    pub result: Option<ContractDeployResult>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub admitted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned by EVM contract-call admission/execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmCallAdmission {
    pub result: Option<ContractCallResult>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub admitted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned by EVM read admission/execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmReadAdmission {
    pub result: Option<ContractReadResult>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub responded_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned by EVM event subscription admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmEventSubscriptionAdmission {
    pub event: Option<ContractEvent>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub admitted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Sanitized EVM Service snapshot safe for Web, CLI, trace, and audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmServiceSnapshot {
    pub service_id: String,
    pub health: String,
    pub availability: EvmAvailability,
    pub provider: OptionalProviderDescriptor,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl EvmServiceSnapshot {
    /// Build an unavailable snapshot for SDK Null Objects and providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            service_id: EVM_SERVICE_ID.into(),
            health: "unavailable".into(),
            availability: EvmAvailability::unavailable("unavailable", reason.clone()),
            provider: OptionalProviderDescriptor::unavailable(EVM_SERVICE_ID, reason.clone()),
            diagnostics: vec![reason],
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Redacted operation summary used by snapshots, trace, and tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvmOperationSummary {
    pub operation: String,
    pub status: String,
    pub request_id: String,
    pub provider: OptionalProviderDescriptor,
    pub observed_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_snapshot_marks_provider_non_real_chain() {
        let snapshot = EvmServiceSnapshot::unavailable("evm missing");
        assert!(!snapshot.provider.real_chain);
        assert_eq!(snapshot.health, "unavailable");
    }
}
