use std::collections::{BTreeMap, BTreeSet};

use super::*;

// The random pack tests validate only provider-neutral contract metadata. They
// intentionally do not generate random values, call host RNG APIs, or mark the
// pack callable before a serviceized provider is installed.

#[test]
fn foundation_random_descriptor_is_discoverable_and_not_callable() {
    let definition = foundation_random_pack_definition();

    assert_eq!(definition.pack_id, FOUNDATION_RANDOM_PACK_ID);
    assert!(!definition.is_callable());
    assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
    assert_eq!(
        definition.metadata.parent_pack_id.as_deref(),
        Some("pack.foundation.v1")
    );
    assert_eq!(
        definition.metadata.diagnostics.unavailable_reason,
        "random_provider_not_installed"
    );
    assert!(definition
        .metadata
        .sdk
        .docs_url
        .contains("developer-packs/foundation/random"));

    let commands = definition
        .metadata
        .service_command_schemas
        .get(FOUNDATION_RANDOM_SERVICE_ID)
        .expect("random descriptor exposes command schemas");
    for command in FOUNDATION_RANDOM_COMMANDS {
        assert!(commands.contains(*command), "missing command {command}");
    }

    for scope in [
        "random.generate",
        "random.identifier",
        "random.token",
        "random.nonce",
        "random.health",
        "random.test_seed",
    ] {
        assert!(
            definition.metadata.permission_scopes.contains(scope),
            "missing permission scope {scope}"
        );
    }

    for provider_class in ["host-csprng", "deterministic-test", "mock", "unavailable"] {
        assert!(
            definition
                .metadata
                .provider_descriptors
                .contains_key(provider_class),
            "missing provider descriptor {provider_class}"
        );
    }
}

#[test]
fn industrial_catalog_uses_foundation_random_contract_descriptor() {
    let definition = industrial_reference_domain_pack_definitions()
        .into_iter()
        .find(|definition| definition.pack_id == FOUNDATION_RANDOM_PACK_ID)
        .expect("industrial catalog includes foundation random");

    assert!(!definition.is_callable());
    assert!(definition
        .metadata
        .service_command_schemas
        .contains_key(FOUNDATION_RANDOM_SERVICE_ID));
    assert_eq!(
        definition
            .metadata
            .provider_descriptors
            .get("deterministic-test")
            .and_then(|descriptor| descriptor.metadata.get("deterministic_test_streams"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn foundation_random_command_dtos_are_serde_compatible() {
    let stream = RandomStreamReference {
        stream_id: "stream-ref".into(),
        algorithm_id: "deterministic-test-v1".into(),
        position: 7,
        replay_binding: "trace-replay".into(),
    };
    let commands = vec![
        serde_json::to_value(RandomBytesCommand {
            length: 32,
            strength: RandomStrengthClass::Cryptographic,
            purpose: RandomPurpose::SessionId,
            encoding: RandomOutputEncoding::Base64Url,
            max_blocking_ms: Some(50),
        })
        .unwrap(),
        serde_json::to_value(RandomFillCommand {
            artifact_ref: "artifact-buffer".into(),
            offset: 0,
            length: 64,
            strength: RandomStrengthClass::StrongWhenAvailable,
            purpose: RandomPurpose::ProviderProtocol,
        })
        .unwrap(),
        serde_json::to_value(RandomIntegerCommand {
            min_inclusive: 10,
            max_exclusive: 20,
            purpose: RandomPurpose::Generic,
            require_bias_free: true,
        })
        .unwrap(),
        serde_json::to_value(RandomUuidV4Command {
            count: 2,
            lowercase: true,
        })
        .unwrap(),
        serde_json::to_value(RandomNonceCommand {
            byte_length: 16,
            purpose: RandomPurpose::Nonce,
            encoding: RandomOutputEncoding::Hex,
            uniqueness_window: Some("session".into()),
        })
        .unwrap(),
        serde_json::to_value(RandomTokenCommand {
            char_length: 24,
            alphabet: RandomAlphabetClass::UrlSafe,
            purpose: RandomPurpose::IdempotencyKey,
            collision_warning_policy: "warn_only".into(),
        })
        .unwrap(),
        serde_json::to_value(RandomTestStreamCreateCommand {
            seed: RandomSeedReference {
                seed_ref: "seed-ref".into(),
                replay_binding: "trace-replay".into(),
            },
            algorithm_id: "deterministic-test-v1".into(),
            replay_policy: RandomReplayPolicy::TestOnly,
        })
        .unwrap(),
        serde_json::to_value(RandomTestStreamBytesCommand {
            stream,
            length: 8,
            expected_position: 7,
        })
        .unwrap(),
        serde_json::to_value(RandomEntropyHealthCommand {
            include_blocking_risk: true,
            include_limits: true,
        })
        .unwrap(),
        serde_json::to_value(RandomProviderCapabilitiesCommand {
            include_preview: true,
            include_unavailable: true,
        })
        .unwrap(),
    ];

    assert_eq!(commands.len(), FOUNDATION_RANDOM_COMMANDS.len());
    assert!(commands.iter().all(|value| value.is_object()));
}

#[test]
fn foundation_random_hashes_change_with_contract_content() {
    let request = RandomBytesCommand {
        length: 32,
        strength: RandomStrengthClass::Cryptographic,
        purpose: RandomPurpose::SessionId,
        encoding: RandomOutputEncoding::RawBytes,
        max_blocking_ms: None,
    };
    let mut changed = request.clone();
    changed.length = 64;

    assert_eq!(random_stable_hash(&request), random_stable_hash(&request));
    assert_ne!(random_stable_hash(&request), random_stable_hash(&changed));

    let hashes = foundation_random_descriptor_hashes();
    let unique = BTreeSet::from([
        hashes.command_schema_hash,
        hashes.result_schema_hash,
        hashes.health_schema_hash,
        hashes.snapshot_schema_hash,
        hashes.provider_capability_schema_hash,
        hashes.unavailable_schema_hash,
    ]);
    assert_eq!(unique.len(), 6);
    assert!(unique.iter().all(|hash| !hash.is_empty()));
}

#[test]
fn foundation_random_result_and_snapshot_dtos_are_bounded() {
    let health = RandomEntropyHealth {
        provider_class: "unavailable".into(),
        entropy_available: false,
        blocking_risk: false,
        max_bytes_per_request: 0,
        unavailable_reason: Some("random_provider_not_installed".into()),
    };
    let snapshot = RandomProviderSnapshot {
        descriptor_hash: "descriptor-hash".into(),
        provider_class: "unavailable".into(),
        health: health.clone(),
        stream_position_hashes: BTreeMap::from([("stream-ref".into(), "position-hash".into())]),
    };
    let unavailable: RandomResultEnvelope<String> = RandomResultEnvelope {
        status: RandomResultStatus::Unavailable,
        data: None,
        error: Some(RandomError {
            code: RandomResultStatus::Unavailable,
            message: "random provider is not installed".into(),
            retryable: false,
        }),
        trace_id: "trace-random-unavailable".into(),
        descriptor_hash: random_stable_hash(&snapshot),
    };

    let serialized = serde_json::to_string(&unavailable).unwrap();
    assert!(serialized.contains("trace-random-unavailable"));
    assert!(!serialized.contains("seed-secret"));
    assert!(!serialized.contains("generated-token"));
    assert_eq!(snapshot.health, health);
}
