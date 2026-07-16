use super::*;

#[test]
fn foundation_preflight_rejects_raw_values_paths_and_unbounded_state() {
    let config_secret = ConfigTypedValueRef {
        kind: ConfigValueKind::SecretReference,
        value_ref: "secret:config-password".into(),
        schema: None,
        secret_reference_required: true,
    };
    assert!(config_secret.is_admissible_reference());
    assert!(!ConfigTypedValueRef {
        value_ref: "DATABASE_PASSWORD=raw".into(),
        ..config_secret
    }
    .is_admissible_reference());

    let path = FilesystemPathRef {
        root: FilesystemRootRef {
            root_id: "workspace".into(),
            root_kind: "app_scoped".into(),
        },
        relative_path: "state/settings.json".into(),
    };
    assert!(path.is_safe_relative_path());
    assert!(!FilesystemPathRef {
        relative_path: "../private/key".into(),
        ..path
    }
    .is_safe_relative_path());

    let ttl = KeyValueTtlPolicy {
        ttl_seconds: Some(60),
        expire_at_epoch_millis: None,
    };
    assert!(ttl.is_bounded(120, 1));
    assert!(!KeyValueTtlPolicy {
        ttl_seconds: Some(0),
        expire_at_epoch_millis: None
    }
    .is_bounded(120, 1));

    let session_value = SessionStateValueRef {
        value_ref: "artifact:session-state".into(),
        schema_id: Some("schema-v1".into()),
        secret_reference_required: false,
    };
    assert!(session_value.is_admissible_reference());
    assert!(!SessionStateValueRef {
        value_ref: "raw-state-value\nsecret".into(),
        ..session_value
    }
    .is_admissible_reference());
}

#[test]
fn foundation_preflight_requires_declared_test_context_and_bounded_timer() {
    let stream = RandomTestStreamCreateCommand {
        seed: RandomSeedReference {
            seed_ref: "artifact:test-seed".into(),
            replay_binding: "replay-1".into(),
        },
        algorithm_id: "deterministic-test-v1".into(),
        replay_policy: RandomReplayPolicy::TestOnly,
    };
    assert!(stream.is_allowed_in_context(RandomReplayPolicy::TestOnly));
    assert!(!stream.is_allowed_in_context(RandomReplayPolicy::ProductionDenied));

    let timer = TimeCreateTimerCommand {
        duration: TimeDuration {
            millis: 1_000,
            nanos_adjustment: 0,
        },
        exactness: TimeExactnessHint::ExactPreferred,
        session_binding: "session-ref".into(),
    };
    assert!(timer.is_bounded_request(2_000));
    assert!(!TimeCreateTimerCommand {
        duration: TimeDuration {
            millis: 3_000,
            nanos_adjustment: 0
        },
        ..timer
    }
    .is_bounded_request(2_000));
}

#[test]
fn foundation_preflight_preserves_reference_only_secret_imports() {
    let import = SecretsImportReferenceCommand {
        locator: SecretExternalLocator {
            provider_class: "secret-store".into(),
            redacted_locator_hash: "locator-hash".into(),
        },
        purpose: SecretPurposeBinding {
            purpose: "runtime-auth".into(),
            service_id: "service.example".into(),
            expires_at_epoch_millis: Some(100),
        },
        policy: SecretAccessPolicy {
            allowed_service_ids: ["service.example".to_string()].into_iter().collect(),
            requires_approval: true,
            max_lease_ttl_seconds: 60,
        },
    };
    assert!(import.has_safe_preconditions(120));
    assert!(!SecretExternalLocator {
        provider_class: "secret-store".into(),
        redacted_locator_hash: "https://provider.example/raw".into()
    }
    .is_safe_reference());

    let write = FilesystemWriteFileCommand {
        path: FilesystemPathRef {
            root: FilesystemRootRef {
                root_id: "workspace".into(),
                root_kind: "app_scoped".into(),
            },
            relative_path: "output.txt".into(),
        },
        content: FilesystemContentRef {
            content_ref: "artifact:output".into(),
            encoding: Some("utf8".into()),
            expected_hash: None,
        },
        conflict_mode: FilesystemConflictMode::Fail,
        atomic: true,
    };
    assert!(write.has_safe_preconditions());
}
