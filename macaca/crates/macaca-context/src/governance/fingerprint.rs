//! Cryptographic fingerprint helpers for reproducible governance audit trails.
//!
//! We fingerprint the **serialized** `ContextGovernanceRuntimeConfig` as received from
//! configuration — not the composed provider list — so two identical configs always produce the
//! same hash even if the provider registry changes independently.

use macaca_proto::config::ContextGovernanceRuntimeConfig;
use sha2::{Digest, Sha256};

/// Returns a lowercase hex-encoded SHA-256 digest over the canonical JSON bytes of `cfg`.
///
/// ### Why JSON instead of Debug formatting?
/// - `serde` field order follows the struct definition and remains stable across builds.
/// - The digest is meant for operator correlation (`same policy == same fingerprint`).
pub fn governance_config_fingerprint(cfg: &ContextGovernanceRuntimeConfig) -> String {
    let blob = serde_json::to_vec(cfg).unwrap_or_default();
    let digest = Sha256::digest(blob);
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::config::ContextGovernanceRuntimeConfig;

    /// Identical serialized configs must yield identical audit fingerprints (operator correlation).
    #[test]
    fn fingerprint_is_stable_for_same_config() {
        let a = ContextGovernanceRuntimeConfig::default();
        let b = ContextGovernanceRuntimeConfig::default();
        assert_eq!(
            governance_config_fingerprint(&a),
            governance_config_fingerprint(&b)
        );
    }

    /// Mutating a single governance knob changes the fingerprint so reports reflect policy drift.
    #[test]
    fn fingerprint_changes_when_policy_changes() {
        let a = ContextGovernanceRuntimeConfig::default();
        let mut b = ContextGovernanceRuntimeConfig::default();
        b.per_provider_timeout_ms = a.per_provider_timeout_ms.saturating_add(1);
        assert_ne!(
            governance_config_fingerprint(&a),
            governance_config_fingerprint(&b)
        );
    }
}
