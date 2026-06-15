//! Application-level optional Web3 capability metadata.
//!
//! Applications can declare that they may need Web3 without instantiating a
//! provider. The runtime treats this as policy input and capability metadata,
//! preserving the rule that application framework code must not own
//! concrete Web3 node, wallet, signing, or transaction implementations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::{ChainId, Web3CapabilityKind};

/// Optional Web3 capability requested by an application or package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppWeb3CapabilityRequest {
    pub capability: Web3CapabilityKind,
    pub chain_id: Option<ChainId>,
    pub required: bool,
    pub metadata: BTreeMap<String, String>,
}

impl AppWeb3CapabilityRequest {
    /// Build an optional capability request.
    pub fn optional(capability: Web3CapabilityKind) -> Self {
        Self {
            capability,
            chain_id: None,
            required: false,
            metadata: BTreeMap::new(),
        }
    }

    /// Record a structured log when application metadata is converted to
    /// policy input. This is intentionally not a provider call.
    pub fn log_policy_input(&self) {
        info!(
            capability = %self.capability,
            chain_id = ?self.chain_id,
            required = self.required,
            "application web3 capability metadata prepared for policy"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web3_capability_request_is_metadata_only() {
        let request = AppWeb3CapabilityRequest::optional(Web3CapabilityKind::chain_query());
        let decoded: AppWeb3CapabilityRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(decoded.capability, Web3CapabilityKind::chain_query());
        assert!(!decoded.required);
        assert!(decoded.chain_id.is_none());
    }
}
