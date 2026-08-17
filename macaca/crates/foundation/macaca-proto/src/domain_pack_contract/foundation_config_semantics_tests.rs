//! Contract tests for configuration admission and redaction specifications.

use std::collections::BTreeSet;

use super::foundation_config::ConfigResultStatus;
use super::foundation_config_semantics::*;

fn context() -> ConfigPolicyContext {
    ConfigPolicyContext {
        declared_scopes: BTreeSet::from([
            "config.read".into(),
            "config.list".into(),
            "config.validate".into(),
            "config.watch".into(),
            "config.reload".into(),
            "config.snapshot".into(),
            "config.export".into(),
        ]),
        policy_allowed: true,
        provider_available: true,
        supports_watch: true,
        supports_reload: true,
        supports_redacted_export: true,
        secret_reference_available: true,
        approval_granted: true,
        test_override_allowed: true,
        limits: ConfigResourceLimits {
            max_key_units: 10,
            max_source_units: 4,
            max_watch_units: 2,
            max_reload_units: 1,
            max_export_units: 2,
            max_snapshot_units: 2,
            max_request_units: 4,
        },
        current: ConfigResourceReservation::default(),
    }
}

fn request() -> ConfigAdmissionRequest {
    ConfigAdmissionRequest {
        key_count: 1,
        source_count: 1,
        watch_units: 0,
        export_units: 0,
        snapshot_units: 0,
        has_valid_key: true,
        has_valid_schema: true,
        selector_supported: true,
        validation_passed: true,
        contains_raw_secret_value: false,
        uses_secret_reference: false,
        external_reload: false,
        broad_export: false,
        test_override: false,
        tenant_wide_change: false,
    }
}

#[test]
fn rejected_preflight_never_invokes_a_provider_closure() {
    let mut denied = context();
    denied.declared_scopes.clear();
    let mut called = false;
    assert_eq!(
        dispatch_after_preflight(preflight_command("config.get", request(), &denied), || {
            called = true;
        }),
        Err(ConfigAdmissionFailure::PermissionNotDeclared)
    );
    assert!(!called);

    let mut unavailable = context();
    unavailable.provider_available = false;
    assert_eq!(
        preflight_command("config.get", request(), &unavailable),
        Err(ConfigAdmissionFailure::ProviderUnavailable)
    );

    let mut quota = context();
    quota.current.request_units = quota.limits.max_request_units;
    assert_eq!(
        preflight_command("config.get", request(), &quota),
        Err(ConfigAdmissionFailure::QuotaExceeded)
    );
}

#[test]
fn approval_secret_and_validation_failures_are_stable_and_side_effect_free() {
    let mut approval = context();
    approval.approval_granted = false;
    let mut reload = request();
    reload.external_reload = true;
    assert_eq!(
        preflight_command("config.reload", reload, &approval),
        Err(ConfigAdmissionFailure::ApprovalRequired)
    );

    let mut secret = request();
    secret.contains_raw_secret_value = true;
    assert_eq!(
        preflight_command("config.validate", secret, &context()),
        Err(ConfigAdmissionFailure::SecretValueForbidden)
    );

    let mut invalid = request();
    invalid.validation_passed = false;
    let failure = preflight_command("config.validate", invalid, &context()).unwrap_err();
    assert_eq!(failure.status(), ConfigResultStatus::ValidationFailed);
}

#[test]
fn audit_projection_is_bounded_and_never_accepts_sensitive_field_names() {
    let event = redacted_config_audit_fields("config.get", "trace:config:1", 1, 1).unwrap();
    let serialized = serde_json::to_string(&event).unwrap();
    assert!(event.values_redacted);
    assert!(!serialized.contains("ui.theme"));
    assert!(redacted_config_audit_fields("config.get", "trace-secret", 1, 1).is_none());
}
