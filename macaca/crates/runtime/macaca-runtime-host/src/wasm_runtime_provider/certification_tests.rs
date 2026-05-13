use macaca_proto::ApplicationHostCommandStatus;

use super::{
    WasmCertificationFixtureSet, WasmCertificationHarness, WasmCertificationProfile,
    WasmCertificationStatus, WasmHardenedProviderEnvelope, WasmHardenedProviderMockAdapter,
};

#[test]
fn wasm_certification_profiles_apply_increasingly_strict_readiness_gates() {
    let harness = WasmCertificationHarness::new("fixture.wasm_certification_profiles");
    let fixtures = WasmCertificationFixtureSet::conformance_matrix("fixture.wasm.certification");

    let dev = harness.certify_profile(WasmCertificationProfile::Dev, fixtures.clone());
    let default = harness.certify_profile(WasmCertificationProfile::Default, fixtures.clone());
    let hardened = harness.certify_profile(WasmCertificationProfile::Hardened, fixtures);

    assert_eq!(dev.profile, WasmCertificationProfile::Dev);
    assert_eq!(default.profile, WasmCertificationProfile::Default);
    assert_eq!(hardened.profile, WasmCertificationProfile::Hardened);
    assert!(dev.is_sanitized(), "{dev:?}");
    assert!(default.is_sanitized(), "{default:?}");
    assert!(hardened.is_sanitized(), "{hardened:?}");
    assert!(!dev.industrial_ready);
    assert!(!default.industrial_ready);
    assert!(hardened.industrial_ready, "{hardened:?}");
    assert_eq!(hardened.status, WasmCertificationStatus::Passed);
    assert!(hardened.evaluated_fixtures >= 4);
    assert!(hardened
        .trace_id
        .contains("fixture.wasm_certification_profiles"));
}

#[test]
fn wasm_certification_conformance_matrix_covers_required_fixture_shapes() {
    let fixtures = WasmCertificationFixtureSet::conformance_matrix("fixture.wasm.conformance");
    let names = fixtures.fixture_names();

    for expected in [
        "valid_minimal",
        "genui_render",
        "host_import_permission",
        "resource_exhausted",
        "abi_mismatch",
        "provider_unavailable",
    ] {
        assert!(names.iter().any(|name| name == expected), "{names:?}");
    }

    let report = WasmCertificationHarness::new("fixture.wasm_conformance")
        .certify_profile(WasmCertificationProfile::Default, fixtures);

    assert_eq!(report.status, WasmCertificationStatus::Passed);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "resource_exhaustion_detected"));
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "abi_mismatch_rejected"));
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "provider_unavailable_reported"));
}

#[test]
fn wasm_certification_hardened_profile_fails_security_negative_cases_without_leaks() {
    let report = WasmCertificationHarness::new("fixture.wasm_security_negative").certify_profile(
        WasmCertificationProfile::Hardened,
        WasmCertificationFixtureSet::security_negative_matrix("fixture.wasm.negative"),
    );
    let encoded = serde_json::to_string(&report).unwrap();

    assert_eq!(report.status, WasmCertificationStatus::Failed);
    assert!(!report.industrial_ready);
    for expected in [
        "raw_env_denied",
        "raw_filesystem_denied",
        "raw_network_denied",
        "missing_trace_denied",
        "missing_capability_denied",
        "oversized_payload_denied",
        "timeout_or_resource_exhaustion_denied",
    ] {
        assert!(
            report.reason_codes.iter().any(|code| code == expected),
            "{report:?}"
        );
    }
    assert!(report.is_sanitized(), "{report:?}");
    assert!(!encoded.contains("must-not-leak"));
    assert!(!encoded.contains("raw_payload"));
    assert!(!encoded.contains("api_key"));
}

#[test]
fn wasm_certification_hardened_provider_mock_adapter_preserves_trace_and_controls() {
    let adapter = WasmHardenedProviderMockAdapter::new("fixture.wasm.hardened.adapter");
    let envelope = WasmHardenedProviderEnvelope::new(
        "trace-wasm-hardened-adapter",
        "request-wasm-hardened-adapter",
        "session.create",
    )
    .timeout_ms(250)
    .cancellation_token("cancel-fixture")
    .backpressure_window(8)
    .diagnostics_level("sanitized");

    let response = adapter.dispatch(envelope);

    assert_eq!(response.trace_id, "trace-wasm-hardened-adapter");
    assert!(matches!(response.status, ApplicationHostCommandStatus::Ok));
    assert_eq!(
        response
            .metadata
            .get("provider_profile")
            .map(String::as_str),
        Some("hardened_out_of_process_mock")
    );
    assert_eq!(
        response.metadata.get("timeout_ms").map(String::as_str),
        Some("250")
    );
    assert!(response.is_sanitized(), "{response:?}");
}
