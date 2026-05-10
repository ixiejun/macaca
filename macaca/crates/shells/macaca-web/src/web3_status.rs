//! Thin-shell Web3 status DTOs for the web layer.
//!
//! The web crate only exposes optional Web3 availability or denial as data.
//! It does not define signing, transaction, chain-query, provider, wallet, or
//! policy semantics. Those remain in protocol, kernel facade, and optional
//! service implementations.

use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::Web3Availability;

/// Web-facing projection of optional Web3 availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Web3StatusView {
    pub state: String,
    pub reason_code: Option<String>,
    pub reason_message: Option<String>,
}

impl Web3StatusView {
    /// Convert provider-neutral availability into a shell-safe DTO.
    pub fn from_availability(availability: &Web3Availability) -> Self {
        info!(
            state = %availability.state,
            "web3 availability projected for web thin shell"
        );
        Self {
            state: availability.state.clone(),
            reason_code: availability
                .reason
                .as_ref()
                .map(|reason| reason.code.clone()),
            reason_message: availability
                .reason
                .as_ref()
                .map(|reason| reason.message.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::Web3UnavailableReason;

    #[test]
    fn web3_status_view_preserves_unavailable_as_data() {
        let availability =
            Web3Availability::unavailable(Web3UnavailableReason::new("missing", "not installed"));
        let view = Web3StatusView::from_availability(&availability);
        assert_eq!(view.state, "unavailable");
        assert_eq!(view.reason_code.as_deref(), Some("missing"));
    }
}
