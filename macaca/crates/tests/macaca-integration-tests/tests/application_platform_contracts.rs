//! Application Platform certification contract tests.
//!
//! These tests certify generic application shapes without executing providers,
//! plugins, Store flows, Web3/EVM modules, UI renderers, or WASM bytes.  They
//! validate the platform-level contracts produced by `macaca-sdk` fixtures and
//! consumed by `macaca-app` certification rules.

use macaca_app::{
    ApplicationCertificationContext, ApplicationCertificationFixture, ApplicationCertificationKit,
    ApplicationCertificationStatus,
};
use macaca_proto::{KernelServiceId, PackageRuntimeKind, WasmEngineCapabilities};
use macaca_sdk::{
    application_platform_agent_fixture, application_platform_genui_fixture,
    application_platform_headless_fixture, application_platform_plugin_enhanced_fixture,
    application_platform_store_entitled_fixture, application_platform_wasm_skeleton_fixture,
    ApplicationPlatformFixture,
};

fn available_context() -> ApplicationCertificationContext {
    ApplicationCertificationContext::new()
        .trace_id("trace-application-platform-certification")
        .runtime(PackageRuntimeKind::Yaml)
        .runtime(PackageRuntimeKind::WasmComponent)
        .wasm_runtime_capabilities(executable_wasm_capabilities())
        .service(KernelServiceId::new("service.fixture.task"))
        .service(KernelServiceId::new("service.fixture.ui"))
        .service(KernelServiceId::new("service.fixture.entitlement"))
        .service(KernelServiceId::new("service.fixture.plugin"))
        .plugin("plugin.fixture.platform")
}

fn to_certification_fixture(
    fixture: ApplicationPlatformFixture,
) -> ApplicationCertificationFixture {
    let mut certification =
        ApplicationCertificationFixture::new(fixture.fixture_id, fixture.manifest);
    if let Some(abi) = fixture.abi {
        certification = certification.abi(abi);
    }
    if let Some(artifact) = fixture.wasm_artifact {
        certification = certification.wasm_artifact(artifact);
    }
    certification
}

fn executable_wasm_capabilities() -> WasmEngineCapabilities {
    let mut capabilities = WasmEngineCapabilities::unavailable();
    capabilities.can_compile = true;
    capabilities.can_instantiate = true;
    capabilities.can_execute = true;
    capabilities.supports_component_model = true;
    capabilities.supports_host_import_bridge = true;
    capabilities
}

#[test]
fn application_platform_fixtures_certify_all_supported_shapes() {
    let kit = ApplicationCertificationKit;
    let context = available_context();
    for fixture in [
        application_platform_agent_fixture(),
        application_platform_genui_fixture(),
        application_platform_headless_fixture(),
        application_platform_store_entitled_fixture(),
        application_platform_plugin_enhanced_fixture(),
        application_platform_wasm_skeleton_fixture(),
    ] {
        let report = kit.certify(&to_certification_fixture(fixture), &context);
        assert!(report.is_success(), "{report:?}");
        assert!(!report.ability_ids.is_empty());
        assert_eq!(report.operation, "application.certify");
        assert_eq!(
            report.trace_id.as_deref(),
            Some("trace-application-platform-certification")
        );
    }
}

#[test]
fn certification_fails_closed_for_missing_service_and_plugin() {
    let kit = ApplicationCertificationKit;
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-application-platform-missing-dependency")
        .runtime(PackageRuntimeKind::Yaml);

    let report = kit.certify(
        &to_certification_fixture(application_platform_plugin_enhanced_fixture()),
        &context,
    );

    assert_eq!(report.status, ApplicationCertificationStatus::Failed);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "required_service_unavailable"));
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "required_plugin_unavailable"));
}

#[test]
fn certification_reports_wasm_runtime_unavailable_without_execution() {
    let kit = ApplicationCertificationKit;
    let context = ApplicationCertificationContext::new()
        .trace_id("trace-application-platform-wasm-unavailable");

    let report = kit.certify(
        &to_certification_fixture(application_platform_wasm_skeleton_fixture()),
        &context,
    );

    assert_eq!(report.status, ApplicationCertificationStatus::Unavailable);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "runtime_unavailable" || code == "wasm_runtime_unavailable"));
}

#[test]
fn certification_report_is_redacted_and_rejects_unsafe_metadata_keys() {
    let kit = ApplicationCertificationKit;
    let mut fixture = application_platform_agent_fixture();
    fixture
        .manifest
        .metadata
        .insert("secret_token".into(), "must-not-leak".into());
    let report = kit.certify(&to_certification_fixture(fixture), &available_context());
    let encoded = serde_json::to_string(&report).unwrap();

    assert_eq!(report.status, ApplicationCertificationStatus::Failed);
    assert!(report
        .reason_codes
        .iter()
        .any(|code| code == "unsafe_metadata_key"));
    assert!(!encoded.contains("must-not-leak"));
    assert!(!encoded.contains("raw_manifest"));
    assert!(!encoded.contains("api_key"));
}
