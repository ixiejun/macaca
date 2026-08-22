//! Contract tests for auth-handoff provider Strategies.

use std::collections::BTreeSet;

use super::identity_auth_handoff_strategy::{
    AuthHandoffProviderStrategy, ConfiguredAuthHandoffStrategy,
};

#[test]
fn strategy_supports_protocol_capability_gaps_without_name_routing() {
    let strategy = ConfiguredAuthHandoffStrategy::new(BTreeSet::from(["oidc_reference".into()]));
    assert!(strategy.validate_protocol(Some("oidc_reference")).accepted);
    assert!(!strategy.validate_protocol(Some("saml_reference")).accepted);
    assert_eq!(strategy.protocol_profiles().len(), 1);
}

#[test]
fn strategy_normalizes_provider_errors_to_bounded_codes() {
    let strategy = ConfiguredAuthHandoffStrategy::synthetic();
    let decision = strategy.normalize_error("rate_limit", true);
    assert!(!decision.accepted);
    assert_eq!(decision.reason_code, "provider_quota");
    assert!(decision.retriable);
}
