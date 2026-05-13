use serde_json::json;

use crate::{
    PackageRuntimeKind, TraceContext, WasmCompiledArtifactCacheKey, WasmEngineCapabilities,
    WasmExecutionProfile, WasmResourceAuditDecision, WasmResourceAuditReport, WasmResourcePolicy,
    WasmRuntimeArtifactRef, WasmRuntimeDiagnostics, WasmRuntimeErrorKind, WasmRuntimeErrorReport,
    WasmRuntimeProviderDescriptor, WasmRuntimeSessionRequest, WasmRuntimeUnavailableReason,
    WasmSandboxPolicy, WasmWasiPreopenGrant,
};

#[test]
fn wasm_runtime_descriptor_serializes_deterministically() {
    let descriptor = WasmRuntimeProviderDescriptor::unavailable_default();

    let encoded = serde_json::to_value(&descriptor).unwrap();

    assert_eq!(encoded["runtime_kind"], json!("wasm_component"));
    assert_eq!(encoded["provider_class"], json!("unavailable"));
    assert_eq!(encoded["capabilities"]["can_execute"], json!(false));
}

#[test]
fn wasm_runtime_diagnostics_redacts_forbidden_material() {
    let diagnostics = WasmRuntimeDiagnostics::new(
        PackageRuntimeKind::WasmComponent,
        "runtime_unavailable",
        "raw wasm bytes and API_KEY should be redacted",
        Some(TraceContext::new("trace-wasm-diagnostics")),
    );

    assert!(diagnostics.is_sanitized());
    assert!(!diagnostics.message.to_lowercase().contains("api_key"));
    assert!(!diagnostics.message.to_lowercase().contains("raw wasm"));
}

#[test]
fn wasm_runtime_session_request_requires_trace_and_identity() {
    let profile = WasmExecutionProfile::default_wasm_component();
    let artifact = WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm");
    let valid = WasmRuntimeSessionRequest::new(
        TraceContext::new("trace-session"),
        "fixture.application",
        "main",
        artifact,
        profile,
    )
    .unwrap();

    assert_eq!(valid.runtime_kind(), PackageRuntimeKind::WasmComponent);

    let missing_trace = WasmRuntimeSessionRequest {
        trace: None,
        application_id: "fixture.application".into(),
        ability_id: "main".into(),
        artifact: WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
        profile: WasmExecutionProfile::default_wasm_component(),
        metadata: Default::default(),
    };

    assert!(missing_trace.validate().is_err());
}

#[test]
fn wasm_runtime_unavailable_reason_is_structured() {
    let reason = WasmRuntimeUnavailableReason::provider_missing("no runtime provider");

    assert_eq!(reason.code, "provider_missing");
    assert_eq!(reason.message, "no runtime provider");
}

#[test]
fn wasm_runtime_engine_capabilities_are_provider_neutral() {
    let capabilities = WasmEngineCapabilities::unavailable();

    assert!(!capabilities.can_compile);
    assert!(!capabilities.can_instantiate);
    assert!(!capabilities.can_execute);
    assert!(capabilities.engine_features.is_empty());
}

#[test]
fn wasm_runtime_error_kind_uses_stable_reason_codes() {
    assert_eq!(
        WasmRuntimeErrorKind::CompileFailed.as_code(),
        "compile_failed"
    );
    assert_eq!(
        serde_json::to_value(WasmRuntimeErrorKind::ResourceExhausted).unwrap(),
        json!("resource_exhausted")
    );
}

#[test]
fn wasm_runtime_error_report_redacts_sensitive_runtime_text() {
    let report = WasmRuntimeErrorReport::new(
        WasmRuntimeErrorKind::CompileFailed,
        PackageRuntimeKind::WasmComponent,
        "raw wasm bytes leaked API_KEY and private key material",
        Some(TraceContext::new("trace-runtime-error")),
    );

    assert_eq!(report.reason_code, "compile_failed");
    assert_eq!(report.trace_id.as_deref(), Some("trace-runtime-error"));
    assert!(report.is_sanitized());
    assert!(!report.message.to_lowercase().contains("raw wasm"));
    assert!(!report.message.to_lowercase().contains("api_key"));
    assert!(!report.message.to_lowercase().contains("private key"));
}

#[test]
fn wasm_compiled_artifact_cache_key_is_deterministic_and_ordered() {
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert("profile_fingerprint".to_string(), "profile-v0".to_string());
    metadata.insert(
        "capability_fingerprint".to_string(),
        "capabilities-v1".to_string(),
    );

    let first = WasmCompiledArtifactCacheKey::new(
        "sha256",
        "abc123",
        "macaca-application-abi-v0",
        &metadata,
    );
    let second = WasmCompiledArtifactCacheKey::new(
        " SHA256 ",
        " abc123 ",
        " macaca-application-abi-v0 ",
        &metadata,
    );

    assert_eq!(first, second);
    assert_eq!(first.digest_algorithm, "sha256");
    assert_eq!(first.digest_value, "abc123");
    assert_eq!(first.abi_version, "macaca-application-abi-v0");
    assert_eq!(first.capability_fingerprint, "capabilities-v1");
    assert_eq!(first.profile_fingerprint, "profile-v0");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn wasm_resource_policy_merge_uses_stricter_limits_deterministically() {
    let mut platform = WasmResourcePolicy::default_for("fixture.application", "main");
    platform.max_payload_bytes = Some(65_536);
    platform.max_concurrent_sessions = Some(4);
    platform.metadata.insert("layer".into(), "platform".into());

    let mut deployment = WasmResourcePolicy::default_for("fixture.application", "main");
    deployment.max_payload_bytes = Some(32_768);
    deployment.max_concurrent_sessions = Some(8);
    deployment
        .metadata
        .insert("layer".into(), "deployment".into());

    let merged = WasmResourcePolicy::merge_deterministic(&[platform, deployment]);

    assert_eq!(merged.max_payload_bytes, Some(32_768));
    assert_eq!(merged.max_concurrent_sessions, Some(4));
    assert_eq!(
        merged.quota.unwrap().stable_label(),
        "fixture.application:main:session"
    );
    assert_eq!(
        merged.metadata.get("layer").map(String::as_str),
        Some("deployment")
    );
}

#[test]
fn wasm_sandbox_policy_denies_raw_wasi_by_default() {
    let policy = WasmSandboxPolicy::default();

    assert!(policy.denies_raw_wasi());
    assert!(!policy.allow_raw_env);
    assert!(!policy.allow_raw_filesystem);
    assert!(!policy.allow_raw_network);
}

#[test]
fn wasm_wasi_preopen_grant_uses_virtual_labels() {
    let grant = WasmWasiPreopenGrant::new("workspace.cache", "macaca:storage/read", "read-only");

    assert_eq!(grant.virtual_label, "workspace.cache");
    assert_eq!(grant.capability_scope, "macaca:storage:read");
    assert_eq!(grant.access, "read-only");
}

#[test]
fn wasm_resource_audit_report_redacts_raw_material() {
    let report = WasmResourceAuditReport::new(
        WasmResourceAuditDecision::Deny,
        "payload_too_large",
        PackageRuntimeKind::WasmComponent,
        "fixture.application",
        "main",
        "dispatch",
        Some(TraceContext::new("trace-resource-audit")),
        "raw payload and /Users/fixture/.env must not escape",
    );

    assert_eq!(report.reason_code, "payload_too_large");
    assert_eq!(report.trace_id.as_deref(), Some("trace-resource-audit"));
    assert!(report.is_sanitized());
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("/Users/fixture"));
}
