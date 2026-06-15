//! Application-level optional DApp/EVM capability metadata.
//!
//! Applications can declare optional `web3.evm` needs as metadata and policy
//! input. This file deliberately does not instantiate providers, wallets,
//! browser bridges, Substrate/Frontier nodes, RPC transports, or contract
//! runtimes. That separation keeps application framework code generic across
//! all applications and preserves the microkernel/service boundary.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::EvmChainId;

/// Optional DApp/EVM capability requested by an application or package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppDappCapabilityRequest {
    pub capability: String,
    pub chain_id: Option<EvmChainId>,
    pub required: bool,
    pub metadata: BTreeMap<String, String>,
}

impl AppDappCapabilityRequest {
    /// Build an optional EVM capability request without provider ownership.
    pub fn optional_evm() -> Self {
        Self {
            capability: "web3.evm".into(),
            chain_id: None,
            required: false,
            metadata: BTreeMap::new(),
        }
    }

    /// Build a required EVM capability request for policy evaluation.
    pub fn required_evm(chain_id: Option<EvmChainId>) -> Self {
        Self {
            capability: "web3.evm".into(),
            chain_id,
            required: true,
            metadata: BTreeMap::new(),
        }
    }

    /// Record that application metadata became policy input, not a provider call.
    pub fn log_policy_input(&self) {
        info!(
            capability = %self.capability,
            chain_id = ?self.chain_id,
            required = self.required,
            "application dapp capability metadata prepared for policy"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dapp_capability_request_is_metadata_only() {
        let request = AppDappCapabilityRequest::optional_evm();
        let decoded: AppDappCapabilityRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(decoded.capability, "web3.evm");
        assert!(!decoded.required);
        assert!(decoded.chain_id.is_none());
    }
}
