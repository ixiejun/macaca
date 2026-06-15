//! ServiceRuntime-facing Web3 Service contracts.
//!
//! The value objects in `web3.rs` describe provider-neutral Web3 concepts.
//! This module wraps those concepts in command/result DTOs that can cross the
//! `ServiceRuntime` boundary.  Keeping command DTOs separate preserves the
//! Command pattern: upper consumers dispatch operations by stable names while
//! runtime-host providers remain free to use unavailable, mock, remote, plugin,
//! or future real adapters behind the same contract.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ChainQueryRequest, SigningProof, SigningRequest, TraceContext, TransactionReceipt,
    TransactionRequest, Web3Availability,
};

/// Stable Web3 service id used by ServiceRuntime, SDK clients, and snapshots.
pub const WEB3_SERVICE_ID: &str = "macaca.web3";

/// Command name for optional Web3 availability inspection.
pub const WEB3_AVAILABILITY_COMMAND: &str = "web3.availability.get";
/// Command name for listing provider-neutral wallet descriptors.
pub const WEB3_WALLET_LIST_COMMAND: &str = "web3.wallet.list";
/// Command name for bounded signing-request admission.
pub const WEB3_SIGNING_REQUEST_COMMAND: &str = "web3.signing.request";
/// Command name for transaction preparation/admission.
pub const WEB3_TRANSACTION_PREPARE_COMMAND: &str = "web3.transaction.prepare";
/// Command name for bounded chain query.
pub const WEB3_CHAIN_QUERY_COMMAND: &str = "web3.chain.query";
/// Command name for sanitized Web3 service snapshot.
pub const WEB3_SNAPSHOT_COMMAND: &str = "web3.snapshot";

/// Provider class reported by optional module providers.
///
/// The enum intentionally avoids vendor, chain, or wallet names.  Runtime code
/// can reason about trust and safety without branching on business-specific
/// identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionalProviderClass {
    Unavailable,
    Mock,
    Local,
    Remote,
    Plugin,
}

/// Provider descriptor shared by Web3 and EVM optional services.
///
/// Descriptors are safe for snapshots and audit records.  They describe the
/// provider boundary without carrying credentials, private keys, raw payloads,
/// or provider-specific response bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalProviderDescriptor {
    pub provider_id: String,
    pub provider_class: OptionalProviderClass,
    pub available: bool,
    pub mock_only: bool,
    pub development_only: bool,
    pub real_chain: bool,
    pub trust_level: String,
    pub audit_mode: String,
    pub redaction_guarantees: Vec<String>,
    pub diagnostics: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl OptionalProviderDescriptor {
    /// Build an unavailable descriptor used by Null Object providers.
    pub fn unavailable(service_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            provider_id: service_id.into(),
            provider_class: OptionalProviderClass::Unavailable,
            available: false,
            mock_only: false,
            development_only: false,
            real_chain: false,
            trust_level: "none".into(),
            audit_mode: "bounded_diagnostics".into(),
            redaction_guarantees: vec!["no_secret_payloads".into()],
            diagnostics: vec![reason.into()],
            metadata: BTreeMap::new(),
        }
    }

    /// Build a deterministic mock/dev descriptor for no-network tests.
    pub fn mock(service_id: impl Into<String>) -> Self {
        Self {
            provider_id: service_id.into(),
            provider_class: OptionalProviderClass::Mock,
            available: true,
            mock_only: true,
            development_only: true,
            real_chain: false,
            trust_level: "development".into(),
            audit_mode: "bounded_mock_events".into(),
            redaction_guarantees: vec![
                "no_private_keys".into(),
                "no_raw_signed_transactions".into(),
                "no_provider_credentials".into(),
            ],
            diagnostics: vec!["mock provider does not perform real chain execution".into()],
            metadata: BTreeMap::new(),
        }
    }
}

/// Wallet descriptor returned by Web3 Service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3WalletView {
    pub wallet_id: String,
    pub label: Option<String>,
    pub capabilities: Vec<String>,
    pub provider: OptionalProviderDescriptor,
    pub metadata: BTreeMap<String, String>,
}

/// Read-only Web3 availability command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3AvailabilityCommand {
    pub trace: TraceContext,
}

/// Read-only wallet-list command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3WalletListCommand {
    pub trace: TraceContext,
}

/// Mutating Web3 signing command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3SigningRequestCommand {
    pub trace: TraceContext,
    pub request: SigningRequest,
}

/// Mutating Web3 transaction-preparation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3TransactionPrepareCommand {
    pub trace: TraceContext,
    pub request: TransactionRequest,
}

/// Read-only chain query command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3ChainQueryCommand {
    pub trace: TraceContext,
    pub request: ChainQueryRequest,
}

/// Sanitized snapshot command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3SnapshotCommand {
    pub trace: TraceContext,
}

/// Result returned when a signing request is admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3SigningAdmission {
    pub proof: Option<SigningProof>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub admitted_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned when a transaction is prepared or simulated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3TransactionPreparation {
    pub receipt: Option<TransactionReceipt>,
    pub provider: OptionalProviderDescriptor,
    pub status: String,
    pub reason: Option<String>,
    pub prepared_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

/// Sanitized Web3 Service snapshot safe for Web, CLI, trace, and audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3ServiceSnapshot {
    pub service_id: String,
    pub health: String,
    pub availability: Web3Availability,
    pub provider: OptionalProviderDescriptor,
    pub wallet_count: usize,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl Web3ServiceSnapshot {
    /// Build an unavailable snapshot for SDK Null Objects and providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            service_id: WEB3_SERVICE_ID.into(),
            health: "unavailable".into(),
            availability: Web3Availability::unavailable(crate::Web3UnavailableReason::new(
                "unavailable",
                reason.clone(),
            )),
            provider: OptionalProviderDescriptor::unavailable(WEB3_SERVICE_ID, reason.clone()),
            wallet_count: 0,
            diagnostics: vec![reason],
            captured_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Specification helpers shared by SDK tests and runtime-host providers.
pub struct OptionalServiceRedactionSpec;

impl OptionalServiceRedactionSpec {
    /// Reject metadata keys that are likely to carry secrets or raw payloads.
    ///
    /// The error only includes the key name, never the value.  This keeps
    /// admission diagnostics auditable without copying sensitive material into
    /// logs, trace events, or snapshots.
    pub fn check_metadata(metadata: &BTreeMap<String, String>) -> Result<(), String> {
        for key in metadata.keys() {
            let normalized = key.to_ascii_lowercase();
            if normalized.contains("secret")
                || normalized.contains("private_key")
                || normalized.contains("mnemonic")
                || normalized.contains("credential")
                || normalized.contains("api_key")
                || normalized.contains("raw_signature")
                || normalized.contains("signed_transaction")
                || normalized.contains("raw_transaction")
                || normalized.contains("raw_abi")
                || normalized.contains("bytecode")
            {
                return Err(format!(
                    "optional service redaction rejected sensitive metadata key '{key}'"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_snapshot_marks_provider_non_real_chain() {
        let snapshot = Web3ServiceSnapshot::unavailable("web3 missing");
        assert!(!snapshot.provider.real_chain);
        assert_eq!(
            snapshot.provider.provider_class,
            OptionalProviderClass::Unavailable
        );
    }

    #[test]
    fn redaction_spec_rejects_secret_like_metadata() {
        let metadata = BTreeMap::from([("wallet_private_key".into(), "value".into())]);
        let error = OptionalServiceRedactionSpec::check_metadata(&metadata).unwrap_err();
        assert!(error.contains("redaction"));
    }
}
