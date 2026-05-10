//! IPC bridge contracts for Optional Web3 Node v0.
//!
//! The bridge models Web3 calls as transport-neutral commands. It does not
//! choose local node, remote RPC, wallet, or signing implementations. Service
//! bus adapters can serialize these commands and route them to a registered
//! optional Web3 provider when one exists.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::{
    ChainQueryRequest, SigningRequest, TransactionRequest, Web3CapabilityKind, Web3RequestId,
};

/// Transport-neutral Web3 service call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Web3ServiceCall {
    Availability { request_id: Web3RequestId },
    Sign { request: SigningRequest },
    SubmitTransaction { request: TransactionRequest },
    QueryChain { request: ChainQueryRequest },
}

impl Web3ServiceCall {
    /// Return the capability required to handle this service call.
    pub fn capability(&self) -> Web3CapabilityKind {
        match self {
            Self::Availability { .. } => Web3CapabilityKind::new("availability"),
            Self::Sign { .. } => Web3CapabilityKind::signing(),
            Self::SubmitTransaction { .. } => Web3CapabilityKind::transaction(),
            Self::QueryChain { .. } => Web3CapabilityKind::chain_query(),
        }
    }

    /// Return a stable request id for trace correlation.
    pub fn request_id(&self) -> &Web3RequestId {
        match self {
            Self::Availability { request_id } => request_id,
            Self::Sign { request } => &request.request_id,
            Self::SubmitTransaction { request } => &request.request_id,
            Self::QueryChain { request } => &request.request_id,
        }
    }

    /// Emit a structured bridge log before routing through IPC.
    pub fn log_dispatch(&self) {
        info!(
            request_id = %self.request_id(),
            capability = %self.capability(),
            timestamp = %Utc::now(),
            "web3 IPC service call dispatch prepared"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use macaca_proto::{ChainId, WalletId, Web3Address};

    #[test]
    fn web3_ipc_call_is_serializable_and_capability_scoped() {
        let call = Web3ServiceCall::Sign {
            request: SigningRequest {
                request_id: Web3RequestId::new("ipc.sign"),
                wallet_id: WalletId::new("wallet.ipc"),
                chain_id: ChainId::new("chain.ipc"),
                address: Web3Address::new("address.ipc"),
                operation: "sign_digest".into(),
                payload_digest: "digest".into(),
                session_id: None,
                task_id: None,
                requested_at: Utc::now(),
                metadata: BTreeMap::new(),
            },
        };
        let decoded: Web3ServiceCall =
            serde_json::from_str(&serde_json::to_string(&call).unwrap()).unwrap();
        assert_eq!(decoded.capability(), Web3CapabilityKind::signing());
        assert_eq!(decoded.request_id().as_str(), "ipc.sign");
    }
}
