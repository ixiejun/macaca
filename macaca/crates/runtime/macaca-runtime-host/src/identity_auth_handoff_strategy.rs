//! Replaceable provider Strategy for identity auth handoff.
//!
//! The Strategy sees only protocol/capability facts and opaque references. It
//! cannot own account lifecycle, profile writes, sessions, secrets, browsers,
//! or application login UI.

use std::collections::BTreeSet;

/// Sanitized provider outcome used by the runtime adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthHandoffStrategyDecision {
    pub accepted: bool,
    pub reason_code: &'static str,
    pub retriable: bool,
}

/// Strategy boundary for protocol support and provider error normalization.
pub trait AuthHandoffProviderStrategy: Send + Sync {
    /// Return supported protocol profiles without provider names.
    fn protocol_profiles(&self) -> &BTreeSet<String>;

    /// Validate a protocol request before provider side effects.
    fn validate_protocol(&self, protocol: Option<&str>) -> AuthHandoffStrategyDecision;

    /// Normalize an opaque provider failure into bounded OS vocabulary.
    fn normalize_error(&self, code: &str, retriable: bool) -> AuthHandoffStrategyDecision;
}

/// Deterministic Strategy used by the built-in mock and capability-gap tests.
#[derive(Debug, Clone)]
pub struct ConfiguredAuthHandoffStrategy {
    protocol_profiles: BTreeSet<String>,
}

impl ConfiguredAuthHandoffStrategy {
    /// Construct a Strategy from declared protocol profiles.
    pub fn new(protocol_profiles: BTreeSet<String>) -> Self {
        Self { protocol_profiles }
    }

    /// Full synthetic protocol support used by conformance tests.
    pub fn synthetic() -> Self {
        Self::new(BTreeSet::from([
            "oauth2_reference".into(),
            "oidc_reference".into(),
            "saml_reference".into(),
            "webauthn_reference".into(),
        ]))
    }
}

impl AuthHandoffProviderStrategy for ConfiguredAuthHandoffStrategy {
    fn protocol_profiles(&self) -> &BTreeSet<String> {
        &self.protocol_profiles
    }

    fn validate_protocol(&self, protocol: Option<&str>) -> AuthHandoffStrategyDecision {
        let Some(protocol) = protocol else {
            return AuthHandoffStrategyDecision {
                accepted: true,
                reason_code: "protocol_unspecified",
                retriable: false,
            };
        };
        if self.protocol_profiles.contains(protocol) {
            AuthHandoffStrategyDecision {
                accepted: true,
                reason_code: "accepted",
                retriable: false,
            }
        } else {
            AuthHandoffStrategyDecision {
                accepted: false,
                reason_code: "protocol_unsupported",
                retriable: false,
            }
        }
    }

    fn normalize_error(&self, code: &str, retriable: bool) -> AuthHandoffStrategyDecision {
        let reason_code = match code {
            "timeout" => "provider_timeout",
            "rate_limit" => "provider_quota",
            "invalid_request" => "provider_invalid_request",
            _ => "provider_failure",
        };
        AuthHandoffStrategyDecision {
            accepted: false,
            reason_code,
            retriable,
        }
    }
}
