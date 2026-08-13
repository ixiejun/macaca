//! Tests for random admission and redaction specifications.

use std::collections::BTreeSet;

use super::foundation_random::*;
use super::foundation_random_semantics::*;

fn context() -> RandomPolicyContext {
    RandomPolicyContext {
        declared_scopes: BTreeSet::from([
            "random.generate".into(),
            "random.identifier".into(),
            "random.token".into(),
            "random.nonce".into(),
            "random.test_seed".into(),
        ]),
        policy_allowed: true,
        provider_available: true,
        entropy_available: true,
        provider_blocked: false,
        supports_bias_free_integer: true,
        supports_uuid_v4: true,
        supports_deterministic_streams: true,
        replay_context: RandomReplayPolicy::TestOnly,
        max_bytes_per_request: 64,
        max_blocking_ms: 100,
        max_token_length: 64,
        limits: RandomResourceLimits {
            max_byte_units: 128,
            max_token_units: 128,
            max_request_units: 4,
            max_deterministic_streams: 1,
        },
        current: RandomResourceReservation::default(),
    }
}

fn bytes() -> RandomBytesCommand {
    RandomBytesCommand {
        length: 16,
        strength: RandomStrengthClass::Cryptographic,
        purpose: RandomPurpose::Nonce,
        encoding: RandomOutputEncoding::Hex,
        max_blocking_ms: Some(50),
    }
}

#[test]
fn denied_unavailable_and_quota_requests_never_dispatch() {
    let mut denied = context();
    denied.declared_scopes.clear();
    let mut called = false;
    assert_eq!(
        dispatch_after_preflight(preflight_bytes(&bytes(), &denied), || called = true),
        Err(RandomAdmissionFailure::PermissionNotDeclared)
    );
    assert!(!called);
    let mut unavailable = context();
    unavailable.provider_available = false;
    assert_eq!(
        preflight_bytes(&bytes(), &unavailable),
        Err(RandomAdmissionFailure::ProviderUnavailable)
    );
    let mut quota = context();
    quota.current.request_units = 4;
    assert_eq!(
        preflight_bytes(&bytes(), &quota),
        Err(RandomAdmissionFailure::QuotaExceeded)
    );
}

#[test]
fn blocked_entropy_and_unsupported_features_have_stable_statuses() {
    let mut blocked = context();
    blocked.provider_blocked = true;
    assert_eq!(
        preflight_bytes(&bytes(), &blocked),
        Err(RandomAdmissionFailure::ProviderBlocked)
    );
    let mut entropy = context();
    entropy.entropy_available = false;
    assert_eq!(
        preflight_bytes(&bytes(), &entropy),
        Err(RandomAdmissionFailure::EntropyUnavailable)
    );
    let mut integer = context();
    integer.supports_bias_free_integer = false;
    assert_eq!(
        preflight_integer(
            &RandomIntegerCommand {
                min_inclusive: 0,
                max_exclusive: 4,
                purpose: RandomPurpose::Generic,
                require_bias_free: true
            },
            &integer
        ),
        Err(RandomAdmissionFailure::Unsupported)
    );
}

#[test]
fn deterministic_streams_are_test_or_replay_only() {
    let command = RandomTestStreamCreateCommand {
        seed: RandomSeedReference {
            seed_ref: "seed:test:opaque".into(),
            replay_binding: "replay:1".into(),
        },
        algorithm_id: "test.counter.v1".into(),
        replay_policy: RandomReplayPolicy::TestOnly,
    };
    assert!(preflight_test_stream(&command, &context()).is_ok());
    let mut production = context();
    production.replay_context = RandomReplayPolicy::ProductionDenied;
    assert_eq!(
        preflight_test_stream(&command, &production),
        Err(RandomAdmissionFailure::DeterministicNotAllowed)
    );
}

#[test]
fn token_identifier_and_audit_projections_remain_bounded_and_redacted() {
    let token = RandomTokenCommand {
        char_length: 16,
        alphabet: RandomAlphabetClass::UrlSafe,
        purpose: RandomPurpose::Generic,
        collision_warning_policy: "policy:collision".into(),
    };
    assert!(preflight_token(&token, &context()).is_ok());
    let nonce = RandomNonceCommand {
        byte_length: 16,
        purpose: RandomPurpose::Nonce,
        encoding: RandomOutputEncoding::Base64Url,
        uniqueness_window: Some("window:1".into()),
    };
    assert!(preflight_identifier(None, Some(&nonce), &context()).is_ok());
    let event = redacted_random_audit_fields("random.bytes", 16, "nonce", "trace:1").unwrap();
    let encoded = serde_json::to_string(&event).unwrap();
    assert!(!encoded.contains("seed"));
    assert!(redacted_random_audit_fields("random.token", 1, "token", "trace:1").is_none());
}
